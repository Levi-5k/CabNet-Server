use crate::config::get_data_dir;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Cloudflare tunnel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelConfig {
    pub tunnel_name: String,
    pub domain: String,
    pub subdomain: String,
    pub enabled: bool,
}

impl TunnelConfig {
    /// Get the config file path
    fn config_path() -> PathBuf {
        get_data_dir().join("tunnel_config.json")
    }

    /// Load config from file
    pub fn load() -> Option<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        Ok(())
    }

    /// Check if tunnel is configured
    pub fn is_configured(&self) -> bool {
        self.enabled && (!self.tunnel_name.is_empty() || tunnel_token().is_some()) && !self.domain.is_empty()
    }

    /// Get the full domain URL
    pub fn full_domain(&self) -> String {
        let subdomain = if self.subdomain.is_empty() { "scans" } else { &self.subdomain };
        format!("{}.{}", subdomain, self.domain)
    }
}

/// Tunnel status
#[derive(Debug, Clone, PartialEq)]
pub enum TunnelStatus {
    Stopped,
    Starting,
    Running,
    Failed(String),
}

impl Default for TunnelStatus {
    fn default() -> Self {
        TunnelStatus::Stopped
    }
}

/// Manages the Cloudflare tunnel process
pub struct TunnelManager {
    config: TunnelConfig,
    status: TunnelStatus,
    process: Option<Child>,
    port: u16,
}

impl TunnelManager {
    pub fn new(port: u16) -> Self {
        let config = TunnelConfig::load().unwrap_or_default();
        Self {
            config,
            status: TunnelStatus::Stopped,
            process: None,
            port,
        }
    }

    pub fn config(&self) -> &TunnelConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut TunnelConfig {
        &mut self.config
    }

    pub fn status(&self) -> &TunnelStatus {
        &self.status
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, TunnelStatus::Running)
    }

    /// Start the tunnel
    pub fn start(&mut self) -> Result<(), String> {
        let token = tunnel_token();
        let has_named_tunnel = self.config.enabled && !self.config.tunnel_name.is_empty();

        if token.is_none() && !has_named_tunnel {
            return Err("Cloudflare tunnel is required but not configured. Set TUNNEL_TOKEN or configure a named tunnel in CabNet Server settings.".to_string());
        }

        if token.is_none() && cloudflared_origin_cert_path().is_none() {
            let msg = "Cloudflare login is required before starting a named tunnel. Run cloudflared tunnel login in CabNet Server settings so cloudflared can create ~/.cloudflared/cert.pem.".to_string();
            self.status = TunnelStatus::Failed(msg.clone());
            return Err(msg);
        }

        let token = if token.is_none() && cloudflared_tunnel_credentials_paths().is_empty() {
            Some(fetch_tunnel_token(&self.config.tunnel_name)?)
        } else {
            token
        };

        if self.is_running() {
            return Ok(()); // Already running
        }

        self.status = TunnelStatus::Starting;

        let mut command = Command::new("cloudflared");
        command
            .args(["tunnel", "--no-autoupdate", "run"])
            .arg("--url")
            .arg(format!("http://localhost:{}", self.port));

        if let Some(token) = token {
            command.env("TUNNEL_TOKEN", token);
        } else {
            command.arg(&self.config.tunnel_name);
        }

        let result = command
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn();

        match result {
            Ok(mut child) => {
                thread::sleep(Duration::from_millis(800));
                if let Ok(Some(status)) = child.try_wait() {
                    let msg = format!("cloudflared exited during startup with status {}", status);
                    self.status = TunnelStatus::Failed(msg.clone());
                    return Err(msg);
                }

                self.process = Some(child);
                self.status = TunnelStatus::Running;
                tracing::info!("Cloudflare tunnel started for {}", self.config.full_domain());
                Ok(())
            }
            Err(e) => {
                let msg = format!("Failed to start tunnel: {}", e);
                self.status = TunnelStatus::Failed(msg.clone());
                Err(msg)
            }
        }
    }

    /// Stop the tunnel
    pub fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            tracing::info!("Cloudflare tunnel stopped");
        }
        self.status = TunnelStatus::Stopped;
    }

    /// Save the current configuration
    pub fn save_config(&self) -> Result<(), String> {
        self.config.save()
    }

    /// Check if the tunnel process is still alive
    pub fn check_status(&mut self) {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process has exited
                    if status.success() {
                        self.status = TunnelStatus::Stopped;
                    } else {
                        self.status = TunnelStatus::Failed("Tunnel process exited unexpectedly".to_string());
                    }
                    self.process = None;
                }
                Ok(None) => {
                    // Still running
                    self.status = TunnelStatus::Running;
                }
                Err(e) => {
                    self.status = TunnelStatus::Failed(format!("Error checking tunnel status: {}", e));
                }
            }
        }
    }
}

impl Drop for TunnelManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Thread-safe wrapper for TunnelManager
pub type SharedTunnelManager = Arc<Mutex<TunnelManager>>;

pub fn create_tunnel_manager(port: u16) -> SharedTunnelManager {
    Arc::new(Mutex::new(TunnelManager::new(port)))
}

fn tunnel_token() -> Option<String> {
    env::var("TUNNEL_TOKEN")
        .or_else(|_| env::var("CLOUDFLARED_TOKEN"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn cloudflared_origin_cert_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("TUNNEL_ORIGIN_CERT") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let mut paths = Vec::new();
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        paths.push(home.join(".cloudflared/cert.pem"));
        paths.push(home.join(".cloudflare-warp/cert.pem"));
        paths.push(home.join("cloudflare-warp/cert.pem"));
    }
    paths.push(PathBuf::from("/etc/cloudflared/cert.pem"));
    paths.push(PathBuf::from("/usr/local/etc/cloudflared/cert.pem"));

    paths.into_iter().find(|path| path.is_file())
}

pub fn cloudflared_tunnel_credentials_paths() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        directories.push(home.join(".cloudflared"));
        directories.push(home.join(".cloudflare-warp"));
        directories.push(home.join("cloudflare-warp"));
    }
    directories.push(PathBuf::from("/etc/cloudflared"));
    directories.push(PathBuf::from("/usr/local/etc/cloudflared"));

    let mut paths = Vec::new();
    for directory in directories {
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|extension| extension == "json") {
                    paths.push(path);
                }
            }
        }
    }
    paths
}

fn fetch_tunnel_token(tunnel_name: &str) -> Result<String, String> {
    let output = Command::new("cloudflared")
        .args(["tunnel", "token", tunnel_name])
        .output()
        .map_err(|error| format!("Failed to fetch Cloudflare tunnel token: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.lines().next().unwrap_or("cloudflared tunnel token failed");
        return Err(format!(
            "Cloudflare tunnel '{}' exists but CabNet could not fetch a run token: {}",
            tunnel_name, message
        ));
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Err(format!(
            "Cloudflare tunnel '{}' exists but cloudflared returned an empty run token",
            tunnel_name
        ));
    }

    Ok(token)
}

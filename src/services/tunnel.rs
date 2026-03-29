use crate::config::get_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

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
        self.enabled && !self.tunnel_name.is_empty() && !self.domain.is_empty()
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
        if !self.config.is_configured() {
            return Err("Tunnel not configured".to_string());
        }

        if self.is_running() {
            return Ok(()); // Already running
        }

        self.status = TunnelStatus::Starting;

        // Build the command with PATH refresh for Windows
        let tunnel_cmd = format!(
            "cloudflared tunnel run --url http://localhost:{} {}",
            self.port, self.config.tunnel_name
        );

        // Start cloudflared as a background process
        let result = Command::new("powershell")
            .args([
                "-WindowStyle", "Hidden",
                "-Command",
                &format!(
                    "$env:Path = [System.Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [System.Environment]::GetEnvironmentVariable('Path','User'); {}",
                    tunnel_cmd
                )
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match result {
            Ok(child) => {
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

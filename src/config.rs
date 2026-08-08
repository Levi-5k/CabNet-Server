use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

/// Get the data directory in Windows Documents folder
pub fn get_data_dir() -> PathBuf {
    // Use Documents/CabNet folder on Windows
    if let Some(docs_dir) = dirs::document_dir() {
        let data_dir = docs_dir.join("CabNet");
        if fs::create_dir_all(&data_dir).is_ok() {
            return data_dir;
        }
    }
    
    // Fallback to AppData/Local/CabNet
    if let Some(local_data) = dirs::data_local_dir() {
        let data_dir = local_data.join("CabNet");
        if fs::create_dir_all(&data_dir).is_ok() {
            return data_dir;
        }
    }
    
    // Last resort: current directory
    let data_dir = PathBuf::from("./data");
    fs::create_dir_all(&data_dir).ok();
    data_dir
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub email: Option<EmailConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub auto_send_enabled: bool,
    pub auto_send_delay_minutes: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            database: DatabaseConfig {
                path: get_data_dir().join("codebar.db"),
            },
            email: None,
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        load_dotenv_files();

        let mut config = Config::default();

        if let Ok(host) = std::env::var("SERVER_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("SERVER_PORT") {
            config.server.port = port.parse().unwrap_or(8080);
        }
        if let Ok(path) = std::env::var("DATABASE_PATH") {
            config.database.path = PathBuf::from(path);
        }

        // Load email config if all required fields are present
        if let (Ok(smtp_host), Ok(smtp_username), Ok(smtp_password), Ok(from_address)) = (
            std::env::var("SMTP_HOST"),
            std::env::var("SMTP_USERNAME"),
            std::env::var("SMTP_PASSWORD"),
            std::env::var("EMAIL_FROM"),
        ) {
            config.email = Some(EmailConfig {
                smtp_host,
                smtp_port: std::env::var("SMTP_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(587),
                smtp_username,
                smtp_password,
                from_address,
                to_addresses: std::env::var("EMAIL_TO")
                    .ok()
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                auto_send_enabled: std::env::var("AUTO_SEND_ENABLED")
                    .ok()
                    .map(|s| s == "true" || s == "1")
                    .unwrap_or(false),
                auto_send_delay_minutes: std::env::var("AUTO_SEND_DELAY_MINUTES")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30),
            });
        }

        Ok(config)
    }

    /// Get the server address as a string
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Get the database URL for SQLx
    pub fn database_url(&self) -> String {
        format!("sqlite:{}", self.database.path.display())
    }
}

fn load_dotenv_files() {
    let _ = dotenvy::dotenv();

    let manifest_env = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    if manifest_env.exists() {
        let _ = dotenvy::from_path(manifest_env);
    }

    if let Ok(executable_path) = std::env::current_exe() {
        if let Some(executable_dir) = executable_path.parent() {
            let executable_env = executable_dir.join(".env");
            if executable_env.exists() {
                let _ = dotenvy::from_path(executable_env);
            }
        }
    }
}

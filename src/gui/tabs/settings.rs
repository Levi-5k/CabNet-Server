use eframe::egui;
use std::process::Command;
use crate::config::get_data_dir;
use crate::services::{SharedTunnelManager, TunnelStatus};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Perform a database backup — copies codebar.db to backups/codebar_YYYY-MM-DD.db
/// Returns Ok(path) on success or Err(message) on failure
pub fn backup_database() -> Result<String, String> {
    let data_dir = get_data_dir();
    let db_path = data_dir.join("codebar.db");
    
    if !db_path.exists() {
        return Err("Database file not found".to_string());
    }
    
    let backup_dir = data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;
    
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let backup_name = format!("codebar_{}.db", today);
    let backup_path = backup_dir.join(&backup_name);
    
    std::fs::copy(&db_path, &backup_path)
        .map_err(|e| format!("Failed to copy database: {}", e))?;
    
    let size = std::fs::metadata(&backup_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    tracing::info!("Database backed up to {} ({:.1} MB)", backup_path.display(), size as f64 / 1_048_576.0);
    Ok(backup_path.display().to_string())
}

/// Check if today's backup already exists
pub fn has_todays_backup() -> bool {
    let data_dir = get_data_dir();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    data_dir.join("backups").join(format!("codebar_{}.db", today)).exists()
}

/// Run auto-backup on startup if enabled (checks if today's backup exists)
pub fn auto_backup_on_startup() {
    if has_todays_backup() {
        tracing::info!("Daily backup already exists for today, skipping");
        return;
    }
    match backup_database() {
        Ok(path) => tracing::info!("Auto daily backup created: {}", path),
        Err(e) => tracing::warn!("Auto daily backup failed: {}", e),
    }
}

/// List existing backups, newest first
pub fn list_backups() -> Vec<(String, u64)> {
    let backup_dir = get_data_dir().join("backups");
    let mut backups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&backup_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("codebar_") && name.ends_with(".db") {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                backups.push((name, size));
            }
        }
    }
    backups.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
    backups
}

// Static state for async CSV import communication with egui
static CSV_IMPORT_DONE: AtomicBool = AtomicBool::new(false);
static CSV_IMPORT_SUCCESS: AtomicBool = AtomicBool::new(true);
static CSV_IMPORT_COUNT: AtomicUsize = AtomicUsize::new(0);
static CSV_IMPORT_ERR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Parse a CSV file and import scans into the database
async fn parse_and_import_csv(
    path: &std::path::Path,
    state: &crate::api::routes::SharedState,
) -> anyhow::Result<usize> {
    use crate::db::models::ScanInput;

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .from_path(path)?;

    let headers = rdr.headers()?.clone();
    let header_names: Vec<String> = headers.iter().map(|h| h.trim().to_lowercase()).collect();

    // Detect format by checking headers
    let is_full_format = header_names.contains(&"barcode data".to_string())
        || header_names.contains(&"barcode_data".to_string())
        || header_names.contains(&"device id".to_string())
        || header_names.contains(&"device_id".to_string());

    let state_guard = state.read().await;
    let repo = &state_guard.repo;
    let mut count = 0usize;

    for result in rdr.records() {
        let record = result?;
        let get = |name: &str| -> Option<String> {
            // Try exact match first, then with underscore/space variants
            let name_lower = name.to_lowercase();
            for (i, h) in header_names.iter().enumerate() {
                if h == &name_lower
                    || h == &name_lower.replace(' ', "_")
                    || h == &name_lower.replace('_', " ")
                {
                    let val = record.get(i).unwrap_or("").trim().to_string();
                    if val.is_empty() { return None; }
                    return Some(val);
                }
            }
            None
        };

        let scan_input = if is_full_format {
            // Full format: ID, Ticket Number, Barcode Data, Barcode Type, Job ID, Device ID, User ID, Location, Latitude, Longitude, Timestamp, Synced
            let barcode = match get("barcode data").or_else(|| get("barcode_data")) {
                Some(b) => b,
                None => continue, // skip rows with no barcode
            };
            let timestamp = get("timestamp")
                .and_then(|t| t.parse::<i64>().ok())
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

            ScanInput {
                id: get("id").unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                barcode_data: barcode,
                barcode_type: get("barcode type").or_else(|| get("barcode_type")),
                ticket_number: get("ticket number").or_else(|| get("ticket_number")),
                barcode_job_ref: None,
                job_id: get("job id").or_else(|| get("job_id")),
                device_id: get("device id").or_else(|| get("device_id")).unwrap_or_else(|| "csv-import".to_string()),
                user_id: get("user id").or_else(|| get("user_id")),
                location: get("location"),
                latitude: get("latitude").and_then(|v| v.parse().ok()),
                longitude: get("longitude").and_then(|v| v.parse().ok()),
                scanned_at: timestamp,
                is_printed: None,
                local_id: None,
            }
        } else {
            // Simple format: Ticket Number, Job Name, GPS Location, Date
            let ticket = get("ticket number").or_else(|| get("ticket_number"));
            let barcode = match &ticket {
                Some(t) => t.clone(),
                None => continue, // skip rows with no ticket/barcode
            };
            let timestamp = get("date")
                .and_then(|d| {
                    chrono::NaiveDateTime::parse_from_str(&d, "%Y-%m-%d %H:%M:%S")
                        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&d, "%m/%d/%Y %H:%M:%S"))
                        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&d, "%Y-%m-%d %H:%M"))
                        .ok()
                })
                .map(|dt| dt.and_utc().timestamp_millis())
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

            // Try to parse GPS location "lat,lng"
            let (lat, lng) = get("gps location")
                .or_else(|| get("gps_location"))
                .and_then(|loc| {
                    let parts: Vec<&str> = loc.split(',').collect();
                    if parts.len() == 2 {
                        let lat: f64 = parts[0].trim().parse().ok()?;
                        let lng: f64 = parts[1].trim().parse().ok()?;
                        Some((Some(lat), Some(lng)))
                    } else {
                        None
                    }
                })
                .unwrap_or((None, None));

            ScanInput {
                id: uuid::Uuid::new_v4().to_string(),
                barcode_data: barcode,
                barcode_type: None,
                ticket_number: ticket,
                barcode_job_ref: get("job name").or_else(|| get("job_name")),
                job_id: None,
                device_id: "csv-import".to_string(),
                user_id: None,
                location: get("gps location").or_else(|| get("gps_location")),
                latitude: lat,
                longitude: lng,
                scanned_at: timestamp,
                is_printed: None,
                local_id: None,
            }
        };

        if let Err(e) = repo.upsert_scan(&scan_input).await {
            tracing::warn!("CSV import: failed to upsert scan: {}", e);
        } else {
            count += 1;
            CSV_IMPORT_COUNT.store(count, Ordering::Relaxed);
        }
    }

    Ok(count)
}

/// Command execution status
#[derive(Default, Clone)]
pub struct CommandStatus {
    pub running: bool,
    pub success: Option<bool>,
    pub output: String,
}

/// Settings tab for application configuration
#[derive(Default)]
pub struct SettingsTab {
    pub server_port: String,
    pub server_host: String,
    pub database_path: String,
    pub auto_backup: bool,
    pub dark_mode: bool,
    // Cloudflare Tunnel setup
    pub show_cloudflare_setup: bool,
    pub cloudflare_step: usize,
    pub cloudflare_domain: String,
    pub cloudflare_subdomain: String,
    pub cloudflare_tunnel_name: String,
    pub cloudflare_tunnel_id: String,
    pub cloudflare_installed: bool,
    pub cloudflare_logged_in: bool,
    pub cloudflare_tunnel_created: bool,
    pub cloudflare_dns_configured: bool,
    // Command execution states
    pub step1_running: bool,
    pub step1_status: CommandStatus,
    pub step2_running: bool,
    pub step2_status: CommandStatus,
    pub step3_running: bool,
    pub step3_status: CommandStatus,
    pub step4_running: bool,
    pub step4_status: CommandStatus,
    // Legacy field for backwards compatibility
    pub cmd_status: CommandStatus,
    pub creating_tunnel: bool,
    // Backup status message
    pub backup_status: Option<(bool, String)>,  // (success, message)
    // CSV Import state
    pub csv_import_status: Option<(bool, String)>,
    pub csv_importing: bool,
    pub csv_import_count: usize,
    pub needs_refresh: bool,
}

impl SettingsTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, server_addr: &str, tunnel_manager: &SharedTunnelManager, state: &crate::api::routes::SharedState, runtime: &tokio::runtime::Handle) -> bool {
        self.needs_refresh = false;
        ui.heading("Settings");
        ui.separator();

        // Get tunnel status
        let (tunnel_running, tunnel_domain) = {
            if let Ok(mut manager) = tunnel_manager.lock() {
                manager.check_status();
                (manager.is_running(), manager.config().full_domain())
            } else {
                (false, String::new())
            }
        };

        // Cloudflare Tunnel Status Section
        ui.add_space(5.0);
        if tunnel_running {
            // Show tunnel running status
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.2))
                .rounding(egui::Rounding::same(12.0))
                .inner_margin(egui::Margin::same(16.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").size(16.0).color(egui::Color32::from_rgb(34, 197, 94)));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Tunnel Active").size(16.0).strong().color(egui::Color32::from_rgb(34, 197, 94)));
                            ui.label(
                                egui::RichText::new(format!("https://{}", tunnel_domain))
                                    .size(14.0)
                                    .color(egui::Color32::WHITE)
                            );
                        });
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let stop_btn = egui::Button::new(
                                egui::RichText::new("Stop Tunnel")
                                    .color(egui::Color32::WHITE)
                            )
                            .fill(egui::Color32::from_rgb(239, 68, 68))
                            .rounding(egui::Rounding::same(6.0));
                            
                            if ui.add(stop_btn).clicked() {
                                if let Ok(mut manager) = tunnel_manager.lock() {
                                    manager.stop();
                                }
                            }
                            
                            ui.add_space(8.0);
                            
                            let setup_btn = egui::Button::new("⚙ Setup")
                                .rounding(egui::Rounding::same(6.0));
                            
                            if ui.add(setup_btn).clicked() {
                                self.show_cloudflare_setup = true;
                            }
                        });
                    });
                });
        } else {
            // Show setup button
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(45, 55, 75))
                .rounding(egui::Rounding::same(12.0))
                .inner_margin(egui::Margin::same(16.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("☁").size(24.0));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Internet Access").size(16.0).strong());
                            ui.label(
                                egui::RichText::new("Set up Cloudflare Tunnel to receive scans from anywhere")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(148, 163, 184))
                            );
                        });
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn = egui::Button::new(
                                egui::RichText::new("Setup Cloudflare Tunnel")
                                    .color(egui::Color32::WHITE)
                            )
                            .fill(egui::Color32::from_rgb(249, 115, 22))
                            .rounding(egui::Rounding::same(6.0));
                            
                            if ui.add(btn).clicked() {
                                self.show_cloudflare_setup = true;
                                if self.cloudflare_tunnel_name.is_empty() {
                                    self.cloudflare_tunnel_name = "cabnet".to_string();
                                }
                                if self.cloudflare_subdomain.is_empty() {
                                    self.cloudflare_subdomain = "scans".to_string();
                                }
                            }
                        });
                    });
                });
        }

        ui.add_space(10.0);

        // Show Cloudflare setup dialog if open
        if self.show_cloudflare_setup {
            self.show_cloudflare_setup_dialog(ui, server_addr, tunnel_manager);
        }

        // Server section
        ui.collapsing("🌐 Server", |ui| {
            ui.horizontal(|ui| {
                ui.label("Current address:");
                ui.monospace(format!("http://{}", server_addr));
            });

            ui.add_space(5.0);
            ui.label("⚠️ Server settings require restart to take effect.");

            egui::Grid::new("server_settings")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Host:");
                    if self.server_host.is_empty() {
                        self.server_host = "0.0.0.0".to_string();
                    }
                    ui.text_edit_singleline(&mut self.server_host);
                    ui.end_row();

                    ui.label("Port:");
                    if self.server_port.is_empty() {
                        self.server_port = "8080".to_string();
                    }
                    ui.text_edit_singleline(&mut self.server_port);
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Database section
        ui.collapsing("🗄 Database", |ui| {
            egui::Grid::new("database_settings")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Path:");
                    if self.database_path.is_empty() {
                        self.database_path = get_data_dir().join("codebar.db").display().to_string();
                    }
                    ui.text_edit_singleline(&mut self.database_path);
                    ui.end_row();

                    ui.label("Auto backup:");
                    ui.checkbox(&mut self.auto_backup, "Enable daily backups");
                    ui.end_row();
                });

            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if ui.button("Backup Now").clicked() {
                    match backup_database() {
                        Ok(path) => {
                            self.backup_status = Some((true, format!("✅ Backup saved to {}", path)));
                        }
                        Err(e) => {
                            self.backup_status = Some((false, format!("❌ Backup failed: {}", e)));
                        }
                    }
                }
                if ui.button("Open Backup Folder").clicked() {
                    let backup_dir = get_data_dir().join("backups");
                    std::fs::create_dir_all(&backup_dir).ok();
                    let _ = std::process::Command::new("explorer").arg(&backup_dir).spawn();
                }
            });

            // Show backup status message
            if let Some((success, msg)) = &self.backup_status {
                ui.add_space(4.0);
                let color = if *success {
                    egui::Color32::from_rgb(34, 197, 94)
                } else {
                    egui::Color32::from_rgb(239, 68, 68)
                };
                ui.label(egui::RichText::new(msg).size(12.0).color(color));
            }

            // Show existing backups
            let backups = list_backups();
            if !backups.is_empty() {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(format!("Backups ({})", backups.len())).strong().size(12.0));
                for (name, size) in backups.iter().take(5) {
                    ui.label(
                        egui::RichText::new(format!("  {} ({:.1} MB)", name, *size as f64 / 1_048_576.0))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(160, 160, 190))
                    );
                }
                if backups.len() > 5 {
                    ui.label(
                        egui::RichText::new(format!("  ...and {} more", backups.len() - 5))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(130, 130, 160))
                    );
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(egui::RichText::new("CSV Import").strong().size(13.0));
            ui.add_space(4.0);

            let is_importing = self.csv_importing;
            ui.horizontal(|ui| {
                let btn = egui::Button::new(if is_importing { "⏳ Importing..." } else { "📂 Import CSV" });
                if ui.add_enabled(!is_importing, btn).clicked() {
                    // Open file dialog (blocking on native thread)
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV Files", &["csv"])
                        .set_title("Select CSV to import")
                        .pick_file()
                    {
                        self.csv_importing = true;
                        self.csv_import_status = None;
                        self.csv_import_count = 0;

                        let state_clone = state.clone();
                        let ctx = ui.ctx().clone();

                        // Reset static counters
                        CSV_IMPORT_DONE.store(false, Ordering::Relaxed);
                        CSV_IMPORT_SUCCESS.store(true, Ordering::Relaxed);
                        CSV_IMPORT_COUNT.store(0, Ordering::Relaxed);
                        if let Ok(mut em) = CSV_IMPORT_ERR.lock() { *em = None; }

                        runtime.spawn(async move {
                            match parse_and_import_csv(&path, &state_clone).await {
                                Ok(n) => {
                                    CSV_IMPORT_COUNT.store(n, Ordering::Relaxed);
                                    CSV_IMPORT_SUCCESS.store(true, Ordering::Relaxed);
                                }
                                Err(e) => {
                                    CSV_IMPORT_SUCCESS.store(false, Ordering::Relaxed);
                                    if let Ok(mut em) = CSV_IMPORT_ERR.lock() {
                                        *em = Some(e.to_string());
                                    }
                                }
                            }
                            CSV_IMPORT_DONE.store(true, Ordering::Relaxed);
                            ctx.request_repaint();
                        });
                    }
                }
            });

            // Poll static state for async import completion
            if self.csv_importing {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                let current_count = CSV_IMPORT_COUNT.load(Ordering::Relaxed);
                if current_count > 0 {
                    ui.label(
                        egui::RichText::new(format!("  Imported {} scans so far...", current_count))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(96, 165, 250))
                    );
                }
                if CSV_IMPORT_DONE.load(Ordering::Relaxed) {
                    self.csv_importing = false;
                    let count = CSV_IMPORT_COUNT.load(Ordering::Relaxed);
                    let ok = CSV_IMPORT_SUCCESS.load(Ordering::Relaxed);
                    if ok {
                        self.csv_import_status = Some((true, format!("✅ Successfully imported {} scans", count)));
                        self.csv_import_count = count;
                        self.needs_refresh = true;
                    } else {
                        let msg = CSV_IMPORT_ERR.lock().ok().and_then(|m| m.clone()).unwrap_or_default();
                        self.csv_import_status = Some((false, format!("❌ Import failed: {}", msg)));
                    }
                }
            }

            // Show CSV import status
            if let Some((success, msg)) = &self.csv_import_status {
                ui.add_space(4.0);
                let color = if *success {
                    egui::Color32::from_rgb(34, 197, 94)
                } else {
                    egui::Color32::from_rgb(239, 68, 68)
                };
                ui.label(egui::RichText::new(msg).size(12.0).color(color));
            }
        });

        ui.add_space(10.0);

        // Appearance section
        ui.collapsing("🎨 Appearance", |ui| {
            ui.horizontal(|ui| {
                ui.label("Theme:");
                if ui.button("Light").clicked() {
                    ui.ctx().set_visuals(egui::Visuals::light());
                }
                if ui.button("Dark").clicked() {
                    ui.ctx().set_visuals(egui::Visuals::dark());
                }
            });
        });

        ui.add_space(10.0);

        // About section
        ui.collapsing("ℹ️ About", |ui| {
            ui.label(format!("CabNet Server v{}", env!("CARGO_PKG_VERSION")));
            ui.label("A Rust desktop application for managing barcode scans");
            ui.label("from Android devices.");
            ui.add_space(5.0);
            ui.hyperlink_to("Documentation", "https://github.com/example/cabnet-server");
        });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save Settings").clicked() {
                    tracing::info!("Settings saved");
                    // TODO: Save to database/file
                }
                if ui.button("Reset to Defaults").clicked() {
                    self.server_host = "0.0.0.0".to_string();
                    self.server_port = "8080".to_string();
                    self.database_path = get_data_dir().join("codebar.db").display().to_string();
                    self.auto_backup = true;
                }
            });
        });

        self.needs_refresh
    }

    fn show_cloudflare_setup_dialog(&mut self, ui: &mut egui::Ui, server_addr: &str, tunnel_manager: &SharedTunnelManager) {
        let port = server_addr.split(':').last().unwrap_or("8080");
        
        egui::Window::new("☁ Cloudflare Tunnel Setup")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([650.0, 580.0])
            .frame(egui::Frame::window(&ui.ctx().style())
                .fill(egui::Color32::from_rgb(30, 30, 46))
                .rounding(egui::Rounding::same(16.0))
                .inner_margin(egui::Margin::same(24.0)))
            .show(ui.ctx(), |ui| {
                // Header
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Connect CabNet to the Internet").size(20.0).strong());
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Follow these steps to access CabNet from anywhere")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(148, 163, 184))
                    );
                });

                ui.add_space(16.0);

                // Progress indicator
                ui.horizontal(|ui| {
                    for step in 0..5 {
                        let is_current = self.cloudflare_step == step;
                        let is_done = self.cloudflare_step > step;
                        
                        let color = if is_done {
                            egui::Color32::from_rgb(34, 197, 94) // Green
                        } else if is_current {
                            egui::Color32::from_rgb(249, 115, 22) // Orange
                        } else {
                            egui::Color32::from_rgb(75, 85, 99) // Gray
                        };
                        
                        let text = if is_done { "✓" } else { &format!("{}", step + 1) };
                        
                        ui.add(
                            egui::Button::new(egui::RichText::new(text).color(egui::Color32::WHITE).size(14.0))
                                .fill(color)
                                .rounding(egui::Rounding::same(20.0))
                                .min_size(egui::vec2(32.0, 32.0))
                        );
                        
                        if step < 4 {
                            ui.add(egui::Separator::default().horizontal().spacing(8.0));
                        }
                    }
                });

                ui.add_space(20.0);

                // Step content
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(45, 45, 65))
                    .rounding(egui::Rounding::same(12.0))
                    .inner_margin(egui::Margin::same(20.0))
                    .show(ui, |ui| {
                        match self.cloudflare_step {
                            0 => self.step_install_cloudflared(ui),
                            1 => self.step_login_cloudflare(ui),
                            2 => self.step_create_tunnel(ui),
                            3 => self.step_configure_domain(ui, port),
                            4 => self.step_run_tunnel(ui, port, tunnel_manager),
                            _ => {}
                        }
                    });

                ui.add_space(16.0);

                // Navigation buttons
                ui.horizontal(|ui| {
                    // Close button
                    if ui.button("Close").clicked() {
                        self.show_cloudflare_setup = false;
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Next button
                        if self.cloudflare_step < 4 {
                            let next_btn = egui::Button::new(
                                egui::RichText::new("Next →").color(egui::Color32::WHITE)
                            )
                            .fill(egui::Color32::from_rgb(99, 102, 241));
                            
                            if ui.add(next_btn).clicked() {
                                self.cloudflare_step += 1;
                            }
                        } else {
                            let done_btn = egui::Button::new(
                                egui::RichText::new("✓ Done").color(egui::Color32::WHITE)
                            )
                            .fill(egui::Color32::from_rgb(34, 197, 94));
                            
                            if ui.add(done_btn).clicked() {
                                self.show_cloudflare_setup = false;
                            }
                        }
                        
                        ui.add_space(8.0);
                        
                        // Back button
                        if self.cloudflare_step > 0 {
                            if ui.button("← Back").clicked() {
                                self.cloudflare_step -= 1;
                            }
                        }
                    });
                });
            });
    }

    fn step_install_cloudflared(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Step 1: Install Cloudflared").size(16.0).strong());
        ui.add_space(12.0);
        
        ui.label("Cloudflared is the tool that creates secure tunnels to Cloudflare.");
        ui.add_space(12.0);
        
        ui.label(egui::RichText::new("Run this command to install:").color(egui::Color32::from_rgb(148, 163, 184)));
        ui.add_space(8.0);
        
        let cmd = "winget install Cloudflare.cloudflared";
        
        // Command box with inline run
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(20, 20, 30))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("$").color(egui::Color32::from_rgb(34, 197, 94)));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(cmd).monospace().color(egui::Color32::WHITE));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = if self.step1_running { "Installing..." } else { "▶ Run" };
                        let run_btn = egui::Button::new(
                            egui::RichText::new(btn_text).color(egui::Color32::WHITE).size(12.0)
                        )
                        .fill(if self.step1_running { 
                            egui::Color32::from_rgb(100, 100, 100) 
                        } else { 
                            egui::Color32::from_rgb(34, 197, 94) 
                        })
                        .rounding(egui::Rounding::same(4.0));
                        
                        if ui.add_enabled(!self.step1_running, run_btn).clicked() {
                            self.step1_running = true;
                            self.step1_status.output.clear();
                            
                            // Run winget install
                            match Command::new("powershell")
                                .args(["-Command", "winget install Cloudflare.cloudflared --accept-package-agreements --accept-source-agreements"])
                                .output()
                            {
                                Ok(output) => {
                                    let stdout = String::from_utf8_lossy(&output.stdout);
                                    let stderr = String::from_utf8_lossy(&output.stderr);
                                    let combined = format!("{}{}", stdout, stderr);
                                    
                                    if output.status.success() || combined.contains("Successfully installed") || combined.contains("already installed") {
                                        self.cloudflare_installed = true;
                                        self.step1_status.success = Some(true);
                                        if combined.contains("already installed") {
                                            self.step1_status.output = "✓ Cloudflared is already installed!".to_string();
                                        } else {
                                            self.step1_status.output = "✓ Cloudflared installed successfully!".to_string();
                                        }
                                    } else {
                                        self.step1_status.success = Some(false);
                                        self.step1_status.output = format!("Installation issue: {}", combined.lines().take(3).collect::<Vec<_>>().join(" "));
                                    }
                                }
                                Err(e) => {
                                    self.step1_status.success = Some(false);
                                    self.step1_status.output = format!("Failed to run winget: {}", e);
                                }
                            }
                            self.step1_running = false;
                        }
                        
                        ui.add_space(8.0);
                        if ui.small_button("📋 Copy").clicked() {
                            ui.ctx().copy_text(cmd.to_string());
                        }
                    });
                });
            });
        
        ui.add_space(12.0);
        
        // Show result status
        if !self.step1_status.output.is_empty() {
            let (color, bg_color) = if self.step1_status.success == Some(true) {
                (egui::Color32::from_rgb(34, 197, 94), egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.2))
            } else {
                (egui::Color32::from_rgb(239, 68, 68), egui::Color32::from_rgb(239, 68, 68).linear_multiply(0.2))
            };
            
            egui::Frame::none()
                .fill(bg_color)
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(&self.step1_status.output).color(color));
                });
        }
        
        ui.add_space(12.0);
        
        // Alternative download link
        ui.label(egui::RichText::new("Alternative: Download manually").size(12.0).color(egui::Color32::from_rgb(148, 163, 184)));
        ui.hyperlink_to("Download from Cloudflare", "https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/");
        
        // Show completion status
        if self.cloudflare_installed {
            ui.add_space(12.0);
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.15))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("✓").color(egui::Color32::from_rgb(34, 197, 94)));
                        ui.label(egui::RichText::new("Ready for next step").color(egui::Color32::from_rgb(34, 197, 94)));
                    });
                });
        }
    }

    fn step_login_cloudflare(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Step 2: Login to Cloudflare").size(16.0).strong());
        ui.add_space(12.0);
        
        ui.label("This will open your browser to authenticate with your Cloudflare account.");
        ui.add_space(12.0);
        
        ui.label(egui::RichText::new("Run this command:").color(egui::Color32::from_rgb(148, 163, 184)));
        ui.add_space(8.0);
        
        let cmd = "cloudflared tunnel login";
        
        // Command box with inline run
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(20, 20, 30))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("$").color(egui::Color32::from_rgb(34, 197, 94)));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(cmd).monospace().color(egui::Color32::WHITE));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = if self.step2_running { "Waiting..." } else { "▶ Run" };
                        let run_btn = egui::Button::new(
                            egui::RichText::new(btn_text).color(egui::Color32::WHITE).size(12.0)
                        )
                        .fill(if self.step2_running { 
                            egui::Color32::from_rgb(100, 100, 100) 
                        } else { 
                            egui::Color32::from_rgb(34, 197, 94) 
                        })
                        .rounding(egui::Rounding::same(4.0));
                        
                        if ui.add_enabled(!self.step2_running, run_btn).clicked() {
                            self.step2_running = true;
                            self.step2_status.output = "Browser opened - complete login in browser...".to_string();
                            self.step2_status.success = None;
                            
                            // Run cloudflared login - this opens a browser and waits for auth
                            let full_cmd = "$env:Path = [System.Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [System.Environment]::GetEnvironmentVariable('Path','User'); cloudflared tunnel login";
                            
                            match Command::new("powershell")
                                .args(["-Command", full_cmd])
                                .output()
                            {
                                Ok(output) => {
                                    let stdout = String::from_utf8_lossy(&output.stdout);
                                    let stderr = String::from_utf8_lossy(&output.stderr);
                                    let combined = format!("{}{}", stdout, stderr);
                                    
                                    if combined.contains("You have successfully logged in") || 
                                       combined.contains("certificate has been") ||
                                       combined.contains("cert.pem") {
                                        self.cloudflare_logged_in = true;
                                        self.step2_status.success = Some(true);
                                        self.step2_status.output = "✓ Successfully logged in to Cloudflare!".to_string();
                                    } else if combined.contains("already") {
                                        self.cloudflare_logged_in = true;
                                        self.step2_status.success = Some(true);
                                        self.step2_status.output = "✓ Already logged in to Cloudflare!".to_string();
                                    } else {
                                        self.step2_status.success = Some(false);
                                        self.step2_status.output = format!("Login may have failed: {}", combined.lines().take(2).collect::<Vec<_>>().join(" "));
                                    }
                                }
                                Err(e) => {
                                    self.step2_status.success = Some(false);
                                    self.step2_status.output = format!("Failed to run command: {}", e);
                                }
                            }
                            self.step2_running = false;
                        }
                        
                        ui.add_space(8.0);
                        if ui.small_button("📋 Copy").clicked() {
                            ui.ctx().copy_text(cmd.to_string());
                        }
                    });
                });
            });
        
        ui.add_space(12.0);
        
        // Show result status
        if !self.step2_status.output.is_empty() {
            let (color, bg_color) = match self.step2_status.success {
                Some(true) => (egui::Color32::from_rgb(34, 197, 94), egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.2)),
                Some(false) => (egui::Color32::from_rgb(239, 68, 68), egui::Color32::from_rgb(239, 68, 68).linear_multiply(0.2)),
                None => (egui::Color32::from_rgb(251, 191, 36), egui::Color32::from_rgb(251, 191, 36).linear_multiply(0.2)),
            };
            
            egui::Frame::none()
                .fill(bg_color)
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(&self.step2_status.output).color(color));
                });
        }
        
        ui.add_space(12.0);
        
        ui.label(
            egui::RichText::new("A browser window will open. Select your domain and authorize access.")
                .size(12.0)
                .color(egui::Color32::from_rgb(148, 163, 184))
        );
        
        // Show completion status
        if self.cloudflare_logged_in {
            ui.add_space(12.0);
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.15))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("✓").color(egui::Color32::from_rgb(34, 197, 94)));
                        ui.label(egui::RichText::new("Ready for next step").color(egui::Color32::from_rgb(34, 197, 94)));
                    });
                });
        }
    }

    fn step_create_tunnel(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Step 3: Create the Tunnel").size(16.0).strong());
        ui.add_space(12.0);
        
        ui.label("Create a named tunnel that will route traffic to CabNet.");
        ui.add_space(12.0);
        
        ui.horizontal(|ui| {
            ui.label("Tunnel name:");
            ui.add_space(8.0);
            ui.add(egui::TextEdit::singleline(&mut self.cloudflare_tunnel_name)
                .desired_width(150.0)
                .hint_text("cabnet"));
        });
        
        ui.add_space(12.0);
        
        ui.label(egui::RichText::new("Run this command:").color(egui::Color32::from_rgb(148, 163, 184)));
        ui.add_space(8.0);
        
        let tunnel_name = if self.cloudflare_tunnel_name.is_empty() { "cabnet" } else { &self.cloudflare_tunnel_name };
        let cmd = format!("cloudflared tunnel create {}", tunnel_name);
        
        // Show command box with special run button that captures output
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(20, 20, 30))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("$").color(egui::Color32::from_rgb(34, 197, 94)));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(&cmd).monospace().color(egui::Color32::WHITE));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Run button - creates tunnel and captures ID
                        let btn_text = if self.creating_tunnel { "Creating..." } else { "▶ Run" };
                        let run_btn = egui::Button::new(
                            egui::RichText::new(btn_text)
                                .color(egui::Color32::WHITE)
                                .size(12.0)
                        )
                        .fill(if self.creating_tunnel { 
                            egui::Color32::from_rgb(100, 100, 100) 
                        } else { 
                            egui::Color32::from_rgb(34, 197, 94) 
                        })
                        .rounding(egui::Rounding::same(4.0));
                        
                        if ui.add_enabled(!self.creating_tunnel, run_btn).clicked() {
                            self.creating_tunnel = true;
                            self.cmd_status.output.clear();
                            
                            // Run command and capture output
                            let full_cmd = format!(
                                "$env:Path = [System.Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [System.Environment]::GetEnvironmentVariable('Path','User'); cloudflared tunnel create {}",
                                tunnel_name
                            );
                            
                            match Command::new("powershell")
                                .args(["-Command", &full_cmd])
                                .output()
                            {
                                Ok(output) => {
                                    let stdout = String::from_utf8_lossy(&output.stdout);
                                    let stderr = String::from_utf8_lossy(&output.stderr);
                                    let combined = format!("{}{}", stdout, stderr);
                                    
                                    // Parse tunnel ID from output
                                    // Output format: "Created tunnel cabnet with id 1787a9ec-176d-48bd-98c3-a8a329ba5cc5"
                                    if let Some(id) = Self::extract_tunnel_id(&combined) {
                                        self.cloudflare_tunnel_id = id;
                                        self.cloudflare_tunnel_created = true;
                                        self.cmd_status.success = Some(true);
                                        self.cmd_status.output = format!("✓ Tunnel created! ID: {}", self.cloudflare_tunnel_id);
                                    } else if combined.contains("already exists") {
                                        // Tunnel already exists - try to get the ID
                                        self.cmd_status.success = Some(false);
                                        self.cmd_status.output = "Tunnel already exists. Use a different name or delete the existing tunnel.".to_string();
                                    } else {
                                        self.cmd_status.success = Some(false);
                                        self.cmd_status.output = format!("Failed to create tunnel: {}", combined.trim());
                                    }
                                }
                                Err(e) => {
                                    self.cmd_status.success = Some(false);
                                    self.cmd_status.output = format!("Failed to run command: {}", e);
                                }
                            }
                            self.creating_tunnel = false;
                        }
                        ui.add_space(8.0);
                        
                        // Copy button
                        if ui.small_button("📋 Copy").clicked() {
                            ui.ctx().copy_text(cmd.clone());
                        }
                    });
                });
            });
        
        ui.add_space(12.0);
        
        // Show result status
        if !self.cmd_status.output.is_empty() {
            let (color, bg_color) = if self.cmd_status.success == Some(true) {
                (egui::Color32::from_rgb(34, 197, 94), egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.2))
            } else {
                (egui::Color32::from_rgb(239, 68, 68), egui::Color32::from_rgb(239, 68, 68).linear_multiply(0.2))
            };
            
            egui::Frame::none()
                .fill(bg_color)
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(&self.cmd_status.output).color(color));
                });
        }
        
        // Show saved tunnel ID if we have one
        if !self.cloudflare_tunnel_id.is_empty() {
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(45, 45, 65))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Tunnel ID:");
                        ui.label(egui::RichText::new(&self.cloudflare_tunnel_id).monospace().strong());
                    });
                });
            
            // Show completion status
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.15))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("✓").color(egui::Color32::from_rgb(34, 197, 94)));
                        ui.label(egui::RichText::new("Ready for next step").color(egui::Color32::from_rgb(34, 197, 94)));
                    });
                });
        }
    }
    
    /// Extract tunnel ID from cloudflared output
    fn extract_tunnel_id(output: &str) -> Option<String> {
        // Pattern: "Created tunnel <name> with id <uuid>"
        // Or: "Tunnel credentials written to ... <uuid>.json"
        
        // Try to find UUID pattern (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
        let uuid_pattern = regex::Regex::new(
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
        ).ok()?;
        
        uuid_pattern.find(output).map(|m| m.as_str().to_string())
    }

    fn step_configure_domain(&mut self, ui: &mut egui::Ui, _port: &str) {
        ui.label(egui::RichText::new("Step 4: Configure Your Domain").size(16.0).strong());
        ui.add_space(12.0);
        
        ui.label("Enter the domain you registered with Cloudflare:");
        ui.add_space(12.0);
        
        egui::Grid::new("domain_config")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Your domain:");
                ui.add(egui::TextEdit::singleline(&mut self.cloudflare_domain)
                    .desired_width(200.0)
                    .hint_text("yourdomain.com"));
                ui.end_row();
                
                ui.label("Subdomain:");
                ui.add(egui::TextEdit::singleline(&mut self.cloudflare_subdomain)
                    .desired_width(200.0)
                    .hint_text("scans (for dashboard)"));
                ui.end_row();
            });
        
        ui.add_space(12.0);
        
        // Check if domain is filled in
        let domain_valid = !self.cloudflare_domain.is_empty() && !self.cloudflare_domain.contains("example");
        let subdomain = if self.cloudflare_subdomain.is_empty() { "scans" } else { &self.cloudflare_subdomain };
        let tunnel_name = if self.cloudflare_tunnel_name.is_empty() { "cabnet" } else { &self.cloudflare_tunnel_name };
        
        if domain_valid {
            let main_domain = &self.cloudflare_domain;
            let full_subdomain = format!("{}.{}", subdomain, self.cloudflare_domain);
            
            ui.label(
                egui::RichText::new("CabNet URLs will be:")
                    .color(egui::Color32::from_rgb(148, 163, 184))
            );
            ui.label(
                egui::RichText::new(format!("• https://{} (ticket search, email, map)", main_domain))
                    .color(egui::Color32::from_rgb(34, 197, 94))
            );
            ui.label(
                egui::RichText::new(format!("• https://{} (dashboard)", full_subdomain))
                    .color(egui::Color32::from_rgb(34, 197, 94))
            );
            
            ui.add_space(12.0);
            
            ui.label(egui::RichText::new("Click to configure DNS for both domains:").color(egui::Color32::from_rgb(148, 163, 184)));
            ui.add_space(8.0);
            
            let cmd1 = format!("cloudflared tunnel route dns {} {}", tunnel_name, main_domain);
            let cmd2 = format!("cloudflared tunnel route dns {} {}", tunnel_name, full_subdomain);
            
            // Show both commands
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(20, 20, 30))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Command 1 - Main domain
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("$").color(egui::Color32::from_rgb(34, 197, 94)));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(&cmd1).monospace().color(egui::Color32::WHITE).size(12.0));
                        });
                        ui.add_space(4.0);
                        // Command 2 - Subdomain
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("$").color(egui::Color32::from_rgb(34, 197, 94)));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(&cmd2).monospace().color(egui::Color32::WHITE).size(12.0));
                        });
                    });
                });
            
            ui.add_space(8.0);
            
            // Run button for both commands
            ui.horizontal(|ui| {
                let btn_text = if self.step4_running { "Configuring..." } else { "▶ Run Both Commands" };
                let run_btn = egui::Button::new(
                    egui::RichText::new(btn_text).color(egui::Color32::WHITE)
                )
                .fill(if self.step4_running { 
                    egui::Color32::from_rgb(100, 100, 100) 
                } else { 
                    egui::Color32::from_rgb(34, 197, 94) 
                })
                .rounding(egui::Rounding::same(4.0));
                
                if ui.add_enabled(!self.step4_running, run_btn).clicked() {
                    self.step4_running = true;
                    self.step4_status.output.clear();
                    
                    let mut results = Vec::new();
                    let mut all_success = true;
                    
                    // Run command for main domain
                    let full_cmd1 = format!(
                        "$env:Path = [System.Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [System.Environment]::GetEnvironmentVariable('Path','User'); cloudflared tunnel route dns {} {}",
                        tunnel_name, main_domain
                    );
                    
                    match Command::new("powershell")
                        .args(["-Command", &full_cmd1])
                        .output()
                    {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let combined = format!("{}{}", stdout, stderr);
                            
                            if combined.contains("Added CNAME") || 
                               combined.contains("successfully") ||
                               combined.contains("already exists") ||
                               output.status.success() {
                                if combined.contains("already exists") {
                                    results.push(format!("✓ {} (already exists)", main_domain));
                                } else {
                                    results.push(format!("✓ {} configured", main_domain));
                                }
                            } else {
                                all_success = false;
                                results.push(format!("✗ {} failed: {}", main_domain, combined.lines().next().unwrap_or("")));
                            }
                        }
                        Err(e) => {
                            all_success = false;
                            results.push(format!("✗ {} error: {}", main_domain, e));
                        }
                    }
                    
                    // Run command for subdomain
                    let full_cmd2 = format!(
                        "$env:Path = [System.Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [System.Environment]::GetEnvironmentVariable('Path','User'); cloudflared tunnel route dns {} {}",
                        tunnel_name, full_subdomain
                    );
                    
                    match Command::new("powershell")
                        .args(["-Command", &full_cmd2])
                        .output()
                    {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let combined = format!("{}{}", stdout, stderr);
                            
                            if combined.contains("Added CNAME") || 
                               combined.contains("successfully") ||
                               combined.contains("already exists") ||
                               output.status.success() {
                                if combined.contains("already exists") {
                                    results.push(format!("✓ {} (already exists)", full_subdomain));
                                } else {
                                    results.push(format!("✓ {} configured", full_subdomain));
                                }
                            } else {
                                all_success = false;
                                results.push(format!("✗ {} failed: {}", full_subdomain, combined.lines().next().unwrap_or("")));
                            }
                        }
                        Err(e) => {
                            all_success = false;
                            results.push(format!("✗ {} error: {}", full_subdomain, e));
                        }
                    }
                    
                    self.cloudflare_dns_configured = all_success;
                    self.step4_status.success = Some(all_success);
                    self.step4_status.output = results.join("\n");
                    self.step4_running = false;
                }
                
                ui.add_space(8.0);
                if ui.small_button("📋 Copy").clicked() {
                    ui.ctx().copy_text(format!("{}\n{}", cmd1, cmd2));
                }
            });
            
            ui.add_space(12.0);
            
            // Show result status
            if !self.step4_status.output.is_empty() {
                let (color, bg_color) = if self.step4_status.success == Some(true) {
                    (egui::Color32::from_rgb(34, 197, 94), egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.2))
                } else {
                    (egui::Color32::from_rgb(239, 68, 68), egui::Color32::from_rgb(239, 68, 68).linear_multiply(0.2))
                };
                
                egui::Frame::none()
                    .fill(bg_color)
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        for line in self.step4_status.output.lines() {
                            ui.label(egui::RichText::new(line).color(color));
                        }
                    });
            }
            
            // Show completion status
            if self.cloudflare_dns_configured {
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.15))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("✓").color(egui::Color32::from_rgb(34, 197, 94)));
                            ui.label(egui::RichText::new("Ready for next step").color(egui::Color32::from_rgb(34, 197, 94)));
                        });
                    });
            }
        } else {
            // Show warning if domain not entered
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(239, 68, 68).linear_multiply(0.2))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("⚠").size(16.0).color(egui::Color32::from_rgb(239, 68, 68)));
                        ui.add_space(8.0);
                        ui.label("Please enter your Cloudflare domain above (the one you selected during login)");
                    });
                });
        }
        
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("This creates DNS records pointing to your tunnel.")
                .size(12.0)
                .color(egui::Color32::from_rgb(148, 163, 184))
        );
    }

    fn step_run_tunnel(&mut self, ui: &mut egui::Ui, _port: &str, tunnel_manager: &SharedTunnelManager) {
        ui.label(egui::RichText::new("Step 5: Start the Tunnel").size(16.0).strong());
        ui.add_space(12.0);
        
        let tunnel_name = if self.cloudflare_tunnel_name.is_empty() { "cabnet" } else { &self.cloudflare_tunnel_name };
        let subdomain = if self.cloudflare_subdomain.is_empty() { "cabnet" } else { &self.cloudflare_subdomain };
        let full_domain = if self.cloudflare_domain.is_empty() {
            format!("{}.yourdomain.com", subdomain)
        } else {
            format!("{}.{}", subdomain, self.cloudflare_domain)
        };
        
        // Get current tunnel status
        let (tunnel_status, is_running) = {
            if let Ok(mut manager) = tunnel_manager.lock() {
                manager.check_status();
                (manager.status().clone(), manager.is_running())
            } else {
                (TunnelStatus::Stopped, false)
            }
        };
        
        if is_running {
            // Tunnel is running - show success
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.2))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(16.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("✓").size(32.0).color(egui::Color32::from_rgb(34, 197, 94)));
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Tunnel Running!").size(18.0).strong().color(egui::Color32::from_rgb(34, 197, 94)));
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!("https://{}", full_domain))
                                .size(16.0)
                                .color(egui::Color32::WHITE)
                                .strong()
                        );
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("CabNet is now accessible from the internet!")
                                .color(egui::Color32::from_rgb(148, 163, 184))
                        );
                        ui.label(
                            egui::RichText::new("The tunnel will auto-start when CabNet launches.")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(148, 163, 184))
                        );
                    });
                });
            
            ui.add_space(12.0);
            
            // Stop button
            ui.horizontal(|ui| {
                let stop_btn = egui::Button::new(
                    egui::RichText::new("⏹ Stop Tunnel").color(egui::Color32::WHITE)
                )
                .fill(egui::Color32::from_rgb(239, 68, 68))
                .rounding(egui::Rounding::same(6.0));
                
                if ui.add(stop_btn).clicked() {
                    if let Ok(mut manager) = tunnel_manager.lock() {
                        manager.stop();
                        // Disable auto-start
                        manager.config_mut().enabled = false;
                        let _ = manager.save_config();
                    }
                }
            });
        } else {
            // Show start button
            ui.label("Click the button below to start the tunnel in the background.");
            ui.label(
                egui::RichText::new("The tunnel will automatically start when CabNet launches.")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(148, 163, 184))
            );
            
            ui.add_space(16.0);
            
            // Show error if there was one
            if let TunnelStatus::Failed(ref error) = tunnel_status {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(239, 68, 68).linear_multiply(0.2))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("✗").color(egui::Color32::from_rgb(239, 68, 68)));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(error).color(egui::Color32::from_rgb(239, 68, 68)));
                        });
                    });
                ui.add_space(8.0);
            }
            
            // Start button
            let start_btn = egui::Button::new(
                egui::RichText::new("🚀 Start Tunnel").color(egui::Color32::WHITE).size(16.0)
            )
            .fill(egui::Color32::from_rgb(34, 197, 94))
            .rounding(egui::Rounding::same(8.0))
            .min_size(egui::vec2(200.0, 40.0));
            
            ui.vertical_centered(|ui| {
                if ui.add(start_btn).clicked() {
                    if let Ok(mut manager) = tunnel_manager.lock() {
                        // Update and save config
                        manager.config_mut().tunnel_name = tunnel_name.to_string();
                        manager.config_mut().domain = self.cloudflare_domain.clone();
                        manager.config_mut().subdomain = self.cloudflare_subdomain.clone();
                        manager.config_mut().enabled = true;
                        let _ = manager.save_config();
                        
                        // Start the tunnel
                        if let Err(e) = manager.start() {
                            tracing::error!("Failed to start tunnel: {}", e);
                        }
                    }
                }
            });
            
            ui.add_space(16.0);
            
            // Preview URL
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(35, 35, 50))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Your CabNet URL will be:");
                        ui.label(
                            egui::RichText::new(format!("https://{}", full_domain))
                                .color(egui::Color32::from_rgb(34, 197, 94))
                                .strong()
                        );
                    });
                });
        }
    }
}

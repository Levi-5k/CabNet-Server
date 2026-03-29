use crate::gui::app::CachedData;
use crate::services::SharedTunnelManager;
use eframe::egui;
use qrcode::QrCode;
use std::sync::Arc;

/// Devices tab for monitoring connected devices
#[derive(Default)]
pub struct DevicesTab {
    pub selected_device: Option<String>,
    pub show_offline: bool,
    pub show_qr_dialog: bool,
    pub qr_code_text: String,
    pub qr_format: QrFormat,
    pub qr_connection_type: QrConnectionType,
    pub local_ip: Option<String>,
    pub qr_texture: Option<egui::TextureHandle>,
    pub qr_image_data: Option<Arc<Vec<u8>>>,
    pub save_status: Option<String>,
    pub view_mode: ViewMode,
    pub search_query: String,
    pub sort_by: SortBy,
    pub sort_ascending: bool,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ViewMode {
    #[default]
    Cards,
    Table,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum SortBy {
    #[default]
    LastSeen,
    Name,
    Scans,
    Status,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum QrFormat {
    #[default]
    CabNet,
    Http,
    Json,
    Simple,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum QrConnectionType {
    #[default]
    LocalNetwork,
    CloudflareTunnel,
}

impl DevicesTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, data: &CachedData, server_addr: &str, tunnel_manager: &SharedTunnelManager) {
        if self.local_ip.is_none() {
            self.local_ip = local_ip_address::local_ip().ok().map(|ip| ip.to_string());
        }

        let port = server_addr.split(':').last().unwrap_or("8080");
        
        let tunnel_info = if let Ok(manager) = tunnel_manager.lock() {
            if manager.config().is_configured() {
                Some((manager.config().full_domain(), manager.is_running()))
            } else {
                None
            }
        } else {
            None
        };

        let total_devices = data.devices.len();
        let online_count = data.devices.iter().filter(|d| d.is_online).count();
        let offline_count = total_devices - online_count;
        let total_scans: i64 = data.devices.iter().map(|d| d.total_scans as i64).sum();

        // Header
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("📱").size(28.0));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Devices").size(24.0).strong());
                ui.label(egui::RichText::new("Manage and monitor connected scanning devices").size(13.0).color(egui::Color32::from_rgb(148, 163, 184)));
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let qr_btn = egui::Button::new(egui::RichText::new("➕ Connect Device").color(egui::Color32::WHITE).size(14.0))
                    .fill(egui::Color32::from_rgb(99, 102, 241))
                    .rounding(egui::Rounding::same(8.0))
                    .min_size(egui::vec2(140.0, 36.0));
                
                if ui.add(qr_btn).clicked() {
                    self.show_qr_dialog = true;
                    self.update_qr_code(port, tunnel_info.as_ref().map(|(d, _)| d.as_str()));
                }
            });
        });

        ui.add_space(20.0);

        // Stats Cards
        ui.horizontal(|ui| {
            self.stat_card(ui, "📱", "Total Devices", &total_devices.to_string(), egui::Color32::from_rgb(99, 102, 241));
            ui.add_space(12.0);
            self.stat_card(ui, "🟢", "Online", &online_count.to_string(), egui::Color32::from_rgb(34, 197, 94));
            ui.add_space(12.0);
            self.stat_card(ui, "⚫", "Offline", &offline_count.to_string(), egui::Color32::from_rgb(107, 114, 128));
            ui.add_space(12.0);
            self.stat_card(ui, "📊", "Total Scans", &total_scans.to_string(), egui::Color32::from_rgb(251, 146, 60));
        });

        ui.add_space(20.0);

        // Toolbar
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔍").size(16.0));
                    ui.add_space(4.0);
                    ui.add(egui::TextEdit::singleline(&mut self.search_query).hint_text("Search devices...").desired_width(200.0));
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);
                    
                    ui.label("View:");
                    ui.add_space(4.0);
                    
                    if ui.add(egui::Button::new(egui::RichText::new("▦ Cards").color(if self.view_mode == ViewMode::Cards { egui::Color32::WHITE } else { egui::Color32::from_rgb(148, 163, 184) }))
                        .fill(if self.view_mode == ViewMode::Cards { egui::Color32::from_rgb(99, 102, 241) } else { egui::Color32::TRANSPARENT })
                        .rounding(egui::Rounding::same(6.0))).clicked() {
                        self.view_mode = ViewMode::Cards;
                    }
                    
                    if ui.add(egui::Button::new(egui::RichText::new("☰ Table").color(if self.view_mode == ViewMode::Table { egui::Color32::WHITE } else { egui::Color32::from_rgb(148, 163, 184) }))
                        .fill(if self.view_mode == ViewMode::Table { egui::Color32::from_rgb(99, 102, 241) } else { egui::Color32::TRANSPARENT })
                        .rounding(egui::Rounding::same(6.0))).clicked() {
                        self.view_mode = ViewMode::Table;
                    }
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);
                    
                    ui.checkbox(&mut self.show_offline, "Show offline");
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(if self.sort_ascending { "↑" } else { "↓" }).rounding(egui::Rounding::same(4.0))).clicked() {
                            self.sort_ascending = !self.sort_ascending;
                        }
                        egui::ComboBox::from_id_source("sort_by")
                            .selected_text(match self.sort_by { SortBy::LastSeen => "Last Seen", SortBy::Name => "Name", SortBy::Scans => "Scans", SortBy::Status => "Status" })
                            .show_ui(ui, |ui| {
                                if ui.selectable_value(&mut self.sort_by, SortBy::LastSeen, "Last Seen").clicked() { self.sort_ascending = false; }
                                if ui.selectable_value(&mut self.sort_by, SortBy::Name, "Name").clicked() { self.sort_ascending = true; }
                                if ui.selectable_value(&mut self.sort_by, SortBy::Scans, "Scans").clicked() { self.sort_ascending = false; }
                                if ui.selectable_value(&mut self.sort_by, SortBy::Status, "Status").clicked() { self.sort_ascending = false; }
                            });
                        ui.label("Sort:");
                    });
                });
            });

        ui.add_space(16.0);

        if self.show_qr_dialog {
            self.show_qr_connect_dialog(ui, port, tunnel_info.as_ref());
        }

        if data.devices.is_empty() {
            self.show_empty_state(ui);
            return;
        }

        let mut filtered_devices: Vec<_> = data.devices.iter()
            .filter(|d| self.show_offline || d.is_online)
            .filter(|d| {
                if self.search_query.is_empty() { return true; }
                let query = self.search_query.to_lowercase();
                d.device.device_id.to_lowercase().contains(&query)
                    || d.device.device_name.as_ref().map_or(false, |n| n.to_lowercase().contains(&query))
                    || d.device.model.as_ref().map_or(false, |m| m.to_lowercase().contains(&query))
            })
            .collect();

        filtered_devices.sort_by(|a, b| {
            let cmp = match self.sort_by {
                SortBy::LastSeen => b.device.last_seen_at.cmp(&a.device.last_seen_at),
                SortBy::Name => a.device.device_name.cmp(&b.device.device_name),
                SortBy::Scans => b.total_scans.cmp(&a.total_scans),
                SortBy::Status => b.is_online.cmp(&a.is_online),
            };
            if self.sort_ascending { cmp.reverse() } else { cmp }
        });

        if filtered_devices.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(egui::RichText::new("🔍").size(48.0));
                ui.add_space(12.0);
                ui.label(egui::RichText::new("No devices match your search").size(16.0).color(egui::Color32::from_rgb(148, 163, 184)));
            });
            return;
        }

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            match self.view_mode {
                ViewMode::Cards => self.show_cards_view(ui, &filtered_devices),
                ViewMode::Table => self.show_table_view(ui, &filtered_devices),
            }
        });
    }

    fn stat_card(&self, ui: &mut egui::Ui, icon: &str, label: &str, value: &str, color: egui::Color32) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_min_width(140.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).size(28.0));
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(value).size(24.0).color(color).strong());
                        ui.label(egui::RichText::new(label).size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    });
                });
            });
    }

    fn show_empty_state(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(40, 40, 58))
                .rounding(egui::Rounding::same(20.0))
                .inner_margin(egui::Margin::same(40.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("📱").size(64.0));
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("No devices connected yet").size(22.0).strong());
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Click 'Connect Device' to generate a QR code").size(14.0).color(egui::Color32::from_rgb(148, 163, 184)));
                    });
                });
        });
    }

    fn show_cards_view(&mut self, ui: &mut egui::Ui, devices: &[&crate::db::models::DeviceWithStats]) {
        let card_width = 320.0;
        let spacing = 16.0;
        let cards_per_row = ((ui.available_width() + spacing) / (card_width + spacing)).floor().max(1.0) as usize;

        egui::Grid::new("device_cards").num_columns(cards_per_row).spacing([spacing, spacing]).show(ui, |ui| {
            for (i, device) in devices.iter().enumerate() {
                self.device_card(ui, device, card_width);
                if (i + 1) % cards_per_row == 0 { ui.end_row(); }
            }
        });
    }

    fn device_card(&mut self, ui: &mut egui::Ui, device: &crate::db::models::DeviceWithStats, width: f32) {
        let is_online = device.is_online;
        let border_color = if is_online { egui::Color32::from_rgb(34, 197, 94) } else { egui::Color32::from_rgb(75, 75, 95) };

        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .stroke(egui::Stroke::new(2.0, border_color))
            .rounding(egui::Rounding::same(14.0))
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                ui.set_min_width(width - 40.0);
                ui.set_max_width(width - 40.0);
                
                ui.horizontal(|ui| {
                    let icon = if device.device.model.as_ref().map_or(false, |m| m.to_lowercase().contains("zebra")) { "📟" } else { "📱" };
                    ui.label(egui::RichText::new(icon).size(36.0));
                    ui.add_space(12.0);
                    
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(device.device.device_name.as_deref().unwrap_or("Unknown Device")).size(16.0).strong());
                        let (status_color, status_text) = if is_online { (egui::Color32::from_rgb(34, 197, 94), "● Online") } else { (egui::Color32::from_rgb(107, 114, 128), "○ Offline") };
                        ui.label(egui::RichText::new(status_text).size(12.0).color(status_color));
                    });
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(99, 102, 241))
                            .rounding(egui::Rounding::same(12.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(format!("{} scans", device.total_scans)).size(11.0).color(egui::Color32::WHITE));
                            });
                    });
                });
                
                ui.add_space(16.0);
                
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 30, 45))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        egui::Grid::new(format!("details_{}", device.device.device_id)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                            ui.label(egui::RichText::new("ID").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                            ui.label(egui::RichText::new(&device.device.device_id).size(11.0)); ui.end_row();
                            ui.label(egui::RichText::new("Model").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                            ui.label(egui::RichText::new(device.device.model.as_deref().unwrap_or("-")).size(11.0)); ui.end_row();
                            ui.label(egui::RichText::new("OS").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                            ui.label(egui::RichText::new(device.device.os_version.as_deref().unwrap_or("-")).size(11.0)); ui.end_row();
                            ui.label(egui::RichText::new("App").size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));
                            ui.label(egui::RichText::new(device.device.app_version.as_deref().unwrap_or("-")).size(11.0)); ui.end_row();
                        });
                    });
                
                ui.add_space(12.0);
                
                // Enhanced status information
                if device.device.battery_level.is_some() || device.device.network_type.is_some() || device.device.connection_count.is_some() {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(30, 30, 45))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("📊 Status").size(12.0).strong().color(egui::Color32::from_rgb(99, 102, 241)));
                            ui.add_space(8.0);
                            
                            egui::Grid::new(format!("status_{}", device.device.device_id)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                                if let Some(battery) = device.device.battery_level {
                                    ui.label(egui::RichText::new("🔋 Battery").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                                    let battery_text = if let Some(charging) = device.device.is_charging {
                                        if charging { format!("{}% ⚡", battery) } else { format!("{}%", battery) }
                                    } else {
                                        format!("{}%", battery)
                                    };
                                    ui.label(egui::RichText::new(battery_text).size(10.0)); ui.end_row();
                                }
                                
                                if let Some(memory) = device.device.available_memory_mb {
                                    ui.label(egui::RichText::new("💾 Memory").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                                    ui.label(egui::RichText::new(format!("{} MB", memory)).size(10.0)); ui.end_row();
                                }
                                
                                if let Some(network) = &device.device.network_type {
                                    ui.label(egui::RichText::new("📡 Network").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                                    ui.label(egui::RichText::new(network).size(10.0)); ui.end_row();
                                }
                                
                                if let Some(quality) = &device.device.connection_quality {
                                    ui.label(egui::RichText::new("📶 Quality").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                                    ui.label(egui::RichText::new(quality).size(10.0)); ui.end_row();
                                }
                                
                                if let Some(connections) = device.device.connection_count {
                                    ui.label(egui::RichText::new("🔄 Sessions").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                                    ui.label(egui::RichText::new(connections.to_string()).size(10.0)); ui.end_row();
                                }
                                
                                if let Some(uptime) = device.device.total_uptime_seconds {
                                    ui.label(egui::RichText::new("⏱️ Uptime").size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
                                    ui.label(egui::RichText::new(format_duration(uptime)).size(10.0)); ui.end_row();
                                }
                            });
                        });
                    
                    ui.add_space(12.0);
                }
                
                let last_seen = device.device.last_seen_at.as_ref().map(|s| format_relative_time(s)).unwrap_or_else(|| "Never".to_string());
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🕐").size(12.0));
                    ui.label(egui::RichText::new(format!("Last seen: {}", last_seen)).size(12.0).color(egui::Color32::from_rgb(148, 163, 184)));
                });
            });
    }

    fn show_table_view(&mut self, ui: &mut egui::Ui, devices: &[&crate::db::models::DeviceWithStats]) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(12.0))
            .show(ui, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(50, 50, 70))
                    .rounding(egui::Rounding { nw: 12.0, ne: 12.0, sw: 0.0, se: 0.0 })
                    .inner_margin(egui::Margin::symmetric(16.0, 14.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_width(ui.available_width());
                            Self::header(ui, "Status", 90.0);
                            Self::header(ui, "Name", 160.0);
                            Self::header(ui, "Device ID", 140.0);
                            Self::header(ui, "Model", 120.0);
                            Self::header(ui, "Scans", 70.0);
                            Self::header(ui, "Last Seen", 140.0);
                        });
                    });

                for (i, device) in devices.iter().enumerate() {
                    let bg = if i % 2 == 0 { egui::Color32::from_rgb(40, 40, 58) } else { egui::Color32::from_rgb(45, 45, 65) };
                    egui::Frame::none().fill(bg).inner_margin(egui::Margin::symmetric(16.0, 12.0)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_width(ui.available_width());
                            
                            ui.allocate_ui(egui::vec2(90.0, 24.0), |ui| {
                                if device.is_online {
                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.2))
                                        .rounding(egui::Rounding::same(4.0))
                                        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                                        .show(ui, |ui| { ui.label(egui::RichText::new("● Online").size(11.0).color(egui::Color32::from_rgb(34, 197, 94))); });
                                } else {
                                    ui.label(egui::RichText::new("○ Offline").size(11.0).color(egui::Color32::from_rgb(107, 114, 128)));
                                }
                            });
                            
                            Self::cell(ui, device.device.device_name.as_deref().unwrap_or("-"), 160.0);
                            Self::cell(ui, &device.device.device_id, 140.0);
                            Self::cell(ui, device.device.model.as_deref().unwrap_or("-"), 120.0);
                            
                            ui.allocate_ui(egui::vec2(70.0, 24.0), |ui| {
                                ui.label(egui::RichText::new(device.total_scans.to_string()).size(13.0).strong().color(egui::Color32::from_rgb(99, 102, 241)));
                            });
                            
                            let last_seen = device.device.last_seen_at.as_ref().map(|s| format_relative_time(s)).unwrap_or_else(|| "Never".to_string());
                            Self::cell(ui, &last_seen, 140.0);
                        });
                    });
                }
            });
    }

    fn show_qr_connect_dialog(&mut self, ui: &mut egui::Ui, port: &str, tunnel_info: Option<&(String, bool)>) {
        egui::Window::new("📲 Connect Device")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([520.0, 680.0])
            .frame(egui::Frame::window(&ui.ctx().style())
                .fill(egui::Color32::from_rgb(30, 30, 46))
                .rounding(egui::Rounding::same(16.0))
                .inner_margin(egui::Margin::same(24.0)))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Scan QR Code to Connect").size(20.0).strong());
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Open the CabNet app and scan this QR code")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(148, 163, 184))
                    );
                });

                ui.add_space(16.0);

                // Connection Type selector
                if tunnel_info.is_some() {
                    ui.horizontal(|ui| {
                        ui.label("Connection:");
                        ui.add_space(8.0);
                        
                        for (conn_type, label) in [(QrConnectionType::LocalNetwork, "🏠 Local"), (QrConnectionType::CloudflareTunnel, "☁ Internet")] {
                            let is_sel = self.qr_connection_type == conn_type;
                            if ui.add(egui::Button::new(egui::RichText::new(label).color(if is_sel { egui::Color32::WHITE } else { egui::Color32::from_rgb(148, 163, 184) }))
                                .fill(if is_sel { egui::Color32::from_rgb(34, 197, 94) } else { egui::Color32::from_rgb(55, 55, 75) })
                                .rounding(egui::Rounding::same(6.0))).clicked() {
                                self.qr_connection_type = conn_type;
                                self.update_qr_code(port, tunnel_info.map(|(d, _)| d.as_str()));
                                self.qr_texture = None;
                            }
                        }
                    });

                    if let Some((domain, is_running)) = tunnel_info {
                        ui.add_space(4.0);
                        if self.qr_connection_type == QrConnectionType::CloudflareTunnel {
                            if *is_running {
                                ui.label(egui::RichText::new(format!("✓ Tunnel active: {}", domain)).size(12.0).color(egui::Color32::from_rgb(34, 197, 94)));
                            } else {
                                ui.label(egui::RichText::new("⚠ Tunnel not running").size(12.0).color(egui::Color32::from_rgb(251, 191, 36)));
                            }
                        }
                    }
                    ui.add_space(12.0);
                }

                // QR Format selector
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    ui.add_space(8.0);
                    
                    for (format, label) in [(QrFormat::CabNet, "cabnet://"), (QrFormat::Http, "https://"), (QrFormat::Json, "JSON"), (QrFormat::Simple, "Address")] {
                        let is_sel = self.qr_format == format;
                        if ui.add(egui::Button::new(egui::RichText::new(label).color(if is_sel { egui::Color32::WHITE } else { egui::Color32::from_rgb(148, 163, 184) }))
                            .fill(if is_sel { egui::Color32::from_rgb(99, 102, 241) } else { egui::Color32::from_rgb(55, 55, 75) })
                            .rounding(egui::Rounding::same(6.0))).clicked() {
                            self.qr_format = format;
                            self.update_qr_code(port, tunnel_info.map(|(d, _)| d.as_str()));
                            self.qr_texture = None;
                        }
                    }
                });

                ui.add_space(16.0);

                // QR Code display
                let qr_size = 280.0;
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .rounding(egui::Rounding::same(12.0))
                    .inner_margin(egui::Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            let can_show = match self.qr_connection_type {
                                QrConnectionType::LocalNetwork => self.local_ip.is_some(),
                                QrConnectionType::CloudflareTunnel => tunnel_info.is_some(),
                            };
                            
                            if can_show {
                                if self.qr_texture.is_none() {
                                    self.generate_qr_texture(ui.ctx());
                                }
                                
                                if let Some(texture) = &self.qr_texture {
                                    ui.image(egui::ImageSource::Texture(
                                        egui::load::SizedTexture::new(texture.id(), egui::vec2(qr_size, qr_size))
                                    ));
                                }
                            } else {
                                ui.allocate_space(egui::vec2(qr_size, qr_size));
                                ui.label(egui::RichText::new("Not available").color(egui::Color32::from_rgb(239, 68, 68)));
                            }
                        });
                    });

                ui.add_space(12.0);

                // Connection string
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(45, 45, 65))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("URL:").color(egui::Color32::from_rgb(148, 163, 184)));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(&self.qr_code_text).monospace().color(egui::Color32::from_rgb(34, 197, 94)));
                        });
                    });

                ui.add_space(16.0);

                if let Some(status) = &self.save_status {
                    let color = if status.starts_with('✓') { egui::Color32::from_rgb(34, 197, 94) } else { egui::Color32::from_rgb(239, 68, 68) };
                    ui.label(egui::RichText::new(status).color(color));
                    ui.add_space(8.0);
                }

                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("💾 Save Image").color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(34, 197, 94))
                        .rounding(egui::Rounding::same(8.0))
                        .min_size(egui::vec2(130.0, 36.0))).clicked() {
                        self.save_qr_image();
                    }
                    
                    ui.add_space(12.0);
                    
                    if ui.add(egui::Button::new(egui::RichText::new("Close").color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(99, 102, 241))
                        .rounding(egui::Rounding::same(8.0))
                        .min_size(egui::vec2(100.0, 36.0))).clicked() {
                        self.show_qr_dialog = false;
                        self.save_status = None;
                    }
                });
            });
    }

    fn generate_qr_texture(&mut self, ctx: &egui::Context) {
        if let Ok(code) = QrCode::new(&self.qr_code_text) {
            let image = code.render::<image::Luma<u8>>()
                .min_dimensions(400, 400)
                .quiet_zone(true)
                .build();
            
            let width = image.width() as usize;
            let height = image.height() as usize;
            
            let mut pixels = Vec::with_capacity(width * height * 4);
            for pixel in image.pixels() {
                let val = pixel.0[0];
                pixels.extend_from_slice(&[val, val, val, 255]);
            }
            
            self.qr_image_data = Some(Arc::new(image.into_raw()));
            
            self.qr_texture = Some(ctx.load_texture(
                "qr_code",
                egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels),
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    fn save_qr_image(&mut self) {
        if let Ok(code) = QrCode::new(&self.qr_code_text) {
            let image = code.render::<image::Luma<u8>>()
                .min_dimensions(600, 600)
                .quiet_zone(true)
                .build();
            
            let default_filename = if let Some(ip) = &self.local_ip {
                format!("CabNet_QR_{}.png", ip.replace('.', "_"))
            } else {
                "CabNet_QR.png".to_string()
            };
            
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Save QR Code Image")
                .set_file_name(&default_filename)
                .add_filter("PNG Image", &["png"])
                .save_file()
            {
                let path = if path.extension().map_or(true, |ext| ext != "png") {
                    path.with_extension("png")
                } else {
                    path
                };
                
                match image.save(&path) {
                    Ok(_) => self.save_status = Some(format!("✓ Saved to {}", path.display())),
                    Err(e) => self.save_status = Some(format!("✗ Failed: {}", e)),
                }
            }
        }
    }

    fn update_qr_code(&mut self, port: &str, tunnel_domain: Option<&str>) {
        match self.qr_connection_type {
            QrConnectionType::LocalNetwork => {
                if let Some(ip) = &self.local_ip {
                    self.qr_code_text = match self.qr_format {
                        QrFormat::CabNet => format!("cabnet://{}:{}", ip, port),
                        QrFormat::Http => format!("http://{}:{}", ip, port),
                        QrFormat::Json => format!("{{\"host\":\"{}\",\"port\":{},\"secure\":false}}", ip, port),
                        QrFormat::Simple => format!("{}:{}", ip, port),
                    };
                }
            }
            QrConnectionType::CloudflareTunnel => {
                if let Some(domain) = tunnel_domain {
                    self.qr_code_text = match self.qr_format {
                        QrFormat::CabNet => format!("cabnet://{}", domain),
                        QrFormat::Http => format!("https://{}", domain),
                        QrFormat::Json => format!("{{\"host\":\"{}\",\"secure\":true}}", domain),
                        QrFormat::Simple => domain.to_string(),
                    };
                }
            }
        }
    }

    fn header(ui: &mut egui::Ui, text: &str, width: f32) {
        ui.allocate_ui(egui::vec2(width, 20.0), |ui| {
            ui.label(egui::RichText::new(text).size(12.0).color(egui::Color32::from_rgb(148, 163, 184)).strong());
        });
    }

    fn cell(ui: &mut egui::Ui, text: &str, width: f32) {
        ui.allocate_ui(egui::vec2(width, 24.0), |ui| {
            ui.label(egui::RichText::new(text).size(13.0).color(egui::Color32::from_rgb(226, 232, 240)));
        });
    }
}

fn format_relative_time(timestamp: &str) -> String {
    use chrono::{DateTime, Utc};
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
        let duration = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
        let secs = duration.num_seconds();
        if secs < 60 { return "Just now".to_string(); }
        let mins = duration.num_minutes();
        if mins < 60 { return format!("{} min ago", mins); }
        let hrs = duration.num_hours();
        if hrs < 24 { return format!("{} hr ago", hrs); }
        let days = duration.num_days();
        if days < 7 { return format!("{} days ago", days); }
        return format!("{} weeks ago", days / 7);
    }
    timestamp.get(..16).unwrap_or(timestamp).replace('T', " ")
}

fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

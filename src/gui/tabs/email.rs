use crate::api::routes::SharedState;
use eframe::egui;

/// Email tab for SMTP configuration and sending reports
#[derive(Default)]
pub struct EmailTab {
    // SMTP Settings
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub email_from: String,
    pub email_to: String,
    
    // Auto-send settings
    pub auto_send_enabled: bool,
    pub auto_send_delay: String,
    
    // Email template settings
    pub email_subject_template: String,
    pub email_body_template: String,
    pub include_csv_attachment: bool,
    pub include_summary_stats: bool,
    pub include_device_info: bool,
    pub include_job_info: bool,
    
    // UI State
    pub test_status: Option<String>,
    pub loaded: bool,
    pub active_section: EmailSection,
    pub show_template_help: bool,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum EmailSection {
    #[default]
    Smtp,
    Template,
    AutoSend,
    History,
}

impl EmailTab {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &SharedState,
        runtime: &tokio::runtime::Handle,
    ) {
        // Load settings on first render
        if !self.loaded {
            self.load_settings(state, runtime);
            self.loaded = true;
        }

        // Header
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("📧").size(28.0));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Email Reports").size(24.0).strong());
                ui.label(egui::RichText::new("Configure SMTP and automated report sending").size(13.0).color(egui::Color32::from_rgb(148, 163, 184)));
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Send Now button
                let send_btn = egui::Button::new(
                    egui::RichText::new("📤 Send Report Now").color(egui::Color32::WHITE).size(14.0)
                )
                .fill(egui::Color32::from_rgb(34, 197, 94))
                .rounding(egui::Rounding::same(8.0))
                .min_size(egui::vec2(160.0, 36.0));
                
                if ui.add(send_btn).clicked() {
                    self.test_status = Some("⚠️ Send report not yet implemented".to_string());
                }
                
                ui.add_space(8.0);
                
                // Save button
                let save_btn = egui::Button::new(
                    egui::RichText::new("💾 Save Settings").color(egui::Color32::WHITE).size(14.0)
                )
                .fill(egui::Color32::from_rgb(99, 102, 241))
                .rounding(egui::Rounding::same(8.0))
                .min_size(egui::vec2(140.0, 36.0));
                
                if ui.add(save_btn).clicked() {
                    self.save_settings(state, runtime);
                    self.test_status = Some("✓ Settings saved".to_string());
                }
            });
        });

        ui.add_space(20.0);

        // Section tabs in toolbar
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Self::section_tab(ui, &mut self.active_section, EmailSection::Smtp, "📧 SMTP Server");
                    ui.add_space(8.0);
                    Self::section_tab(ui, &mut self.active_section, EmailSection::Template, "📝 Email Template");
                    ui.add_space(8.0);
                    Self::section_tab(ui, &mut self.active_section, EmailSection::AutoSend, "⏰ Auto-Send");
                    ui.add_space(8.0);
                    Self::section_tab(ui, &mut self.active_section, EmailSection::History, "📜 History");
                });
            });

        ui.add_space(16.0);

        // Status message
        if let Some(status) = &self.test_status {
            egui::Frame::none()
                .fill(if status.starts_with('✓') {
                    egui::Color32::from_rgb(34, 197, 94).linear_multiply(0.2)
                } else {
                    egui::Color32::from_rgb(251, 191, 36).linear_multiply(0.2)
                })
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(status).size(13.0));
                });
            ui.add_space(12.0);
        }

        // Content based on active section
        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.active_section {
                EmailSection::Smtp => self.render_smtp_section(ui),
                EmailSection::Template => self.render_template_section(ui),
                EmailSection::AutoSend => self.render_autosend_section(ui),
                EmailSection::History => self.render_history_section(ui),
            }
        });
    }

    fn section_tab(ui: &mut egui::Ui, current: &mut EmailSection, section: EmailSection, label: &str) {
        let is_selected = *current == section;
        let btn = egui::Button::new(
            egui::RichText::new(label)
                .color(if is_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(148, 163, 184) })
        )
        .fill(if is_selected { egui::Color32::from_rgb(99, 102, 241) } else { egui::Color32::from_rgb(55, 55, 75) })
        .rounding(egui::Rounding::same(6.0));
        
        if ui.add(btn).clicked() {
            *current = section;
        }
    }

    fn render_smtp_section(&mut self, ui: &mut egui::Ui) {
        Self::section_card(ui, "SMTP Server Configuration", |ui| {
            Self::form_field(ui, "SMTP Host", &mut self.smtp_host, "smtp.gmail.com");
            Self::form_field(ui, "SMTP Port", &mut self.smtp_port, "587");
            Self::form_field(ui, "Username", &mut self.smtp_username, "your-email@gmail.com");
            
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Password").size(13.0).color(egui::Color32::from_rgb(148, 163, 184)));
                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.smtp_password)
                        .password(true)
                        .desired_width(300.0)
                );
            });
            ui.add_space(8.0);
        });

        ui.add_space(16.0);

        Self::section_card(ui, "Email Addresses", |ui| {
            Self::form_field(ui, "From Address", &mut self.email_from, "reports@yourcompany.com");
            Self::form_field(ui, "To Addresses", &mut self.email_to, "recipient1@email.com, recipient2@email.com");
            ui.label(
                egui::RichText::new("Separate multiple addresses with commas")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(100, 116, 139))
            );
        });

        ui.add_space(16.0);

        // Test connection button
        ui.horizontal(|ui| {
            let test_btn = egui::Button::new("🔌 Test Connection")
                .fill(egui::Color32::from_rgb(55, 55, 75))
                .rounding(egui::Rounding::same(6.0));
            
            if ui.add(test_btn).clicked() {
                self.test_status = Some("⚠️ SMTP connection test not yet implemented".to_string());
            }
            
            ui.add_space(8.0);
            
            let test_email_btn = egui::Button::new("📧 Send Test Email")
                .fill(egui::Color32::from_rgb(55, 55, 75))
                .rounding(egui::Rounding::same(6.0));
            
            if ui.add(test_email_btn).clicked() {
                self.test_status = Some("⚠️ Test email not yet implemented".to_string());
            }
        });
    }

    fn render_template_section(&mut self, ui: &mut egui::Ui) {
        Self::section_card(ui, "Email Subject", |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.email_subject_template)
                        .desired_width(ui.available_width() - 100.0)
                        .hint_text("Cabinet Scan Report - {date}")
                );
                if ui.button("Reset").clicked() {
                    self.email_subject_template = "Cabinet Scan Report - {date}".to_string();
                }
            });
        });

        ui.add_space(16.0);

        Self::section_card(ui, "Email Body Template", |ui| {
            // Template help toggle
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_template_help, "Show template variables");
            });

            if self.show_template_help {
                ui.add_space(8.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(35, 35, 55))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Available Variables:").strong());
                        ui.add_space(4.0);
                        let variables = [
                            ("{date}", "Current date (MM/DD/YYYY)"),
                            ("{time}", "Current time (HH:MM:SS)"),
                            ("{scan_count}", "Total number of scans"),
                            ("{device_count}", "Number of devices"),
                            ("{job_count}", "Number of active jobs"),
                            ("{server_name}", "Server hostname"),
                            ("{report_period}", "Report time period"),
                        ];
                        for (var, desc) in variables {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(var).monospace().color(egui::Color32::from_rgb(139, 92, 246)));
                                ui.label(egui::RichText::new(format!("- {}", desc)).color(egui::Color32::from_rgb(148, 163, 184)));
                            });
                        }
                    });
                ui.add_space(8.0);
            }

            ui.add_space(8.0);

            // Body template editor
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.email_body_template)
                        .desired_width(ui.available_width())
                        .desired_rows(10)
                        .font(egui::TextStyle::Monospace)
                );
            });

            ui.add_space(8.0);

            if ui.button("Reset to Default Template").clicked() {
                self.email_body_template = Self::default_email_template();
            }
        });

        ui.add_space(16.0);

        Self::section_card(ui, "Content Options", |ui| {
            ui.checkbox(&mut self.include_csv_attachment, "📎 Include CSV attachment with scan data");
            ui.add_space(4.0);
            ui.checkbox(&mut self.include_summary_stats, "📊 Include summary statistics");
            ui.add_space(4.0);
            ui.checkbox(&mut self.include_device_info, "📱 Include device information");
            ui.add_space(4.0);
            ui.checkbox(&mut self.include_job_info, "📋 Include job details");
        });
    }

    fn render_autosend_section(&mut self, ui: &mut egui::Ui) {
        Self::section_card(ui, "Automatic Report Sending", |ui| {
            ui.checkbox(&mut self.auto_send_enabled, "Enable automatic email reports");
            
            ui.add_space(12.0);

            if self.auto_send_enabled {
                ui.horizontal(|ui| {
                    ui.label("Send report after");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.auto_send_delay)
                            .desired_width(60.0)
                    );
                    ui.label("minutes of inactivity");
                });

                ui.add_space(12.0);

                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(99, 102, 241).linear_multiply(0.15))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("💡").size(16.0));
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Reports will be sent automatically when no new scans are received for the specified duration.")
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(199, 210, 254))
                            );
                        });
                    });
            }
        });

        ui.add_space(16.0);

        Self::section_card(ui, "Schedule Options", |ui| {
            ui.label(
                egui::RichText::new("Coming soon: Scheduled daily/weekly reports")
                    .color(egui::Color32::from_rgb(148, 163, 184))
                    .italics()
            );
        });
    }

    fn render_history_section(&mut self, ui: &mut egui::Ui) {
        Self::section_card(ui, "Recent Email Sends", |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(egui::RichText::new("📬").size(32.0));
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("No emails sent yet")
                        .size(16.0)
                        .color(egui::Color32::from_rgb(148, 163, 184))
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Email history will appear here after sending reports")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 116, 139))
                );
                ui.add_space(20.0);
            });
        });
    }

    fn section_card(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(45, 45, 65))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(title).size(15.0).strong());
                ui.add_space(12.0);
                content(ui);
            });
    }

    fn form_field(ui: &mut egui::Ui, label: &str, value: &mut String, placeholder: &str) {
        ui.horizontal(|ui| {
            ui.allocate_ui(egui::vec2(120.0, 20.0), |ui| {
                ui.label(egui::RichText::new(label).size(13.0).color(egui::Color32::from_rgb(148, 163, 184)));
            });
            ui.add(
                egui::TextEdit::singleline(value)
                    .desired_width(300.0)
                    .hint_text(placeholder)
            );
        });
        ui.add_space(8.0);
    }

    fn default_email_template() -> String {
        r#"Cabinet Scan Report
═══════════════════════════════════════

Date: {date}
Time: {time}

SUMMARY
───────────────────────────────────────
Total Scans: {scan_count}
Active Devices: {device_count}
Active Jobs: {job_count}

This is an automated report from Cabinet.
Please find the detailed scan data attached as a CSV file.

───────────────────────────────────────
Generated by Cabinet
"#.to_string()
    }

    fn load_settings(&mut self, state: &SharedState, runtime: &tokio::runtime::Handle) {
        let state = state.clone();
        if let Some(config) = runtime.block_on(async {
            let state = state.read().await;
            state.repo.get_email_config().await.ok().flatten()
        }) {
            self.smtp_host = config.smtp_host.unwrap_or_default();
            self.smtp_port = config.smtp_port.map(|p| p.to_string()).unwrap_or_else(|| "587".to_string());
            self.smtp_username = config.smtp_username.unwrap_or_default();
            self.smtp_password = config.smtp_password.unwrap_or_default();
            self.email_from = config.email_from.unwrap_or_default();
            self.email_to = config.email_to.unwrap_or_default();
            self.auto_send_enabled = config.auto_send_enabled != 0;
            self.auto_send_delay = config.auto_send_delay_minutes.to_string();
            self.email_subject_template = config.email_subject_template.unwrap_or_else(|| "Cabinet Scan Report - {date}".to_string());
        } else {
            self.smtp_port = "587".to_string();
            self.auto_send_delay = "30".to_string();
            self.email_subject_template = "Cabinet Scan Report - {date}".to_string();
        }
        
        // Set defaults for new fields
        if self.email_body_template.is_empty() {
            self.email_body_template = Self::default_email_template();
        }
        self.include_csv_attachment = true;
        self.include_summary_stats = true;
        self.include_device_info = true;
        self.include_job_info = true;
    }

    fn save_settings(&self, state: &SharedState, runtime: &tokio::runtime::Handle) {
        let state = state.clone();
        let smtp_host = self.smtp_host.clone();
        let smtp_port: i32 = self.smtp_port.parse().unwrap_or(587);
        let smtp_username = self.smtp_username.clone();
        let smtp_password = self.smtp_password.clone();
        let email_from = self.email_from.clone();
        let email_to = self.email_to.clone();
        let auto_send_enabled = self.auto_send_enabled;
        let auto_send_delay: i32 = self.auto_send_delay.parse().unwrap_or(30);

        runtime.block_on(async {
            let state = state.read().await;
            let _ = state.repo.update_email_config(
                &smtp_host,
                smtp_port,
                &smtp_username,
                &smtp_password,
                &email_from,
                &email_to,
                auto_send_enabled,
                auto_send_delay,
            ).await;
        });

        tracing::info!("Email configuration saved");
    }
}

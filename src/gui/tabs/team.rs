use crate::api::routes::SharedState;
use crate::db::models::{TeamMemberInput, TeamMemberStatus};
use eframe::egui;

/// Team tab showing all team members with current status
pub struct TeamTab {
    pub team_status: Vec<TeamMemberStatus>,
    pub last_refresh: Option<std::time::Instant>,
    pub search_query: String,
    pub show_add_dialog: bool,
    pub new_member_name: String,
    pub new_member_device: String,
    pub new_member_role: String,
    pub new_member_admin: bool,
    pub filter_status: StatusFilter,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum StatusFilter {
    #[default]
    All,
    Working,
    OnBreak,
    Offline,
}

impl Default for TeamTab {
    fn default() -> Self {
        Self {
            team_status: Vec::new(),
            last_refresh: None,
            search_query: String::new(),
            show_add_dialog: false,
            new_member_name: String::new(),
            new_member_device: String::new(),
            new_member_role: "worker".to_string(),
            new_member_admin: false,
            filter_status: StatusFilter::All,
        }
    }
}

impl TeamTab {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &SharedState,
        runtime: &tokio::runtime::Handle,
    ) -> bool {
        let mut needs_refresh = false;

        // Auto-refresh team status every 5 seconds
        let should_refresh = self.last_refresh
            .map(|t| t.elapsed().as_secs() >= 5)
            .unwrap_or(true);

        if should_refresh {
            let s = state.clone();
            self.team_status = runtime.block_on(async {
                let state = s.read().await;
                state.repo.get_team_status().await.unwrap_or_default()
            });
            self.last_refresh = Some(std::time::Instant::now());
        }

        // Header
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("👥").size(28.0));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Team").size(24.0).strong());
                ui.label(
                    egui::RichText::new("Monitor your team's current status and activity")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(148, 163, 184)),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let add_btn = egui::Button::new(
                    egui::RichText::new("➕ Add Member").color(egui::Color32::WHITE).size(14.0),
                )
                .fill(egui::Color32::from_rgb(99, 102, 241))
                .rounding(egui::Rounding::same(8.0))
                .min_size(egui::vec2(130.0, 36.0));

                if ui.add(add_btn).clicked() {
                    self.show_add_dialog = true;
                }
            });
        });

        ui.add_space(20.0);

        // Stats cards
        let working = self.team_status.iter().filter(|m| m.status == "working").count();
        let on_break = self.team_status.iter().filter(|m| m.status == "break").count();
        let offline = self.team_status.iter().filter(|m| m.status == "offline").count();
        let total_scans: i64 = self.team_status.iter().map(|m| m.total_scans_today).sum();

        ui.horizontal(|ui| {
            self.stat_card(ui, "👥", "Total", &self.team_status.len().to_string(), egui::Color32::from_rgb(99, 102, 241));
            ui.add_space(12.0);
            self.stat_card(ui, "🟢", "Working", &working.to_string(), egui::Color32::from_rgb(34, 197, 94));
            ui.add_space(12.0);
            self.stat_card(ui, "☕", "On Break", &on_break.to_string(), egui::Color32::from_rgb(251, 191, 36));
            ui.add_space(12.0);
            self.stat_card(ui, "⚫", "Offline", &offline.to_string(), egui::Color32::from_rgb(107, 114, 128));
            ui.add_space(12.0);
            self.stat_card(ui, "📊", "Scans Today", &total_scans.to_string(), egui::Color32::from_rgb(251, 146, 60));
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
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Search team...")
                            .desired_width(200.0),
                    );

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);

                    ui.label("Filter:");
                    ui.add_space(4.0);
                    for (filter, label) in [
                        (StatusFilter::All, "All"),
                        (StatusFilter::Working, "🟢 Working"),
                        (StatusFilter::OnBreak, "☕ Break"),
                        (StatusFilter::Offline, "⚫ Offline"),
                    ] {
                        let is_selected = self.filter_status == filter;
                        let btn = egui::Button::new(
                            egui::RichText::new(label).color(if is_selected {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_rgb(148, 163, 184)
                            }),
                        )
                        .fill(if is_selected {
                            egui::Color32::from_rgb(99, 102, 241)
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .rounding(egui::Rounding::same(6.0));

                        if ui.add(btn).clicked() {
                            self.filter_status = filter;
                        }
                    }
                });
            });

        ui.add_space(16.0);

        // Team member cards
        let filtered: Vec<&TeamMemberStatus> = self
            .team_status
            .iter()
            .filter(|m| {
                let matches_search = self.search_query.is_empty()
                    || m.display_name.to_lowercase().contains(&self.search_query.to_lowercase())
                    || m.device_id.to_lowercase().contains(&self.search_query.to_lowercase());

                let matches_filter = match self.filter_status {
                    StatusFilter::All => true,
                    StatusFilter::Working => m.status == "working",
                    StatusFilter::OnBreak => m.status == "break",
                    StatusFilter::Offline => m.status == "offline",
                };

                matches_search && matches_filter
            })
            .collect();

        if filtered.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(egui::RichText::new("👥").size(48.0));
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("No team members found")
                        .size(16.0)
                        .color(egui::Color32::from_rgb(148, 163, 184)),
                );
                ui.label(
                    egui::RichText::new("Add team members to start tracking their status")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(107, 114, 128)),
                );
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for member in &filtered {
                    self.member_card(ui, member);
                    ui.add_space(8.0);
                }
            });
        }

        // Add member dialog
        if self.show_add_dialog {
            egui::Window::new("Add Team Member")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(egui::TextEdit::singleline(&mut self.new_member_name).desired_width(200.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Device ID:");
                        ui.add(egui::TextEdit::singleline(&mut self.new_member_device).desired_width(200.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Role:");
                        egui::ComboBox::from_id_source("new_member_role")
                            .selected_text(&self.new_member_role)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.new_member_role, "worker".to_string(), "Worker");
                                ui.selectable_value(&mut self.new_member_role, "lead".to_string(), "Lead");
                                ui.selectable_value(&mut self.new_member_role, "manager".to_string(), "Manager");
                            });
                    });
                    ui.checkbox(&mut self.new_member_admin, "Admin access");

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_add_dialog = false;
                        }

                        let can_save = !self.new_member_name.is_empty() && !self.new_member_device.is_empty();
                        ui.add_enabled_ui(can_save, |ui| {
                            let save_btn = egui::Button::new(
                                egui::RichText::new("Save").color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(99, 102, 241));

                            if ui.add(save_btn).clicked() {
                                let input = TeamMemberInput {
                                    device_id: self.new_member_device.clone(),
                                    display_name: self.new_member_name.clone(),
                                    phone_number: None,
                                    role: Some(self.new_member_role.clone()),
                                    is_admin: Some(self.new_member_admin),
                                    avatar_color: None,
                                };
                                let s = state.clone();
                                runtime.block_on(async {
                                    let state = s.read().await;
                                    let _ = state.repo.upsert_team_member(&input).await;
                                });
                                self.show_add_dialog = false;
                                self.new_member_name.clear();
                                self.new_member_device.clear();
                                self.new_member_role = "worker".to_string();
                                self.new_member_admin = false;
                                self.last_refresh = None; // force refresh
                                needs_refresh = true;
                            }
                        });
                    });
                });
        }

        needs_refresh
    }

    fn stat_card(&self, ui: &mut egui::Ui, icon: &str, label: &str, value: &str, color: egui::Color32) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.set_min_width(100.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(icon).size(20.0));
                    ui.label(egui::RichText::new(value).size(22.0).strong().color(color));
                    ui.label(
                        egui::RichText::new(label)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(148, 163, 184)),
                    );
                });
            });
    }

    fn member_card(&self, ui: &mut egui::Ui, member: &TeamMemberStatus) {
        let border_color = match member.status.as_str() {
            "working" => egui::Color32::from_rgb(34, 197, 94),
            "break" => egui::Color32::from_rgb(251, 191, 36),
            _ => egui::Color32::from_rgb(107, 114, 128),
        };

        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(16.0, 14.0))
            .stroke(egui::Stroke::new(2.0, border_color))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Avatar circle
                    let avatar_color = member
                        .avatar_color
                        .as_deref()
                        .and_then(|c| {
                            let c = c.trim_start_matches('#');
                            if c.len() == 6 {
                                let r = u8::from_str_radix(&c[0..2], 16).ok()?;
                                let g = u8::from_str_radix(&c[2..4], 16).ok()?;
                                let b = u8::from_str_radix(&c[4..6], 16).ok()?;
                                Some(egui::Color32::from_rgb(r, g, b))
                            } else {
                                None
                            }
                        })
                        .unwrap_or(egui::Color32::from_rgb(99, 102, 241));

                    let initials: String = member
                        .display_name
                        .split_whitespace()
                        .take(2)
                        .filter_map(|w| w.chars().next())
                        .collect::<String>()
                        .to_uppercase();

                    let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 20.0, avatar_color);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &initials,
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE,
                    );

                    ui.add_space(12.0);

                    // Name and info
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&member.display_name)
                                    .size(15.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                            if member.is_admin {
                                ui.label(
                                    egui::RichText::new("👑 Admin")
                                        .size(10.0)
                                        .color(egui::Color32::from_rgb(251, 191, 36)),
                                );
                            }
                            ui.label(
                                egui::RichText::new(&member.role)
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(148, 163, 184)),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&member.device_id)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(107, 114, 128)),
                            );
                        });
                    });

                    // Right side: status + stats
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                let (status_icon, status_text, status_color) = match member.status.as_str() {
                                    "working" => ("🟢", "Working", egui::Color32::from_rgb(34, 197, 94)),
                                    "break" => ("☕", "On Break", egui::Color32::from_rgb(251, 191, 36)),
                                    _ => ("⚫", "Offline", egui::Color32::from_rgb(107, 114, 128)),
                                };
                                ui.label(
                                    egui::RichText::new(format!("{} {}", status_icon, status_text))
                                        .color(status_color)
                                        .strong(),
                                );
                            });

                            if let Some(ref job) = member.current_job {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                    ui.label(
                                        egui::RichText::new(job)
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(148, 163, 184)),
                                    );
                                });
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label(
                                    egui::RichText::new(format!("📊 {} scans today", member.total_scans_today))
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(148, 163, 184)),
                                );
                            });
                        });
                    });
                });
            });
    }
}

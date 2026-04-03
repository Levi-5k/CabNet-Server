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
    pub confirm_delete: Option<String>, // device_id pending confirmation
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum StatusFilter {
    #[default]
    All,
    Working,
    OnBreak,
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
            confirm_delete: None,
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
        let total_scans: i64 = self.team_status.iter().map(|m| m.total_scans_today).sum();

        ui.horizontal(|ui| {
            self.stat_card(ui, "👥", "Total", &self.team_status.len().to_string(), egui::Color32::from_rgb(99, 102, 241));
            ui.add_space(12.0);
            self.stat_card(ui, "🟢", "Working", &working.to_string(), egui::Color32::from_rgb(34, 197, 94));
            ui.add_space(12.0);
            self.stat_card(ui, "☕", "On Break", &on_break.to_string(), egui::Color32::from_rgb(251, 191, 36));
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
        let filtered: Vec<TeamMemberStatus> = self
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
                };

                matches_search && matches_filter
            })
            .cloned()
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
            // Collect deferred actions: (device_id, action)
            let mut admin_toggle: Option<(String, bool)> = None;
            let mut role_change: Option<(String, String)> = None;
            let mut delete_request: Option<String> = None;

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Responsive rectangular grid
                let card_width = 300.0_f32;
                let card_h_margin = 12.0_f32;
                let card_stroke = 2.0_f32;
                let card_outer = card_width + (card_h_margin + card_stroke) * 2.0;
                let spacing = 12.0_f32;
                let available = ui.available_width();
                let cols = ((available + spacing) / (card_outer + spacing)).floor().max(1.0) as usize;

                let chunks: Vec<&[TeamMemberStatus]> = filtered.chunks(cols).collect();
                for row in chunks {
                    ui.horizontal(|ui| {
                        for member in row {
                            let border_color = match member.status.as_str() {
                                "working" => egui::Color32::from_rgb(34, 197, 94),
                                "break" => egui::Color32::from_rgb(251, 191, 36),
                                _ => egui::Color32::from_rgb(60, 60, 80),
                            };

                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(40, 40, 58))
                                .rounding(egui::Rounding::same(12.0))
                                .inner_margin(egui::Margin::symmetric(card_h_margin, 10.0))
                                .stroke(egui::Stroke::new(card_stroke, border_color))
                                .show(ui, |ui| {
                                    ui.set_width(card_width);
                                    ui.set_max_width(card_width);

                                    // ── Top row: avatar + name + info ──
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

                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(36.0, 36.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(rect.center(), 18.0, avatar_color);
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            &initials,
                                            egui::FontId::proportional(13.0),
                                            egui::Color32::WHITE,
                                        );

                                        ui.add_space(6.0);

                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&member.display_name)
                                                        .size(14.0)
                                                        .strong()
                                                        .color(egui::Color32::WHITE),
                                                );
                                                if let Some(ref job) = member.current_job {
                                                    ui.label(
                                                        egui::RichText::new(format!("📋 {}", job))
                                                            .size(11.0)
                                                            .color(egui::Color32::from_rgb(148, 163, 184)),
                                                    );
                                                }
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "📊 {} scans today",
                                                        member.total_scans_today
                                                    ))
                                                    .size(11.0)
                                                    .color(egui::Color32::from_rgb(148, 163, 184)),
                                                );
                                            });

                                            // Status badge
                                            let (status_icon, status_text, status_color) =
                                                match member.status.as_str() {
                                                    "working" => (
                                                        "🟢",
                                                        "Working",
                                                        egui::Color32::from_rgb(34, 197, 94),
                                                    ),
                                                    "break" => (
                                                        "☕",
                                                        "On Break",
                                                        egui::Color32::from_rgb(251, 191, 36),
                                                    ),
                                                    _ => (
                                                        "⚫",
                                                        "Offline",
                                                        egui::Color32::from_rgb(107, 114, 128),
                                                    ),
                                                };
                                            ui.label(
                                                egui::RichText::new(format!("{} {}", status_icon, status_text))
                                                    .size(11.0)
                                                    .color(status_color),
                                            );
                                        });
                                    });

                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);

                                    // ── Role + Admin + Delete row ──
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("Role:")
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(148, 163, 184)),
                                        );
                                        let combo_id = format!("role_{}", member.device_id);
                                        let mut current_role = member.role.clone();
                                        let prev_role = current_role.clone();
                                        egui::ComboBox::from_id_source(&combo_id)
                                            .selected_text(&current_role)
                                            .width(80.0)
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut current_role,
                                                    "worker".to_string(),
                                                    "Worker",
                                                );
                                                ui.selectable_value(
                                                    &mut current_role,
                                                    "lead".to_string(),
                                                    "Lead",
                                                );
                                                ui.selectable_value(
                                                    &mut current_role,
                                                    "manager".to_string(),
                                                    "Manager",
                                                );
                                            });
                                        if current_role != prev_role {
                                            role_change = Some((
                                                member.device_id.clone(),
                                                current_role,
                                            ));
                                        }

                                        ui.add_space(4.0);

                                        // Admin toggle
                                        let (admin_label, admin_color) = if member.is_admin {
                                            ("👑 Admin", egui::Color32::from_rgb(251, 191, 36))
                                        } else {
                                            ("User", egui::Color32::from_rgb(107, 114, 128))
                                        };
                                        let admin_btn = egui::Button::new(
                                            egui::RichText::new(admin_label)
                                                .size(11.0)
                                                .color(admin_color),
                                        )
                                        .fill(egui::Color32::from_rgb(30, 30, 46))
                                        .rounding(egui::Rounding::same(4.0));
                                        if ui
                                            .add(admin_btn)
                                            .on_hover_text("Click to toggle admin")
                                            .clicked()
                                        {
                                            admin_toggle =
                                                Some((member.device_id.clone(), !member.is_admin));
                                        }

                                        // Delete button pushed right
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let del_btn = egui::Button::new(
                                                    egui::RichText::new("🗑")
                                                        .size(13.0)
                                                        .color(egui::Color32::from_rgb(239, 68, 68)),
                                                )
                                                .fill(egui::Color32::TRANSPARENT);
                                                if ui
                                                    .add(del_btn)
                                                    .on_hover_text("Delete member")
                                                    .clicked()
                                                {
                                                    delete_request =
                                                        Some(member.device_id.clone());
                                                }
                                            },
                                        );
                                    });
                                });

                            ui.add_space(spacing);
                        }
                    });
                    ui.add_space(spacing);
                }
            });

            // Apply role change
            if let Some((device_id, new_role)) = role_change {
                let s = state.clone();
                runtime.block_on(async {
                    let state = s.read().await;
                    let _ = state.repo.set_team_member_role(&device_id, &new_role).await;
                });
                self.last_refresh = None;
                needs_refresh = true;
            }

            // Apply admin toggle
            if let Some((device_id, new_admin)) = admin_toggle {
                let s = state.clone();
                runtime.block_on(async {
                    let state = s.read().await;
                    let _ = state.repo.set_team_member_admin(&device_id, new_admin).await;
                });
                self.last_refresh = None;
                needs_refresh = true;
            }

            // Request delete confirmation
            if let Some(device_id) = delete_request {
                self.confirm_delete = Some(device_id);
            }
        }

        // ── Delete confirmation dialog ──
        if let Some(ref device_id) = self.confirm_delete.clone() {
            let member_name = self
                .team_status
                .iter()
                .find(|m| &m.device_id == device_id)
                .map(|m| m.display_name.clone())
                .unwrap_or_else(|| device_id.clone());

            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Are you sure you want to delete \"{}\"?",
                            member_name
                        ))
                        .size(14.0)
                        .color(egui::Color32::WHITE),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("This action cannot be undone.")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(239, 68, 68)),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm_delete = None;
                        }
                        ui.add_space(8.0);
                        let del_btn = egui::Button::new(
                            egui::RichText::new("Delete").color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(220, 38, 38));
                        if ui.add(del_btn).clicked() {
                            let did = device_id.clone();
                            let s = state.clone();
                            runtime.block_on(async {
                                let state = s.read().await;
                                let _ = state.repo.close_open_time_entries(&did).await;
                                let _ = state.repo.delete_team_member(&did).await;
                            });
                            self.confirm_delete = None;
                            self.last_refresh = None;
                            needs_refresh = true;
                        }
                    });
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

    /// Returns Some((device_id, new_admin_value)) if admin was toggled
    #[allow(dead_code)]
    fn member_card(&self, _ui: &mut egui::Ui, _member: &TeamMemberStatus) -> Option<(String, bool)> {
        // Grid-based rendering is now inline in ui()
        None
    }
}

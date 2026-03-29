use crate::api::routes::SharedState;
use crate::db::models::{TimesheetDay, LocationPingRecord};
use eframe::egui;

/// Timesheets tab showing time entries grouped by day
/// Admins see all workers; regular users see only their own entries
pub struct TimesheetsTab {
    pub timesheets: Vec<TimesheetDay>,
    pub last_refresh: Option<std::time::Instant>,
    pub filter_device: String,
    pub filter_date_from: String,
    pub filter_date_to: String,
    pub available_workers: Vec<(String, String)>, // (device_id, display_name)
    pub expanded_day: Option<(String, String)>,   // (device_id, date)
    pub show_map_window: bool,
    pub map_pings: Vec<LocationPingRecord>,
    pub map_title: String,
}

impl Default for TimesheetsTab {
    fn default() -> Self {
        Self {
            timesheets: Vec::new(),
            last_refresh: None,
            filter_device: "all".to_string(),
            filter_date_from: String::new(),
            filter_date_to: String::new(),
            available_workers: Vec::new(),
            expanded_day: None,
            show_map_window: false,
            map_pings: Vec::new(),
            map_title: String::new(),
        }
    }
}

impl TimesheetsTab {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &SharedState,
        runtime: &tokio::runtime::Handle,
    ) -> bool {
        let needs_refresh = false;

        // Auto-refresh every 10 seconds
        let should_refresh = self.last_refresh
            .map(|t| t.elapsed().as_secs() >= 10)
            .unwrap_or(true);

        if should_refresh {
            self.refresh_data(state, runtime);
        }

        // Header
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("📋").size(28.0));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Timesheets").size(24.0).strong());
                ui.label(
                    egui::RichText::new("View time entries and GPS tracking for all workers")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(148, 163, 184)),
                );
            });
        });

        ui.add_space(20.0);

        // Summary stats
        let total_days = self.timesheets.len();
        let total_work_hours: f64 = self.timesheets.iter().map(|d| d.total_work_seconds as f64 / 3600.0).sum();
        let total_break_hours: f64 = self.timesheets.iter().map(|d| d.total_break_seconds as f64 / 3600.0).sum();
        let gps_days = self.timesheets.iter().filter(|d| d.has_gps).count();

        ui.horizontal(|ui| {
            self.stat_card(ui, "📅", "Days", &total_days.to_string(), egui::Color32::from_rgb(99, 102, 241));
            ui.add_space(12.0);
            self.stat_card(ui, "🟢", "Work Hours", &format!("{:.1}h", total_work_hours), egui::Color32::from_rgb(34, 197, 94));
            ui.add_space(12.0);
            self.stat_card(ui, "☕", "Break Hours", &format!("{:.1}h", total_break_hours), egui::Color32::from_rgb(251, 191, 36));
            ui.add_space(12.0);
            self.stat_card(ui, "🗺", "GPS Days", &gps_days.to_string(), egui::Color32::from_rgb(139, 92, 246));
        });

        ui.add_space(20.0);

        // Filters toolbar
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Worker:");
                    ui.add_space(4.0);
                    let current_label = if self.filter_device == "all" {
                        "All Workers".to_string()
                    } else {
                        self.available_workers
                            .iter()
                            .find(|(d, _)| d == &self.filter_device)
                            .map(|(_, n)| n.clone())
                            .unwrap_or_else(|| self.filter_device.clone())
                    };
                    let mut changed = false;
                    egui::ComboBox::from_id_source("ts_filter_device")
                        .selected_text(&current_label)
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.filter_device, "all".to_string(), "All Workers").clicked() {
                                changed = true;
                            }
                            for (did, name) in &self.available_workers {
                                if ui.selectable_value(&mut self.filter_device, did.clone(), name).clicked() {
                                    changed = true;
                                }
                            }
                        });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);

                    ui.label("From:");
                    ui.add_space(4.0);
                    if ui.add(
                        egui::TextEdit::singleline(&mut self.filter_date_from)
                            .hint_text("YYYY-MM-DD")
                            .desired_width(100.0),
                    ).changed() {
                        changed = true;
                    }

                    ui.add_space(12.0);
                    ui.label("To:");
                    ui.add_space(4.0);
                    if ui.add(
                        egui::TextEdit::singleline(&mut self.filter_date_to)
                            .hint_text("YYYY-MM-DD")
                            .desired_width(100.0),
                    ).changed() {
                        changed = true;
                    }

                    if changed {
                        self.last_refresh = None; // trigger re-fetch
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let refresh_btn = egui::Button::new("🔄 Refresh")
                            .rounding(egui::Rounding::same(6.0));
                        if ui.add(refresh_btn).clicked() {
                            self.last_refresh = None;
                        }
                    });
                });
            });

        ui.add_space(16.0);

        // Timesheet table
        if self.timesheets.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(egui::RichText::new("📋").size(48.0));
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("No timesheet entries found")
                        .size(16.0)
                        .color(egui::Color32::from_rgb(148, 163, 184)),
                );
                ui.label(
                    egui::RichText::new("Time entries will appear here when workers clock in/out")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(107, 114, 128)),
                );
            });
        } else {
            let days = self.timesheets.clone();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for day in &days {
                    let is_expanded = self.expanded_day.as_ref() == Some(&(day.device_id.clone(), day.date.clone()));
                    self.day_card(ui, day, is_expanded, state, runtime);
                    ui.add_space(6.0);
                }
            });
        }

        // GPS Map Window
        if self.show_map_window {
            let mut open = true;
            egui::Window::new(&self.map_title)
                .open(&mut open)
                .default_size(egui::vec2(500.0, 400.0))
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    if self.map_pings.is_empty() {
                        ui.label(
                            egui::RichText::new("No GPS points recorded for this day")
                                .color(egui::Color32::from_rgb(148, 163, 184)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!("{} GPS points tracked", self.map_pings.len()))
                                .strong(),
                        );
                        ui.add_space(8.0);

                        // GPS points table
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            egui::Grid::new("gps_points_grid")
                                .striped(true)
                                .min_col_width(80.0)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Time").strong());
                                    ui.label(egui::RichText::new("Lat").strong());
                                    ui.label(egui::RichText::new("Lon").strong());
                                    ui.label(egui::RichText::new("Speed").strong());
                                    ui.label(egui::RichText::new("Moving").strong());
                                    ui.end_row();

                                    for ping in &self.map_pings {
                                        let time_short = ping.timestamp.split('T').last()
                                            .unwrap_or(&ping.timestamp)
                                            .trim_end_matches('Z')
                                            .split('.')
                                            .next()
                                            .unwrap_or("");
                                        ui.label(time_short);
                                        ui.label(format!("{:.5}", ping.latitude));
                                        ui.label(format!("{:.5}", ping.longitude));
                                        ui.label(
                                            ping.speed
                                                .map(|s| format!("{:.1}", s))
                                                .unwrap_or_else(|| "-".to_string()),
                                        );
                                        ui.label(if ping.is_moving { "🏃 Yes" } else { "🧍 No" });
                                        ui.end_row();
                                    }
                                });
                        });

                        ui.add_space(8.0);

                        // Open in browser map button
                        if !self.map_pings.is_empty() {
                            if ui.button("🌐 Open in Browser Map").clicked() {
                                // Build a simple Google Maps or OSM link with first point
                                let first = &self.map_pings[0];
                                let url = format!(
                                    "https://www.google.com/maps/@{},{},15z",
                                    first.latitude, first.longitude
                                );
                                let _ = open::that(&url);
                            }
                        }
                    }
                });
            if !open {
                self.show_map_window = false;
            }
        }

        needs_refresh
    }

    fn refresh_data(&mut self, state: &SharedState, runtime: &tokio::runtime::Handle) {
        let s = state.clone();
        let device_filter = if self.filter_device == "all" { None } else { Some(self.filter_device.clone()) };
        let date_from = if self.filter_date_from.is_empty() { None } else { Some(self.filter_date_from.clone()) };
        let date_to = if self.filter_date_to.is_empty() { None } else { Some(self.filter_date_to.clone()) };

        let (timesheets, workers) = runtime.block_on(async {
            let state = s.read().await;
            let ts = state.repo.get_timesheets(
                device_filter.as_deref(),
                date_from.as_deref(),
                date_to.as_deref(),
                1000,
            ).await.unwrap_or_default();

            let members = state.repo.get_team_members().await.unwrap_or_default();
            let workers: Vec<(String, String)> = members.into_iter().map(|m| (m.device_id, m.display_name)).collect();

            (ts, workers)
        });

        self.timesheets = timesheets;
        if !workers.is_empty() {
            self.available_workers = workers;
        } else {
            // Fallback: derive worker list from timesheet data
            let mut seen = std::collections::HashSet::new();
            self.available_workers = self.timesheets.iter()
                .filter(|d| seen.insert(d.device_id.clone()))
                .map(|d| (d.device_id.clone(), d.display_name.clone()))
                .collect();
        }
        self.last_refresh = Some(std::time::Instant::now());
    }

    fn day_card(
        &mut self,
        ui: &mut egui::Ui,
        day: &TimesheetDay,
        is_expanded: bool,
        state: &SharedState,
        runtime: &tokio::runtime::Handle,
    ) {
        let work_h = day.total_work_seconds as f64 / 3600.0;
        let break_h = day.total_break_seconds as f64 / 3600.0;

        // Collect actions from the closure instead of mutating self inside
        let mut toggle_expand = false;
        let mut open_map = false;

        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                // Header row
                ui.horizontal(|ui| {
                    // Date
                    ui.label(
                        egui::RichText::new(&day.date)
                            .size(14.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(12.0);

                    // Worker name
                    ui.label(
                        egui::RichText::new(&day.display_name)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(139, 92, 246)),
                    );

                    ui.add_space(12.0);

                    // Work hours
                    ui.label(
                        egui::RichText::new(format!("🟢 {:.1}h work", work_h))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(34, 197, 94)),
                    );

                    // Break hours
                    if break_h > 0.0 {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!("☕ {:.1}h break", break_h))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(251, 191, 36)),
                        );
                    }

                    // Entries count
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("{} entries", day.entries.len()))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(148, 163, 184)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Expand/collapse
                        let arrow = if is_expanded { "▼" } else { "▶" };
                        if ui.button(arrow).clicked() {
                            toggle_expand = true;
                        }

                        // GPS map button
                        if day.has_gps {
                            let map_btn = egui::Button::new(
                                egui::RichText::new("🗺 Map").color(egui::Color32::WHITE).size(12.0),
                            )
                            .fill(egui::Color32::from_rgb(139, 92, 246))
                            .rounding(egui::Rounding::same(6.0));

                            if ui.add(map_btn).clicked() {
                                open_map = true;
                            }
                        }
                    });
                });

                // Expanded: show individual entries
                if is_expanded {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    egui::Grid::new(format!("ts_entries_{}_{}", day.device_id, day.date))
                        .striped(true)
                        .min_col_width(80.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Type").strong().size(11.0));
                            ui.label(egui::RichText::new("Job").strong().size(11.0));
                            ui.label(egui::RichText::new("Clock In").strong().size(11.0));
                            ui.label(egui::RichText::new("Clock Out").strong().size(11.0));
                            ui.label(egui::RichText::new("Duration").strong().size(11.0));
                            ui.label(egui::RichText::new("Note").strong().size(11.0));
                            ui.end_row();

                            for entry in &day.entries {
                                let type_label = if entry.is_break != 0 {
                                    egui::RichText::new("☕ Break").color(egui::Color32::from_rgb(251, 191, 36)).size(11.0)
                                } else {
                                    egui::RichText::new("🟢 Work").color(egui::Color32::from_rgb(34, 197, 94)).size(11.0)
                                };
                                ui.label(type_label);

                                let job = entry.job_name.as_deref()
                                    .or(entry.customer_name.as_deref())
                                    .unwrap_or("-");
                                ui.label(egui::RichText::new(job).size(11.0));

                                let clock_in_short = entry.clock_in.split('T').last()
                                    .unwrap_or(&entry.clock_in)
                                    .trim_end_matches('Z')
                                    .split('.')
                                    .next()
                                    .unwrap_or("");
                                ui.label(egui::RichText::new(clock_in_short).size(11.0));

                                let clock_out_short = entry.clock_out.as_deref()
                                    .map(|s| s.split('T').last().unwrap_or(s).trim_end_matches('Z').split('.').next().unwrap_or(""))
                                    .unwrap_or("—");
                                ui.label(egui::RichText::new(clock_out_short).size(11.0));

                                // Duration
                                if let Some(ref co) = entry.clock_out {
                                    let start = chrono::DateTime::parse_from_rfc3339(&entry.clock_in).ok();
                                    let end = chrono::DateTime::parse_from_rfc3339(co).ok();
                                    if let (Some(s), Some(e)) = (start, end) {
                                        let secs = (e - s).num_seconds().max(0);
                                        let h = secs / 3600;
                                        let m = (secs % 3600) / 60;
                                        ui.label(egui::RichText::new(if h > 0 { format!("{}h {}m", h, m) } else { format!("{}m", m) }).size(11.0));
                                    } else {
                                        ui.label(egui::RichText::new("-").size(11.0));
                                    }
                                } else {
                                    ui.label(egui::RichText::new("Active...").size(11.0).color(egui::Color32::from_rgb(34, 197, 94)));
                                }

                                let note = entry.note.as_deref().unwrap_or("");
                                ui.label(egui::RichText::new(note).size(11.0).color(egui::Color32::from_rgb(148, 163, 184)));

                                ui.end_row();
                            }
                        });
                }
            });

        // Apply mutations after all UI closures are done
        if toggle_expand {
            if is_expanded {
                self.expanded_day = None;
            } else {
                self.expanded_day = Some((day.device_id.clone(), day.date.clone()));
            }
        }

        if open_map {
            let entry_uuids: Vec<String> = day.entries.iter().map(|e| e.uuid.clone()).collect();
            let s = state.clone();
            let pings = runtime.block_on(async {
                let state = s.read().await;
                let mut all_pings = Vec::new();
                for uuid in &entry_uuids {
                    if let Ok(pings) = state.repo.get_location_pings(
                        None,
                        Some(uuid),
                        None,
                        false,
                        5000,
                    ).await {
                        all_pings.extend(pings);
                    }
                }
                all_pings
            });
            self.map_pings = pings;
            self.map_title = format!("GPS Track – {} – {}", day.display_name, day.date);
            self.show_map_window = true;
        }
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
}

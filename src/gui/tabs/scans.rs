use crate::api::routes::SharedState;
use crate::gui::app::CachedData;
use eframe::egui;

/// Scans tab for viewing all barcode scans grouped by time
pub struct ScansTab {
    pub search_query: String,
    pub filter_job: Option<i64>,
    pub time_gap_hours: f32,
    pub expanded_groups: std::collections::HashSet<usize>,
    /// Which group currently has the kebab popup open (None = closed)
    pub kebab_open: Option<usize>,
    /// Selected job_id for the kebab edit (per group index)
    pub kebab_job: Option<i64>,
    /// Selected device_id for the kebab edit (per group index)
    pub kebab_device: Option<String>,
    /// Per-card search queries (group index -> search text)
    pub card_search: std::collections::HashMap<usize, String>,
}

impl Default for ScansTab {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            filter_job: None,
            time_gap_hours: 8.0,
            expanded_groups: std::collections::HashSet::new(),
            kebab_open: None,
            kebab_job: None,
            kebab_device: None,
            card_search: std::collections::HashMap::new(),
        }
    }
}

impl ScansTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, data: &CachedData, state: &SharedState, runtime: &tokio::runtime::Handle) -> bool {
        let mut needs_refresh = false;

        // Header
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("📷").size(28.0));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Scans").size(24.0).strong());
                ui.label(egui::RichText::new("View all barcode scans grouped by time").size(13.0).color(egui::Color32::from_rgb(148, 163, 184)));
            });
        });

        ui.add_space(20.0);

        // Stats
        let total_scans = data.scans.len();
        let total_jobs = data.jobs.len();
        let today_scans = data.scans.iter().filter(|s| {
            s.scanned_at.starts_with(&chrono::Local::now().format("%Y-%m-%d").to_string())
        }).count();
        ui.horizontal(|ui| {
            Self::stat_card(ui, "📷", "Total Scans", &total_scans.to_string(), egui::Color32::from_rgb(99, 102, 241));
            ui.add_space(12.0);
            Self::stat_card(ui, "📅", "Today", &today_scans.to_string(), egui::Color32::from_rgb(34, 197, 94));
            ui.add_space(12.0);
            Self::stat_card(ui, "📋", "Jobs", &total_jobs.to_string(), egui::Color32::from_rgb(251, 146, 60));
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
                            .hint_text("Search barcode...")
                            .desired_width(150.0),
                    );

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);

                    // Job filter
                    ui.label("Job:");
                    ui.add_space(4.0);
                    egui::ComboBox::from_id_source("scans_job_filter")
                        .selected_text(match self.filter_job {
                            None => "All Jobs".to_string(),
                            Some(id) => data.jobs.iter()
                                .find(|j| j.id == id)
                                .map(|j| j.name.clone())
                                .unwrap_or_else(|| format!("Job {}", id)),
                        })
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.filter_job, None, "All Jobs").clicked() {
                                self.expanded_groups.clear();
                            }
                            for job in &data.jobs {
                                if ui.selectable_value(&mut self.filter_job, Some(job.id), &job.name).clicked() {
                                    self.expanded_groups.clear();
                                }
                            }
                        });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);

                    // Group by dropdown
                    ui.label("Group by:");
                    ui.add_space(4.0);
                    egui::ComboBox::from_id_source("scans_time_gap")
                        .selected_text(format!("{:.0}h", self.time_gap_hours))
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for hours in [1.0, 2.0, 4.0, 8.0, 24.0] {
                                if ui.selectable_value(&mut self.time_gap_hours, hours, format!("{:.0} hour", hours)).clicked() {
                                    self.expanded_groups.clear();
                                }
                            }
                        });
                });
            });

        ui.add_space(16.0);

        // Filter scans
        let filtered_scans: Vec<_> = data
            .scans
            .iter()
            .filter(|s| {
                let job_match = self.filter_job.map_or(true, |id| s.job_id == Some(id));
                let search_match = self.search_query.is_empty()
                    || s.barcode.to_lowercase().contains(&self.search_query.to_lowercase())
                    || s.ticket_number.as_ref().map_or(false, |t| t.to_lowercase().contains(&self.search_query.to_lowercase()));
                job_match && search_match
            })
            .collect();

        // Export button
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let export_btn = egui::Button::new(egui::RichText::new("📄 Export CSV").color(egui::Color32::WHITE).size(12.0))
                    .fill(egui::Color32::from_rgb(34, 197, 94))
                    .rounding(egui::Rounding::same(6.0));
                if ui.add(export_btn).clicked() {
                    let scans: Vec<_> = filtered_scans.iter().map(|s| (*s).clone()).collect();
                    let csv_data = crate::services::email::generate_csv_report(&scans);
                    match csv_data {
                        Ok(data) => {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Save All Scans CSV")
                                .set_file_name("all_scans.csv")
                                .add_filter("CSV Files", &["csv"])
                                .save_file()
                            {
                                match std::fs::write(&path, data) {
                                    Ok(_) => tracing::info!("Exported {} scans to {}", filtered_scans.len(), path.display()),
                                    Err(e) => tracing::error!("Failed to save CSV: {}", e),
                                }
                            }
                        }
                        Err(e) => tracing::error!("Failed to generate CSV: {}", e),
                    }
                }
            });
        });

        ui.add_space(8.0);

        if filtered_scans.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No scans found.\n\nScans will appear here when received from the Android app.");
            });
            return false;
        }

        // Group scans by time
        let groups = self.group_scans_by_time(&filtered_scans, data);

        // Show groups as card grid
        let card_width = 240.0_f32;
        let card_spacing = 12.0_f32;
        let accent = egui::Color32::from_rgb(99, 102, 241);
        let card_bg = egui::Color32::from_rgb(55, 55, 80);
        let card_border = egui::Color32::from_rgb(80, 80, 110);
        let muted = egui::Color32::from_rgb(160, 160, 190);

        let panel_width = ui.available_width();
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(panel_width);
            let available = ui.available_width();
            let cols = ((available + card_spacing) / (card_width + card_spacing)).floor().max(1.0) as usize;

            let mut idx = 0;
            while idx < groups.len() {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = card_spacing;
                    for _ in 0..cols {
                        if idx >= groups.len() { break; }
                        let group = &groups[idx];
                        let is_expanded = self.expanded_groups.contains(&idx);
                        let current_idx = idx;
                        idx += 1;

                        let time_str = if group.scans.len() == 1 {
                            format_datetime(&group.start_time)
                        } else {
                            format!(
                                "{} - {} on {}",
                                format_time(&group.end_time),
                                format_time(&group.start_time),
                                format_date(&group.start_time)
                            )
                        };
                        let job_label = group.job_name.as_deref().unwrap_or("Unknown Job");
                        let inner_width = card_width - 24.0;

                        let frame_resp = egui::Frame::none()
                            .fill(card_bg)
                            .rounding(egui::Rounding::same(10.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .stroke(egui::Stroke::new(1.5, card_border))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(inner_width);

                                    // Header: job name, time, scan count — clickable to expand/collapse
                                    let header_resp = ui.scope(|ui| {
                                        ui.label(
                                            egui::RichText::new(job_label)
                                                .strong()
                                                .size(15.0)
                                                .color(accent),
                                        );
                                        ui.add_space(2.0);
                                        ui.label(
                                            egui::RichText::new(&time_str)
                                                .size(13.0)
                                                .color(muted),
                                        );
                                        ui.add_space(2.0);
                                        ui.label(
                                            egui::RichText::new(format!("{} scans", group.scans.len()))
                                                .size(13.0)
                                                .color(egui::Color32::from_rgb(130, 130, 160)),
                                        );
                                    });

                                    // Toggle expand/collapse on header click
                                    if header_resp.response.interact(egui::Sense::click()).clicked() {
                                        if self.kebab_open.is_some() {
                                            self.kebab_open = None;
                                        } else if is_expanded {
                                            self.expanded_groups.remove(&current_idx);
                                            self.card_search.remove(&current_idx);
                                        } else {
                                            self.expanded_groups.insert(current_idx);
                                        }
                                    }

                                    // Expanded scan list
                                    if is_expanded {
                                        ui.add_space(6.0);
                                        ui.separator();

                                        // Kebab ⋮ button - right aligned
                                        let kebab_id = egui::Id::new("kebab_popup").with(current_idx);
                                        ui.horizontal(|ui| {
                                            ui.add_space(ui.available_width() - 20.0);
                                            let kebab_btn = ui.add(
                                                egui::Button::new(egui::RichText::new("⋮").size(14.0).color(muted))
                                                    .fill(egui::Color32::TRANSPARENT)
                                                    .stroke(egui::Stroke::NONE)
                                            );
                                            if kebab_btn.clicked() {
                                                if self.kebab_open == Some(current_idx) {
                                                    self.kebab_open = None;
                                                } else {
                                                    let first_job = group.scans.iter().find_map(|s| s.job_id);
                                                    let first_device = group.scans.first().map(|s| s.device_id.clone());
                                                    self.kebab_job = first_job;
                                                    self.kebab_device = first_device;
                                                    self.kebab_open = Some(current_idx);
                                                }
                                            }
                                        });

                                        // Kebab popup as a floating Area
                                        if self.kebab_open == Some(current_idx) {
                                            egui::Area::new(kebab_id)
                                                .order(egui::Order::Foreground)
                                                .current_pos(ui.next_widget_position())
                                                .show(ui.ctx(), |ui| {
                                                    egui::Frame::popup(ui.style())
                                                        .fill(egui::Color32::from_rgb(45, 45, 65))
                                                        .rounding(egui::Rounding::same(8.0))
                                                        .stroke(egui::Stroke::new(1.0, card_border))
                                                        .inner_margin(egui::Margin::same(10.0))
                                                        .show(ui, |ui| {
                                                            ui.set_width(200.0);
                                                            ui.label(egui::RichText::new("Assign Group").strong().size(12.0).color(accent));
                                                            ui.add_space(6.0);

                                                            ui.label(egui::RichText::new("Job:").size(11.0).color(muted));
                                                            let job_text = self.kebab_job
                                                                .and_then(|id| data.jobs.iter().find(|j| j.id == id))
                                                                .map(|j| j.name.clone())
                                                                .unwrap_or_else(|| "None".to_string());
                                                            egui::ComboBox::from_id_source(egui::Id::new("kebab_job_cb").with(current_idx))
                                                                .selected_text(&job_text)
                                                                .width(180.0)
                                                                .show_ui(ui, |ui| {
                                                                    ui.selectable_value(&mut self.kebab_job, None, "None");
                                                                    for job in &data.jobs {
                                                                        ui.selectable_value(&mut self.kebab_job, Some(job.id), &job.name);
                                                                    }
                                                                });

                                                            ui.add_space(4.0);
                                                            ui.label(egui::RichText::new("Device:").size(11.0).color(muted));
                                                            let dev_text = self.kebab_device.as_ref()
                                                                .and_then(|did| data.devices.iter().find(|d| &d.device.device_id == did))
                                                                .and_then(|d| d.device.device_name.clone())
                                                                .unwrap_or_else(|| self.kebab_device.as_deref().unwrap_or("None").to_string());
                                                            egui::ComboBox::from_id_source(egui::Id::new("kebab_dev_cb").with(current_idx))
                                                                .selected_text(&dev_text)
                                                                .width(180.0)
                                                                .show_ui(ui, |ui| {
                                                                    for dev in &data.devices {
                                                                        let label = dev.device.device_name.as_deref().unwrap_or(&dev.device.device_id);
                                                                        ui.selectable_value(&mut self.kebab_device, Some(dev.device.device_id.clone()), label);
                                                                    }
                                                                });

                                                            ui.add_space(8.0);
                                                            if ui.add(
                                                                egui::Button::new(egui::RichText::new("Export CSV").size(12.0))
                                                                    .fill(egui::Color32::from_rgb(34, 197, 94))
                                                                    .rounding(egui::Rounding::same(4.0))
                                                                    .min_size(egui::vec2(180.0, 24.0))
                                                            ).clicked() {
                                                                // Generate CSV for this group
                                                                let scans: Vec<_> = group.scans.iter().map(|s| (*s).clone()).collect();
                                                                let csv_data = crate::services::email::generate_csv_report(&scans);
                                                                match csv_data {
                                                                    Ok(data) => {
                                                                        // Open save dialog
                                                                        if let Some(path) = rfd::FileDialog::new()
                                                                            .set_title("Save Scan Group CSV")
                                                                            .set_file_name(&format!("scan_group_{}_{}.csv", 
                                                                                group.job_name.as_deref().unwrap_or("unknown"),
                                                                                format_datetime(&group.start_time).replace(['/', ':', ' '], "_").replace("__", "_")
                                                                            ))
                                                                            .add_filter("CSV Files", &["csv"])
                                                                            .save_file()
                                                                        {
                                                                            match std::fs::write(&path, data) {
                                                                                Ok(_) => {
                                                                                    tracing::info!("Exported {} scans to {}", group.scans.len(), path.display());
                                                                                }
                                                                                Err(e) => {
                                                                                    tracing::error!("Failed to save CSV: {}", e);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        tracing::error!("Failed to generate CSV: {}", e);
                                                                    }
                                                                }
                                                                self.kebab_open = None;
                                                            }

                                                            ui.add_space(8.0);
                                                        });
                                                });
                                        }

                                        // Search box for this card
                                        let card_query = self.card_search.entry(current_idx).or_default();
                                        ui.add(
                                            egui::TextEdit::singleline(card_query)
                                                .hint_text("🔍 Search scans...")
                                                .desired_width(inner_width - 12.0)
                                                .font(egui::TextStyle::Body)
                                        );
                                        let filter_text = card_query.to_lowercase();

                                        ui.add_space(4.0);

                                        // Scan list with max height scroll
                                        let max_scan_height = 400.0;
                                        egui::ScrollArea::vertical()
                                            .id_source(egui::Id::new("scan_scroll").with(current_idx))
                                            .max_height(max_scan_height)
                                            .show(ui, |ui| {
                                                ui.set_min_width(inner_width);
                                                for scan in &group.scans {
                                                    let ticket = scan.ticket_number.as_deref()
                                                        .unwrap_or_else(|| strip_job_prefix(&scan.barcode));
                                                    if !filter_text.is_empty()
                                                        && !ticket.to_lowercase().contains(&filter_text)
                                                        && !scan.barcode.to_lowercase().contains(&filter_text)
                                                    {
                                                        continue;
                                                    }
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(ticket).monospace().size(14.0).color(accent));
                                                        ui.add_space(8.0);
                                                        ui.label(egui::RichText::new(format_time(&scan.scanned_at)).size(13.0).color(muted));

                                                        // Map pin button if scan has GPS coords
                                                        let has_gps = scan.latitude.map_or(false, |lat| lat != 0.0)
                                                            || scan.longitude.map_or(false, |lon| lon != 0.0);
                                                        if has_gps {
                                                            if let (Some(lat), Some(lon)) = (scan.latitude, scan.longitude) {
                                                                let pin_btn = egui::Button::new(
                                                                    egui::RichText::new("📍").size(13.0)
                                                                )
                                                                .fill(egui::Color32::TRANSPARENT)
                                                                .stroke(egui::Stroke::NONE);
                                                                if ui.add(pin_btn).on_hover_text(format!("{:.4}, {:.4}", lat, lon)).clicked() {
                                                                    let url = format!(
                                                                        "https://www.openstreetmap.org/?mlat={}&mlon={}#map=18/{}/{}",
                                                                        lat, lon, lat, lon
                                                                    );
                                                                    let _ = open::that(&url);
                                                                }
                                                            }
                                                        }
                                                    });
                                                }
                                            });
                                    }
                                });
                            });

                        // For collapsed cards, also allow clicking anywhere on the card
                        if !is_expanded {
                            if frame_resp.response.interact(egui::Sense::click()).clicked() {
                                if self.kebab_open.is_some() {
                                    self.kebab_open = None;
                                } else {
                                    self.expanded_groups.insert(current_idx);
                                }
                            }
                        }
                    }
                });
                ui.add_space(card_spacing);
            }
        });

        needs_refresh
    }

    fn stat_card(ui: &mut egui::Ui, icon: &str, label: &str, value: &str, color: egui::Color32) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_width(140.0);
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

    fn group_scans_by_time<'a>(&self, scans: &[&'a crate::db::models::Scan], data: &CachedData) -> Vec<ScanGroup<'a>> {
        let gap_ms = (self.time_gap_hours * 3600.0 * 1000.0) as i64;
        
        // Sort by time (newest first)
        let mut sorted: Vec<_> = scans.iter().map(|s| *s).collect();
        sorted.sort_by(|a, b| b.scanned_at.cmp(&a.scanned_at));
        
        let mut groups: Vec<ScanGroup> = Vec::new();
        
        for scan in sorted {
            let scan_time = parse_timestamp(&scan.scanned_at);
            
            let should_new_group = groups.last().map_or(true, |g| {
                let last_time = parse_timestamp(&g.start_time);
                (last_time - scan_time) > gap_ms
            });
            
            if should_new_group {
                groups.push(ScanGroup {
                    start_time: scan.scanned_at.clone(),
                    end_time: scan.scanned_at.clone(),
                    scans: vec![scan],
                    job_name: None,
                });
            } else if let Some(group) = groups.last_mut() {
                group.end_time = scan.scanned_at.clone();
                group.scans.push(scan);
            }
        }
        
        // Resolve job name for each group from the first barcode_job_ref found
        for group in &mut groups {
            // Look through scans for the first barcode_job_ref
            let first_ref = group.scans.iter()
                .filter_map(|s| s.barcode_job_ref.as_deref())
                .find(|r| !r.is_empty());
            
            // Try matching barcode_job_ref to a job's reference_number
            let name_from_ref = first_ref.and_then(|ref_num| {
                data.jobs.iter()
                    .find(|j| j.reference_number.as_deref() == Some(ref_num))
                    .map(|j| j.name.clone())
            });

            if let Some(name) = name_from_ref {
                group.job_name = Some(name);
            } else {
                // Fallback to job_id lookup
                let job_ids: std::collections::HashSet<_> = group.scans.iter()
                    .filter_map(|s| s.job_id)
                    .collect();
                
                group.job_name = if job_ids.len() == 1 {
                    let id = *job_ids.iter().next().unwrap();
                    data.jobs.iter().find(|j| j.id == id).map(|j| j.name.clone())
                } else if job_ids.len() > 1 {
                    Some("Mixed".to_string())
                } else {
                    None
                };
            }
        }
        
        groups
    }
}

struct ScanGroup<'a> {
    start_time: String,
    end_time: String,
    scans: Vec<&'a crate::db::models::Scan>,
    job_name: Option<String>,
}

fn parse_timestamp(ts: &str) -> i64 {
    // Parse ISO timestamp to milliseconds since epoch
    chrono::DateTime::parse_from_rfc3339(ts)
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S")
            .map(|dt| dt.and_utc().fixed_offset()))
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}

fn to_local(ts: &str) -> Option<chrono::DateTime<chrono::Local>> {
    use chrono::TimeZone;
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&chrono::Local))
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S")
                .ok()
                .and_then(|dt| chrono::Local.from_local_datetime(&dt).single())
        })
}

fn format_datetime(ts: &str) -> String {
    to_local(ts)
        .map(|dt| dt.format("%m/%d/%Y %I:%M %p").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn format_date(ts: &str) -> String {
    to_local(ts)
        .map(|dt| dt.format("%m/%d/%Y").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn format_time(ts: &str) -> String {
    to_local(ts)
        .map(|dt| dt.format("%I:%M %p").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn strip_job_prefix(barcode: &str) -> &str {
    // Match pattern: digits followed by zeros, then the actual ticket number
    if let Some(pos) = barcode.find(|c: char| c == '0').and_then(|first_zero| {
        // Find where the zeros end and non-zero digits begin
        barcode[first_zero..].find(|c: char| c != '0').map(|p| first_zero + p)
    }) {
        &barcode[pos..]
    } else {
        barcode
    }
}

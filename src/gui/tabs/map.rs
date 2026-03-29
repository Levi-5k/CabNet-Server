use crate::gui::app::CachedData;
use eframe::egui;

/// Map tab showing scan locations with time-based grouping
pub struct MapTab {
    pub search_query: String,
    pub time_gap_hours: f32,
    pub expanded_groups: std::collections::HashSet<usize>,
}

impl Default for MapTab {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            time_gap_hours: 8.0,
            expanded_groups: std::collections::HashSet::new(),
        }
    }
}

impl MapTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, data: &CachedData) {
        ui.horizontal(|ui| {
            ui.heading("🗺️ Scan Locations");

            // Right-aligned controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Open in Map button
                if ui.button("🌍 Open All in Map").clicked() {
                    if let Some(loc) = data.scan_locations.first() {
                        let url = format!(
                            "https://www.openstreetmap.org/?mlat={}&mlon={}#map=14/{}/{}",
                            loc.latitude, loc.longitude, loc.latitude, loc.longitude
                        );
                        let _ = open::that(&url);
                    } else {
                        let _ = open::that("https://www.openstreetmap.org/#map=4/39.83/-98.58");
                    }
                }

                ui.add_space(16.0);

                // Group by dropdown
                ui.label("Group by:");
                ui.add_space(4.0);
                egui::ComboBox::from_id_source("map_time_gap")
                    .selected_text(format!("{:.0}h", self.time_gap_hours))
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for hours in [1.0, 2.0, 4.0, 8.0, 24.0] {
                            if ui.selectable_value(&mut self.time_gap_hours, hours, format!("{:.0} hour", hours)).clicked() {
                                self.expanded_groups.clear();
                            }
                        }
                    });

                ui.add_space(16.0);

                // Search box
                ui.label("🔍");
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search barcode...")
                        .desired_width(150.0),
                );
            });
        });

        ui.separator();

        // Count scans with GPS
        let scans_with_gps: Vec<_> = data.scans.iter()
            .filter(|s| {
                let has_coords = s.latitude.map_or(false, |lat| lat != 0.0) 
                    || s.longitude.map_or(false, |lon| lon != 0.0);
                let has_location = s.location.as_ref().map_or(false, |loc| loc.contains(','));
                has_coords || has_location
            })
            .collect();

        ui.label(format!(
            "📍 {} of {} scans have GPS coordinates",
            scans_with_gps.len(),
            data.scans.len()
        ));

        if scans_with_gps.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("📭").size(64.0));
                ui.add_space(15.0);
                ui.label(egui::RichText::new("No scans with GPS coordinates").size(18.0));
                ui.add_space(5.0);
                ui.label(egui::RichText::new("Scans with location data from the Android app will appear here.").weak());
                ui.add_space(20.0);
                ui.label(egui::RichText::new("💡 Tip: Make sure GPS is enabled on the scanning device.").small().weak());
            });
            return;
        }

        // Filter scans
        let filtered_scans: Vec<_> = scans_with_gps.iter()
            .filter(|s| {
                self.search_query.is_empty()
                    || s.barcode.to_lowercase().contains(&self.search_query.to_lowercase())
                    || s.ticket_number.as_ref().map_or(false, |t| t.to_lowercase().contains(&self.search_query.to_lowercase()))
            })
            .collect();

        // Group scans by time
        let groups = self.group_scans_by_time(&filtered_scans, data);
        
        ui.label(format!("{} locations in {} groups", filtered_scans.len(), groups.len()));
        ui.separator();

        // Show groups
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, group) in groups.iter().enumerate() {
                let is_expanded = self.expanded_groups.contains(&idx);
                
                // Group header
                let header_response = ui.horizontal(|ui| {
                    let arrow = if is_expanded { "▼" } else { "▶" };
                    ui.label(arrow);
                    
                    ui.label("🗺️");
                    
                    // Job name before time range
                    if let Some(job_name) = &group.job_name {
                        ui.strong(format!("{} —", job_name));
                    }
                    
                    // Time range
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
                    ui.strong(&time_str);
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🌍 View").clicked() {
                            // Open first scan in group
                            if let Some(scan) = group.scans.first() {
                                if let (Some(lat), Some(lon)) = (scan.latitude, scan.longitude) {
                                    let url = format!(
                                        "https://www.openstreetmap.org/?mlat={}&mlon={}#map=16/{}/{}",
                                        lat, lon, lat, lon
                                    );
                                    let _ = open::that(&url);
                                }
                            }
                        }
                        
                        ui.label(format!("{} scans", group.scans.len()));
                    });
                });
                
                if header_response.response.interact(egui::Sense::click()).clicked() {
                    if is_expanded {
                        self.expanded_groups.remove(&idx);
                    } else {
                        self.expanded_groups.insert(idx);
                    }
                }
                
                // Expanded content
                if is_expanded {
                    ui.indent(format!("map_group_{}", idx), |ui| {
                        egui::Grid::new(format!("map_group_grid_{}", idx))
                            .num_columns(5)
                            .striped(true)
                            .min_col_width(80.0)
                            .show(ui, |ui| {
                                ui.strong("Barcode");
                                ui.strong("Coordinates");
                                ui.strong("Device");
                                ui.strong("Time");
                                ui.strong("Actions");
                                ui.end_row();

                                for scan in &group.scans {
                                    let coords = if let (Some(lat), Some(lon)) = (scan.latitude, scan.longitude) {
                                        format!("{:.4}, {:.4}", lat, lon)
                                    } else if let Some(loc) = &scan.location {
                                        loc.clone()
                                    } else {
                                        "-".to_string()
                                    };
                                    
                                    ui.label(&scan.barcode);
                                    ui.label(&coords);
                                    ui.label(&scan.device_id[..scan.device_id.len().min(8)]);
                                    ui.label(format_time(&scan.scanned_at));
                                    
                                    if let (Some(lat), Some(lon)) = (scan.latitude, scan.longitude) {
                                        if ui.small_button("📍").clicked() {
                                            let url = format!(
                                                "https://www.openstreetmap.org/?mlat={}&mlon={}#map=18/{}/{}",
                                                lat, lon, lat, lon
                                            );
                                            let _ = open::that(&url);
                                        }
                                    } else {
                                        ui.label("-");
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                }
                
                ui.add_space(4.0);
            }
        });
    }

    fn group_scans_by_time<'a>(&self, scans: &[&&'a crate::db::models::Scan], _data: &CachedData) -> Vec<ScanGroup<'a>> {
        let gap_ms = (self.time_gap_hours * 3600.0 * 1000.0) as i64;
        
        // Sort by time (newest first)
        let mut sorted: Vec<_> = scans.iter().map(|s| **s).collect();
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
            let first_ref = group.scans.iter()
                .filter_map(|s| s.barcode_job_ref.as_deref())
                .find(|r| !r.is_empty());
            
            let name_from_ref = first_ref.and_then(|ref_num| {
                _data.jobs.iter()
                    .find(|j| j.reference_number.as_deref() == Some(ref_num))
                    .map(|j| j.name.clone())
            });

            if let Some(name) = name_from_ref {
                group.job_name = Some(name);
            } else {
                let job_ids: std::collections::HashSet<_> = group.scans.iter()
                    .filter_map(|s| s.job_id)
                    .collect();
                
                group.job_name = if job_ids.len() == 1 {
                    let id = *job_ids.iter().next().unwrap();
                    _data.jobs.iter().find(|j| j.id == id).map(|j| j.name.clone())
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

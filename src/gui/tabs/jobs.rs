use crate::api::routes::SharedState;
use crate::db::models::JobInput;
use crate::gui::app::CachedData;
use eframe::egui;

/// Jobs tab for managing jobs
pub struct JobsTab {
    pub show_create_dialog: bool,
    pub new_job_name: String,
    pub new_job_description: String,
    pub new_job_reference: String,
    pub filter_status: String,
    pub search_query: String,
    pub expanded_jobs: std::collections::HashSet<i64>,
    /// Which job currently has the edit panel open
    pub editing_job: Option<i64>,
    /// Edit fields
    pub edit_name: String,
    pub edit_reference: String,
    pub edit_description: String,
    pub edit_status: String,
    pub edit_priority: String,
    pub edit_customer: String,
    pub edit_notes: String,
}

impl Default for JobsTab {
    fn default() -> Self {
        Self {
            show_create_dialog: false,
            new_job_name: String::new(),
            new_job_description: String::new(),
            new_job_reference: String::new(),
            filter_status: String::new(),
            search_query: String::new(),
            expanded_jobs: std::collections::HashSet::new(),
            editing_job: None,
            edit_name: String::new(),
            edit_reference: String::new(),
            edit_description: String::new(),
            edit_status: String::new(),
            edit_priority: String::new(),
            edit_customer: String::new(),
            edit_notes: String::new(),
        }
    }
}

impl JobsTab {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        data: &CachedData,
        state: &SharedState,
        runtime: &tokio::runtime::Handle,
    ) -> bool {
        let mut needs_refresh = false;

        let accent = egui::Color32::from_rgb(99, 102, 241);
        let card_bg = egui::Color32::from_rgb(55, 55, 80);
        let card_border = egui::Color32::from_rgb(80, 80, 110);
        let muted = egui::Color32::from_rgb(160, 160, 190);

        ui.horizontal(|ui| {
            ui.heading("Jobs");

            // Right-aligned controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Status filter
                ui.label("Status:");
                ui.add_space(4.0);
                egui::ComboBox::from_id_source("job_filter")
                    .selected_text(if self.filter_status.is_empty() {
                        "All"
                    } else {
                        &self.filter_status
                    })
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filter_status, String::new(), "All");
                        ui.selectable_value(&mut self.filter_status, "PENDING".to_string(), "Pending");
                        ui.selectable_value(&mut self.filter_status, "ACTIVE".to_string(), "Active");
                        ui.selectable_value(&mut self.filter_status, "COMPLETED".to_string(), "Completed");
                        ui.selectable_value(&mut self.filter_status, "CANCELLED".to_string(), "Cancelled");
                    });

                ui.add_space(16.0);

                // Search box
                ui.label("🔍");
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search jobs...")
                        .desired_width(150.0),
                );

                ui.add_space(16.0);

                // New Job button
                if ui.button("➕ New Job").clicked() {
                    self.show_create_dialog = true;
                    self.new_job_name.clear();
                    self.new_job_description.clear();
                    self.new_job_reference.clear();
                }
            });
        });

        ui.separator();

        // Create job dialog
        if self.show_create_dialog {
            egui::Window::new("Create New Job")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    egui::Grid::new("create_job_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.new_job_name);
                            ui.end_row();

                            ui.label("Reference:");
                            ui.text_edit_singleline(&mut self.new_job_reference);
                            ui.end_row();

                            ui.label("Description:");
                            ui.text_edit_multiline(&mut self.new_job_description);
                            ui.end_row();
                        });

                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() && !self.new_job_name.is_empty() {
                            let job_input = JobInput {
                                id: uuid::Uuid::new_v4().to_string(),
                                name: self.new_job_name.clone(),
                                description: Some(self.new_job_description.clone()),
                                reference_number: Some(self.new_job_reference.clone()),
                                customer_name: None,
                                expected_count: Some(0),
                                scan_count: None,
                                status: Some("PENDING".to_string()),
                                created_at: None,
                                updated_at: None,
                                started_at: None,
                                completed_at: None,
                                device_id: None,
                                created_by: None,
                                notes: None,
                                priority: Some("NORMAL".to_string()),
                                due_date: None,
                            };

                            let state_c = state.clone();
                            runtime.block_on(async {
                                let state_c = state_c.read().await;
                                let _ = state_c.repo.upsert_job(&job_input).await;
                            });

                            self.show_create_dialog = false;
                            needs_refresh = true;
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_create_dialog = false;
                        }
                    });
                });
        }

        // Filter jobs
        let filtered_jobs: Vec<_> = data
            .jobs
            .iter()
            .filter(|j| {
                let status_match = self.filter_status.is_empty() || j.status == self.filter_status;
                let search_match = self.search_query.is_empty()
                    || j.name.to_lowercase().contains(&self.search_query.to_lowercase())
                    || j.reference_number.as_ref().map_or(false, |r| {
                        r.to_lowercase().contains(&self.search_query.to_lowercase())
                    })
                    || j.customer_name.as_ref().map_or(false, |c| {
                        c.to_lowercase().contains(&self.search_query.to_lowercase())
                    });
                status_match && search_match
            })
            .collect();

        if filtered_jobs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    "No jobs found.\n\nClick 'New Job' to create one, or jobs will sync from the Android app.",
                );
            });
            return needs_refresh;
        }

        ui.label(format!("{} jobs", filtered_jobs.len()));
        ui.separator();

        // Card grid
        let card_width = 260.0_f32;
        let card_spacing = 12.0_f32;

        egui::ScrollArea::vertical().show(ui, |ui| {
            let available = ui.available_width();
            let cols =
                ((available + card_spacing) / (card_width + card_spacing)).floor().max(1.0) as usize;

            let mut idx = 0;
            while idx < filtered_jobs.len() {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = card_spacing;
                    for _ in 0..cols {
                        if idx >= filtered_jobs.len() {
                            break;
                        }
                        let job = filtered_jobs[idx];
                        let is_expanded = self.expanded_jobs.contains(&job.id);
                        let job_id = job.id;
                        idx += 1;

                        // Status color
                        let status_color = match job.status.as_str() {
                            "ACTIVE" => egui::Color32::from_rgb(76, 175, 80),
                            "COMPLETED" => egui::Color32::from_rgb(33, 150, 243),
                            "CANCELLED" => egui::Color32::from_rgb(244, 67, 54),
                            _ => egui::Color32::from_rgb(255, 193, 7), // PENDING
                        };

                        egui::Frame::none()
                            .fill(card_bg)
                            .rounding(egui::Rounding::same(10.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .stroke(egui::Stroke::new(1.5, card_border))
                            .show(ui, |ui| {
                                ui.set_width(card_width - 24.0);

                                // Clickable header area (no buttons here)
                                let header_resp = ui.scope(|ui| {
                                    // Job name
                                    ui.label(
                                        egui::RichText::new(&job.name)
                                            .strong()
                                            .size(14.0)
                                            .color(accent),
                                    );

                                    ui.add_space(2.0);

                                    // Reference number
                                    if let Some(ref_num) = &job.reference_number {
                                        if !ref_num.is_empty() {
                                            ui.label(
                                                egui::RichText::new(format!("Ref: {}", ref_num))
                                                    .size(11.0)
                                                    .color(muted),
                                            );
                                        }
                                    }

                                    ui.add_space(2.0);

                                    // Status + priority row
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&job.status)
                                                .size(11.0)
                                                .color(status_color)
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("·").size(11.0).color(muted),
                                        );
                                        ui.label(
                                            egui::RichText::new(&job.priority)
                                                .size(11.0)
                                                .color(muted),
                                        );
                                    });

                                    ui.add_space(2.0);

                                    // Created date
                                    let created = job
                                        .created_at
                                        .as_deref()
                                        .and_then(|s| to_local(s))
                                        .map(|dt| dt.format("%m/%d/%Y").to_string())
                                        .unwrap_or_else(|| "-".to_string());
                                    ui.label(
                                        egui::RichText::new(format!("Created {}", created))
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(130, 130, 160)),
                                    );
                                });

                                // Click on header to expand/collapse
                                if header_resp.response.interact(egui::Sense::click()).clicked() {
                                    if self.editing_job == Some(job_id) {
                                        // Don't collapse while editing
                                    } else if is_expanded {
                                        self.expanded_jobs.remove(&job_id);
                                    } else {
                                        self.expanded_jobs.insert(job_id);
                                    }
                                }

                                // Expanded details
                                if is_expanded {
                                    ui.add_space(6.0);
                                    ui.separator();
                                    ui.add_space(4.0);

                                    // Details section
                                    if let Some(desc) = &job.description {
                                        if !desc.is_empty() {
                                            ui.label(
                                                egui::RichText::new("Description:")
                                                    .size(10.0)
                                                    .color(muted),
                                            );
                                            ui.label(
                                                egui::RichText::new(desc).size(11.0),
                                            );
                                            ui.add_space(4.0);
                                        }
                                    }

                                    if let Some(customer) = &job.customer_name {
                                        if !customer.is_empty() {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new("Customer:")
                                                        .size(10.0)
                                                        .color(muted),
                                                );
                                                ui.label(
                                                    egui::RichText::new(customer).size(11.0),
                                                );
                                            });
                                        }
                                    }

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("Expected:")
                                                .size(10.0)
                                                .color(muted),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{}", job.expected_count))
                                                .size(11.0),
                                        );
                                    });

                                    if let Some(notes) = &job.notes {
                                        if !notes.is_empty() {
                                            ui.add_space(2.0);
                                            ui.label(
                                                egui::RichText::new("Notes:")
                                                    .size(10.0)
                                                    .color(muted),
                                            );
                                            ui.label(
                                                egui::RichText::new(notes).size(11.0),
                                            );
                                        }
                                    }

                                    ui.add_space(6.0);

                                    // Edit / Delete buttons row
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new("✏ Edit").size(11.0),
                                                )
                                                .fill(accent)
                                                .rounding(egui::Rounding::same(4.0)),
                                            )
                                            .clicked()
                                        {
                                            // Populate edit fields from current job
                                            self.edit_name = job.name.clone();
                                            self.edit_reference = job
                                                .reference_number
                                                .clone()
                                                .unwrap_or_default();
                                            self.edit_description =
                                                job.description.clone().unwrap_or_default();
                                            self.edit_status = job.status.clone();
                                            self.edit_priority = job.priority.clone();
                                            self.edit_customer =
                                                job.customer_name.clone().unwrap_or_default();
                                            self.edit_notes =
                                                job.notes.clone().unwrap_or_default();
                                            self.editing_job = Some(job_id);
                                        }

                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new("🗑 Delete")
                                                        .size(11.0)
                                                        .color(egui::Color32::from_rgb(
                                                            244, 67, 54,
                                                        )),
                                                )
                                                .fill(egui::Color32::TRANSPARENT)
                                                .stroke(egui::Stroke::new(
                                                    1.0,
                                                    egui::Color32::from_rgb(244, 67, 54),
                                                ))
                                                .rounding(egui::Rounding::same(4.0)),
                                            )
                                            .clicked()
                                        {
                                            let state_c = state.clone();
                                            runtime.block_on(async {
                                                let state_c = state_c.read().await;
                                                let _ = state_c.repo.delete_job(job_id).await;
                                            });
                                            self.expanded_jobs.remove(&job_id);
                                            needs_refresh = true;
                                        }
                                    });

                                    // Inline edit panel
                                    if self.editing_job == Some(job_id) {
                                        ui.add_space(6.0);
                                        ui.separator();
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new("Edit Job")
                                                .strong()
                                                .size(12.0)
                                                .color(accent),
                                        );
                                        ui.add_space(4.0);

                                        egui::Grid::new(
                                            egui::Id::new("edit_job_grid").with(job_id),
                                        )
                                        .num_columns(2)
                                        .spacing([8.0, 4.0])
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new("Name:")
                                                    .size(11.0)
                                                    .color(muted),
                                            );
                                            ui.add(
                                                egui::TextEdit::singleline(&mut self.edit_name)
                                                    .desired_width(160.0),
                                            );
                                            ui.end_row();

                                            ui.label(
                                                egui::RichText::new("Reference:")
                                                    .size(11.0)
                                                    .color(muted),
                                            );
                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.edit_reference,
                                                )
                                                .desired_width(160.0),
                                            );
                                            ui.end_row();

                                            ui.label(
                                                egui::RichText::new("Status:")
                                                    .size(11.0)
                                                    .color(muted),
                                            );
                                            egui::ComboBox::from_id_source(
                                                egui::Id::new("edit_status_cb").with(job_id),
                                            )
                                            .selected_text(&self.edit_status)
                                            .width(160.0)
                                            .show_ui(ui, |ui| {
                                                for s in
                                                    &["PENDING", "ACTIVE", "COMPLETED", "CANCELLED"]
                                                {
                                                    ui.selectable_value(
                                                        &mut self.edit_status,
                                                        s.to_string(),
                                                        *s,
                                                    );
                                                }
                                            });
                                            ui.end_row();

                                            ui.label(
                                                egui::RichText::new("Priority:")
                                                    .size(11.0)
                                                    .color(muted),
                                            );
                                            egui::ComboBox::from_id_source(
                                                egui::Id::new("edit_priority_cb").with(job_id),
                                            )
                                            .selected_text(&self.edit_priority)
                                            .width(160.0)
                                            .show_ui(ui, |ui| {
                                                for p in &["LOW", "NORMAL", "HIGH", "URGENT"] {
                                                    ui.selectable_value(
                                                        &mut self.edit_priority,
                                                        p.to_string(),
                                                        *p,
                                                    );
                                                }
                                            });
                                            ui.end_row();

                                            ui.label(
                                                egui::RichText::new("Customer:")
                                                    .size(11.0)
                                                    .color(muted),
                                            );
                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.edit_customer,
                                                )
                                                .desired_width(160.0),
                                            );
                                            ui.end_row();

                                            ui.label(
                                                egui::RichText::new("Description:")
                                                    .size(11.0)
                                                    .color(muted),
                                            );
                                            ui.add(
                                                egui::TextEdit::multiline(
                                                    &mut self.edit_description,
                                                )
                                                .desired_width(160.0)
                                                .desired_rows(2),
                                            );
                                            ui.end_row();

                                            ui.label(
                                                egui::RichText::new("Notes:")
                                                    .size(11.0)
                                                    .color(muted),
                                            );
                                            ui.add(
                                                egui::TextEdit::multiline(&mut self.edit_notes)
                                                    .desired_width(160.0)
                                                    .desired_rows(2),
                                            );
                                            ui.end_row();
                                        });

                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new("Save").size(12.0),
                                                    )
                                                    .fill(accent)
                                                    .rounding(egui::Rounding::same(4.0))
                                                    .min_size(egui::vec2(70.0, 24.0)),
                                                )
                                                .clicked()
                                            {
                                                let name = self.edit_name.clone();
                                                let status = self.edit_status.clone();
                                                let reference = self.edit_reference.clone();
                                                let description = self.edit_description.clone();
                                                let customer = self.edit_customer.clone();
                                                let notes = self.edit_notes.clone();
                                                let priority = self.edit_priority.clone();

                                                let state_c = state.clone();
                                                runtime.block_on(async {
                                                    let state_c = state_c.read().await;
                                                    let _ = state_c
                                                        .repo
                                                        .update_job_details(
                                                            job_id,
                                                            Some(&name),
                                                            Some(&status),
                                                            Some(if reference.is_empty() {
                                                                None
                                                            } else {
                                                                Some(reference.as_str())
                                                            }),
                                                            Some(if description.is_empty() {
                                                                None
                                                            } else {
                                                                Some(description.as_str())
                                                            }),
                                                            Some(if customer.is_empty() {
                                                                None
                                                            } else {
                                                                Some(customer.as_str())
                                                            }),
                                                            Some(if notes.is_empty() {
                                                                None
                                                            } else {
                                                                Some(notes.as_str())
                                                            }),
                                                            Some(&priority),
                                                            None,
                                                            None,
                                                        )
                                                        .await;
                                                });
                                                self.editing_job = None;
                                                needs_refresh = true;
                                            }

                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new("Cancel").size(12.0),
                                                    )
                                                    .fill(egui::Color32::TRANSPARENT)
                                                    .stroke(egui::Stroke::new(1.0, muted))
                                                    .rounding(egui::Rounding::same(4.0))
                                                    .min_size(egui::vec2(70.0, 24.0)),
                                                )
                                                .clicked()
                                            {
                                                self.editing_job = None;
                                            }
                                        });
                                    }
                                }
                            });
                    }
                });
                ui.add_space(card_spacing);
            }
        });

        needs_refresh
    }
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

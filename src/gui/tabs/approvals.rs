//! Approvals Tab - Manage pending changes and web clients

use crate::api::routes::SharedState;
use crate::db::models::{PendingChange, TeamMember, WebClient};
use eframe::egui;

/// View mode for the approvals tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApprovalsView {
    #[default]
    Pending,
    WebClients,
}

/// State for the Approvals tab
pub struct ApprovalsTab {
    pub view: ApprovalsView,
    pub pending_changes: Vec<PendingChange>,
    pub web_clients: Vec<WebClient>,
    pub team_members: Vec<TeamMember>,
    pub pending_count: i64,
    pub last_refresh: Option<std::time::Instant>,
    pub selected_change: Option<i64>,
    pub expanded_change: Option<i64>,
    pub filter_entity_type: Option<String>,
    /// client_id of the web client currently showing the link dropdown
    pub linking_client_id: Option<String>,
}

impl Default for ApprovalsTab {
    fn default() -> Self {
        Self {
            view: ApprovalsView::Pending,
            pending_changes: Vec::new(),
            web_clients: Vec::new(),
            team_members: Vec::new(),
            pending_count: 0,
            last_refresh: None,
            selected_change: None,
            expanded_change: None,
            filter_entity_type: None,
            linking_client_id: None,
        }
    }
}

impl ApprovalsTab {
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

    /// Main UI render function
    pub fn ui(
        &mut self, 
        ui: &mut egui::Ui, 
        state: &SharedState, 
        runtime: &tokio::runtime::Handle
    ) -> bool {
        let mut needs_refresh = false;
        
        // Auto-refresh data
        let should_refresh = self.last_refresh
            .map(|t| t.elapsed().as_secs() >= 3)
            .unwrap_or(true);
            
        if should_refresh {
            self.refresh_data(state, runtime);
        }
        
        // Header
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("⚡").size(28.0));
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Approvals").size(24.0).strong());
                ui.label(egui::RichText::new("Manage pending changes and web clients").size(13.0).color(egui::Color32::from_rgb(148, 163, 184)));
            });
        });

        ui.add_space(20.0);

        // Stats
        ui.horizontal(|ui| {
            Self::stat_card(ui, "📝", "Pending", &self.pending_count.to_string(), egui::Color32::from_rgb(251, 191, 36));
            ui.add_space(12.0);
            Self::stat_card(ui, "🌐", "Web Clients", &self.web_clients.len().to_string(), egui::Color32::from_rgb(99, 102, 241));
            ui.add_space(12.0);
            let trusted = self.web_clients.iter().filter(|c| c.is_trusted != 0).count();
            Self::stat_card(ui, "✅", "Trusted", &trusted.to_string(), egui::Color32::from_rgb(34, 197, 94));
        });

        ui.add_space(20.0);

        // Toolbar with view toggle
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 58))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let pending_btn = egui::Button::new(
                        egui::RichText::new("📝 Pending Changes")
                            .color(if self.view == ApprovalsView::Pending { 
                                egui::Color32::WHITE 
                            } else { 
                                egui::Color32::from_rgb(148, 163, 184) 
                            })
                    )
                    .fill(if self.view == ApprovalsView::Pending {
                        egui::Color32::from_rgb(99, 102, 241)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .rounding(egui::Rounding::same(6.0));
                    
                    if ui.add(pending_btn).clicked() {
                        self.view = ApprovalsView::Pending;
                    }
                    
                    ui.add_space(8.0);
                    
                    let clients_btn = egui::Button::new(
                        egui::RichText::new("🌐 Web Clients")
                            .color(if self.view == ApprovalsView::WebClients { 
                                egui::Color32::WHITE 
                            } else { 
                                egui::Color32::from_rgb(148, 163, 184) 
                            })
                    )
                    .fill(if self.view == ApprovalsView::WebClients {
                        egui::Color32::from_rgb(99, 102, 241)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .rounding(egui::Rounding::same(6.0));
                    
                    if ui.add(clients_btn).clicked() {
                        self.view = ApprovalsView::WebClients;
                    }
                });
            });

        ui.add_space(16.0);
        
        // Content based on view
        match self.view {
            ApprovalsView::Pending => {
                if self.render_pending_changes(ui, state, runtime) {
                    needs_refresh = true;
                }
            }
            ApprovalsView::WebClients => {
                if self.render_web_clients(ui, state, runtime) {
                    needs_refresh = true;
                }
            }
        }
        
        needs_refresh
    }
    
    /// Refresh data from database
    fn refresh_data(&mut self, state: &SharedState, runtime: &tokio::runtime::Handle) {
        let state_clone = state.clone();
        
        let pending = runtime.block_on(async {
            let state = state_clone.read().await;
            state.repo.get_pending_changes().await.unwrap_or_default()
        });
        
        let pending_count = runtime.block_on(async {
            let state = state_clone.read().await;
            state.repo.get_pending_change_count().await.unwrap_or(0)
        });
        
        let clients = runtime.block_on(async {
            let state = state_clone.read().await;
            state.repo.get_web_clients().await.unwrap_or_default()
        });
        
        let members = runtime.block_on(async {
            let state = state_clone.read().await;
            state.repo.get_team_members().await.unwrap_or_default()
        });
        
        self.pending_changes = pending;
        self.pending_count = pending_count;
        self.web_clients = clients;
        self.team_members = members;
        self.last_refresh = Some(std::time::Instant::now());
    }
    
    /// Render pending changes view
    fn render_pending_changes(
        &mut self, 
        ui: &mut egui::Ui, 
        state: &SharedState, 
        runtime: &tokio::runtime::Handle
    ) -> bool {
        let mut needs_refresh = false;
        
        if self.pending_changes.is_empty() {
            // Empty state
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(egui::RichText::new("✨").size(48.0));
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("No pending changes")
                        .size(18.0)
                        .color(egui::Color32::from_rgb(148, 163, 184))
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Changes from untrusted web clients will appear here for approval")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(100, 116, 139))
                );
            });
            return false;
        }
        
        // Changes list
        egui::ScrollArea::vertical().show(ui, |ui| {
            for change in &self.pending_changes.clone() {
                let is_expanded = self.expanded_change == Some(change.id);
                
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 41, 59))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(16.0))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(51, 65, 85)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Change type icon
                            let (icon, color) = match change.change_type.as_str() {
                                "create" => ("➕", egui::Color32::from_rgb(34, 197, 94)),
                                "update" => ("✏️", egui::Color32::from_rgb(251, 191, 36)),
                                "delete" => ("🗑️", egui::Color32::from_rgb(239, 68, 68)),
                                _ => ("❓", egui::Color32::GRAY),
                            };
                            
                            ui.label(egui::RichText::new(icon).size(18.0));
                            ui.add_space(8.0);
                            
                            // Change info
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&change.change_type.to_uppercase())
                                            .color(color)
                                            .strong()
                                    );
                                    ui.label(
                                        egui::RichText::new(&change.entity_type)
                                            .color(egui::Color32::WHITE)
                                    );
                                    if let Some(ref id) = change.entity_id {
                                        ui.label(
                                            egui::RichText::new(format!("({})", id))
                                                .color(egui::Color32::from_rgb(148, 163, 184))
                                                .size(12.0)
                                        );
                                    }
                                });
                                
                                // Client info
                                let client_name = self.web_clients
                                    .iter()
                                    .find(|c| c.client_id == change.client_id)
                                    .and_then(|c| c.client_name.clone())
                                    .unwrap_or_else(|| change.client_id[..8.min(change.client_id.len())].to_string());
                                    
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("From: {}", client_name))
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(100, 116, 139))
                                    );
                                    if let Some(ref created) = change.created_at {
                                        ui.label(
                                            egui::RichText::new(format!(" • {}", format_time_ago(created)))
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(100, 116, 139))
                                        );
                                    }
                                });
                            });
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Reject button
                                let reject_btn = egui::Button::new(
                                    egui::RichText::new("✗ Reject")
                                        .color(egui::Color32::WHITE)
                                )
                                .fill(egui::Color32::from_rgb(239, 68, 68))
                                .rounding(egui::Rounding::same(6.0));
                                
                                if ui.add(reject_btn).clicked() {
                                    let state_clone = state.clone();
                                    let change_id = change.id;
                                    runtime.block_on(async {
                                        let state = state_clone.read().await;
                                        let _ = state.repo.reject_change(change_id, Some("desktop")).await;
                                    });
                                    needs_refresh = true;
                                }
                                
                                ui.add_space(8.0);
                                
                                // Approve button
                                let approve_btn = egui::Button::new(
                                    egui::RichText::new("✓ Approve")
                                        .color(egui::Color32::WHITE)
                                )
                                .fill(egui::Color32::from_rgb(34, 197, 94))
                                .rounding(egui::Rounding::same(6.0));
                                
                                if ui.add(approve_btn).clicked() {
                                    self.approve_and_apply_change(change, state, runtime);
                                    needs_refresh = true;
                                }
                                
                                ui.add_space(8.0);
                                
                                // Expand/collapse button
                                let expand_btn = egui::Button::new(
                                    if is_expanded { "▼" } else { "▶" }
                                )
                                .fill(egui::Color32::TRANSPARENT);
                                
                                if ui.add(expand_btn).clicked() {
                                    self.expanded_change = if is_expanded { None } else { Some(change.id) };
                                }
                            });
                        });
                        
                        // Expanded details
                        if is_expanded {
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);
                            
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(15, 23, 42))
                                .rounding(egui::Rounding::same(4.0))
                                .inner_margin(egui::Margin::same(12.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Change Data:")
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(100, 116, 139))
                                    );
                                    ui.add_space(4.0);
                                    
                                    // Pretty print JSON
                                    let formatted = match serde_json::from_str::<serde_json::Value>(&change.change_data) {
                                        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(change.change_data.clone()),
                                        Err(_) => change.change_data.clone(),
                                    };
                                    
                                    ui.label(
                                        egui::RichText::new(&formatted)
                                            .monospace()
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(148, 163, 184))
                                    );
                                });
                        }
                    });
                
                ui.add_space(8.0);
            }
        });
        
        needs_refresh
    }
    
    /// Approve and apply a change
    fn approve_and_apply_change(
        &self,
        change: &PendingChange,
        state: &SharedState,
        runtime: &tokio::runtime::Handle,
    ) {
        let state_clone = state.clone();
        let change_id = change.id;
        
        runtime.block_on(async {
            let state = state_clone.read().await;
            // The approve endpoint in the API already applies the change
            let _ = state.repo.approve_change(change_id, Some("desktop")).await;
            
            // Apply the change based on type
            if let Ok(change_data) = serde_json::from_str::<serde_json::Value>(&change.change_data) {
                match (change.change_type.as_str(), change.entity_type.as_str()) {
                    ("create", "job") | ("update", "job") => {
                        if let Ok(job) = serde_json::from_value::<crate::db::models::JobInput>(change_data) {
                            let _ = state.repo.upsert_job(&job).await;
                        }
                    }
                    ("delete", "job") => {
                        if let Some(ref id) = change.entity_id {
                            if let Ok(job_id) = id.parse::<i64>() {
                                let _ = state.repo.delete_job(job_id).await;
                            }
                        }
                    }
                    ("update", "device") => {
                        if let Ok(device) = serde_json::from_value::<crate::db::models::DeviceInput>(change_data) {
                            let _ = state.repo.upsert_device(&device).await;
                        }
                    }
                    ("delete", "device") => {
                        if let Some(ref id) = change.entity_id {
                            let _ = state.repo.delete_device(id).await;
                        }
                    }
                    _ => {}
                }
            }
        });
    }
    
    /// Render web clients view
    fn render_web_clients(
        &mut self, 
        ui: &mut egui::Ui, 
        state: &SharedState, 
        runtime: &tokio::runtime::Handle
    ) -> bool {
        let mut needs_refresh = false;
        
        if self.web_clients.is_empty() {
            // Empty state
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(egui::RichText::new("🌐").size(48.0));
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("No web clients")
                        .size(18.0)
                        .color(egui::Color32::from_rgb(148, 163, 184))
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Web clients will appear here when they access the dashboard")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(100, 116, 139))
                );
            });
            return false;
        }
        
        // Clone what we need for iteration
        let clients = self.web_clients.clone();
        let members = self.team_members.clone();
        
        // Clients table
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Header
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(30, 41, 59))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Client").strong().color(egui::Color32::from_rgb(148, 163, 184)));
                        ui.add_space(100.0);
                        ui.label(egui::RichText::new("Status").strong().color(egui::Color32::from_rgb(148, 163, 184)));
                        ui.add_space(40.0);
                        ui.label(egui::RichText::new("Linked User").strong().color(egui::Color32::from_rgb(148, 163, 184)));
                        ui.add_space(40.0);
                        ui.label(egui::RichText::new("Last Seen").strong().color(egui::Color32::from_rgb(148, 163, 184)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Actions").strong().color(egui::Color32::from_rgb(148, 163, 184)));
                        });
                    });
                });
            
            ui.add_space(8.0);
            
            // Client rows
            for client in &clients {
                let linked_member = client.user_id.as_ref().and_then(|uid| {
                    members.iter().find(|m| m.device_id == *uid)
                });
                let is_linking = self.linking_client_id.as_ref() == Some(&client.client_id);
                
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 41, 59))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(51, 65, 85)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Client info
                            ui.vertical(|ui| {
                                let name = client.client_name.as_deref().unwrap_or("Unnamed Client");
                                ui.label(
                                    egui::RichText::new(name)
                                        .color(egui::Color32::WHITE)
                                        .strong()
                                );
                                ui.label(
                                    egui::RichText::new(&client.client_id[..16.min(client.client_id.len())])
                                        .monospace()
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(100, 116, 139))
                                );
                            });
                            
                            ui.add_space(20.0);
                            
                            // Trust status
                            let (status_text, status_color) = if client.is_trusted != 0 {
                                ("✓ Trusted", egui::Color32::from_rgb(34, 197, 94))
                            } else {
                                ("⏳ Pending", egui::Color32::from_rgb(251, 191, 36))
                            };
                            
                            egui::Frame::none()
                                .fill(status_color.linear_multiply(0.2))
                                .rounding(egui::Rounding::same(4.0))
                                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(status_text)
                                            .color(status_color)
                                            .size(12.0)
                                    );
                                });
                            
                            ui.add_space(20.0);
                            
                            // Linked team member
                            if let Some(member) = linked_member {
                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(99, 102, 241).linear_multiply(0.2))
                                    .rounding(egui::Rounding::same(4.0))
                                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(format!("👤 {}", member.display_name))
                                                .color(egui::Color32::from_rgb(165, 180, 252))
                                                .size(12.0)
                                        );
                                    });
                            } else {
                                ui.label(
                                    egui::RichText::new("—")
                                        .color(egui::Color32::from_rgb(100, 116, 139))
                                        .size(12.0)
                                );
                            }
                            
                            ui.add_space(20.0);
                            
                            // Last seen
                            if let Some(ref last_seen) = client.last_seen_at {
                                ui.label(
                                    egui::RichText::new(format_time_ago(last_seen))
                                        .color(egui::Color32::from_rgb(148, 163, 184))
                                );
                            }
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Delete button
                                let delete_btn = egui::Button::new(
                                    egui::RichText::new("🗑️")
                                )
                                .fill(egui::Color32::TRANSPARENT);
                                
                                if ui.add(delete_btn).on_hover_text("Delete client").clicked() {
                                    let state_clone = state.clone();
                                    let client_id = client.client_id.clone();
                                    runtime.block_on(async {
                                        let state = state_clone.read().await;
                                        let _ = state.repo.delete_web_client(&client_id).await;
                                    });
                                    needs_refresh = true;
                                }
                                
                                ui.add_space(8.0);
                                
                                // Link/Unlink button
                                if linked_member.is_some() {
                                    let unlink_btn = egui::Button::new(
                                        egui::RichText::new("🔗 Unlink").color(egui::Color32::WHITE)
                                    )
                                    .fill(egui::Color32::from_rgb(239, 68, 68))
                                    .rounding(egui::Rounding::same(6.0));
                                    
                                    if ui.add(unlink_btn).on_hover_text("Unlink from team member").clicked() {
                                        let state_clone = state.clone();
                                        let client_id = client.client_id.clone();
                                        runtime.block_on(async {
                                            let state = state_clone.read().await;
                                            let _ = state.repo.unlink_web_client_user(&client_id).await;
                                        });
                                        needs_refresh = true;
                                    }
                                } else {
                                    let link_btn = egui::Button::new(
                                        egui::RichText::new("🔗 Link").color(egui::Color32::WHITE)
                                    )
                                    .fill(egui::Color32::from_rgb(99, 102, 241))
                                    .rounding(egui::Rounding::same(6.0));
                                    
                                    if ui.add(link_btn).on_hover_text("Link to a team member").clicked() {
                                        self.linking_client_id = if is_linking { None } else { Some(client.client_id.clone()) };
                                    }
                                }
                                
                                ui.add_space(8.0);
                                
                                // Trust toggle button
                                let (trust_text, trust_color) = if client.is_trusted != 0 {
                                    ("Revoke Trust", egui::Color32::from_rgb(239, 68, 68))
                                } else {
                                    ("Trust Device", egui::Color32::from_rgb(34, 197, 94))
                                };
                                
                                let trust_btn = egui::Button::new(
                                    egui::RichText::new(trust_text).color(egui::Color32::WHITE)
                                )
                                .fill(trust_color)
                                .rounding(egui::Rounding::same(6.0));
                                
                                if ui.add(trust_btn).clicked() {
                                    let state_clone = state.clone();
                                    let client_id = client.client_id.clone();
                                    let new_trust = client.is_trusted == 0;
                                    runtime.block_on(async {
                                        let state = state_clone.read().await;
                                        let _ = state.repo.set_web_client_trust(&client_id, new_trust).await;
                                    });
                                    needs_refresh = true;
                                }
                            });
                        });
                        
                        // Team member selection dropdown (shown when Link is clicked)
                        if is_linking && !members.is_empty() {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(4.0);
                            
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Select team member:")
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(148, 163, 184))
                                );
                            });
                            
                            ui.add_space(4.0);
                            
                            // Show team member buttons in a horizontal wrap
                            ui.horizontal_wrapped(|ui| {
                                for member in &members {
                                    let member_btn = egui::Button::new(
                                        egui::RichText::new(format!("👤 {} ({})", member.display_name, member.role))
                                            .color(egui::Color32::WHITE)
                                            .size(12.0)
                                    )
                                    .fill(egui::Color32::from_rgb(55, 65, 81))
                                    .rounding(egui::Rounding::same(6.0));
                                    
                                    if ui.add(member_btn).clicked() {
                                        let state_clone = state.clone();
                                        let client_id = client.client_id.clone();
                                        let device_id = member.device_id.clone();
                                        runtime.block_on(async {
                                            let state = state_clone.read().await;
                                            let _ = state.repo.link_web_client_to_user(&client_id, &device_id).await;
                                        });
                                        self.linking_client_id = None;
                                        needs_refresh = true;
                                    }
                                }
                                
                                // Cancel button
                                let cancel_btn = egui::Button::new(
                                    egui::RichText::new("✕ Cancel")
                                        .color(egui::Color32::from_rgb(148, 163, 184))
                                        .size(12.0)
                                )
                                .fill(egui::Color32::TRANSPARENT);
                                
                                if ui.add(cancel_btn).clicked() {
                                    self.linking_client_id = None;
                                }
                            });
                        }
                    });
                
                ui.add_space(8.0);
            }
        });
        
        needs_refresh
    }
}

/// Format time ago string
fn format_time_ago(timestamp: &str) -> String {
    use chrono::{DateTime, Utc};
    
    let parsed = DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| timestamp.parse::<DateTime<Utc>>());
    
    match parsed {
        Ok(dt) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(dt);
            
            if duration.num_seconds() < 60 {
                "just now".to_string()
            } else if duration.num_minutes() < 60 {
                format!("{}m ago", duration.num_minutes())
            } else if duration.num_hours() < 24 {
                format!("{}h ago", duration.num_hours())
            } else if duration.num_days() < 7 {
                format!("{}d ago", duration.num_days())
            } else {
                dt.format("%m/%d/%Y").to_string()
            }
        }
        Err(_) => timestamp.to_string(),
    }
}

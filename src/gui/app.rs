use crate::api::routes::SharedState;
use crate::db::models::*;
use crate::services::SharedTunnelManager;
use eframe::egui;

use super::tabs::{ApprovalsTab, DevicesTab, EmailTab, JobsTab, MapTab, ScansTab, SettingsTab, TeamTab, TimesheetsTab};

/// Available tabs in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Map,
    Jobs,
    Scans,
    Devices,
    Team,
    Timesheets,
    Approvals,
    Email,
    Settings,
}

/// Cached data from database for GUI display
#[derive(Default)]
pub struct CachedData {
    pub scans: Vec<Scan>,
    pub jobs: Vec<Job>,
    pub devices: Vec<DeviceWithStats>,
    pub scan_locations: Vec<ScanLocation>,
    pub scan_count: i64,
    pub last_refresh: Option<std::time::Instant>,
}

/// Main application struct
pub struct CodeBarApp {
    /// Shared state with API server
    pub state: SharedState,
    /// Current active tab
    pub current_tab: Tab,
    /// Cached data for display
    pub cached_data: CachedData,
    /// Whether data needs refresh
    pub needs_refresh: bool,
    /// Tokio runtime handle for async operations
    pub runtime: tokio::runtime::Handle,
    /// Server address
    pub server_addr: String,
    /// Tab-specific state
    pub tabs: TabStates,
    /// Tunnel manager
    pub tunnel_manager: SharedTunnelManager,
}

/// State for individual tabs
pub struct TabStates {
    pub map: MapTab,
    pub jobs: JobsTab,
    pub scans: ScansTab,
    pub devices: DevicesTab,
    pub approvals: ApprovalsTab,
    pub settings: SettingsTab,
    pub email: EmailTab,
    pub team: TeamTab,
    pub timesheets: TimesheetsTab,
}

impl Default for TabStates {
    fn default() -> Self {
        Self {
            map: MapTab::default(),
            jobs: JobsTab::default(),
            scans: ScansTab::default(),
            devices: DevicesTab::default(),
            approvals: ApprovalsTab::default(),
            settings: SettingsTab::default(),
            email: EmailTab::default(),
            team: TeamTab::default(),
            timesheets: TimesheetsTab::default(),
        }
    }
}

/// Modern color palette
pub struct Theme {
    pub primary: egui::Color32,
    pub primary_dark: egui::Color32,
    pub accent: egui::Color32,
    pub success: egui::Color32,
    pub warning: egui::Color32,
    pub error: egui::Color32,
    pub surface: egui::Color32,
    pub surface_variant: egui::Color32,
    pub on_surface: egui::Color32,
    pub muted: egui::Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: egui::Color32::from_rgb(99, 102, 241),      // Indigo
            primary_dark: egui::Color32::from_rgb(79, 70, 229),  // Darker indigo
            accent: egui::Color32::from_rgb(139, 92, 246),       // Purple
            success: egui::Color32::from_rgb(34, 197, 94),       // Green
            warning: egui::Color32::from_rgb(251, 191, 36),      // Amber
            error: egui::Color32::from_rgb(239, 68, 68),         // Red
            surface: egui::Color32::from_rgb(30, 30, 46),        // Dark surface
            surface_variant: egui::Color32::from_rgb(45, 45, 65), // Lighter surface
            on_surface: egui::Color32::from_rgb(226, 232, 240),  // Light text
            muted: egui::Color32::from_rgb(148, 163, 184),       // Muted text
        }
    }
}

impl CodeBarApp {
    /// Create new application instance
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        state: SharedState,
        runtime: tokio::runtime::Handle,
        server_addr: String,
        tunnel_manager: SharedTunnelManager,
    ) -> Self {
        // Apply modern dark theme
        Self::apply_modern_theme(&cc.egui_ctx);

        Self {
            state,
            current_tab: Tab::Scans,
            cached_data: CachedData::default(),
            needs_refresh: true,
            runtime,
            server_addr,
            tabs: TabStates::default(),
            tunnel_manager,
        }
    }

    /// Apply a modern dark theme to the application
    fn apply_modern_theme(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        // Modern spacing
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.window_margin = egui::Margin::same(16.0);
        
        // Rounded corners for modern look
        style.visuals.window_rounding = egui::Rounding::same(12.0);
        style.visuals.menu_rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        
        // Dark theme colors
        let theme = Theme::default();
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = theme.surface;
        style.visuals.window_fill = theme.surface_variant;
        style.visuals.faint_bg_color = theme.surface_variant;
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(20, 20, 30);
        
        // Widget colors
        style.visuals.widgets.noninteractive.bg_fill = theme.surface_variant;
        style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, theme.on_surface);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(55, 55, 75);
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, theme.muted);
        style.visuals.widgets.hovered.bg_fill = theme.primary;
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        style.visuals.widgets.active.bg_fill = theme.primary_dark;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        
        // Selection colors
        style.visuals.selection.bg_fill = theme.primary.linear_multiply(0.4);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, theme.primary);
        
        // Hyperlink color
        style.visuals.hyperlink_color = theme.accent;
        
        ctx.set_style(style);
    }

    /// Refresh cached data from database
    fn refresh_data(&mut self) {
        let state = self.state.clone();
        let runtime = self.runtime.clone();

        let scans = runtime.block_on(async {
            let state = state.read().await;
            state.repo.get_scans(None, None, None, 500).await.unwrap_or_default()
        });

        let jobs = runtime.block_on(async {
            let state = state.read().await;
            state.repo.get_jobs().await.unwrap_or_default()
        });

        let devices = runtime.block_on(async {
            let state = state.read().await;
            state.repo.get_devices_with_stats().await.unwrap_or_default()
        });

        let scan_locations = runtime.block_on(async {
            let state = state.read().await;
            state.repo.get_scan_locations().await.unwrap_or_default()
        });

        let scan_count = runtime.block_on(async {
            let state = state.read().await;
            state.repo.get_scan_count().await.unwrap_or(0)
        });

        self.cached_data = CachedData {
            scans,
            jobs,
            devices,
            scan_locations,
            scan_count,
            last_refresh: Some(std::time::Instant::now()),
        };

        self.needs_refresh = false;
    }

    /// Render a modern styled tab button
    fn tab_button(ui: &mut egui::Ui, current: &mut Tab, tab: Tab, icon: &str, label: &str) -> bool {
        let is_selected = *current == tab;
        let theme = Theme::default();
        
        let button = egui::Button::new(
            egui::RichText::new(format!("{} {}", icon, label))
                .color(if is_selected { egui::Color32::WHITE } else { theme.muted })
        )
        .fill(if is_selected { theme.primary } else { egui::Color32::TRANSPARENT })
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(8.0));
        
        if ui.add(button).clicked() {
            *current = tab;
            return true;
        }
        false
    }
}

impl eframe::App for CodeBarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = Theme::default();
        
        // Auto-refresh every 5 seconds
        let should_refresh = self.cached_data.last_refresh
            .map(|t| t.elapsed().as_secs() >= 5)
            .unwrap_or(true);

        if self.needs_refresh || should_refresh {
            self.refresh_data();
        }

        ctx.request_repaint_after(std::time::Duration::from_secs(1));

        // Modern top navigation bar
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none()
                .fill(theme.surface_variant)
                .inner_margin(egui::Margin::symmetric(16.0, 12.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo/Title
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("◼ CabNet")
                            .size(20.0)
                            .color(egui::Color32::WHITE)
                            .strong()
                    );
                    
                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(16.0);

                    // Main navigation tabs (left side)
                    Self::tab_button(ui, &mut self.current_tab, Tab::Scans, "📊", "Scans");
                    ui.add_space(4.0);
                    Self::tab_button(ui, &mut self.current_tab, Tab::Jobs, "📋", "Jobs");
                    ui.add_space(4.0);
                    Self::tab_button(ui, &mut self.current_tab, Tab::Devices, "📱", "Devices");
                    ui.add_space(4.0);
                    Self::tab_button(ui, &mut self.current_tab, Tab::Team, "👥", "Team");
                    ui.add_space(4.0);
                    Self::tab_button(ui, &mut self.current_tab, Tab::Timesheets, "📋", "Timesheets");
                    ui.add_space(4.0);
                    Self::tab_button(ui, &mut self.current_tab, Tab::Map, "🗺", "Map");
                    ui.add_space(4.0);
                    Self::tab_button(ui, &mut self.current_tab, Tab::Email, "✉", "Email");
                    ui.add_space(4.0);
                    Self::tab_button(ui, &mut self.current_tab, Tab::Approvals, "⚡", "Approvals");

                    // Right side - Settings pushed to the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Refresh button
                        let refresh_btn = egui::Button::new(
                            egui::RichText::new("↻").size(16.0).color(theme.muted)
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE);
                        
                        if ui.add(refresh_btn).on_hover_text("Refresh data").clicked() {
                            self.needs_refresh = true;
                        }
                        
                        ui.add_space(8.0);
                        
                        // Settings tab on the right
                        Self::tab_button(ui, &mut self.current_tab, Tab::Settings, "⚙", "Settings");
                    });
                });
            });

        // Modern status bar
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none()
                .fill(theme.surface_variant)
                .inner_margin(egui::Margin::symmetric(16.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Server status with indicator
                    ui.label(egui::RichText::new("●").color(theme.success).size(10.0));
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("http://{}", self.server_addr))
                            .color(theme.muted)
                            .size(12.0)
                    );
                    
                    ui.add_space(24.0);
                    
                    // Stats badges
                    Self::stat_badge(ui, "📊", &self.cached_data.scan_count.to_string(), "scans", &theme);
                    ui.add_space(16.0);
                    Self::stat_badge(ui, "📱", &self.cached_data.devices.len().to_string(), "devices", &theme);
                    ui.add_space(16.0);
                    
                    let online_count = self.cached_data.devices.iter().filter(|d| d.is_online).count();
                    ui.label(egui::RichText::new("●").color(theme.success).size(8.0));
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(format!("{} online", online_count)).color(theme.muted).size(12.0));

                    // Last update on right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(last_refresh) = self.cached_data.last_refresh {
                            ui.label(
                                egui::RichText::new(format!("Updated {}s ago", last_refresh.elapsed().as_secs()))
                                    .color(theme.muted)
                                    .size(11.0)
                            );
                        }
                    });
                });
            });

        // Main content area with padding
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(theme.surface)
                .inner_margin(egui::Margin::same(20.0)))
            .show(ctx, |ui| {
                match self.current_tab {
                    Tab::Map => self.tabs.map.ui(ui, &self.cached_data),
                    Tab::Jobs => {
                        if self.tabs.jobs.ui(ui, &self.cached_data, &self.state, &self.runtime) {
                            self.needs_refresh = true;
                        }
                    }
                    Tab::Scans => {
                        if self.tabs.scans.ui(ui, &self.cached_data, &self.state, &self.runtime) {
                            self.needs_refresh = true;
                        }
                    }
                    Tab::Devices => self.tabs.devices.ui(ui, &self.cached_data, &self.server_addr, &self.tunnel_manager),
                    Tab::Team => {
                        if self.tabs.team.ui(ui, &self.state, &self.runtime) {
                            self.needs_refresh = true;
                        }
                    }
                    Tab::Timesheets => {
                        if self.tabs.timesheets.ui(ui, &self.state, &self.runtime) {
                            self.needs_refresh = true;
                        }
                    }
                    Tab::Approvals => {
                        if self.tabs.approvals.ui(ui, &self.state, &self.runtime) {
                            self.needs_refresh = true;
                        }
                    }
                    Tab::Settings => {
                        if self.tabs.settings.ui(ui, &self.server_addr, &self.tunnel_manager, &self.state, &self.runtime) {
                            self.needs_refresh = true;
                        }
                    }
                    Tab::Email => self.tabs.email.ui(ui, &self.state, &self.runtime),
                }
            });
    }
}

impl CodeBarApp {
    fn stat_badge(ui: &mut egui::Ui, icon: &str, value: &str, label: &str, theme: &Theme) {
        ui.label(egui::RichText::new(icon).size(12.0));
        ui.add_space(2.0);
        ui.label(egui::RichText::new(value).color(egui::Color32::WHITE).size(12.0).strong());
        ui.add_space(2.0);
        ui.label(egui::RichText::new(label).color(theme.muted).size(11.0));
    }
}

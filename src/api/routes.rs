use crate::db::repository::Repository;
use crate::services::PushNotificationService;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

use super::handlers;
use super::web;

/// Shared application state for API handlers
#[derive(Clone)]
pub struct AppState {
    pub repo: Repository,
    pub push_notifications: PushNotificationService,
    pub start_time: std::time::Instant,
}

pub type SharedState = Arc<RwLock<AppState>>;

/// Create the Axum router with all API routes
pub fn create_router(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Root route - serves landing page on main domain, dashboard on subdomain
        .route("/", get(web::root_handler))
        .route("/dashboard", get(web::dashboard))
        .route("/email", get(web::email_page))
        .route("/map", get(web::map_page))
        .route("/favicon.ico", get(web::favicon))
        .route("/logo.png", get(web::serve_logo))
        // Health check
        .route("/api/health", get(handlers::health_check))
        // Scans
        .route("/api/scans", get(handlers::get_scans))
        .route("/api/scans", post(handlers::sync_scans))
        .route("/api/scans/download", get(handlers::download_scans))
        .route("/api/scans/locations", get(handlers::get_scan_locations))
        .route("/api/scans/verify", post(handlers::verify_scans))
        .route("/api/scans/assign", put(handlers::update_scan_assignments))
        .route("/api/scans/:scan_id", delete(handlers::delete_scan))
        // Jobs
        .route("/api/jobs", get(handlers::get_jobs))
        .route("/api/jobs", post(handlers::sync_jobs))
        .route("/api/jobs/:job_id", put(handlers::update_job))
        .route("/api/jobs/:job_id", delete(handlers::delete_job))
        // Devices
        .route("/api/devices", get(handlers::get_devices))
        .route("/api/devices/register", post(handlers::register_device))
        .route("/api/devices/approve", post(handlers::approve_device))
        .route("/api/devices/auth", post(handlers::authenticate_device))
        .route("/api/devices/block", post(handlers::block_device))
        .route("/api/devices/unblock", post(handlers::unblock_device))
        .route("/api/devices/heartbeat", post(handlers::device_heartbeat))
        .route("/api/devices/heartbeat/enhanced", post(handlers::device_enhanced_heartbeat))
        .route("/api/devices/disconnect", post(handlers::device_disconnect))
        .route("/api/connect", post(handlers::device_connect))
        // Web clients (for trust system)
        .route("/api/web/register", post(handlers::register_web_client))
        .route("/api/web/status", get(handlers::get_web_client_status))
        .route("/api/web/clients", get(handlers::get_web_clients))
        .route("/api/web/clients/:client_id/trust", put(handlers::set_web_client_trust))
        .route("/api/web/clients/:client_id/name", put(handlers::update_web_client_name))
        .route("/api/web/clients/:client_id/link", put(handlers::link_web_client))
        .route("/api/web/clients/:client_id", delete(handlers::delete_web_client))
        // Pending changes (for approval workflow)
        .route("/api/web/changes", post(handlers::submit_change))
        .route("/api/web/changes/pending", get(handlers::get_pending_changes))
        .route("/api/web/changes/:id/approve", post(handlers::approve_change))
        .route("/api/web/changes/:id/reject", post(handlers::reject_change))
        // Security settings
        .route("/api/security/settings", get(handlers::get_security_settings))
        .route("/api/security/settings", put(handlers::update_security_settings))
        .route("/api/security/generate-code", post(handlers::generate_registration_code))
        .route("/api/security/codes", get(handlers::get_registration_codes))
        // Time entries
        .route("/api/time-entries", get(handlers::get_time_entries))
        .route("/api/time-entries", post(handlers::sync_time_entries))
        // Reports
        .route("/api/reports", get(handlers::get_reports))
        .route("/api/reports", post(handlers::sync_reports))
        .route("/api/reports/:report_id/photos", post(handlers::upload_report_photo))
        // Room Progress
        .route("/api/jobs/:job_id/rooms", get(handlers::get_job_rooms))
        .route("/api/jobs/:job_id/rooms/:room_name", get(handlers::get_room_detail))
        // Room Items Config
        .route("/api/settings/room-items", get(handlers::get_room_items))
        .route("/api/settings/room-items", put(handlers::set_room_items))
        // Time Rounding Config
        .route("/api/settings/time-rounding", get(handlers::get_time_rounding))
        .route("/api/settings/time-rounding", put(handlers::set_time_rounding))
        // Auto Break Config
        .route("/api/settings/auto-break", get(handlers::get_auto_break))
        .route("/api/settings/auto-break", put(handlers::set_auto_break))
        // Location Pings (GPS tracking)
        .route("/api/location-pings", post(handlers::sync_location_pings))
        .route("/api/location-pings", get(handlers::get_location_pings))
        .route("/api/location-pings/latest", get(handlers::get_latest_positions))
        // Team Members
        .route("/api/team", get(handlers::get_team_members))
        .route("/api/team/status", get(handlers::get_team_status))
        .route("/api/team/members", post(handlers::upsert_team_member))
        .route("/api/team/members/:device_id", delete(handlers::delete_team_member))
        .route("/api/team/threads", get(handlers::get_team_threads))
        .route("/api/team/threads", post(handlers::create_team_thread))
        .route("/api/team/threads/:thread_id/messages", get(handlers::get_team_messages))
        .route("/api/team/threads/:thread_id/messages", post(handlers::send_team_message))
        .route("/api/team/threads/:thread_id/read", post(handlers::mark_team_thread_read))
        .route("/api/team/unread-count", get(handlers::get_team_unread_count))
        .route("/api/devices/push-tokens", post(handlers::register_push_token))
        // Timesheets
        .route("/api/timesheets", get(handlers::get_timesheets))
        // Job Files (PDF uploads + search)
        .route("/api/jobs/files/search", get(handlers::search_job_files))
        .route("/api/jobs/:job_id/files", get(handlers::get_job_files))
        .route("/api/jobs/:job_id/files", post(handlers::upload_job_file))
        .route("/api/jobs/:job_id/files/:file_uuid", get(handlers::download_job_file))
        .route("/api/jobs/:job_id/files/:file_uuid", delete(handlers::delete_job_file))
        .route("/api/jobs/:job_id/files/:file_uuid/reparse", post(handlers::reparse_job_file))
        // Lading Tickets (parsed from shipping PDFs)
        .route("/api/jobs/:job_id/lading-tickets", get(handlers::get_lading_tickets))
        .route("/api/lading-tickets/:ticket_number", get(handlers::get_ticket_description))
        // User Backgrounds
        .route("/api/user/background", post(handlers::upload_user_background))
        .route("/api/user/background", delete(handlers::delete_user_background))
        .route("/api/user/background/type", put(handlers::set_user_background_type))
        .route("/api/user/background/:device_id", get(handlers::get_user_background))
        .route("/api/user/background/:device_id/type", get(handlers::get_user_background_type))
        .with_state(state)
        .layer(RequestDecompressionLayer::new()) // Decompress gzip requests from Android
        .layer(CompressionLayer::new()) // Compress responses with gzip/deflate
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

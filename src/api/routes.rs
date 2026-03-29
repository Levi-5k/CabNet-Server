use crate::db::repository::Repository;
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
        // Location Pings (GPS tracking)
        .route("/api/location-pings", post(handlers::sync_location_pings))
        .route("/api/location-pings", get(handlers::get_location_pings))
        .route("/api/location-pings/latest", get(handlers::get_latest_positions))
        .with_state(state)
        .layer(RequestDecompressionLayer::new()) // Decompress gzip requests from Android
        .layer(CompressionLayer::new()) // Compress responses with gzip/deflate
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

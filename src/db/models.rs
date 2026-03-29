use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Scan record from barcode scanner
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Scan {
    pub id: i64,
    pub uuid: String,
    pub barcode: String,
    pub barcode_type: Option<String>,
    pub ticket_number: Option<String>,
    pub barcode_job_ref: Option<String>,
    pub job_id: Option<i64>,
    pub device_id: String,
    pub user_id: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub scanned_at: String,
    pub synced_at: String,
    pub is_printed: i32,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}

/// Scan data received from Android API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanInput {
    pub id: String,
    pub barcode_data: String,
    pub barcode_type: Option<String>,
    pub ticket_number: Option<String>,
    pub barcode_job_ref: Option<String>,
    pub job_id: Option<String>,
    pub device_id: String,
    pub user_id: Option<String>,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub scanned_at: i64,
    pub is_printed: Option<bool>,
    /// Local ID from Android device - used for sync confirmation
    pub local_id: Option<i64>,
}

/// Job record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Job {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub description: Option<String>,
    pub reference_number: Option<String>,
    pub customer_name: Option<String>,
    pub expected_count: i32,
    pub status: String,
    pub device_id: Option<String>,
    pub created_by: Option<String>,
    pub notes: Option<String>,
    pub priority: String,
    pub due_date: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Job data received from Android API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub reference_number: Option<String>,
    pub customer_name: Option<String>,
    pub expected_count: Option<i32>,
    pub scan_count: Option<i32>,
    pub status: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub device_id: Option<String>,
    pub created_by: Option<String>,
    pub notes: Option<String>,
    pub priority: Option<String>,
    pub due_date: Option<i64>,
}

/// Device record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: i64,
    pub device_id: String,
    pub device_name: Option<String>,
    pub model: Option<String>,
    pub os_version: Option<String>,
    pub app_version: Option<String>,
    pub last_seen_at: Option<String>,
    pub registered_at: Option<String>,
    pub is_active: i32,
    // Security fields
    pub registration_code: Option<String>,
    pub is_approved: i32,
    pub auth_token: Option<String>,
    pub auth_token_expires_at: Option<String>,
    pub fingerprint_hash: Option<String>,
    pub registration_attempts: i32,
    pub last_registration_attempt_at: Option<String>,
    pub blocked_until: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    // Enhanced status tracking
    pub status: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub connection_count: Option<i64>,
    pub total_uptime_seconds: Option<i64>,
    pub error_count: Option<i64>,
    pub last_error_at: Option<String>,
    pub last_error_message: Option<String>,
    // Latest heartbeat data
    pub battery_level: Option<i32>,
    pub is_charging: Option<bool>,
    pub available_memory_mb: Option<i32>,
    pub total_memory_mb: Option<i32>,
    pub network_type: Option<String>,
    pub connection_quality: Option<String>,
}

/// Device registration input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInput {
    pub device_id: String,
    pub device_name: Option<String>,
    pub model: Option<String>,
    pub os_version: Option<String>,
    pub app_version: Option<String>,
    // Security fields
    pub registration_code: Option<String>,
    pub fingerprint: Option<String>, // Device fingerprint for security
    pub user_agent: Option<String>,
}

/// Device registration approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceApprovalRequest {
    pub device_id: String,
    pub approved: bool,
    pub notes: Option<String>,
}

/// Registration code information
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RegistrationCodeInfo {
    pub code: String,
    pub description: Option<String>,
    pub created_at: String,
    pub used_by: Option<String>,
    pub used_at: Option<String>,
}

/// Device authentication request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAuthRequest {
    pub device_id: String,
    pub auth_token: String,
}

/// Security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub max_registration_attempts_per_device: u32,
    pub max_registration_attempts_per_ip: u32,
    pub rate_limit_window_minutes: u32,
    pub block_duration_minutes: i64,
    pub token_expiry_hours: u32,
    pub require_registration_code: bool,
}

/// Email configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailConfigRecord {
    pub id: i64,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub email_from: Option<String>,
    pub email_to: Option<String>,
    pub email_subject_template: Option<String>,
    pub auto_send_enabled: i32,
    pub auto_send_delay_minutes: i32,
    pub last_auto_send_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Email history record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailHistory {
    pub id: i64,
    pub sent_at: String,
    pub recipients: String,
    pub subject: String,
    pub scan_count: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: Option<String>,
}

/// Setting key-value pair
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Setting {
    pub key: String,
    pub value: Option<String>,
    pub updated_at: Option<String>,
}

/// Job with scan count for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobWithCount {
    #[serde(flatten)]
    pub job: Job,
    pub scan_count: i32,
}

/// Device with statistics for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceWithStats {
    #[serde(flatten)]
    pub device: Device,
    pub total_scans: i32,
    pub is_online: bool,
}

/// Scan with location for map display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanLocation {
    pub id: i64,
    pub uuid: String,
    pub barcode: String,
    pub latitude: f64,
    pub longitude: f64,
    pub scanned_at: String,
    pub device_id: String,
    pub job_name: Option<String>,
}

/// Web client for dashboard trust system
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebClient {
    pub id: i64,
    pub client_id: String,
    pub client_name: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub is_trusted: i32,
    pub last_seen_at: Option<String>,
    pub created_at: Option<String>,
}

/// Pending change awaiting approval
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PendingChange {
    pub id: i64,
    pub client_id: String,
    pub change_type: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub change_data: String,
    pub status: String,
    pub created_at: Option<String>,
    pub reviewed_at: Option<String>,
    pub reviewed_by: Option<String>,
}

/// Input for creating a pending change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChangeInput {
    pub change_type: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub change_data: serde_json::Value,
}

// ==================== TIME ENTRIES ====================

/// Time entry record from mobile time clock
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TimeEntryRecord {
    pub id: i64,
    pub uuid: String,
    pub device_id: String,
    pub customer_name: Option<String>,
    pub job_name: Option<String>,
    pub job_id: Option<String>,
    pub clock_in: String,
    pub clock_out: Option<String>,
    pub note: Option<String>,
    pub is_break: i32,
    pub is_paid: i32,
    pub synced_at: Option<String>,
    pub created_at: Option<String>,
}

/// Time entry data received from device API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntryInput {
    pub id: String,
    pub device_id: String,
    pub customer_name: Option<String>,
    pub job_name: Option<String>,
    pub job_id: Option<String>,
    pub clock_in: i64,
    pub clock_out: Option<i64>,
    pub note: Option<String>,
    pub is_break: Option<bool>,
    pub is_paid: Option<bool>,
}

// ==================== REPORTS ====================

/// Report record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReportRecord {
    pub id: i64,
    pub uuid: String,
    pub job_id: Option<String>,
    pub title: String,
    pub room_name: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub author_device_id: Option<String>,
    pub author_name: Option<String>,
    pub assigned_to_device_id: Option<String>,
    pub assigned_to_name: Option<String>,
    pub cabinet_count: i32,
    pub is_complete: i32,
    pub has_fillers: i32,
    pub has_handles: i32,
    pub has_fast_caps: i32,
    pub has_set_boxes: i32,
    pub has_caulking: i32,
    pub punch_list: Option<String>,
    pub synced_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Report data received from device API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportInput {
    pub id: String,
    pub job_id: Option<String>,
    pub title: String,
    pub room_name: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub author_device_id: Option<String>,
    pub author_name: Option<String>,
    pub assigned_to_device_id: Option<String>,
    pub assigned_to_name: Option<String>,
    pub cabinet_count: Option<i32>,
    pub is_complete: Option<bool>,
    pub has_fillers: Option<bool>,
    pub has_handles: Option<bool>,
    pub has_fast_caps: Option<bool>,
    pub has_set_boxes: Option<bool>,
    pub has_caulking: Option<bool>,
    pub punch_list: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Report photo record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReportPhotoRecord {
    pub id: i64,
    pub uuid: String,
    pub report_id: String,
    pub caption: Option<String>,
    pub file_name: String,
    pub synced_at: Option<String>,
    pub created_at: Option<String>,
}

// ─── Location Pings ────────────────────────────────────────

/// Location ping input from mobile device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationPingInput {
    pub id: String,
    pub time_entry_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
    pub altitude: Option<f64>,
    pub speed: Option<f64>,
    pub heading: Option<f64>,
    pub timestamp: i64,
    pub battery_level: Option<i32>,
    pub is_moving: Option<bool>,
}

/// Location ping record from database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LocationPingRecord {
    pub id: i64,
    pub uuid: String,
    pub device_id: String,
    pub time_entry_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
    pub altitude: Option<f64>,
    pub speed: Option<f64>,
    pub heading: Option<f64>,
    pub timestamp: String,
    pub battery_level: Option<i32>,
    pub is_moving: bool,
    pub synced_at: Option<String>,
    pub created_at: Option<String>,
}

/// Latest position for an active worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWorkerPosition {
    pub device_id: String,
    pub customer_name: Option<String>,
    pub job_name: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
    pub speed: Option<f64>,
    pub is_moving: bool,
    pub battery_level: Option<i32>,
    pub timestamp: String,
    pub clock_in: String,
    pub is_on_break: bool,
}

// ==================== TEAM MEMBERS ====================

/// Team member record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMember {
    pub id: i64,
    pub device_id: String,
    pub display_name: String,
    pub role: String,
    pub is_admin: i32,
    pub avatar_color: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Team member input from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberInput {
    pub device_id: String,
    pub display_name: String,
    pub role: Option<String>,
    pub is_admin: Option<bool>,
    pub avatar_color: Option<String>,
}

/// Team member with current status (joined from devices + time_entries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberStatus {
    pub device_id: String,
    pub display_name: String,
    pub role: String,
    pub is_admin: bool,
    pub avatar_color: Option<String>,
    pub status: String,           // "working", "break", "offline"
    pub current_job: Option<String>,
    pub clock_in: Option<String>,
    pub last_seen: Option<String>,
    pub total_scans_today: i64,
}

/// Timesheet summary for a day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimesheetDay {
    pub date: String,
    pub device_id: String,
    pub display_name: String,
    pub entries: Vec<TimeEntryRecord>,
    pub total_work_seconds: i64,
    pub total_break_seconds: i64,
    pub has_gps: bool,
}

// ─── Room Progress ─────────────────────────────────────────

/// Room progress record from database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RoomProgress {
    pub id: i64,
    pub job_id: String,
    pub room_name: String,
    pub cabinet_count: i32,
    pub is_complete: i32,
    pub has_fillers: i32,
    pub has_handles: i32,
    pub has_fast_caps: i32,
    pub has_set_boxes: i32,
    pub has_caulking: i32,
    pub punch_list: Option<String>,
    pub notes: Option<String>,
    pub last_report_id: Option<String>,
    pub last_updated_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Room progress input for creating/updating
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomProgressInput {
    pub job_id: String,
    pub room_name: String,
    pub cabinet_count: Option<i32>,
    pub is_complete: Option<bool>,
    pub has_fillers: Option<bool>,
    pub has_handles: Option<bool>,
    pub has_fast_caps: Option<bool>,
    pub has_set_boxes: Option<bool>,
    pub has_caulking: Option<bool>,
    pub punch_list: Option<String>,
    pub notes: Option<String>,
}

/// Summary stats for room progress on a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomProgressSummary {
    pub total_rooms: i64,
    pub completed_rooms: i64,
    pub total_cabinets: i64,
}

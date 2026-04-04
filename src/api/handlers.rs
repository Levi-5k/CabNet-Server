use super::routes::SharedState;
use crate::db::models::*;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use crate::config::get_data_dir;

// ==================== RESPONSE TYPES ====================

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: i64,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(flatten)]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: None,
            error: None,
            data: Some(data),
        }
    }

    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
            error: None,
            data: Some(data),
        }
    }
}

impl ApiResponse<()> {
    pub fn error(error: impl Into<String>) -> Self {
        ApiResponse {
            success: false,
            message: None,
            error: Some(error.into()),
            data: None,
        }
    }
}

// ==================== REQUEST TYPES ====================

#[derive(Deserialize)]
pub struct SyncScansRequest {
    pub device_id: String,
    pub scans: Vec<ScanInput>,
}

#[derive(Serialize)]
pub struct SyncScansResponse {
    pub synced_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct VerifyScansRequest {
    pub device_id: String,
    /// Can be local IDs (integers as strings) or UUIDs
    #[serde(alias = "local_ids")]
    pub scan_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct VerifyScansResponse {
    /// IDs that were found on the server
    #[serde(rename = "verified_ids")]
    pub verified: Vec<String>,
    /// IDs that were not found on the server
    #[serde(rename = "missing_ids")]
    pub missing: Vec<String>,
}

#[derive(Deserialize)]
pub struct SyncJobsRequest {
    pub device_id: String,
    pub jobs: Vec<JobInput>,
}

#[derive(Serialize)]
pub struct SyncJobsResponse {
    pub synced_job_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct GetScansQuery {
    pub job_id: Option<i64>,
    pub device_id: Option<String>,
    pub since: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct GetScansResponse {
    pub scans: Vec<Scan>,
    pub total: i64,
}

/// Scan payload formatted for iOS/Android download (matches ServerScan in iOS)
#[derive(Serialize)]
pub struct ScanDownloadPayload {
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
    pub synced_at: Option<i64>,
    pub is_printed: bool,
}

#[derive(Serialize)]
pub struct ScanDownloadResponse {
    pub success: bool,
    pub scans: Vec<ScanDownloadPayload>,
    pub total: Option<i64>,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct GetScanLocationsResponse {
    pub locations: Vec<crate::db::models::ScanLocation>,
}

#[derive(Deserialize)]
pub struct GetJobsQuery {
    pub device_id: Option<String>,
    pub since: Option<i64>,
}

/// Job payload formatted for Android download (matches JobPayload in Android)
#[derive(Serialize)]
pub struct JobDownloadPayload {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i64>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub reference_number: String,
    #[serde(default)]
    pub customer_name: String,
    pub expected_count: i32,
    pub scan_count: i32,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub created_by: String,
    #[serde(default)]
    pub notes: String,
    pub priority: String,
    pub due_date: Option<i64>,
    pub file_count: i32,
}

/// Convert RFC3339 string to epoch millis
fn rfc3339_to_millis(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}

#[derive(Serialize)]
pub struct GetJobsResponse {
    pub jobs: Vec<JobDownloadPayload>,
}

#[derive(Deserialize)]
pub struct UpdateJobRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub completed_at: Option<i64>,
    pub reference_number: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub customer_name: Option<Option<String>>,
    pub notes: Option<Option<String>>,
    pub priority: Option<String>,
    pub expected_count: Option<i32>,
    pub due_date: Option<Option<String>>,
    pub address: Option<Option<String>>,
}

#[derive(Deserialize)]
pub struct UpdateScanAssignmentsRequest {
    pub scan_ids: Vec<i64>,
    pub job_id: Option<i64>,
    pub device_id: Option<String>,
}

#[derive(Serialize)]
pub struct UpdateScanAssignmentsResponse {
    pub updated: u64,
}

#[derive(Serialize)]
pub struct GetDevicesResponse {
    pub devices: Vec<Device>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct DevicesResponse {
    pub devices: Vec<Device>,
}

#[derive(Serialize)]
pub struct JobResponse {
    pub job: Job,
}

#[derive(Serialize)]
pub struct DeviceRegisterResponse {
    pub device_id: String,
}

#[derive(Serialize)]
pub struct DeviceAuthResponse {
    pub authenticated: bool,
    pub device_id: String,
    pub device_name: Option<String>,
}

#[derive(Deserialize)]
pub struct BlockDeviceRequest {
    pub device_id: String,
    pub minutes: i32,
}

#[derive(Deserialize)]
pub struct UnblockDeviceRequest {
    pub device_id: String,
}

#[derive(Deserialize)]
pub struct GenerateCodeRequest {
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct RegistrationCodeResponse {
    pub code: String,
}

#[derive(Deserialize)]
pub struct HeartbeatRequest {
    pub device_id: String,
    /// Number of scans waiting to be synced
    #[serde(default)]
    pub pending_scans: Option<i32>,
    /// Number of jobs waiting to be synced
    #[serde(default)]
    pub pending_jobs: Option<i32>,
}

#[derive(Serialize)]
pub struct HeartbeatResponse {
    pub acknowledged: bool,
    pub server_time: i64,
}

#[derive(Deserialize)]
pub struct EnhancedHeartbeatRequest {
    pub device_id: String,
    /// Number of scans waiting to be synced
    #[serde(default)]
    pub pending_scans: Option<i32>,
    /// Number of jobs waiting to be synced
    #[serde(default)]
    pub pending_jobs: Option<i32>,
    /// Battery level (0-100)
    #[serde(default)]
    pub battery_level: Option<i32>,
    /// Is device charging
    #[serde(default)]
    pub is_charging: Option<bool>,
    /// Total scans performed
    #[serde(default)]
    pub total_scans: Option<i32>,
    /// Memory usage description
    #[serde(default)]
    pub memory_usage: Option<String>,
    /// App version
    #[serde(default)]
    pub app_version: Option<String>,
    /// Network type (WiFi, Mobile, etc.)
    #[serde(default)]
    pub network_type: Option<String>,
    /// Connection quality description
    #[serde(default)]
    pub connection_quality: Option<String>,
    /// Timestamp of heartbeat
    #[serde(default)]
    pub timestamp: Option<i64>,
}

#[derive(Serialize)]
pub struct EnhancedHeartbeatResponse {
    pub acknowledged: bool,
    pub server_time: i64,
    /// Suggested heartbeat interval in seconds (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_interval_seconds: Option<i32>,
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub device_id: String,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub os_version: Option<String>,
    #[serde(default)]
    pub app_version: Option<String>,
    #[serde(default)]
    pub user_name: Option<String>,
    #[serde(default)]
    pub phone_number: Option<String>,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub connected: bool,
    pub device_id: String,
    pub server_version: String,
    pub server_time: i64,
    pub is_admin: bool,
}

#[derive(Deserialize)]
pub struct DisconnectRequest {
    pub device_id: String,
}

// ==================== SECURITY TYPES ====================

#[derive(Serialize)]
pub struct SecuritySettingsResponse {
    pub max_registration_attempts_per_device: u32,
    pub max_registration_attempts_per_ip: u32,
    pub rate_limit_window_minutes: u32,
    pub block_duration_minutes: i64,
    pub token_expiry_hours: u32,
    pub require_registration_code: bool,
}

#[derive(Deserialize)]
pub struct UpdateSecuritySettingsRequest {
    pub max_registration_attempts_per_device: Option<u32>,
    pub max_registration_attempts_per_ip: Option<u32>,
    pub rate_limit_window_minutes: Option<u32>,
    pub block_duration_minutes: Option<i64>,
    pub token_expiry_hours: Option<u32>,
    pub require_registration_code: Option<bool>,
}

#[derive(Serialize)]
pub struct RegistrationCodesResponse {
    pub codes: Vec<RegistrationCodeInfo>,
}

// ==================== HANDLERS ====================

/// GET /api/health - Health check endpoint
pub async fn health_check(
    State(state): State<SharedState>,
) -> Json<ApiResponse<HealthResponse>> {
    let state = state.read().await;
    let uptime = state.start_time.elapsed().as_secs();

    Json(ApiResponse::success(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        uptime_seconds: uptime,
    }))
}

/// POST /api/scans - Sync scans from Android device
pub async fn sync_scans(
    State(state): State<SharedState>,
    Json(request): Json<SyncScansRequest>,
) -> Result<Json<ApiResponse<SyncScansResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let mut synced_ids = Vec::new();

    // Update device heartbeat with enhanced status tracking
    let _ = state.repo.update_device_heartbeat(&request.device_id).await;

    for scan in &request.scans {
        match state.repo.upsert_scan(scan).await {
            Ok(scan_id) => {
                // Resolve job_id: Android sends job UUID, server uses integer IDs
                let resolved_job_id = if let Some(ref job_uuid) = scan.job_id {
                    if !job_uuid.is_empty() {
                        match state.repo.get_job_by_uuid(job_uuid).await {
                            Ok(Some(job)) => Some(job.id),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // If scan has barcode_job_ref, try to resolve and link
                if let Some(ref job_ref) = scan.barcode_job_ref {
                    if !job_ref.is_empty() {
                        if let Some(job_id) = resolved_job_id {
                            // Set this job's reference_number from the barcode
                            let _ = state.repo.set_job_reference_number(job_id, job_ref).await;
                        } else {
                            // No job_id from device — try matching by reference_number
                            if let Ok(Some(job)) = state.repo.get_job_by_reference(job_ref).await {
                                let _ = state.repo.set_scan_job_id(scan_id, job.id).await;
                            }
                        }
                    }
                }

                // Update scan's job_id to the resolved integer ID
                if let Some(job_id) = resolved_job_id {
                    let _ = state.repo.set_scan_job_id(scan_id, job_id).await;
                }

                // Prefer returning local_id if available, otherwise return UUID
                // Android can use local_id directly to mark scans as synced
                if let Some(local_id) = scan.local_id {
                    synced_ids.push(local_id.to_string());
                } else {
                    synced_ids.push(scan.id.clone());
                }
            }
            Err(e) => {
                tracing::error!("Failed to sync scan {}: {}", scan.id, e);
            }
        }
    }

    let count = synced_ids.len();
    Ok(Json(ApiResponse::success_with_message(
        SyncScansResponse { synced_ids },
        format!("Synced {} scan(s)", count),
    )))
}

/// POST /api/scans/verify - Verify which scans are stored on server
pub async fn verify_scans(
    State(state): State<SharedState>,
    Json(request): Json<VerifyScansRequest>,
) -> Result<Json<ApiResponse<VerifyScansResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    // Update device last seen
    let _ = state.repo.update_device_last_seen(&request.device_id).await;

    // Use the new verify method that handles both local_ids and UUIDs
    let (verified, missing) = state
        .repo
        .verify_scans(&request.scan_ids)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    Ok(Json(ApiResponse::success(VerifyScansResponse {
        verified,
        missing,
    })))
}

/// GET /api/scans - Get all scans with optional filtering (web dashboard format)
pub async fn get_scans(
    State(state): State<SharedState>,
    Query(query): Query<GetScansQuery>,
) -> Result<Json<ApiResponse<GetScansResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let limit = query.limit.unwrap_or(1000);

    let scans = state
        .repo
        .get_scans(query.job_id, query.device_id.as_deref(), query.since, limit)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let total = state.repo.get_scan_count().await.unwrap_or(0);

    Ok(Json(ApiResponse::success(GetScansResponse { scans, total })))
}

/// GET /api/scans/download - Download scans in iOS/Android-compatible format
pub async fn download_scans(
    State(state): State<SharedState>,
    Query(query): Query<GetScansQuery>,
) -> Result<Json<ScanDownloadResponse>, (StatusCode, Json<ScanDownloadResponse>)> {
    let state = state.read().await;
    let limit = query.limit.unwrap_or(5000);

    let scans = state
        .repo
        .get_scans(query.job_id, query.device_id.as_deref(), query.since, limit)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ScanDownloadResponse {
                    success: false,
                    scans: vec![],
                    total: None,
                    message: Some(e.to_string()),
                }),
            )
        })?;

    let total = state.repo.get_scan_count().await.unwrap_or(0);

    // Build a cache of job_id -> job_uuid
    let mut job_uuid_cache: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    for scan in &scans {
        if let Some(jid) = scan.job_id {
            if !job_uuid_cache.contains_key(&jid) {
                if let Ok(Some(job)) = state.repo.get_job(jid).await {
                    job_uuid_cache.insert(jid, job.uuid);
                }
            }
        }
    }

    let payloads: Vec<ScanDownloadPayload> = scans
        .into_iter()
        .map(|s| ScanDownloadPayload {
            id: s.uuid,
            barcode_data: s.barcode,
            barcode_type: s.barcode_type,
            ticket_number: s.ticket_number,
            barcode_job_ref: s.barcode_job_ref,
            job_id: s.job_id.and_then(|jid| job_uuid_cache.get(&jid).cloned()),
            device_id: s.device_id,
            user_id: s.user_id,
            location: s.location,
            latitude: s.latitude,
            longitude: s.longitude,
            scanned_at: rfc3339_to_millis(&s.scanned_at),
            synced_at: Some(rfc3339_to_millis(&s.synced_at)),
            is_printed: s.is_printed != 0,
        })
        .collect();

    Ok(Json(ScanDownloadResponse {
        success: true,
        scans: payloads,
        total: Some(total),
        message: None,
    }))
}

/// GET /api/scans/locations - Get scans with GPS coordinates for map display
pub async fn get_scan_locations(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<GetScanLocationsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let locations = state
        .repo
        .get_scan_locations()
        .await
        .map_err(|e| {
            tracing::error!("Failed to get scan locations: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    tracing::debug!("Returning {} scan locations", locations.len());
    Ok(Json(ApiResponse::success(GetScanLocationsResponse { locations })))
}

/// POST /api/jobs - Sync jobs from Android device
pub async fn sync_jobs(
    State(state): State<SharedState>,
    Json(request): Json<SyncJobsRequest>,
) -> Result<Json<ApiResponse<SyncJobsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let mut synced_job_ids = Vec::new();

    // Update device last seen
    let _ = state.repo.update_device_last_seen(&request.device_id).await;

    for job in &request.jobs {
        match state.repo.upsert_job(job).await {
            Ok(_) => {
                synced_job_ids.push(job.id.clone());
                // Auto-link any unmatched scans by reference_number
                if let Some(ref ref_num) = job.reference_number {
                    if !ref_num.is_empty() {
                        if let Ok(Some(db_job)) = state.repo.get_job_by_uuid(&job.id).await {
                            let _ = state.repo.link_scans_by_reference(db_job.id, ref_num).await;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to sync job {}: {}", job.id, e);
            }
        }
    }

    let count = synced_job_ids.len();
    Ok(Json(ApiResponse::success_with_message(
        SyncJobsResponse { synced_job_ids },
        format!("Synced {} job(s)", count),
    )))
}

/// GET /api/jobs - Get all jobs (formatted for Android download)
pub async fn get_jobs(
    State(state): State<SharedState>,
    Query(query): Query<GetJobsQuery>,
) -> Result<Json<ApiResponse<GetJobsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let jobs_with_counts = state.repo.get_jobs_with_counts().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    // Convert to Android-compatible format
    let payloads: Vec<JobDownloadPayload> = jobs_with_counts
        .into_iter()
        .filter(|jwc| {
            // Apply 'since' filter if provided (compare updated_at)
            if let Some(since_millis) = query.since {
                if since_millis > 0 {
                    let updated_millis = jwc.job.updated_at
                        .as_deref()
                        .map(rfc3339_to_millis)
                        .unwrap_or(0);
                    return updated_millis > since_millis;
                }
            }
            true
        })
        .map(|jwc| {
            let j = jwc.job;
            JobDownloadPayload {
                id: j.uuid,
                server_id: Some(j.id),
                name: j.name,
                description: j.description.unwrap_or_default(),
                reference_number: j.reference_number.unwrap_or_default(),
                customer_name: j.customer_name.unwrap_or_default(),
                expected_count: j.expected_count,
                scan_count: jwc.scan_count,
                status: j.status,
                created_at: j.created_at.as_deref().map(rfc3339_to_millis).unwrap_or(0),
                updated_at: j.updated_at.as_deref().map(rfc3339_to_millis).unwrap_or(0),
                started_at: j.started_at.as_deref().map(rfc3339_to_millis),
                completed_at: j.completed_at.as_deref().map(rfc3339_to_millis),
                device_id: j.device_id.unwrap_or_default(),
                created_by: j.created_by.unwrap_or_default(),
                notes: j.notes.unwrap_or_default(),
                priority: j.priority,
                due_date: j.due_date.as_deref().map(rfc3339_to_millis),
                file_count: jwc.file_count,
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(GetJobsResponse { jobs: payloads })))
}

/// PUT /api/jobs/:job_id - Update a job
pub async fn update_job(
    State(state): State<SharedState>,
    Path(job_id): Path<i64>,
    Json(request): Json<UpdateJobRequest>,
) -> Result<Json<ApiResponse<JobResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let existing = state.repo.get_job(job_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    let _job = existing.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Job not found")),
        )
    })?;

    state
        .repo
        .update_job_details(
            job_id,
            request.name.as_deref(),
            request.status.as_deref(),
            request.reference_number.as_ref().map(|v| v.as_deref()),
            request.description.as_ref().map(|v| v.as_deref()),
            request.customer_name.as_ref().map(|v| v.as_deref()),
            request.notes.as_ref().map(|v| v.as_deref()),
            request.priority.as_deref(),
            request.expected_count,
            request.due_date.as_ref().map(|v| v.as_deref()),
            request.address.as_ref().map(|v| v.as_deref()),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    let updated_job = state
        .repo
        .get_job(job_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?
        .unwrap();

    // If reference_number was set/changed, auto-link any unmatched scans
    if let Some(ref ref_num) = updated_job.reference_number {
        if !ref_num.is_empty() {
            let linked = state.repo.link_scans_by_reference(job_id, ref_num).await.unwrap_or(0);
            if linked > 0 {
                tracing::info!("Auto-linked {} scans to job {} via reference_number '{}'", linked, job_id, ref_num);
            }
        }
    }

    Ok(Json(ApiResponse::success_with_message(
        JobResponse { job: updated_job },
        "Job updated",
    )))
}

/// DELETE /api/scans/:scan_id - Delete a scan
pub async fn delete_scan(
    State(state): State<SharedState>,
    Path(scan_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    state.repo.delete_scan(scan_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse {
        success: true,
        message: Some("Scan deleted".to_string()),
        error: None,
        data: None,
    }))
}

/// DELETE /api/jobs/:job_id - Delete a job
pub async fn delete_job(
    State(state): State<SharedState>,
    Path(job_id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    state.repo.delete_job(job_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse {
        success: true,
        message: Some("Job deleted".to_string()),
        error: None,
        data: None,
    }))
}

/// GET /api/devices - Get all devices
pub async fn get_devices(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<DevicesResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let devices = state.repo.get_devices_with_status().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse::success(DevicesResponse { devices })))
}

/// POST /api/devices/register - Register or update a device with security validation
pub async fn register_device(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(mut request): Json<DeviceInput>,
) -> Result<Json<ApiResponse<DeviceRegisterResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let device_id = request.device_id.clone();
    let ip_address = addr.ip().to_string();

    // Extract user agent from headers
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Update request with additional security info
    request.user_agent = user_agent.clone();

    // Check if device is blocked
    if state.repo.is_device_blocked(&device_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })? {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::error("Device is temporarily blocked due to excessive registration attempts")),
        ));
    }

    // Check rate limiting
    if state.repo.check_device_rate_limit(&device_id, &ip_address).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })? {
        // Block the device for rate limit violation
        let settings = state.repo.get_security_settings().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            )
        })?;

        state.repo.block_device(&device_id, settings.block_duration_minutes as i32).await.map_err(|e| {
            tracing::warn!("Failed to block device {}: {}", device_id, e);
        }).ok();

        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::error("Too many registration attempts. Device blocked temporarily.")),
        ));
    }

    // Update rate limit counters
    state.repo.update_device_rate_limit(&device_id, &ip_address).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })?;

    // Get security settings
    let settings = state.repo.get_security_settings().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })?;

    // Validate registration code if required
    if settings.require_registration_code {
        if let Some(code) = &request.registration_code {
            // TODO: Validate registration code against a list of valid codes
            // For now, accept any non-empty code
            if code.trim().is_empty() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error("Valid registration code is required")),
                ));
            }
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("Registration code is required")),
            ));
        }
    }

    // Register the device
    state.repo.upsert_device(&request).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    // Log successful registration
    tracing::info!(
        device_id = %device_id,
        ip_address = %ip_address,
        user_agent = ?user_agent,
        "Device registered successfully"
    );

    Ok(Json(ApiResponse::success_with_message(
        DeviceRegisterResponse { device_id },
        "Device registered successfully",
    )))
}

/// POST /api/devices/heartbeat - Device heartbeat to update last seen
pub async fn device_heartbeat(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<HeartbeatRequest>,
) -> Result<Json<ApiResponse<HeartbeatResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let ip_address = addr.ip().to_string();

    // Check if device is blocked
    if state.repo.is_device_blocked(&request.device_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })? {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("Device is blocked")),
        ));
    }

    // Basic rate limiting for heartbeat (more lenient than registration)
    // Allow up to 60 heartbeats per hour per device/IP
    if state.repo.check_heartbeat_rate_limit(&request.device_id, &ip_address).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })? {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::error("Too many heartbeat requests")),
        ));
    }

    // Update device heartbeat with enhanced status tracking
    state.repo.update_device_heartbeat(&request.device_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse::success(HeartbeatResponse {
        acknowledged: true,
        server_time: chrono::Utc::now().timestamp_millis(),
    })))
}

/// POST /api/devices/heartbeat/enhanced - Enhanced heartbeat with device status
pub async fn device_enhanced_heartbeat(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<EnhancedHeartbeatRequest>,
) -> Result<Json<ApiResponse<EnhancedHeartbeatResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let ip_address = addr.ip().to_string();

    // Check if device is blocked
    if state.repo.is_device_blocked(&request.device_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })? {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("Device is blocked")),
        ));
    }

    // Basic rate limiting for heartbeat (more lenient than registration)
    // Allow up to 60 heartbeats per hour per device/IP
    if state.repo.check_heartbeat_rate_limit(&request.device_id, &ip_address).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })? {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::error("Too many heartbeat requests")),
        ));
    }

    // Update device heartbeat with enhanced status tracking
    state.repo.update_device_heartbeat(&request.device_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    // Store the enhanced heartbeat data
    state.repo.update_device_heartbeat_data(
        &request.device_id,
        request.battery_level,
        request.is_charging,
        None, // available_memory_mb - not in current request
        None, // total_memory_mb - not in current request
        request.app_version.as_deref(),
        request.network_type.as_deref(),
        request.connection_quality.as_deref(),
    ).await.map_err(|e| {
        tracing::warn!("Failed to store heartbeat data: {}", e);
        // Don't fail the request if we can't store the data
    }).ok();

    // Log enhanced heartbeat data for monitoring
    tracing::debug!(
        device_id = %request.device_id,
        battery_level = ?request.battery_level,
        is_charging = ?request.is_charging,
        pending_scans = ?request.pending_scans,
        pending_jobs = ?request.pending_jobs,
        total_scans = ?request.total_scans,
        memory_usage = ?request.memory_usage,
        app_version = ?request.app_version,
        timestamp = ?request.timestamp,
        "Enhanced heartbeat received"
    );

    Ok(Json(ApiResponse::success(EnhancedHeartbeatResponse {
        acknowledged: true,
        server_time: chrono::Utc::now().timestamp_millis(),
        suggested_interval_seconds: None, // TODO: Implement adaptive interval suggestions
    })))
}

/// POST /api/connect - Device connection with full device info
pub async fn device_connect(
    State(state): State<SharedState>,
    Json(request): Json<ConnectRequest>,
) -> Result<Json<ApiResponse<ConnectResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let device_id = request.device_id.clone();

    // Create DeviceInput from ConnectRequest
    let device_input = DeviceInput {
        device_id: request.device_id.clone(),
        device_name: request.device_name.clone(),
        model: request.model,
        os_version: request.os_version,
        app_version: request.app_version,
        registration_code: None,
        fingerprint: None,
        user_agent: None,
    };

    // Register/update the device
    state.repo.upsert_device(&device_input).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    // Update existing team member's display name if they are already registered
    if let Some(ref user_name) = request.user_name {
        if !user_name.trim().is_empty() {
            if let Ok(Some(_)) = state.repo.get_team_member_by_device(&device_id).await {
                let team_input = crate::db::models::TeamMemberInput {
                    device_id: request.device_id.clone(),
                    display_name: user_name.trim().to_string(),
                    phone_number: request.phone_number.clone(),
                    role: None,
                    is_admin: None,
                    avatar_color: None,
                };
                if let Err(e) = state.repo.upsert_team_member(&team_input).await {
                    tracing::warn!("Failed to update team member: {}", e);
                }
            }
        }
    }

    // Update heartbeat for the newly connected device
    state.repo.update_device_heartbeat(&device_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    tracing::info!("Device connected: {} ({})", device_id, device_input.device_name.as_deref().unwrap_or("unnamed"));

    // Look up admin status from team member record
    let is_admin = state.repo.get_team_member_by_device(&device_id).await
        .ok()
        .flatten()
        .map(|m| m.is_admin != 0)
        .unwrap_or(false);

    Ok(Json(ApiResponse::success_with_message(
        ConnectResponse {
            connected: true,
            device_id,
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            server_time: chrono::Utc::now().timestamp_millis(),
            is_admin,
        },
        "Device connected successfully",
    )))
}

/// POST /api/devices/disconnect - Device explicitly disconnecting
pub async fn device_disconnect(
    State(state): State<SharedState>,
    Json(request): Json<DisconnectRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    // Clear last_seen_at so server immediately shows device as offline
    state.repo.clear_device_last_seen(&request.device_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;
    
    tracing::info!("Device disconnected: {}", request.device_id);
    
    Ok(Json(ApiResponse::success_with_message(
        (),
        "Device disconnected",
    )))
}

// ==================== WEB CLIENT HANDLERS ====================

#[derive(Deserialize)]
pub struct RegisterWebClientRequest {
    #[serde(default)]
    pub client_name: Option<String>,
}

#[derive(Serialize)]
pub struct WebClientResponse {
    pub client_id: String,
    pub is_trusted: bool,
    pub client_name: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Serialize)]
pub struct WebClientsResponse {
    pub clients: Vec<WebClient>,
}

/// POST /api/web/register - Register a web client
pub async fn register_web_client(
    State(state): State<SharedState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<RegisterWebClientRequest>,
) -> Result<Json<ApiResponse<WebClientResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    // Generate a unique client ID
    let client_id = uuid::Uuid::new_v4().to_string();
    
    // Extract user agent and try to get IP
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    
    // For IP, we'd need to trust proxy headers in production
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    
    let client = state.repo.upsert_web_client(
        &client_id,
        request.client_name.as_deref(),
        user_agent.as_deref(),
        ip_address.as_deref(),
    ).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    tracing::info!("Web client registered: {}", client_id);
    
    Ok(Json(ApiResponse::success(WebClientResponse {
        client_id: client.client_id,
        is_trusted: client.is_trusted != 0,
        client_name: client.client_name,
        user_id: client.user_id,
    })))
}

#[derive(Deserialize)]
pub struct WebClientStatusQuery {
    pub client_id: String,
}

/// GET /api/web/status - Check web client status
pub async fn get_web_client_status(
    State(state): State<SharedState>,
    Query(query): Query<WebClientStatusQuery>,
) -> Result<Json<ApiResponse<WebClientResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    let client = state.repo.get_web_client(&query.client_id).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    match client {
        Some(c) => Ok(Json(ApiResponse::success(WebClientResponse {
            client_id: c.client_id,
            is_trusted: c.is_trusted != 0,
            client_name: c.client_name,
            user_id: c.user_id,
        }))),
        None => Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("Client not found")))),
    }
}

/// GET /api/web/clients - Get all web clients (for desktop app)
pub async fn get_web_clients(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<WebClientsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    let clients = state.repo.get_web_clients().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    Ok(Json(ApiResponse::success(WebClientsResponse { clients })))
}

#[derive(Deserialize)]
pub struct SetTrustRequest {
    pub is_trusted: bool,
}

/// PUT /api/web/clients/:client_id/trust - Set client trust status
pub async fn set_web_client_trust(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(request): Json<SetTrustRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    state.repo.set_web_client_trust(&client_id, request.is_trusted).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    let action = if request.is_trusted { "trusted" } else { "untrusted" };
    tracing::info!("Web client {} marked as {}", client_id, action);
    
    Ok(Json(ApiResponse::success_with_message((), format!("Client {}", action))))
}

/// DELETE /api/web/clients/:client_id - Delete a web client
pub async fn delete_web_client(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    state.repo.delete_web_client(&client_id).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    Ok(Json(ApiResponse::success_with_message((), "Client deleted")))
}

#[derive(Deserialize)]
pub struct UpdateClientNameRequest {
    pub name: String,
}

/// PUT /api/web/clients/:client_id/name - Update a web client's display name
pub async fn update_web_client_name(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(request): Json<UpdateClientNameRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    state.repo.update_web_client_name(&client_id, &request.name).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    tracing::info!("Web client {} renamed to '{}'", client_id, request.name);
    
    Ok(Json(ApiResponse::success_with_message((), "Name updated")))
}

#[derive(Deserialize)]
pub struct LinkWebClientRequest {
    pub user_id: Option<String>,
}

/// PUT /api/web/clients/:client_id/link - Link or unlink a web client to/from a team member
pub async fn link_web_client(
    State(state): State<SharedState>,
    Path(client_id): Path<String>,
    Json(request): Json<LinkWebClientRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    match request.user_id {
        Some(user_id) => {
            state.repo.link_web_client_to_user(&client_id, &user_id).await.map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
            })?;
            tracing::info!("Web client {} linked to user {}", client_id, user_id);
            Ok(Json(ApiResponse::success_with_message((), "Client linked to user")))
        }
        None => {
            state.repo.unlink_web_client_user(&client_id).await.map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
            })?;
            tracing::info!("Web client {} unlinked from user", client_id);
            Ok(Json(ApiResponse::success_with_message((), "Client unlinked")))
        }
    }
}

// ==================== PENDING CHANGES HANDLERS ====================

#[derive(Deserialize)]
pub struct SubmitChangeRequest {
    pub client_id: String,
    pub change_type: String,    // "create", "update", "delete"
    pub entity_type: String,    // "job", "device", "scan"
    #[serde(default)]
    pub entity_id: Option<String>,
    pub change_data: serde_json::Value,
}

#[derive(Serialize)]
pub struct SubmitChangeResponse {
    pub applied: bool,
    pub pending_id: Option<i64>,
    pub message: String,
}

#[derive(Serialize)]
pub struct PendingChangesResponse {
    pub changes: Vec<PendingChange>,
    pub count: i64,
}

/// POST /api/web/changes - Submit a change (queued if untrusted, applied if trusted)
pub async fn submit_change(
    State(state): State<SharedState>,
    Json(request): Json<SubmitChangeRequest>,
) -> Result<Json<ApiResponse<SubmitChangeResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    // Get client to check trust status
    let client = state.repo.get_web_client(&request.client_id).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    let client = match client {
        Some(c) => c,
        None => return Err((StatusCode::UNAUTHORIZED, Json(ApiResponse::error("Unknown client - please register first")))),
    };
    
    // Update last seen
    let _ = state.repo.upsert_web_client(
        &request.client_id,
        None, None, None,
    ).await;
    
    if client.is_trusted != 0 {
        // Apply change directly
        match apply_change(&state.repo, &request).await {
            Ok(msg) => Ok(Json(ApiResponse::success(SubmitChangeResponse {
                applied: true,
                pending_id: None,
                message: msg,
            }))),
            Err(e) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(e)))),
        }
    } else {
        // Queue for approval
        let change_data = serde_json::to_string(&request.change_data).unwrap_or_default();
        
        let pending_id = state.repo.add_pending_change(
            &request.client_id,
            &request.change_type,
            &request.entity_type,
            request.entity_id.as_deref(),
            &change_data,
        ).await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
        })?;
        
        tracing::info!("Pending change {} queued from client {}", pending_id, request.client_id);
        
        Ok(Json(ApiResponse::success(SubmitChangeResponse {
            applied: false,
            pending_id: Some(pending_id),
            message: "Change queued for approval".to_string(),
        })))
    }
}

/// Apply a change directly (called for trusted clients or when approving)
async fn apply_change(
    repo: &crate::db::repository::Repository,
    request: &SubmitChangeRequest,
) -> Result<String, String> {
    match (request.change_type.as_str(), request.entity_type.as_str()) {
        ("create", "job") => {
            let job: JobInput = serde_json::from_value(request.change_data.clone())
                .map_err(|e| format!("Invalid job data: {}", e))?;
            repo.upsert_job(&job).await.map_err(|e| e.to_string())?;
            Ok(format!("Job '{}' created", job.name))
        }
        ("update", "job") => {
            let job: JobInput = serde_json::from_value(request.change_data.clone())
                .map_err(|e| format!("Invalid job data: {}", e))?;
            repo.upsert_job(&job).await.map_err(|e| e.to_string())?;
            Ok(format!("Job '{}' updated", job.name))
        }
        ("delete", "job") => {
            let job_id = request.entity_id.as_ref()
                .ok_or("Job ID required for delete")?
                .parse::<i64>()
                .map_err(|_| "Invalid job ID")?;
            repo.delete_job(job_id).await.map_err(|e| e.to_string())?;
            Ok("Job deleted".to_string())
        }
        ("update", "device") => {
            let device: DeviceInput = serde_json::from_value(request.change_data.clone())
                .map_err(|e| format!("Invalid device data: {}", e))?;
            repo.upsert_device(&device).await.map_err(|e| e.to_string())?;
            Ok(format!("Device '{}' updated", device.device_name.unwrap_or_default()))
        }
        ("delete", "device") => {
            let device_id = request.entity_id.as_ref()
                .ok_or("Device ID required for delete")?;
            repo.delete_device(device_id).await.map_err(|e| e.to_string())?;
            Ok("Device deleted".to_string())
        }
        ("create", "scan") => {
            let scan: ScanInput = serde_json::from_value(request.change_data.clone())
                .map_err(|e| format!("Invalid scan data: {}", e))?;
            let scan_id = repo.upsert_scan(&scan).await.map_err(|e| e.to_string())?;

            // Resolve job linkage if job_id (UUID) provided
            if let Some(ref job_uuid) = scan.job_id {
                if !job_uuid.is_empty() {
                    if let Ok(Some(job)) = repo.get_job_by_uuid(job_uuid).await {
                        let _ = repo.set_scan_job_id(scan_id, job.id).await;
                    }
                }
            }

            Ok(format!("Scan '{}' created", scan.ticket_number.as_deref().unwrap_or(&scan.barcode_data)))
        }
        ("delete", "scan") => {
            let scan_id = request.entity_id.as_ref()
                .ok_or("Scan ID required for delete")?
                .parse::<i64>()
                .map_err(|_| "Invalid scan ID")?;
            repo.delete_scan(scan_id).await.map_err(|e| e.to_string())?;
            Ok("Scan deleted".to_string())
        }
        ("update", "scan_assignments") => {
            #[derive(Deserialize)]
            struct AssignData {
                scan_ids: Vec<i64>,
                job_id: Option<i64>,
                device_id: Option<String>,
            }
            let data: AssignData = serde_json::from_value(request.change_data.clone())
                .map_err(|e| format!("Invalid assign data: {}", e))?;
            let updated = repo.update_scan_assignments(
                &data.scan_ids,
                data.job_id,
                data.device_id.as_deref(),
            ).await.map_err(|e| e.to_string())?;
            Ok(format!("{} scans updated", updated))
        }
        ("update", "job_details") => {
            let job_id = request.entity_id.as_ref()
                .ok_or("Job ID required for update")?
                .parse::<i64>()
                .map_err(|_| "Invalid job ID")?;
            #[derive(Deserialize)]
            struct JobEditData {
                name: Option<String>,
                status: Option<String>,
                reference_number: Option<Option<String>>,
                description: Option<Option<String>>,
                customer_name: Option<Option<String>>,
                notes: Option<Option<String>>,
                priority: Option<String>,
                expected_count: Option<i32>,
                due_date: Option<Option<String>>,
            }
            let data: JobEditData = serde_json::from_value(request.change_data.clone())
                .map_err(|e| format!("Invalid job edit data: {}", e))?;
            repo.update_job_details(
                job_id,
                data.name.as_deref(),
                data.status.as_deref(),
                data.reference_number.as_ref().map(|v| v.as_deref()),
                data.description.as_ref().map(|v| v.as_deref()),
                data.customer_name.as_ref().map(|v| v.as_deref()),
                data.notes.as_ref().map(|v| v.as_deref()),
                data.priority.as_deref(),
                data.expected_count,
                data.due_date.as_ref().map(|v| v.as_deref()),
                None,
            ).await.map_err(|e| e.to_string())?;
            Ok("Job updated".to_string())
        }
        _ => Err(format!("Unsupported operation: {} {}", request.change_type, request.entity_type)),
    }
}

/// GET /api/web/changes/pending - Get pending changes (for desktop app)
pub async fn get_pending_changes(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<PendingChangesResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    let changes = state.repo.get_pending_changes().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    let count = changes.len() as i64;
    
    Ok(Json(ApiResponse::success(PendingChangesResponse { changes, count })))
}

/// POST /api/web/changes/:id/approve - Approve a pending change
pub async fn approve_change(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<SubmitChangeResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    // Get the pending change
    let change = state.repo.get_pending_change(id).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    let change = match change {
        Some(c) => c,
        None => return Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("Change not found")))),
    };
    
    if change.status != "pending" {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("Change already {}", change.status)))));
    }
    
    // Parse the change data
    let change_data: serde_json::Value = serde_json::from_str(&change.change_data)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    
    // Build a request to apply
    let apply_request = SubmitChangeRequest {
        client_id: change.client_id.clone(),
        change_type: change.change_type.clone(),
        entity_type: change.entity_type.clone(),
        entity_id: change.entity_id.clone(),
        change_data,
    };
    
    // Apply the change
    match apply_change(&state.repo, &apply_request).await {
        Ok(msg) => {
            // Mark as approved
            state.repo.approve_change(id, Some("desktop")).await.map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
            })?;
            
            tracing::info!("Change {} approved and applied", id);
            
            Ok(Json(ApiResponse::success(SubmitChangeResponse {
                applied: true,
                pending_id: Some(id),
                message: msg,
            })))
        }
        Err(e) => {
            // Still mark as approved but note the error
            tracing::error!("Change {} approved but failed to apply: {}", id, e);
            Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(e))))
        }
    }
}

/// POST /api/web/changes/:id/reject - Reject a pending change
pub async fn reject_change(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    
    // Verify the change exists and is pending
    let change = state.repo.get_pending_change(id).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    match change {
        Some(c) if c.status != "pending" => {
            return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("Change already {}", c.status)))));
        }
        None => return Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("Change not found")))),
        _ => {}
    }
    
    state.repo.reject_change(id, Some("desktop")).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))
    })?;
    
    tracing::info!("Change {} rejected", id);
    
    Ok(Json(ApiResponse::success_with_message((), "Change rejected")))
}

/// PUT /api/scans/assign - Bulk update scan job/device assignments
pub async fn update_scan_assignments(
    State(state): State<SharedState>,
    Json(request): Json<UpdateScanAssignmentsRequest>,
) -> Result<Json<ApiResponse<UpdateScanAssignmentsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    if request.scan_ids.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("No scan IDs provided")),
        ));
    }

    let updated = state
        .repo
        .update_scan_assignments(
            &request.scan_ids,
            request.job_id,
            request.device_id.as_deref(),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(e.to_string())),
            )
        })?;

    tracing::info!(
        "Updated {} scans: job_id={:?}, device_id={:?}",
        updated,
        request.job_id,
        request.device_id
    );

    Ok(Json(ApiResponse::success(UpdateScanAssignmentsResponse {
        updated,
    })))
}

// ==================== SECURITY HANDLERS ====================

/// POST /api/devices/approve - Approve or reject device registration
pub async fn approve_device(
    State(state): State<SharedState>,
    Json(request): Json<DeviceApprovalRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut state = state.write().await;

    // Check if device exists
    let device = state.repo.get_device(&request.device_id).await
        .map_err(|_| (StatusCode::NOT_FOUND, Json(ApiResponse::error("Device not found"))))?;

    if device.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("Device not found"))));
    }

    // Update device approval status
    state.repo.update_device_approval(&request.device_id, request.approved, request.notes.as_deref()).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    let action = if request.approved { "approved" } else { "rejected" };
    Ok(Json(ApiResponse::success_with_message((), format!("Device {} {}", request.device_id, action))))
}

/// POST /api/devices/auth - Authenticate device with token
pub async fn authenticate_device(
    State(state): State<SharedState>,
    Json(request): Json<DeviceAuthRequest>,
) -> Result<Json<ApiResponse<DeviceAuthResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    // Verify device authentication
    let device = state.repo.authenticate_device(&request.device_id, &request.auth_token).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiResponse::error("Invalid credentials"))))?;

    // Check if device is approved
    if device.is_approved != 1 {
        return Err((StatusCode::FORBIDDEN, Json(ApiResponse::error("Device not approved"))));
    }

    // Check if device is blocked
    if let Some(blocked_until) = &device.blocked_until {
        if chrono::Utc::now().timestamp() < chrono::DateTime::parse_from_rfc3339(blocked_until)
            .map_err(|_| (StatusCode::FORBIDDEN, Json(ApiResponse::error("Device temporarily blocked"))))?
            .timestamp() {
            return Err((StatusCode::FORBIDDEN, Json(ApiResponse::error("Device temporarily blocked"))));
        }
    }

    Ok(Json(ApiResponse::success(DeviceAuthResponse {
        authenticated: true,
        device_id: device.device_id,
        device_name: device.device_name,
    })))
}

/// POST /api/devices/block - Block device temporarily
pub async fn block_device(
    State(state): State<SharedState>,
    Json(request): Json<BlockDeviceRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut state = state.write().await;

    state.repo.block_device(&request.device_id, request.minutes).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success_with_message((), format!("Device {} blocked for {} minutes", request.device_id, request.minutes))))
}

/// POST /api/devices/unblock - Unblock device
pub async fn unblock_device(
    State(state): State<SharedState>,
    Json(request): Json<UnblockDeviceRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut state = state.write().await;

    state.repo.unblock_device(&request.device_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success_with_message((), format!("Device {} unblocked", request.device_id))))
}

/// GET /api/security/settings - Get security settings
pub async fn get_security_settings(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<SecuritySettings>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let settings = state.repo.get_security_settings().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success(settings)))
}

/// PUT /api/security/settings - Update security settings
pub async fn update_security_settings(
    State(state): State<SharedState>,
    Json(settings): Json<SecuritySettings>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut state = state.write().await;

    state.repo.update_security_settings(&settings).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success_with_message((), "Security settings updated")))
}

/// POST /api/security/generate-code - Generate registration code
pub async fn generate_registration_code(
    State(state): State<SharedState>,
    Json(request): Json<GenerateCodeRequest>,
) -> Result<Json<ApiResponse<RegistrationCodeResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.write().await;

    let code = state.repo.generate_registration_code(request.description.as_deref()).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success(RegistrationCodeResponse { code })))
}

/// GET /api/security/codes - Get active registration codes
pub async fn get_registration_codes(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<Vec<RegistrationCodeInfo>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let codes = state.repo.get_registration_codes().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success(codes)))
}

// ==================== TIME ENTRIES ====================

#[derive(Deserialize)]
pub struct SyncTimeEntriesRequest {
    pub device_id: String,
    pub entries: Vec<TimeEntryInput>,
}

#[derive(Serialize)]
pub struct SyncTimeEntriesResponse {
    pub synced_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct GetTimeEntriesQuery {
    pub device_id: Option<String>,
    pub job_id: Option<String>,
    pub since: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct GetTimeEntriesResponse {
    pub entries: Vec<TimeEntryRecord>,
    pub total: i64,
}

/// POST /api/time-entries - Sync time entries from device
pub async fn sync_time_entries(
    State(state): State<SharedState>,
    Json(request): Json<SyncTimeEntriesRequest>,
) -> Result<Json<ApiResponse<SyncTimeEntriesResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let mut synced_ids = Vec::new();

    for entry in &request.entries {
        match state.repo.upsert_time_entry(entry).await {
            Ok(_) => synced_ids.push(entry.id.clone()),
            Err(e) => tracing::warn!("Failed to sync time entry {}: {}", entry.id, e),
        }
    }

    // Auto-break: if enabled, insert a break for work entries that exceed the threshold
    let auto_break = match state.repo.get_setting(AUTO_BREAK_SETTING_KEY).await {
        Ok(Some(json_str)) => serde_json::from_str::<AutoBreakConfig>(&json_str).unwrap_or_default(),
        _ => AutoBreakConfig::default(),
    };

    if auto_break.enabled {
        let threshold_ms = auto_break.threshold_hours as i64 * 3_600_000;
        let break_ms = auto_break.break_minutes as i64 * 60_000;

        for entry in &request.entries {
            // Only applies to work entries (not breaks) that have a clock_out
            if entry.is_break.unwrap_or(false) {
                continue;
            }
            let clock_out_ms = match entry.clock_out {
                Some(co) => co,
                None => continue,
            };

            let worked_ms = clock_out_ms - entry.clock_in;
            if worked_ms < threshold_ms {
                continue;
            }

            // Check if an auto-break already exists for this entry
            let auto_break_id = format!("auto-break-{}", entry.id);
            if state.repo.time_entry_exists(&auto_break_id).await.unwrap_or(true) {
                continue;
            }

            // Insert a 30-min unpaid break starting at the threshold point
            let break_start = entry.clock_in + threshold_ms;
            let break_end = break_start + break_ms;

            let break_entry = TimeEntryInput {
                id: auto_break_id.clone(),
                device_id: entry.device_id.clone(),
                customer_name: entry.customer_name.clone(),
                job_name: entry.job_name.clone(),
                job_id: entry.job_id.clone(),
                clock_in: break_start,
                clock_out: Some(break_end),
                note: Some(format!("Auto {}min break - worked over {}hrs", auto_break.break_minutes, auto_break.threshold_hours)),
                is_break: Some(true),
                is_paid: Some(false),
            };

            match state.repo.upsert_time_entry(&break_entry).await {
                Ok(_) => tracing::info!("Auto-break inserted for entry {}", entry.id),
                Err(e) => tracing::warn!("Failed to insert auto-break for {}: {}", entry.id, e),
            }
        }
    }

    let count = synced_ids.len();
    Ok(Json(ApiResponse::success_with_message(
        SyncTimeEntriesResponse { synced_ids },
        format!("Synced {} time entries", count),
    )))
}

/// GET /api/time-entries - Get time entries with optional filtering
pub async fn get_time_entries(
    State(state): State<SharedState>,
    Query(params): Query<GetTimeEntriesQuery>,
) -> Result<Json<ApiResponse<GetTimeEntriesResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let limit = params.limit.unwrap_or(500);

    let entries = state.repo.get_time_entries(
        params.device_id.as_deref(),
        params.job_id.as_deref(),
        params.since,
        limit,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    let total = state.repo.get_time_entry_count().await.unwrap_or(0);

    Ok(Json(ApiResponse::success(GetTimeEntriesResponse { entries, total })))
}

// ==================== REPORTS ====================

#[derive(Deserialize)]
pub struct SyncReportsRequest {
    pub device_id: String,
    pub reports: Vec<ReportInput>,
}

#[derive(Serialize)]
pub struct SyncReportsResponse {
    pub synced_report_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct GetReportsQuery {
    pub job_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct GetReportsResponse {
    pub reports: Vec<ReportRecord>,
    pub total: i64,
}

/// POST /api/reports - Sync reports from device
pub async fn sync_reports(
    State(state): State<SharedState>,
    Json(request): Json<SyncReportsRequest>,
) -> Result<Json<ApiResponse<SyncReportsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let mut synced_ids = Vec::new();

    for report in &request.reports {
        match state.repo.upsert_report(report).await {
            Ok(_) => {
                // Auto-update room progress from this report
                if let Err(e) = state.repo.update_room_progress_from_report(report).await {
                    tracing::warn!("Failed to update room progress for report {}: {}", report.id, e);
                }
                synced_ids.push(report.id.clone());
            }
            Err(e) => tracing::warn!("Failed to sync report {}: {}", report.id, e),
        }
    }

    let count = synced_ids.len();
    Ok(Json(ApiResponse::success_with_message(
        SyncReportsResponse { synced_report_ids: synced_ids },
        format!("Synced {} reports", count),
    )))
}

/// GET /api/reports - Get reports with optional filtering
pub async fn get_reports(
    State(state): State<SharedState>,
    Query(params): Query<GetReportsQuery>,
) -> Result<Json<ApiResponse<GetReportsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let limit = params.limit.unwrap_or(500);

    let reports = state.repo.get_reports(
        params.job_id.as_deref(),
        params.status.as_deref(),
        limit,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    let total = state.repo.get_report_count().await.unwrap_or(0);

    Ok(Json(ApiResponse::success(GetReportsResponse { reports, total })))
}

/// POST /api/reports/:report_id/photos - Upload a report photo (multipart)
pub async fn upload_report_photo(
    State(state): State<SharedState>,
    Path(report_id): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ApiResponse<SyncReportsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let mut file_name = String::from("photo.jpg");
    let mut photo_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "photo" || name == "file" {
            if let Some(fname) = field.file_name() {
                file_name = fname.to_string();
            }
            if let Ok(bytes) = field.bytes().await {
                photo_data = Some(bytes.to_vec());
            }
        }
    }

    let uuid = uuid::Uuid::new_v4().to_string();
    state.repo.upsert_report_photo(
        &uuid,
        &report_id,
        None,
        &file_name,
        photo_data.as_deref(),
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success_with_message(
        SyncReportsResponse { synced_report_ids: vec![uuid] },
        "Photo uploaded".to_string(),
    )))
}

// ==================== LOCATION PINGS ====================

#[derive(Deserialize)]
pub struct SyncLocationPingsRequest {
    pub device_id: String,
    pub pings: Vec<LocationPingInput>,
}

#[derive(Serialize)]
pub struct SyncLocationPingsResponse {
    pub synced_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct GetLocationPingsQuery {
    pub device_id: Option<String>,
    pub time_entry_id: Option<String>,
    pub since: Option<i64>,
    pub active_only: Option<bool>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct GetLocationPingsResponse {
    pub pings: Vec<LocationPingRecord>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct LatestPositionsResponse {
    pub positions: Vec<ActiveWorkerPosition>,
}

pub async fn sync_location_pings(
    State(state): State<SharedState>,
    Json(request): Json<SyncLocationPingsRequest>,
) -> Result<Json<ApiResponse<SyncLocationPingsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let mut synced_ids = Vec::new();

    for ping in &request.pings {
        match state.repo.upsert_location_ping(&request.device_id, ping).await {
            Ok(_) => synced_ids.push(ping.id.clone()),
            Err(e) => {
                tracing::warn!("Failed to upsert location ping {}: {}", ping.id, e);
            }
        }
    }

    Ok(Json(ApiResponse::success(SyncLocationPingsResponse { synced_ids })))
}

pub async fn get_location_pings(
    State(state): State<SharedState>,
    Query(query): Query<GetLocationPingsQuery>,
) -> Result<Json<ApiResponse<GetLocationPingsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let limit = query.limit.unwrap_or(5000);
    let active_only = query.active_only.unwrap_or(false);

    let pings = state
        .repo
        .get_location_pings(
            query.device_id.as_deref(),
            query.time_entry_id.as_deref(),
            query.since,
            active_only,
            limit,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    let total = pings.len();
    Ok(Json(ApiResponse::success(GetLocationPingsResponse { pings, total })))
}

pub async fn get_latest_positions(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<LatestPositionsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let positions = state
        .repo
        .get_latest_active_positions()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success(LatestPositionsResponse { positions })))
}

// ==================== TEAM MEMBERS ====================

#[derive(Serialize)]
pub struct TeamStatusResponse {
    pub members: Vec<TeamMemberStatus>,
}

#[derive(Serialize)]
pub struct TeamMembersResponse {
    pub members: Vec<crate::db::models::TeamMember>,
}

pub async fn get_team_status(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<TeamStatusResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let members = state.repo.get_team_status().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success(TeamStatusResponse { members })))
}

pub async fn get_team_members(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<TeamMembersResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let members = state.repo.get_team_members().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success(TeamMembersResponse { members })))
}

pub async fn upsert_team_member(
    State(state): State<SharedState>,
    Json(input): Json<TeamMemberInput>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    state.repo.upsert_team_member(&input).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"device_id": input.device_id}),
        "Team member saved",
    )))
}

pub async fn delete_team_member(
    State(state): State<SharedState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    // Close any open time entries for this device before deleting
    if let Err(e) = state.repo.close_open_time_entries(&device_id).await {
        tracing::warn!("Failed to close open time entries for {}: {}", device_id, e);
    }

    let deleted = state.repo.delete_team_member(&device_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    if deleted {
        Ok(Json(ApiResponse::success_with_message(serde_json::json!({}), "Team member removed")))
    } else {
        Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("Team member not found"))))
    }
}

// ==================== TIMESHEETS ====================

#[derive(Deserialize)]
pub struct GetTimesheetsQuery {
    pub device_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct TimesheetsResponse {
    pub days: Vec<TimesheetDay>,
    pub total: usize,
}

pub async fn get_timesheets(
    State(state): State<SharedState>,
    Query(query): Query<GetTimesheetsQuery>,
) -> Result<Json<ApiResponse<TimesheetsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let limit = query.limit.unwrap_or(1000);

    let days = state.repo.get_timesheets(
        query.device_id.as_deref(),
        query.date_from.as_deref(),
        query.date_to.as_deref(),
        limit,
    ).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    let total = days.len();
    Ok(Json(ApiResponse::success(TimesheetsResponse { days, total })))
}

// ==================== ROOM PROGRESS ====================

#[derive(Deserialize)]
pub struct GetRoomProgressQuery {
    pub search: Option<String>,
}

#[derive(Serialize)]
pub struct RoomProgressResponse {
    pub rooms: Vec<crate::db::models::RoomProgress>,
    pub summary: crate::db::models::RoomProgressSummary,
}

/// GET /api/jobs/:job_id/rooms - Get room progress for a job with optional search
pub async fn get_job_rooms(
    State(state): State<SharedState>,
    Path(job_id): Path<String>,
    Query(query): Query<GetRoomProgressQuery>,
) -> Result<Json<ApiResponse<RoomProgressResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    // Resolve job UUID to internal job_id
    let job_uuid = job_id;

    let rooms = if let Some(ref search) = query.search {
        if !search.is_empty() {
            state.repo.search_room_progress(&job_uuid, search).await
        } else {
            state.repo.get_room_progress_for_job(&job_uuid).await
        }
    } else {
        state.repo.get_room_progress_for_job(&job_uuid).await
    }
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    let summary = state.repo.get_room_progress_summary(&job_uuid).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success(RoomProgressResponse { rooms, summary })))
}

/// GET /api/jobs/:job_id/rooms/:room_name - Get a single room's progress
pub async fn get_room_detail(
    State(state): State<SharedState>,
    Path((job_id, room_name)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Option<crate::db::models::RoomProgress>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let room = state.repo.get_room_progress(&job_id, &room_name).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success(room)))
}

// ==================== ROOM ITEMS CONFIG ====================

const DEFAULT_ROOM_ITEMS: &[&str] = &["fillers", "handles", "fast_caps", "set_boxes", "caulking"];
const ROOM_ITEMS_SETTING_KEY: &str = "room_items";

/// GET /api/settings/room-items - Get the list of room completion items
pub async fn get_room_items(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<Vec<String>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let items = match state.repo.get_setting(ROOM_ITEMS_SETTING_KEY).await {
        Ok(Some(json_str)) => {
            serde_json::from_str::<Vec<String>>(&json_str)
                .unwrap_or_else(|_| DEFAULT_ROOM_ITEMS.iter().map(|s| s.to_string()).collect())
        }
        _ => DEFAULT_ROOM_ITEMS.iter().map(|s| s.to_string()).collect(),
    };

    Ok(Json(ApiResponse::success(items)))
}

/// PUT /api/settings/room-items - Update the list of room completion items (admin)
pub async fn set_room_items(
    State(state): State<SharedState>,
    Json(items): Json<Vec<String>>,
) -> Result<Json<ApiResponse<Vec<String>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    // Filter out empty strings
    let items: Vec<String> = items.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let json_str = serde_json::to_string(&items)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    state.repo.set_setting(ROOM_ITEMS_SETTING_KEY, &json_str).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    tracing::info!("Room items updated: {:?}", items);

    Ok(Json(ApiResponse::success(items)))
}

// ==================== AUTO BREAK CONFIG ====================

const AUTO_BREAK_SETTING_KEY: &str = "auto_break";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBreakConfig {
    pub enabled: bool,
    pub threshold_hours: i32,
    pub break_minutes: i32,
}

impl Default for AutoBreakConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_hours: 8,
            break_minutes: 30,
        }
    }
}

/// GET /api/settings/auto-break - Get the auto break config
pub async fn get_auto_break(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<AutoBreakConfig>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let config = match state.repo.get_setting(AUTO_BREAK_SETTING_KEY).await {
        Ok(Some(json_str)) => {
            serde_json::from_str::<AutoBreakConfig>(&json_str)
                .unwrap_or_default()
        }
        _ => AutoBreakConfig::default(),
    };

    Ok(Json(ApiResponse::success(config)))
}

/// PUT /api/settings/auto-break - Update the auto break config
pub async fn set_auto_break(
    State(state): State<SharedState>,
    Json(config): Json<AutoBreakConfig>,
) -> Result<Json<ApiResponse<AutoBreakConfig>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let config = AutoBreakConfig {
        enabled: config.enabled,
        threshold_hours: config.threshold_hours.clamp(1, 24),
        break_minutes: config.break_minutes.clamp(1, 120),
    };

    let json_str = serde_json::to_string(&config)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    state.repo.set_setting(AUTO_BREAK_SETTING_KEY, &json_str).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    tracing::info!("Auto break config updated: {:?}", config);

    Ok(Json(ApiResponse::success(config)))
}

// ==================== TIME ROUNDING CONFIG ====================

const TIME_ROUNDING_SETTING_KEY: &str = "time_rounding";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRoundingConfig {
    pub enabled: bool,
    pub interval_minutes: i32,
}

impl Default for TimeRoundingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 15,
        }
    }
}

/// GET /api/settings/time-rounding - Get the time rounding config
pub async fn get_time_rounding(
    State(state): State<SharedState>,
) -> Result<Json<ApiResponse<TimeRoundingConfig>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let config = match state.repo.get_setting(TIME_ROUNDING_SETTING_KEY).await {
        Ok(Some(json_str)) => {
            serde_json::from_str::<TimeRoundingConfig>(&json_str)
                .unwrap_or_default()
        }
        _ => TimeRoundingConfig::default(),
    };

    Ok(Json(ApiResponse::success(config)))
}

/// PUT /api/settings/time-rounding - Update the time rounding config (admin)
pub async fn set_time_rounding(
    State(state): State<SharedState>,
    Json(config): Json<TimeRoundingConfig>,
) -> Result<Json<ApiResponse<TimeRoundingConfig>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    // Clamp interval to sensible range (1-60 minutes)
    let config = TimeRoundingConfig {
        enabled: config.enabled,
        interval_minutes: config.interval_minutes.clamp(1, 60),
    };

    let json_str = serde_json::to_string(&config)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    state.repo.set_setting(TIME_ROUNDING_SETTING_KEY, &json_str).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    tracing::info!("Time rounding config updated: {:?}", config);

    Ok(Json(ApiResponse::success(config)))
}

// ==================== JOB FILES ====================

#[derive(Serialize)]
pub struct JobFilesResponse {
    pub files: Vec<crate::db::models::JobFileMeta>,
}

#[derive(Serialize)]
pub struct JobFileSearchResponse {
    pub results: Vec<crate::db::models::JobFileSearchResult>,
}

#[derive(Deserialize)]
pub struct JobFileSearchQuery {
    pub q: String,
    pub job_id: Option<String>,
}

/// POST /api/jobs/:job_id/files - Upload a file (PDF) for a job
pub async fn upload_job_file(
    State(state): State<SharedState>,
    Path(job_id): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ApiResponse<crate::db::models::JobFileMeta>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    let mut file_name = String::from("file.pdf");
    let mut file_data: Option<Vec<u8>> = None;
    let mut device_id: Option<String> = None;
    let mut uploader_name: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                if let Some(fname) = field.file_name() {
                    file_name = fname.to_string();
                }
                match field.bytes().await {
                    Ok(bytes) => file_data = Some(bytes.to_vec()),
                    Err(e) => return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("Failed to read file: {}", e))))),
                }
            }
            "device_id" => {
                if let Ok(text) = field.text().await {
                    device_id = Some(text.trim().to_string());
                }
            }
            "uploader_name" => {
                if let Ok(text) = field.text().await {
                    uploader_name = Some(text.trim().to_string());
                }
            }
            _ => {}
        }
    }

    let file_bytes = file_data.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error("No file data found")))
    })?;

    // Save PDF to disk: {data_dir}/jobs/{job_id}/
    let job_dir = get_data_dir().join("jobs").join(&job_id);
    if let Err(e) = tokio::fs::create_dir_all(&job_dir).await {
        tracing::warn!("[PDF] Failed to create job directory {:?}: {}", job_dir, e);
    } else {
        let file_path = job_dir.join(&file_name);
        match tokio::fs::write(&file_path, &file_bytes).await {
            Ok(_) => tracing::info!("[PDF] Saved file to disk: {:?} ({} bytes)", file_path, file_bytes.len()),
            Err(e) => tracing::warn!("[PDF] Failed to save file to disk {:?}: {}", file_path, e),
        }
    }

    // Basic PDF text extraction for search
    let extracted_text = extract_pdf_text(&file_bytes, &file_name);

    let uuid = uuid::Uuid::new_v4().to_string();
    let file_size = file_bytes.len() as i64;

    state.repo.insert_job_file(
        &uuid,
        &job_id,
        &file_name,
        "application/pdf",
        &file_bytes,
        extracted_text.as_deref(),
        device_id.as_deref(),
        uploader_name.as_deref(),
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    tracing::info!("[PDF] Upload complete: job={}, file='{}', size={} bytes, extracted={} chars",
        job_id, file_name, file_size,
        extracted_text.as_ref().map(|t| t.len()).unwrap_or(0));

    // If this is a bill of lading PDF, parse ticket data
    let lower_name = file_name.to_lowercase();
    let is_lading = lower_name.contains("lading") || lower_name.contains("bol");

    if is_lading {
        tracing::info!("[PDF] File '{}' detected as bill of lading PDF, attempting ticket parsing", file_name);
        if let Some(ref text) = extracted_text {
            let tickets = parse_lading_tickets(text, &job_id, &uuid, &file_name);
            if !tickets.is_empty() {
                let count = tickets.len();
                match state.repo.insert_lading_tickets(&tickets).await {
                    Ok(inserted) => tracing::info!("[PDF] Inserted {} lading tickets from '{}'", inserted, file_name),
                    Err(e) => tracing::error!("[PDF] Failed to insert lading tickets from '{}': {}", file_name, e),
                }
                tracing::info!("[PDF] Parsed {} lading tickets from '{}'", count, file_name);
            } else {
                tracing::warn!("[PDF] Lading file '{}' yielded 0 tickets from {} chars of text. First 500 chars: {:?}",
                    file_name, text.len(), &text[..text.len().min(500)]);
            }
        } else {
            tracing::warn!("[PDF] Lading file '{}' has no extracted text — cannot parse tickets", file_name);
        }

        // Extract address from BOL and set on job if not already set
        if let Some(ref text) = extracted_text {
            if let Some(address) = extract_bol_address(text) {
                match state.repo.set_job_address_if_empty(&job_id, &address).await {
                    Ok(true) => tracing::info!("[PDF] Set job address from BOL: '{}'", address),
                    Ok(false) => tracing::debug!("[PDF] Job already has an address, not overwriting"),
                    Err(e) => tracing::warn!("[PDF] Failed to set job address: {}", e),
                }
            }
        }
    } else {
        tracing::debug!("[PDF] File '{}' not a lading PDF, skipping ticket parsing", file_name);
    }

    Ok(Json(ApiResponse::success(crate::db::models::JobFileMeta {
        uuid,
        job_id,
        file_name,
        content_type: "application/pdf".to_string(),
        file_size,
        uploaded_by_name: uploader_name,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
    })))
}

/// GET /api/jobs/:job_id/files - List files for a job
pub async fn get_job_files(
    State(state): State<SharedState>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<JobFilesResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let files = state.repo.get_job_files(&job_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success(JobFilesResponse { files })))
}

/// GET /api/jobs/:job_id/files/:file_uuid - Download a file
pub async fn download_job_file(
    State(state): State<SharedState>,
    Path((job_id, file_uuid)): Path<(String, String)>,
) -> Result<axum::response::Response, (StatusCode, Json<ApiResponse<()>>)> {
    let _ = job_id;
    let state = state.read().await;
    let (file_name, content_type, data) = state.repo.get_job_file_data(&file_uuid).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiResponse::error("File not found"))))?;

    Ok(axum::response::Response::builder()
        .status(200)
        .header("content-type", content_type)
        .header("content-disposition", format!("attachment; filename=\"{}\"", file_name))
        .body(axum::body::Body::from(data))
        .unwrap())
}

/// DELETE /api/jobs/:job_id/files/:file_uuid - Delete a file
pub async fn delete_job_file(
    State(state): State<SharedState>,
    Path((_job_id, file_uuid)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    state.repo.delete_job_file(&file_uuid).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success_with_message((), "File deleted")))
}

/// GET /api/jobs/files/search?q=room+123&job_id=optional - Search file contents
pub async fn search_job_files(
    State(state): State<SharedState>,
    Query(params): Query<JobFileSearchQuery>,
) -> Result<Json<ApiResponse<JobFileSearchResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let results = state.repo.search_job_files(params.job_id.as_deref(), &params.q).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success(JobFileSearchResponse { results })))
}

/// Extract text from a PDF byte slice (basic extraction for search).
/// Supports FlateDecode-compressed streams and per-font /ToUnicode CMap encoding.
fn extract_pdf_text(data: &[u8], file_name: &str) -> Option<String> {
    use flate2::read::ZlibDecoder;
    use std::collections::HashMap;
    use std::io::Read;

    tracing::debug!("[PDF:extract] Starting text extraction for '{}' ({} bytes)", file_name, data.len());

    // --- Phase 1: Index objects and decompress streams ---
    // Build obj_number -> decompressed stream content
    let mut obj_streams: HashMap<u32, String> = HashMap::new();
    // Also keep a flat list for fallback, with optional object number
    let mut all_streams: Vec<String> = Vec::new();
    let mut all_stream_obj_nums: Vec<Option<u32>> = Vec::new();

    let raw = data;
    let mut pos = 0;
    while pos < raw.len() {
        if let Some(idx) = find_bytes(&raw[pos..], b"stream") {
            let abs = pos + idx;
            if abs > 0 && raw[abs - 1] == b'd' {
                pos = abs + 6;
                continue;
            }
            let mut start = abs + 6;
            if start < raw.len() && raw[start] == b'\r' { start += 1; }
            if start < raw.len() && raw[start] == b'\n' { start += 1; }

            if let Some(end_off) = find_bytes(&raw[start..], b"endstream") {
                let end = start + end_off;
                let stream_data = &raw[start..end];

                let dict_start = if abs > 2048 { abs - 2048 } else { 0 };
                let dict_region = &raw[dict_start..abs];
                let dict_str = String::from_utf8_lossy(dict_region);
                let is_flate = dict_str.contains("/FlateDecode") || dict_str.contains("/Fl");

                // Try to find the object number: look for "N 0 obj" before this stream
                let obj_num = parse_obj_number_before(&dict_str);

                let content = if is_flate {
                    let mut decoder = ZlibDecoder::new(stream_data);
                    let mut decompressed = Vec::new();
                    match decoder.read_to_end(&mut decompressed) {
                        Ok(n) => {
                            tracing::debug!("[PDF:extract] Decompressed stream (obj {:?}): {} -> {} bytes",
                                obj_num, stream_data.len(), n);
                            Some(String::from_utf8_lossy(&decompressed).to_string())
                        }
                        Err(e) => {
                            tracing::debug!("[PDF:extract] Failed to decompress stream at offset {}: {}", abs, e);
                            None
                        }
                    }
                } else {
                    Some(String::from_utf8_lossy(stream_data).to_string())
                };

                if let Some(text) = content {
                    if let Some(num) = obj_num {
                        obj_streams.insert(num, text.clone());
                    }
                    all_streams.push(text);
                    all_stream_obj_nums.push(obj_num);
                }

                pos = end + 9;
            } else {
                pos = abs + 6;
            }
        } else {
            break;
        }
    }

    tracing::debug!("[PDF:extract] Found {} streams ({} with object numbers)", all_streams.len(), obj_streams.len());

    if all_streams.is_empty() {
        let whole = String::from_utf8_lossy(data).to_string();
        if whole.contains("BT") {
            tracing::debug!("[PDF:extract] No streams found, falling back to whole-file scan");
            all_streams.push(whole);
        }
    }

    // --- Phase 2: Build per-page font CMap tables ---
    // Different pages may use the SAME font name (e.g., /c) mapped to DIFFERENT font objects
    // with completely different CMaps. We must resolve fonts per-page (per-content-stream).

    // Step 2a: Parse /Type /Page objects to build content_stream → { font_name → font_obj }
    // Handles both /Contents N 0 R (direct) and /Contents [N 0 R] (array) syntax.
    // Handles both inline /Font << ... >> and indirect /Resources N 0 R (resolves the ref).
    let mut stream_font_map: HashMap<u32, HashMap<String, u32>> = HashMap::new();

    // Helper: parse a /Font << /name obj 0 R ... >> dict from a text region
    fn parse_font_dict_from_region(region: &str) -> Option<HashMap<String, u32>> {
        let f_pos = region.find("/Font")?;
        let after_f = &region[f_pos + "/Font".len()..];
        let open = after_f.find("<<")?;
        if open >= 30 { return None; }
        let inner = &after_f[open + 2..];
        let close = inner.find(">>")?;
        let font_dict = &inner[..close];
        let tokens: Vec<&str> = font_dict.split_whitespace().collect();
        let mut fonts: HashMap<String, u32> = HashMap::new();
        let mut t = 0;
        while t + 3 < tokens.len() {
            if tokens[t].starts_with('/') && tokens[t + 2] == "0" && tokens[t + 3] == "R" {
                let name = tokens[t][1..].to_string();
                if let Ok(obj) = tokens[t + 1].parse::<u32>() {
                    fonts.insert(name, obj);
                }
                t += 4;
            } else {
                t += 1;
            }
        }
        if fonts.is_empty() { None } else { Some(fonts) }
    }

    // Helper: find object N definition in raw PDF and return its text region
    fn find_obj_text(raw: &[u8], obj_num: u32) -> Option<String> {
        let needle = format!("{} 0 obj", obj_num);
        let needle_bytes = needle.as_bytes();
        let mut pos = 0;
        while pos < raw.len() {
            match find_bytes(&raw[pos..], needle_bytes) {
                Some(found) => {
                    let abs = pos + found;
                    // Ensure it's not part of a larger number (e.g., "26 0 obj" for "6 0 obj")
                    if abs == 0 || !raw[abs - 1].is_ascii_digit() {
                        let end = std::cmp::min(abs + 1000, raw.len());
                        return Some(String::from_utf8_lossy(&raw[abs..end]).to_string());
                    }
                    pos = abs + needle_bytes.len();
                }
                None => break,
            }
        }
        None
    }

    {
        let page_needle = b"/Type /Page";
        let mut search_pos: usize = 0;
        while search_pos < raw.len() {
            // Find next /Type /Page in raw bytes
            let found = match find_bytes(&raw[search_pos..], page_needle) {
                Some(p) => p,
                None => break,
            };
            let abs_pos = search_pos + found;
            let after_pos = abs_pos + page_needle.len();

            // Skip /Type /Pages (page tree node)
            if after_pos < raw.len() {
                let mut check = after_pos;
                while check < raw.len() && matches!(raw[check], b' ' | b'\t' | b'\r' | b'\n') {
                    check += 1;
                }
                if check < raw.len() && raw[check] == b's' {
                    search_pos = after_pos;
                    continue;
                }
            }

            // Extract region around /Type /Page for parsing the page dict
            let region_start = abs_pos.saturating_sub(500);
            let region_end = std::cmp::min(abs_pos + 2000, raw.len());
            let region = String::from_utf8_lossy(&raw[region_start..region_end]).to_string();

            // Extract /Contents reference - handle both direct N 0 R and array [N 0 R]
            let mut content_obj: Option<u32> = None;
            if let Some(c_pos) = region.find("/Contents") {
                let after_c = region[c_pos + "/Contents".len()..].trim_start();
                if after_c.starts_with('[') {
                    // Array syntax: /Contents [N 0 R]
                    let inner = &after_c[1..]; // skip '['
                    let tokens: Vec<&str> = inner.split_whitespace().take(3).collect();
                    if tokens.len() >= 3 && tokens[1] == "0" && (tokens[2] == "R" || tokens[2].starts_with("R]") || tokens[2].starts_with("R\n")) {
                        content_obj = tokens[0].parse::<u32>().ok();
                    }
                } else {
                    // Direct syntax: /Contents N 0 R
                    let tokens: Vec<&str> = after_c.split_whitespace().take(3).collect();
                    if tokens.len() >= 3 && tokens[1] == "0" && tokens[2] == "R" {
                        content_obj = tokens[0].parse::<u32>().ok();
                    }
                }
            }

            if let Some(content_num) = content_obj {
                let mut fonts_found: Option<HashMap<String, u32>> = None;

                // FIRST try: indirect /Resources N 0 R → look up that object for /Font
                // This is preferred because the region search can accidentally pick up
                // a nearby Resource object from a DIFFERENT page.
                if let Some(r_pos) = region.find("/Resources") {
                    let after_r = region[r_pos + "/Resources".len()..].trim_start();
                    if !after_r.starts_with("<<") {
                        // Indirect reference: /Resources N 0 R
                        let tokens: Vec<&str> = after_r.split_whitespace().take(3).collect();
                        if tokens.len() >= 3 && tokens[1] == "0" && tokens[2] == "R" {
                            if let Ok(res_obj_num) = tokens[0].parse::<u32>() {
                                if let Some(res_text) = find_obj_text(&raw, res_obj_num) {
                                    fonts_found = parse_font_dict_from_region(&res_text);
                                }
                            }
                        }
                    } else {
                        // Inline resources: /Resources << ... /Font << ... >> ... >>
                        fonts_found = parse_font_dict_from_region(&region);
                    }
                }

                // Fallback: try inline /Font in the region (for pages with no /Resources key)
                if fonts_found.is_none() {
                    fonts_found = parse_font_dict_from_region(&region);
                }

                if let Some(fonts) = fonts_found {
                    tracing::debug!("[PDF:extract] Page with /Contents {} has fonts: {:?}", content_num, fonts);
                    stream_font_map.insert(content_num, fonts);
                }
            }

            search_pos = after_pos;
        }
    }
    tracing::debug!("[PDF:extract] Parsed {} page font mappings", stream_font_map.len());

    // Step 2b: Collect ALL unique font objects and their ToUnicode references
    // Key by font OBJECT NUMBER (globally unique) instead of font name (ambiguous across pages)
    let mut font_obj_to_tounicode: HashMap<u32, u32> = HashMap::new();
    let mut font_obj_cmaps: HashMap<u32, HashMap<u16, char>> = HashMap::new();
    let mut font_obj_twobyte: std::collections::HashSet<u32> = std::collections::HashSet::new();

    // Collect all unique font object numbers from all pages
    let mut all_font_objs: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for fonts in stream_font_map.values() {
        for &obj in fonts.values() {
            all_font_objs.insert(obj);
        }
    }

    // For each font object, find its /ToUnicode reference
    for &font_obj_num in &all_font_objs {
        if let Some(search_region) = find_obj_text(raw, font_obj_num) {
            if let Some(tu_pos) = search_region.find("/ToUnicode") {
                let after_tu = &search_region[tu_pos + 10..];
                let tokens: Vec<&str> = after_tu.split_whitespace().take(3).collect();
                if tokens.len() >= 3 && tokens[1] == "0" && tokens[2] == "R" {
                    if let Ok(tu_obj) = tokens[0].parse::<u32>() {
                        font_obj_to_tounicode.insert(font_obj_num, tu_obj);
                    }
                }
            }
        }
    }
    tracing::debug!("[PDF:extract] Font obj -> ToUnicode obj ({} mappings): {:?}",
        font_obj_to_tounicode.len(), font_obj_to_tounicode);

    // Parse each font object's ToUnicode CMap
    for (&font_obj_num, &tu_obj_num) in &font_obj_to_tounicode {
        if let Some(cmap_content) = obj_streams.get(&tu_obj_num) {
            let mut cmap: HashMap<u16, char> = HashMap::new();
            let entries = parse_tounicode_cmap_u16(cmap_content, &mut cmap);
            if entries > 0 {
                let is_twobyte = cmap.keys().any(|&k| k > 255);
                if is_twobyte {
                    font_obj_twobyte.insert(font_obj_num);
                }
                tracing::debug!("[PDF:extract] Font obj {} CMap (ToUnicode obj {}): {} entries, twobyte={}",
                    font_obj_num, tu_obj_num, entries, is_twobyte);
                font_obj_cmaps.insert(font_obj_num, cmap);
            }
        }
    }

    // Fallback: if we found CMap streams but couldn't associate them with fonts,
    // build a merged map from any CMap-looking streams (for simple single-font PDFs)
    let mut fallback_cmap: HashMap<u16, char> = HashMap::new();
    if font_obj_cmaps.is_empty() {
        for content in &all_streams {
            if content.contains("beginbfchar") || content.contains("beginbfrange") {
                parse_tounicode_cmap_u16(content, &mut fallback_cmap);
            }
        }
        if !fallback_cmap.is_empty() {
            tracing::debug!("[PDF:extract] Using fallback merged CMap with {} entries (no per-font association found)", fallback_cmap.len());
        }
    }

    let has_per_page = !font_obj_cmaps.is_empty();
    let has_fallback = !fallback_cmap.is_empty();

    tracing::info!("[PDF:extract] CMap status for '{}': {} font object CMaps across {} pages, {} fallback entries",
        file_name, font_obj_cmaps.len(), stream_font_map.len(), fallback_cmap.len());

    // --- Phase 3: Extract text from BT..ET blocks with per-page font tracking ---
    let mut all_texts: Vec<String> = Vec::new();

    for (stream_idx, content) in all_streams.iter().enumerate() {
        if content.contains("beginbfchar") || content.contains("begincmap") {
            continue;
        }
        if !content.contains("BT") {
            continue;
        }

        // Determine per-stream font mappings: font_name → font_obj_num
        let stream_obj = all_stream_obj_nums.get(stream_idx).copied().flatten();
        let per_stream_fonts: Option<&HashMap<String, u32>> = stream_obj.and_then(|n| stream_font_map.get(&n));

        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut in_bt = false;
        let mut line_texts: Vec<String> = Vec::new();
        let mut current_font: Option<String> = None;
        let mut current_font_obj: Option<u32> = None;
        let mut last_y: Option<f64> = None;
        // Graphics state stack for q/Q operators — saves/restores current font
        let mut gstate_stack: Vec<(Option<String>, Option<u32>)> = Vec::new();

        while i < len {
            // Handle q (save graphics state) — can appear outside BT..ET
            if chars[i] == 'q'
                && (i == 0 || chars[i - 1].is_whitespace())
                && (i + 1 >= len || chars[i + 1].is_whitespace() || chars[i + 1] == '\n' || chars[i + 1] == '\r')
            {
                gstate_stack.push((current_font.clone(), current_font_obj));
                i += 1;
                continue;
            }

            // Handle Q (restore graphics state) — can appear outside BT..ET
            if chars[i] == 'Q'
                && (i == 0 || chars[i - 1].is_whitespace())
                && (i + 1 >= len || chars[i + 1].is_whitespace() || chars[i + 1] == '\n' || chars[i + 1] == '\r')
            {
                if let Some((saved_font, saved_obj)) = gstate_stack.pop() {
                    current_font = saved_font;
                    current_font_obj = saved_obj;
                }
                i += 1;
                continue;
            }

            // Detect BT
            if !in_bt && i + 1 < len && chars[i] == 'B' && chars[i + 1] == 'T'
                && (i == 0 || chars[i - 1].is_whitespace())
                && (i + 2 >= len || chars[i + 2].is_whitespace() || chars[i + 2] == '\n' || chars[i + 2] == '\r')
            {
                in_bt = true;
                i += 2;
                continue;
            }

            // Detect ET
            if in_bt && i + 1 < len && chars[i] == 'E' && chars[i + 1] == 'T'
                && (i == 0 || chars[i - 1].is_whitespace())
                && (i + 2 >= len || chars[i + 2].is_whitespace() || chars[i + 2] == '\n' || chars[i + 2] == '\r')
            {
                in_bt = false;
                if !line_texts.is_empty() {
                    all_texts.push(line_texts.join(""));
                    line_texts.clear();
                }
                i += 2;
                continue;
            }

            // Inside BT..ET, detect text positioning operators for proper line/column breaks
            // T* → explicit new line
            if in_bt && chars[i] == 'T' && i + 1 < len && chars[i + 1] == '*'
                && (i == 0 || chars[i - 1].is_whitespace())
                && (i + 2 >= len || chars[i + 2].is_whitespace() || chars[i + 2] == '\n')
            {
                if !line_texts.is_empty() {
                    all_texts.push(line_texts.join(""));
                    line_texts.clear();
                }
                i += 2;
                continue;
            }

            // Td / TD → "tx ty Td" — look backward for the ty number
            if in_bt && chars[i] == 'T' && i + 1 < len && (chars[i + 1] == 'd' || chars[i + 1] == 'D')
                && (i == 0 || chars[i - 1].is_whitespace())
                && (i + 2 >= len || chars[i + 2].is_whitespace() || chars[i + 2] == '\n')
            {
                // Scan backward to find ty (the number right before "Td")
                let mut j = i as isize - 1;
                while j >= 0 && chars[j as usize].is_whitespace() { j -= 1; }
                let num_end = j as usize + 1;
                while j >= 0 && (chars[j as usize].is_ascii_digit() || chars[j as usize] == '.' || chars[j as usize] == '-') { j -= 1; }
                let num_start = (j + 1) as usize;
                if num_start < num_end {
                    let ty_str: String = chars[num_start..num_end].iter().collect();
                    if let Ok(ty) = ty_str.parse::<f64>() {
                        if ty < -1.0 {
                            // Y moved down → new row
                            if !line_texts.is_empty() {
                                all_texts.push(line_texts.join(""));
                                line_texts.clear();
                            }
                        } else if ty.abs() < 0.5 {
                            // Same row, horizontal move → column separator
                            if !line_texts.is_empty() {
                                line_texts.push(" ".to_string());
                            }
                        }
                    }
                }
                i += 2;
                continue;
            }

            // Tm → "a b c d e f Tm" — f is Y position, track for row changes
            if in_bt && chars[i] == 'T' && i + 1 < len && chars[i + 1] == 'm'
                && (i == 0 || chars[i - 1].is_whitespace())
                && (i + 2 >= len || chars[i + 2].is_whitespace() || chars[i + 2] == '\n')
                // Avoid matching "Tm" inside other tokens like "Tmc"
                && (i + 2 >= len || !chars[i + 2].is_ascii_alphabetic())
            {
                // Scan backward to find f (Y), which is the last number before "Tm"
                let mut j = i as isize - 1;
                while j >= 0 && chars[j as usize].is_whitespace() { j -= 1; }
                let num_end = j as usize + 1;
                while j >= 0 && (chars[j as usize].is_ascii_digit() || chars[j as usize] == '.' || chars[j as usize] == '-') { j -= 1; }
                let num_start = (j + 1) as usize;
                if num_start < num_end {
                    let y_str: String = chars[num_start..num_end].iter().collect();
                    if let Ok(y) = y_str.parse::<f64>() {
                        if let Some(prev_y) = last_y {
                            let dy = y - prev_y;
                            if dy.abs() > 1.0 {
                                // Y changed → new row
                                if !line_texts.is_empty() {
                                    all_texts.push(line_texts.join(""));
                                    line_texts.clear();
                                }
                            } else {
                                // Same Y → column separator
                                if !line_texts.is_empty() {
                                    line_texts.push(" ".to_string());
                                }
                            }
                        }
                        last_y = Some(y);
                    }
                }
                i += 2;
                continue;
            }

            // ' (single quote) → move to next line and show text (equivalent to T* + Tj)
            if in_bt && chars[i] == '\''
                && (i == 0 || chars[i - 1].is_whitespace())
                && (i + 1 >= len || chars[i + 1].is_whitespace() || chars[i + 1] == '(')
            {
                if !line_texts.is_empty() {
                    all_texts.push(line_texts.join(""));
                    line_texts.clear();
                }
                i += 1;
                continue;
            }

            // Inside BT..ET, detect Tf operator to track current font: /F1 12 Tf
            if in_bt && chars[i] == '/' {
                // Parse font name: /F1, /F2, /TT0, etc.
                let mut name = String::new();
                let mut j = i + 1;
                while j < len && !chars[j].is_whitespace() {
                    name.push(chars[j]);
                    j += 1;
                }
                // Check if this is followed by "size Tf"
                let remaining: String = chars[j..std::cmp::min(j + 30, len)].iter().collect();
                let tf_tokens: Vec<&str> = remaining.split_whitespace().take(2).collect();
                if tf_tokens.len() >= 2 && tf_tokens[1] == "Tf" {
                    current_font = Some(name.clone());
                    // Resolve font name → font object number using per-stream page mapping
                    current_font_obj = per_stream_fonts.and_then(|fonts| fonts.get(&name).copied());
                    // Fallback: if font not in this page's resources, search all pages
                    // (handles PDFs where page Resources are incomplete, e.g., merged PDFs)
                    if current_font_obj.is_none() {
                        // Prefer a page whose content stream obj number is close to ours
                        // (likely same "copy" in multi-copy PDFs)
                        let mut best_obj: Option<u32> = None;
                        let my_stream_num = stream_obj.unwrap_or(0);
                        let mut best_dist = u32::MAX;
                        for (&other_stream, other_fonts) in &stream_font_map {
                            if let Some(&font_obj) = other_fonts.get(&name) {
                                let dist = (other_stream as i64 - my_stream_num as i64).unsigned_abs() as u32;
                                if dist < best_dist {
                                    best_dist = dist;
                                    best_obj = Some(font_obj);
                                }
                            }
                        }
                        current_font_obj = best_obj;
                    }
                    // Skip past the Tf
                    i = j;
                    // Skip past "size Tf"
                    while i < len && chars[i] != 'T' { i += 1; }
                    if i + 1 < len && chars[i] == 'T' && chars[i + 1] == 'f' { i += 2; }
                    continue;
                }
            }

            // Extract parenthesized strings and apply per-font CMap
            if in_bt && chars[i] == '(' {
                let raw_chars = extract_paren_string(&chars, &mut i, len);

                let active_cmap = if has_per_page {
                    current_font_obj.and_then(|obj| font_obj_cmaps.get(&obj))
                } else if has_fallback {
                    Some(&fallback_cmap)
                } else {
                    None
                };

                let is_twobyte = current_font_obj.map_or(false, |obj| font_obj_twobyte.contains(&obj));
                let text: String = if let Some(cmap) = active_cmap {
                    if is_twobyte {
                        // 2-byte font: read pairs of chars as big-endian u16 codes
                        let mut result = String::new();
                        let mut ci = 0;
                        while ci < raw_chars.len() {
                            if ci + 1 < raw_chars.len() {
                                let code = (raw_chars[ci] as u16) << 8 | (raw_chars[ci + 1] as u16);
                                if let Some(&ch) = cmap.get(&code) {
                                    result.push(ch);
                                } else {
                                    // Try single byte fallback
                                    let code1 = raw_chars[ci] as u16;
                                    result.push(cmap.get(&code1).copied().unwrap_or(raw_chars[ci]));
                                    let code2 = raw_chars[ci + 1] as u16;
                                    result.push(cmap.get(&code2).copied().unwrap_or(raw_chars[ci + 1]));
                                }
                                ci += 2;
                            } else {
                                let code = raw_chars[ci] as u16;
                                result.push(cmap.get(&code).copied().unwrap_or(raw_chars[ci]));
                                ci += 1;
                            }
                        }
                        result
                    } else {
                        raw_chars.iter().map(|&c| {
                            let code = c as u32;
                            if code <= 0xFFFF {
                                cmap.get(&(code as u16)).copied().unwrap_or(c)
                            } else {
                                c
                            }
                        }).collect()
                    }
                } else {
                    raw_chars.into_iter().collect()
                };

                if !text.is_empty() {
                    line_texts.push(text);
                }
                continue;
            }

            // Handle hex strings
            if in_bt && chars[i] == '<' && (i + 1 < len && chars[i + 1] != '<') {
                let bytes = extract_hex_string(&chars, &mut i, len);

                let active_cmap = if has_per_page {
                    current_font_obj.and_then(|obj| font_obj_cmaps.get(&obj))
                } else if has_fallback {
                    Some(&fallback_cmap)
                } else {
                    None
                };

                let is_twobyte = current_font_obj.map_or(false, |obj| font_obj_twobyte.contains(&obj));
                let decoded: String = if let Some(cmap) = active_cmap {
                    if is_twobyte {
                        // 2-byte font: read byte pairs as big-endian u16 codes
                        let mut result = String::new();
                        let mut bi = 0;
                        while bi < bytes.len() {
                            if bi + 1 < bytes.len() {
                                let code = (bytes[bi] as u16) << 8 | (bytes[bi + 1] as u16);
                                if let Some(&ch) = cmap.get(&code) {
                                    result.push(ch);
                                } else {
                                    // Try single byte fallback
                                    result.push(cmap.get(&(bytes[bi] as u16)).copied().unwrap_or(bytes[bi] as char));
                                    result.push(cmap.get(&(bytes[bi + 1] as u16)).copied().unwrap_or(bytes[bi + 1] as char));
                                }
                                bi += 2;
                            } else {
                                result.push(cmap.get(&(bytes[bi] as u16)).copied().unwrap_or(bytes[bi] as char));
                                bi += 1;
                            }
                        }
                        result
                    } else {
                        bytes.iter().map(|&b| {
                            cmap.get(&(b as u16)).copied().unwrap_or(b as char)
                        }).collect()
                    }
                } else {
                    String::from_utf8_lossy(&bytes).to_string()
                };
                if !decoded.trim().is_empty() {
                    line_texts.push(decoded);
                }
                continue;
            }

            i += 1;
        }

        if !line_texts.is_empty() {
            all_texts.push(line_texts.join(""));
        }
    }

    tracing::debug!("[PDF:extract] Extracted {} text lines from BT/ET blocks", all_texts.len());

    // --- Phase 4: Fallback if no BT/ET text found ---
    if all_texts.is_empty() {
        tracing::debug!("[PDF:extract] No BT/ET text found, trying ASCII fallback");
        let mut current = String::new();
        for &b in data {
            if b >= 32 && b < 127 {
                current.push(b as char);
            } else {
                if current.len() > 8 {
                    let trimmed = current.trim();
                    if !trimmed.is_empty()
                        && !trimmed.starts_with("<<")
                        && !trimmed.starts_with('/')
                        && !trimmed.starts_with("stream")
                        && !trimmed.starts_with("endstream")
                        && !trimmed.starts_with("obj")
                        && !trimmed.starts_with("endobj")
                        && !trimmed.starts_with("xref")
                        && !trimmed.starts_with("trailer")
                    {
                        all_texts.push(trimmed.to_string());
                    }
                }
                current.clear();
            }
        }
        if !all_texts.is_empty() {
            tracing::debug!("[PDF:extract] ASCII fallback found {} text segments", all_texts.len());
        }
    }

    if all_texts.is_empty() {
        tracing::warn!("[PDF:extract] No text extracted from '{}' ({} bytes)", file_name, data.len());
        None
    } else {
        let result = all_texts.join("\n");
        let sample: String = result.chars().take(300).collect();
        tracing::info!("[PDF:extract] Extracted {} chars ({} lines) from '{}'. Sample: {:?}",
            result.len(), all_texts.len(), file_name, sample);
        Some(result)
    }
}

/// Extract a parenthesized string from PDF content, handling escapes.
/// Advances `i` past the closing paren.
fn extract_paren_string(chars: &[char], i: &mut usize, len: usize) -> Vec<char> {
    let mut depth = 1;
    let mut result = Vec::new();
    *i += 1; // skip opening '('
    while *i < len && depth > 0 {
        if chars[*i] == '(' && (*i == 0 || chars[*i - 1] != '\\') { depth += 1; }
        else if chars[*i] == ')' && (*i == 0 || chars[*i - 1] != '\\') {
            depth -= 1;
            if depth == 0 { *i += 1; break; }
        }
        if chars[*i] == '\\' && *i + 1 < len {
            *i += 1;
            match chars[*i] {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '(' => result.push('('),
                ')' => result.push(')'),
                '\\' => result.push('\\'),
                _ => result.push(chars[*i]),
            }
        } else {
            result.push(chars[*i]);
        }
        *i += 1;
    }
    result
}

/// Extract a hex string <XX XX> from PDF content, returning raw bytes.
/// Advances `i` past the closing '>'.
fn extract_hex_string(chars: &[char], i: &mut usize, len: usize) -> Vec<u8> {
    let mut hex = String::new();
    *i += 1; // skip '<'
    while *i < len && chars[*i] != '>' {
        if chars[*i].is_ascii_hexdigit() {
            hex.push(chars[*i]);
        }
        *i += 1;
    }
    if *i < len { *i += 1; } // skip '>'
    (0..hex.len())
        .step_by(2)
        .filter_map(|j| {
            if j + 2 <= hex.len() {
                u8::from_str_radix(&hex[j..j + 2], 16).ok()
            } else {
                None
            }
        })
        .collect()
}

/// Find the object number from a string region before "stream".
/// Looks for "N 0 obj" pattern.
fn parse_obj_number_before(dict_str: &str) -> Option<u32> {
    // Search backward for "N 0 obj"
    if let Some(obj_pos) = dict_str.rfind(" 0 obj") {
        let before = &dict_str[..obj_pos];
        // Find the last whitespace or newline before the number
        if let Some(num_start) = before.rfind(|c: char| c.is_whitespace() || c == '\n' || c == '\r') {
            let num_str = before[num_start + 1..].trim();
            return num_str.parse::<u32>().ok();
        } else {
            // Number starts at beginning of string
            return before.trim().parse::<u32>().ok();
        }
    }
    None
}

/// Find a byte pattern in a slice, returning the offset of the first match
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Split a CMap line into individual hex tokens.
/// Handles both space-separated (<20> <20> <0052>) and concatenated (<0026><0026><0043>) formats.
fn split_cmap_hex_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_bracket = false;
    for ch in line.chars() {
        if ch == '<' {
            in_bracket = true;
            current.clear();
        } else if ch == '>' {
            if in_bracket && !current.is_empty() {
                tokens.push(format!("<{}>", current));
            }
            in_bracket = false;
            current.clear();
        } else if ch == '[' || ch == ']' {
            // Array markers — push as separate token
            tokens.push(ch.to_string());
        } else if in_bracket {
            current.push(ch);
        }
    }
    tokens
}

/// Parse a /ToUnicode CMap stream into a u16 → char mapping (supports multi-byte codes).
fn parse_tounicode_cmap_u16(content: &str, map: &mut std::collections::HashMap<u16, char>) -> usize {
    let mut added = 0usize;
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // --- beginbfchar / endbfchar ---
        if line.ends_with("beginbfchar") {
            i += 1;
            while i < lines.len() {
                let l = lines[i].trim();
                if l.contains("endbfchar") { break; }
                let tokens = split_cmap_hex_tokens(l);
                if tokens.len() >= 2 {
                    if let (Some(src), Some(dst)) = (parse_cmap_hex(&tokens[0]), parse_cmap_hex(&tokens[1])) {
                        if src <= 0xFFFF {
                            if let Some(c) = char::from_u32(dst) {
                                map.insert(src as u16, c);
                                added += 1;
                            }
                        }
                    }
                }
                i += 1;
            }
        }

        // --- beginbfrange / endbfrange ---
        if line.ends_with("beginbfrange") {
            i += 1;
            while i < lines.len() {
                let l = lines[i].trim();
                if l.contains("endbfrange") { break; }
                let tokens = split_cmap_hex_tokens(l);
                if tokens.len() >= 3 {
                    // Check for array form: <start> <end> [ <u1> <u2> ... ]
                    if tokens[2] == "[" {
                        if let (Some(range_start), Some(range_end)) = (parse_cmap_hex(&tokens[0]), parse_cmap_hex(&tokens[1])) {
                            // Everything after "[" up to "]" are unicode values
                            let mut offset = 0u32;
                            for t in &tokens[3..] {
                                if *t == "]" { break; }
                                let src = range_start + offset;
                                if src > 0xFFFF || src > range_end { break; }
                                if let Some(dst) = parse_cmap_hex(t) {
                                    if let Some(c) = char::from_u32(dst) {
                                        map.insert(src as u16, c);
                                        added += 1;
                                    }
                                }
                                offset += 1;
                            }
                        }
                    } else {
                        // Simple form: <start> <end> <unicode_start>
                        if let (Some(range_start), Some(range_end), Some(uni_start)) =
                            (parse_cmap_hex(&tokens[0]), parse_cmap_hex(&tokens[1]), parse_cmap_hex(&tokens[2]))
                        {
                            if range_end >= range_start {
                                for offset in 0..=(range_end - range_start) {
                                    let src = range_start + offset;
                                    let dst = uni_start + offset;
                                    if src <= 0xFFFF {
                                        if let Some(c) = char::from_u32(dst) {
                                            map.insert(src as u16, c);
                                            added += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                i += 1;
            }
        }

        i += 1;
    }

    added
}

/// Parse a hex token like `<0053>` or `<20>` into a u32 value
fn parse_cmap_hex(s: &str) -> Option<u32> {
    let s = s.trim().trim_start_matches('<').trim_end_matches('>');
    if s.is_empty() { return None; }
    u32::from_str_radix(s, 16).ok()
}

/// Extract a delivery address from BOL text (lines before the first section header).
/// BOLs typically have the delivery address in the header area before "HARDWARE TICKETS", etc.
fn extract_bol_address(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let mut address_lines: Vec<String> = Vec::new();

    for line in &lines {
        let upper = line.to_uppercase();
        // Stop at the first section header
        if upper.contains("TICKETS") || upper.contains("HARDWARE") || upper.contains("SHIPPING")
            || upper.contains("ADVICE") || upper.contains("BILL OF LADING")
            || (upper.contains("PRIORITY") && upper.contains("TICKET"))
            || upper.contains("REPORT REF") || upper.contains("RUN DATE")
        {
            break;
        }

        // Look for lines that contain address-like words
        let lower = line.to_lowercase();
        let has_address_indicator = lower.contains("loop") || lower.contains("street")
            || lower.contains(" ave ") || lower.contains("blvd") || lower.contains("drive")
            || lower.contains("road") || lower.contains("lane") || lower.contains(" st ")
            || lower.contains(" ct ") || lower.contains(" dr ") || lower.contains(" nw")
            || lower.contains(" sw") || lower.contains(" ne") || lower.contains(" se,")
            || lower.contains("unit ") || lower.contains("suite");

        // Also match lines that look like "city, state zip" (e.g. "Albuquerque, NM 87120")
        let has_city_state_zip = {
            let re_like = lower.contains(',') && line.chars().filter(|c| c.is_ascii_digit()).count() >= 5
                && line.len() >= 10;
            re_like
        };

        // Match lines starting with a number followed by text (street address pattern: "123 Main St")
        let starts_with_street_number = {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.len() >= 2 && parts[0].len() <= 6
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].chars().next().map_or(false, |c| c.is_ascii_alphabetic())
        };

        if has_address_indicator || has_city_state_zip || starts_with_street_number {
            // Filter out garbled text
            let alpha = line.chars().filter(|c| c.is_ascii_alphabetic()).count();
            if (alpha as f32 / line.len().max(1) as f32) >= 0.4 {
                address_lines.push(line.to_string());
            }
        }
    }

    if address_lines.is_empty() {
        return None;
    }

    // Combine into a single address string
    let address = address_lines.join(", ");
    tracing::info!("[PDF:address] Extracted address from BOL: '{}'", address);
    Some(address)
}

/// Parse lading/shipping PDF text into ticket records
/// Returns tuples of (job_id, file_uuid, ticket_number, description, room, qty, section)
fn parse_lading_tickets(text: &str, job_id: &str, file_uuid: &str, file_name: &str) -> Vec<(String, String, String, Option<String>, Option<String>, i32, Option<String>)> {
    let mut tickets: Vec<(String, String, String, Option<String>, Option<String>, i32, Option<String>)> = Vec::new();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut current_section: Option<String> = None;
    let mut seen_section_header = false;
    let mut skipped_lines = 0u32;
    let raw_line_count = text.split('\n').count();
    // Pre-filter empty lines so lookahead works on meaningful content
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let total_lines = lines.len();

    tracing::debug!("[PDF:tickets] Parsing {} lines ({} non-empty) of text from '{}' for job {}",
        raw_line_count, total_lines, file_name, job_id);

    // Helper: parse a digit-only string as room(3)+qty(1-2)
    fn parse_room_qty(s: &str) -> Option<(String, i32)> {
        if s.len() >= 4 && s.chars().all(|c| c.is_ascii_digit()) {
            // Try room(3) + qty(2) first for 5+ digit strings, then room(3) + qty(1)
            if s.len() >= 5 {
                let split2 = s.len() - 2;
                if let (Ok(room_num), Ok(q)) = (s[..split2].parse::<i32>(), s[split2..].parse::<i32>()) {
                    if room_num >= 100 && room_num <= 999 && q >= 1 && q <= 99 {
                        return Some((s[..split2].to_string(), q));
                    }
                }
            }
            let split = s.len() - 1;
            if let (Ok(room_num), Ok(q)) = (s[..split].parse::<i32>(), s[split..].parse::<i32>()) {
                if room_num >= 100 && room_num <= 999 && q >= 1 && q <= 9 {
                    return Some((s[..split].to_string(), q));
                }
            }
        }
        None
    }

    // Helper: check if text looks like a street address
    fn has_address_words(text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("loop") || lower.contains("street") || lower.contains(" ave ")
            || lower.contains("blvd") || lower.contains("unit ") || lower.contains("suite")
            || lower.contains(" nw") || lower.contains(" sw") || lower.contains(" ne,")
            || lower.contains(" se,") || lower.contains("cynthia") || lower.contains("drive")
            || lower.contains("road") || lower.contains("lane") || lower.contains(" st ")
            || lower.contains(" ct ") || lower.contains(" dr ")
    }

    // Helper: check if text is garbled (low alphabetic ratio)
    fn is_garbled_text(text: &str, threshold: f32) -> bool {
        let alpha = text.chars().filter(|c| c.is_ascii_alphabetic()).count();
        (alpha as f32 / text.len().max(1) as f32) < threshold
    }

    let mut i = 0;
    while i < total_lines {
        let line = lines[i];

        // Detect section headers like "HARDWARE TICKETS", "Shipping Advice", etc.
        let upper = line.to_uppercase();
        if upper.contains("TICKETS") || upper.contains("HARDWARE") || upper.contains("SHIPPING")
            || upper.contains("ADVICE") || upper.contains("BILL OF LADING")
        {
            current_section = Some(line.to_string());
            seen_section_header = true;
            tracing::debug!("[PDF:tickets] Section header detected: '{}'", line);
            i += 1;
            continue;
        }

        // Skip header lines
        if upper.contains("PRIORITY") && upper.contains("TICKET") { skipped_lines += 1; i += 1; continue; }
        if upper.contains("REPORT REF") || upper.contains("RUN DATE") { skipped_lines += 1; i += 1; continue; }
        if upper.contains("DESCRIPTION") && upper.len() < 20 { skipped_lines += 1; i += 1; continue; }
        if upper == "QTY" || upper == "ROOM" { skipped_lines += 1; i += 1; continue; }

        // Skip all lines before the first section header (page headers, addresses, etc.)
        if !seen_section_header {
            skipped_lines += 1;
            i += 1;
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { i += 1; continue; }

        // First token should be a ticket number (all digits, 3-8 chars)
        let first = parts[0];
        if first.len() < 3 || first.len() > 8 || !first.chars().all(|c| c.is_ascii_digit()) {
            skipped_lines += 1;
            i += 1;
            continue;
        }

        // Skip if it looks like "Multi-Ticketed" or header marker (single-line format)
        if parts.len() > 1 && parts[1].starts_with('*') {
            i += 1;
            continue;
        }

        let ticket_number = first.to_string();
        let mut room: Option<String> = None;
        let mut qty: i32 = 1;
        let mut description: Option<String> = None;
        let mut lines_consumed: usize = 1;

        if parts.len() == 1 {
            // *** MULTILINE FORMAT: ticket number alone on its own line ***
            // Look ahead for description and/or room+qty on subsequent lines
            if i + 1 < total_lines {
                let next = lines[i + 1];

                // If next line starts with * it's a Multi-Ticketed grouping header — skip
                if next.starts_with('*') {
                    i += 2;
                    continue;
                }

                let next_is_all_digits = !next.is_empty() && next.chars().all(|c| c.is_ascii_digit());

                if next_is_all_digits && next.len() >= 3 && next.len() <= 5 {
                    // Next line is room+qty directly (no description line)
                    if let Some((r, q)) = parse_room_qty(next) {
                        room = Some(r);
                        qty = q;
                    }
                    lines_consumed = 2;
                } else if !next_is_all_digits && !next.is_empty() {
                    // Don't consume section headers or column headers
                    let nu = next.to_uppercase();
                    let is_header = nu.contains("TICKETS") || nu.contains("HARDWARE")
                        || nu.contains("SHIPPING") || (nu.contains("PRIORITY") && nu.contains("TICKET"))
                        || nu.contains("REPORT REF") || nu.contains("PAGE ");

                    // Reject garbled/too-short "descriptions" (e.g. "y6", "pB", garbled CMap leftovers)
                    // BUT exempt lines with " - " separator (part-number descriptions like "530.1094 - Elbow Catch")
                    let next_alpha = next.chars().filter(|c| c.is_ascii_alphabetic()).count();
                    let has_part_number_sep = next.contains(" - ");
                    let is_garbled_desc = next.len() < 4
                        || ((next_alpha as f32 / next.len().max(1) as f32) < 0.4 && !has_part_number_sep);

                    if !is_header && !is_garbled_desc {
                        // Next line is the description
                        let desc_text = next.to_string();
                        lines_consumed = 2;

                        // Check if the description line has trailing digits (room+qty merged)
                        // BUT skip this for part-number descriptions (contain " - " separator)
                        // e.g. "530.1094 - Elbow Catch 1018" — the "1018" is part of the product name
                        let desc_bytes = desc_text.as_bytes();
                        let desc_len = desc_text.len();
                        let mut dstart = desc_len;
                        while dstart > 0 && desc_bytes[dstart - 1].is_ascii_digit() {
                            dstart -= 1;
                        }
                        let desc_trailing = &desc_text[dstart..];
                        let has_part_sep = desc_text.contains(" - ");

                        let mut did_extract_trailing = false;
                        if !has_part_sep && !desc_trailing.is_empty() {
                            if let Some((r, q)) = parse_room_qty(desc_trailing) {
                                // Room+qty merged at end of description line
                                room = Some(r);
                                qty = q;
                                let desc_clean = desc_text[..dstart].trim_end().trim_end_matches(',').trim_end();
                                description = if desc_clean.is_empty() { None } else { Some(desc_clean.to_string()) };
                                did_extract_trailing = true;
                            }
                        }

                        if !did_extract_trailing {
                            // Description is clean, look for room+qty on the NEXT line
                            description = Some(desc_text);

                            if i + 2 < total_lines {
                                let rq_line = lines[i + 2];
                                let rq_all_digits = !rq_line.is_empty() && rq_line.chars().all(|c| c.is_ascii_digit());

                                if rq_all_digits && rq_line.len() >= 3 && rq_line.len() <= 5 {
                                    if let Some((r, q)) = parse_room_qty(rq_line) {
                                        room = Some(r);
                                        qty = q;
                                        lines_consumed = 3;
                                    }
                                } else if rq_all_digits && rq_line.len() >= 1 && rq_line.len() <= 3 {
                                    // Standalone room or qty
                                    let rq_num = rq_line.parse::<i32>().unwrap_or(0);
                                    if rq_num >= 100 && rq_num <= 999 {
                                        // 3-digit: standalone room, check next line for qty
                                        room = Some(rq_line.to_string());
                                        lines_consumed = 3;
                                        if i + 3 < total_lines {
                                            let qty_line = lines[i + 3].trim();
                                            if let Ok(q) = qty_line.parse::<i32>() {
                                                if q >= 1 && q <= 999 {
                                                    qty = q;
                                                    lines_consumed = 4;
                                                }
                                            }
                                        }
                                    } else if rq_num >= 1 && rq_num <= 99 {
                                        // 1-2 digit: standalone qty (common in HARDWARE section)
                                        qty = rq_num;
                                        lines_consumed = 3;
                                    }
                                } else if !rq_all_digits && !rq_line.is_empty() {
                                    // Room might be a text label (e.g. "INSTAL", "SINKS") + qty on next line
                                    let rq_upper = rq_line.to_uppercase();
                                    let is_room_label = rq_upper.chars().all(|c| c.is_ascii_alphabetic() || c.is_whitespace())
                                        && rq_line.len() >= 2 && rq_line.len() <= 15;
                                    if is_room_label {
                                        room = Some(rq_line.to_string());
                                        lines_consumed = 3;
                                        // Check next line for qty
                                        if i + 3 < total_lines {
                                            let qty_line = lines[i + 3].trim();
                                            if let Ok(q) = qty_line.parse::<i32>() {
                                                if q >= 1 && q <= 999 {
                                                    qty = q;
                                                    lines_consumed = 4;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            tracing::debug!("[PDF:tickets] Multiline ticket #{}: desc={:?}, room={:?}, qty={} (consumed {} lines)",
                ticket_number, description, room, qty, lines_consumed);

            // If standalone number yielded no useful data, it's likely a garbled room/qty — skip
            if description.is_none() && room.is_none() {
                skipped_lines += 1;
                i += lines_consumed;
                continue;
            }

        } else {
            // *** SINGLE-LINE FORMAT: ticket number + description on same line ***

            // Filter out bogus "ticket numbers" that are actually zip codes or street addresses
            let rest_of_line = parts[1..].join(" ");
            if first.len() == 5 && !first.starts_with('0') {
                if has_address_words(&rest_of_line) || is_garbled_text(&rest_of_line, 0.5) {
                    skipped_lines += 1;
                    i += 1;
                    continue;
                }
            }
            // 3-4 digit numbers followed by address words ("230 Cynthia Loop NW")
            if first.len() <= 4 && !first.starts_with('0') {
                if has_address_words(&rest_of_line) {
                    skipped_lines += 1;
                    i += 1;
                    continue;
                }
                if is_garbled_text(&rest_of_line, 0.4) {
                    skipped_lines += 1;
                    i += 1;
                    continue;
                }
            }

            // --- Room/Qty extraction using trailing digit analysis ---
            // PDF columns (ticket#, description, style, room#, qty) merge together in text extraction.
            // The last 4+ consecutive digits at end of line encode: room_number (3 digits) + qty (1-2 digits)
            let full_line = parts[1..].join(" ");
            let line_bytes = full_line.as_bytes();
            let line_len = full_line.len();

            // Walk backwards from end to find trailing consecutive digits
            let mut digit_start = line_len;
            while digit_start > 0 && line_bytes[digit_start - 1].is_ascii_digit() {
                digit_start -= 1;
            }
            let trailing_digits = &full_line[digit_start..];

            // Use the consolidated parse_room_qty helper which handles both 1 and 2-digit qty
            let mut extracted = false;
            if let Some((r, q)) = parse_room_qty(trailing_digits) {
                room = Some(r);
                qty = q;
                extracted = true;
                let desc_raw = &full_line[..digit_start];
                let desc_clean = desc_raw.trim_end().trim_end_matches(',').trim_end();
                description = if desc_clean.is_empty() { None } else { Some(desc_clean.to_string()) };
            }

            if !extracted {
                // Fallback: try parsing last whitespace-separated tokens as qty and room
                let mut end_idx = parts.len();
                // Last token might be qty (integer 1-99)
                if let Ok(q) = parts[parts.len() - 1].parse::<i32>() {
                    if q >= 1 && q <= 99 {
                        qty = q;
                        end_idx -= 1;
                    }
                }
                // Second-to-last might be a 3-digit room number
                if end_idx > 2 {
                    let potential_room = parts[end_idx - 1];
                    if potential_room.len() == 3 && potential_room.chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(r) = potential_room.parse::<i32>() {
                            if r >= 100 && r <= 999 {
                                room = Some(potential_room.to_string());
                                end_idx -= 1;
                            }
                        }
                    }
                }
                let desc_parts: Vec<&str> = parts[1..end_idx].to_vec();
                description = if desc_parts.is_empty() { None } else { Some(desc_parts.join(" ")) };
            }
        }

        // Skip if "See inside" or similar non-item descriptions
        if let Some(ref d) = description {
            if d.to_lowercase().contains("see inside") { i += lines_consumed; continue; }
        }

        // Deduplicate: if we already have this ticket number, keep the one with the better data
        if let Some(&existing_idx) = seen.get(&ticket_number) {
            let existing = &tickets[existing_idx];
            let existing_desc_len = existing.3.as_ref().map_or(0, |d| d.len());
            let new_desc_len = description.as_ref().map_or(0, |d| d.len());
            let existing_has_room = existing.4.is_some();
            let new_has_room = room.is_some();
            // Prefer entry with: room > no room, then longer description, then higher qty
            let replace = (!existing_has_room && new_has_room)
                || (existing_has_room == new_has_room && new_desc_len > existing_desc_len);
            if replace {
                tracing::debug!("[PDF:tickets] Dedup ticket {} — replacing (desc={}chars, room={:?}) with (desc={}chars, room={:?})",
                    ticket_number, existing_desc_len, existing.4, new_desc_len, room);
                tickets[existing_idx] = (
                    job_id.to_string(),
                    file_uuid.to_string(),
                    ticket_number,
                    description,
                    room,
                    qty,
                    current_section.clone(),
                );
            } else {
                tracing::debug!("[PDF:tickets] Dedup ticket {} — keeping existing (desc={}chars, room={:?})",
                    ticket_number, existing_desc_len, existing.4);
            }
            i += lines_consumed;
            continue;
        }

        tracing::debug!("[PDF:tickets] Ticket #{}: desc={:?}, room={:?}, qty={}",
            ticket_number, description, room, qty);

        let idx = tickets.len();
        seen.insert(ticket_number.clone(), idx);
        tickets.push((
            job_id.to_string(),
            file_uuid.to_string(),
            ticket_number,
            description,
            room,
            qty,
            current_section.clone(),
        ));

        i += lines_consumed;
    }

    let dedup_count = (raw_line_count as u32).saturating_sub(skipped_lines).saturating_sub(tickets.len() as u32);
    tracing::info!("[PDF:tickets] Parsed {} unique tickets from '{}' ({} lines, {} skipped, {} duplicates filtered, section: {:?})",
        tickets.len(), file_name, raw_line_count, skipped_lines,
        dedup_count, current_section);

    if tickets.is_empty() && total_lines > 5 {
        // Log a sample of lines to help diagnose why no tickets were found
        let sample: Vec<&str> = text.split('\n').filter(|l| !l.trim().is_empty()).take(10).collect();
        tracing::warn!("[PDF:tickets] No tickets parsed from {} non-empty lines. Sample lines:\n{}",
            total_lines, sample.join("\n  "));
    }

    tickets
}

/// POST /api/jobs/:job_id/files/:file_uuid/reparse - Re-extract and re-parse lading tickets from a stored PDF
pub async fn reparse_job_file(
    State(state): State<SharedState>,
    Path((job_id, file_uuid)): Path<(String, String)>,
) -> Result<Json<ApiResponse<ReparseResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;

    // Get the PDF blob from DB
    let (file_name, _content_type, file_data) = state.repo.get_job_file_data(&file_uuid).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiResponse::error("File not found"))))?;

    tracing::info!("[PDF:reparse] Re-parsing '{}' ({} bytes) for job {}", file_name, file_data.len(), job_id);

    // Re-extract text from PDF
    let extracted_text = extract_pdf_text(&file_data, &file_name);

    if extracted_text.is_none() {
        return Ok(Json(ApiResponse::success_with_message(
            ReparseResponse { tickets_deleted: 0, tickets_inserted: 0, tickets: Vec::new(), extracted_text: None },
            "No text could be extracted from PDF".to_string(),
        )));
    }

    let text = extracted_text.unwrap();

    // Delete old tickets for this file
    let deleted = state.repo.delete_lading_tickets_for_file(&file_uuid).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    tracing::info!("[PDF:reparse] Deleted {} old tickets for file {}", deleted, file_uuid);

    // Re-parse tickets
    let ticket_tuples = parse_lading_tickets(&text, &job_id, &file_uuid, &file_name);
    let _ticket_count = ticket_tuples.len();

    // Build response tickets before consuming the tuples
    let response_tickets: Vec<ReparseTicket> = ticket_tuples.iter().map(|t| ReparseTicket {
        ticket_number: t.2.clone(),
        description: t.3.clone(),
        room: t.4.clone(),
        qty: t.5,
        section: t.6.clone(),
    }).collect();

    // Insert new tickets
    let inserted = if !ticket_tuples.is_empty() {
        state.repo.insert_lading_tickets(&ticket_tuples).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?
    } else {
        0
    };

    tracing::info!("[PDF:reparse] Inserted {} new tickets from '{}' (deleted {} old)", inserted, file_name, deleted);

    // Re-extract address from BOL
    if let Some(address) = extract_bol_address(&text) {
        match state.repo.set_job_address_if_empty(&job_id, &address).await {
            Ok(true) => tracing::info!("[PDF:reparse] Set job address from BOL: '{}'", address),
            Ok(false) => tracing::debug!("[PDF:reparse] Job already has address, not overwriting"),
            Err(e) => tracing::warn!("[PDF:reparse] Failed to set job address: {}", e),
        }
    }

    Ok(Json(ApiResponse::success(ReparseResponse {
        tickets_deleted: deleted,
        tickets_inserted: inserted as u64,
        tickets: response_tickets,
        extracted_text: Some(text),
    })))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReparseResponse {
    pub tickets_deleted: u64,
    pub tickets_inserted: u64,
    pub tickets: Vec<ReparseTicket>,
    pub extracted_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReparseTicket {
    pub ticket_number: String,
    pub description: Option<String>,
    pub room: Option<String>,
    pub qty: i32,
    pub section: Option<String>,
}

/// GET /api/jobs/:job_id/lading-tickets - List all lading tickets for a job
pub async fn get_lading_tickets(
    State(state): State<SharedState>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<LadingTicketsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let tickets = state.repo.get_lading_tickets_for_job(&job_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success(LadingTicketsResponse { tickets })))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LadingTicketsResponse {
    pub tickets: Vec<crate::db::models::LadingTicket>,
}

/// GET /api/lading-tickets/:ticket_number - Get description for a ticket number
pub async fn get_ticket_description(
    State(state): State<SharedState>,
    Path(ticket_number): Path<String>,
    Query(params): Query<TicketDescriptionQuery>,
) -> Result<Json<ApiResponse<Option<crate::db::models::LadingTicket>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let ticket = state.repo.get_lading_ticket_description(&ticket_number, params.job_id.as_deref()).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success(ticket)))
}

#[derive(Debug, Deserialize)]
pub struct TicketDescriptionQuery {
    pub job_id: Option<String>,
}

// Public wrappers for GUI access
pub fn extract_pdf_text_pub(data: &[u8], file_name: &str) -> Option<String> {
    extract_pdf_text(data, file_name)
}

pub fn parse_lading_tickets_pub(text: &str, job_id: &str, file_uuid: &str, file_name: &str) -> Vec<(String, String, String, Option<String>, Option<String>, i32, Option<String>)> {
    parse_lading_tickets(text, job_id, file_uuid, file_name)
}

pub fn extract_bol_address_pub(text: &str) -> Option<String> {
    extract_bol_address(text)
}

// ==================== USER BACKGROUNDS ====================

/// POST /api/user/background - Upload background image (multipart: device_id + image)
pub async fn upload_user_background(
    State(state): State<SharedState>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let mut device_id: Option<String> = None;
    let mut image_data: Option<Vec<u8>> = None;
    let mut content_type = "image/jpeg".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "device_id" => {
                if let Ok(text) = field.text().await {
                    device_id = Some(text);
                }
            }
            "image" => {
                if let Some(ct) = field.content_type() {
                    content_type = ct.to_string();
                }
                if let Ok(bytes) = field.bytes().await {
                    image_data = Some(bytes.to_vec());
                }
            }
            _ => {}
        }
    }

    let device_id = device_id.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error("Missing device_id")))
    })?;
    let image_data = image_data.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error("Missing image data")))
    })?;

    let state = state.read().await;
    state.repo.save_user_background(&device_id, &image_data, &content_type).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success_with_message((), "Background uploaded".to_string())))
}

/// GET /api/user/background/:device_id - Serve the raw background image
pub async fn get_user_background(
    State(state): State<SharedState>,
    Path(device_id): Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    let state = state.read().await;
    let (data, ct) = state.repo.get_user_background(&device_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(axum::response::Response::builder()
        .status(200)
        .header("Content-Type", ct)
        .header("Cache-Control", "public, max-age=3600")
        .body(axum::body::Body::from(data))
        .unwrap())
}

/// DELETE /api/user/background - Remove background image
pub async fn delete_user_background(
    State(state): State<SharedState>,
    Json(body): Json<DeleteBackgroundRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    state.repo.delete_user_background(&body.device_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    Ok(Json(ApiResponse::success_with_message((), "Background removed".to_string())))
}

#[derive(Deserialize)]
pub struct DeleteBackgroundRequest {
    pub device_id: String,
}

/// PUT /api/user/background/type - Set background type for a user
pub async fn set_user_background_type(
    State(state): State<SharedState>,
    Json(body): Json<SetBackgroundTypeRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let valid_types = ["image", "shader", "none"];
    if !valid_types.contains(&body.background_type.as_str()) {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error("Invalid background_type. Must be: image, shader, or none"))));
    }
    let state = state.read().await;
    if body.background_type == "none" {
        state.repo.delete_user_background(&body.device_id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    } else {
        state.repo.set_user_background_type(&body.device_id, &body.background_type).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
    }
    Ok(Json(ApiResponse::success_with_message((), format!("Background type set to {}", body.background_type))))
}

/// GET /api/user/background/:device_id/type - Get background type for a user
pub async fn get_user_background_type(
    State(state): State<SharedState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<BackgroundTypeResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state = state.read().await;
    let bg_type = state.repo.get_user_background_type(&device_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?
        .unwrap_or_else(|| "none".to_string());
    Ok(Json(ApiResponse::success(BackgroundTypeResponse { background_type: bg_type })))
}

#[derive(Deserialize)]
pub struct SetBackgroundTypeRequest {
    pub device_id: String,
    pub background_type: String,
}

#[derive(Serialize)]
pub struct BackgroundTypeResponse {
    pub background_type: String,
}

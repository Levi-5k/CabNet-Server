use super::routes::SharedState;
use crate::db::models::*;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

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
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub connected: bool,
    pub device_id: String,
    pub server_version: String,
    pub server_time: i64,
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

/// GET /api/scans - Get all scans with optional filtering
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
        device_id: request.device_id,
        device_name: request.device_name,
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

    // Update heartbeat for the newly connected device
    state.repo.update_device_heartbeat(&device_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        )
    })?;

    tracing::info!("Device connected: {} ({})", device_id, device_input.device_name.as_deref().unwrap_or("unnamed"));

    Ok(Json(ApiResponse::success_with_message(
        ConnectResponse {
            connected: true,
            device_id,
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            server_time: chrono::Utc::now().timestamp_millis(),
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
    let mut state = state.write().await;

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

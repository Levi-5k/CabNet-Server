use super::models::*;
use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;
use md5;

/// Repository for database operations
#[derive(Clone)]
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ==================== SCANS ====================

    /// Insert or update a scan
    pub async fn upsert_scan(&self, input: &ScanInput) -> anyhow::Result<i64> {
        let now = Utc::now().to_rfc3339();
        let scanned_at =
            chrono::DateTime::from_timestamp_millis(input.scanned_at)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| now.clone());

        // First check if a scan with same local_id + device_id already exists (duplicate from retried sync)
        if let Some(local_id) = input.local_id {
            let existing: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM scans WHERE local_id = ? AND device_id = ?"
            )
            .bind(local_id)
            .bind(&input.device_id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some((existing_id,)) = existing {
                // Update the existing row instead of inserting a duplicate
                sqlx::query(
                    r#"UPDATE scans SET uuid = ?, barcode = ?, barcode_type = ?, 
                        ticket_number = ?, barcode_job_ref = ?, synced_at = ?, is_printed = ?
                        WHERE id = ?"#
                )
                .bind(&input.id)
                .bind(&input.barcode_data)
                .bind(&input.barcode_type)
                .bind(&input.ticket_number)
                .bind(&input.barcode_job_ref)
                .bind(&now)
                .bind(input.is_printed.unwrap_or(false) as i32)
                .bind(existing_id)
                .execute(&self.pool)
                .await?;
                return Ok(existing_id);
            }
        }

        let result = sqlx::query(
            r#"
            INSERT INTO scans (uuid, barcode, barcode_type, ticket_number, barcode_job_ref, 
                              device_id, user_id, location, latitude, longitude, 
                              scanned_at, synced_at, is_printed, local_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(uuid) DO UPDATE SET
                barcode = excluded.barcode,
                barcode_type = excluded.barcode_type,
                synced_at = excluded.synced_at,
                local_id = excluded.local_id
            "#,
        )
        .bind(&input.id)
        .bind(&input.barcode_data)
        .bind(&input.barcode_type)
        .bind(&input.ticket_number)
        .bind(&input.barcode_job_ref)
        .bind(&input.device_id)
        .bind(&input.user_id)
        .bind(&input.location)
        .bind(input.latitude)
        .bind(input.longitude)
        .bind(&scanned_at)
        .bind(&now)
        .bind(input.is_printed.unwrap_or(false) as i32)
        .bind(input.local_id)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get all scans with optional filtering
    pub async fn get_scans(
        &self,
        job_id: Option<i64>,
        device_id: Option<&str>,
        since: Option<i64>,
        limit: i64,
    ) -> anyhow::Result<Vec<Scan>> {
        let mut query = String::from("SELECT * FROM scans WHERE 1=1");

        if job_id.is_some() {
            query.push_str(" AND job_id = ?");
        }
        if device_id.is_some() {
            query.push_str(" AND device_id = ?");
        }
        if since.is_some() {
            query.push_str(" AND scanned_at > ?");
        }
        query.push_str(" ORDER BY scanned_at DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, Scan>(&query);

        if let Some(jid) = job_id {
            q = q.bind(jid);
        }
        if let Some(did) = device_id {
            q = q.bind(did);
        }
        if let Some(since_ts) = since {
            // Convert millis timestamp to RFC3339 for comparison with stored scanned_at
            let since_str = chrono::DateTime::from_timestamp_millis(since_ts)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            q = q.bind(since_str);
        }
        q = q.bind(limit);

        Ok(q.fetch_all(&self.pool).await?)
    }

    /// Get scans by UUIDs
    pub async fn get_scans_by_uuids(&self, uuids: &[String]) -> anyhow::Result<Vec<String>> {
        if uuids.is_empty() {
            return Ok(vec![]);
        }

        let placeholders: Vec<&str> = uuids.iter().map(|_| "?").collect();
        let query = format!(
            "SELECT uuid FROM scans WHERE uuid IN ({})",
            placeholders.join(",")
        );

        let mut q = sqlx::query_scalar::<_, String>(&query);
        for uuid in uuids {
            q = q.bind(uuid);
        }

        Ok(q.fetch_all(&self.pool).await?)
    }

    /// Get scans by local IDs (from Android device)
    /// Returns the local_ids that exist in the database
    pub async fn get_scans_by_local_ids(&self, local_ids: &[i64]) -> anyhow::Result<Vec<i64>> {
        if local_ids.is_empty() {
            return Ok(vec![]);
        }

        let placeholders: Vec<&str> = local_ids.iter().map(|_| "?").collect();
        let query = format!(
            "SELECT local_id FROM scans WHERE local_id IN ({}) AND local_id IS NOT NULL",
            placeholders.join(",")
        );

        let mut q = sqlx::query_scalar::<_, i64>(&query);
        for local_id in local_ids {
            q = q.bind(local_id);
        }

        Ok(q.fetch_all(&self.pool).await?)
    }

    /// Verify scans by ID (supports both local_ids and UUIDs)
    /// Returns a tuple of (found_ids, missing_ids) as strings
    pub async fn verify_scans(&self, ids: &[String]) -> anyhow::Result<(Vec<String>, Vec<String>)> {
        if ids.is_empty() {
            return Ok((vec![], vec![]));
        }

        // Check if these look like local IDs (all numeric) or UUIDs
        let are_local_ids = ids.iter().all(|id| id.parse::<i64>().is_ok());

        let found: Vec<String> = if are_local_ids {
            // Parse as local IDs and look up
            let local_ids: Vec<i64> = ids.iter().filter_map(|id| id.parse().ok()).collect();
            self.get_scans_by_local_ids(&local_ids)
                .await?
                .into_iter()
                .map(|id| id.to_string())
                .collect()
        } else {
            // Look up by UUID
            self.get_scans_by_uuids(ids).await?
        };

        let found_set: std::collections::HashSet<&String> = found.iter().collect();
        let missing: Vec<String> = ids.iter()
            .filter(|id| !found_set.contains(id))
            .cloned()
            .collect();

        Ok((found, missing))
    }

    /// Get scans with GPS coordinates for map display
    pub async fn get_scan_locations(&self) -> anyhow::Result<Vec<ScanLocation>> {
        // First try scans with explicit lat/lon
        let scans = sqlx::query_as::<_, (i64, String, String, Option<f64>, Option<f64>, Option<String>, String, String)>(
            r#"
            SELECT s.id, s.uuid, s.barcode, s.latitude, s.longitude, s.location, s.scanned_at, s.device_id
            FROM scans s
            ORDER BY s.scanned_at DESC
            LIMIT 1000
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(scans
            .into_iter()
            .filter_map(|(id, uuid, barcode, lat, lon, location, scanned_at, device_id)| {
                // Try explicit lat/lon first
                if let (Some(lat), Some(lon)) = (lat, lon) {
                    if lat != 0.0 || lon != 0.0 {
                        return Some(ScanLocation {
                            id,
                            uuid,
                            barcode,
                            latitude: lat,
                            longitude: lon,
                            scanned_at,
                            device_id,
                            job_name: None,
                        });
                    }
                }
                
                // Try parsing location text field as "lat,lon" or "lat, lon"
                if let Some(loc) = location {
                    if let Some((lat_str, lon_str)) = loc.split_once(',') {
                        if let (Ok(lat), Ok(lon)) = (lat_str.trim().parse::<f64>(), lon_str.trim().parse::<f64>()) {
                            if lat != 0.0 || lon != 0.0 {
                                return Some(ScanLocation {
                                    id,
                                    uuid,
                                    barcode,
                                    latitude: lat,
                                    longitude: lon,
                                    scanned_at,
                                    device_id,
                                    job_name: None,
                                });
                            }
                        }
                    }
                }
                
                None
            })
            .collect())
    }

    /// Update job_id and/or device_id for a list of scan IDs
    pub async fn update_scan_assignments(
        &self,
        scan_ids: &[i64],
        job_id: Option<i64>,
        device_id: Option<&str>,
    ) -> anyhow::Result<u64> {
        if scan_ids.is_empty() {
            return Ok(0);
        }

        let mut set_clauses = Vec::new();
        if job_id.is_some() {
            set_clauses.push("job_id = ?".to_string());
        }
        if device_id.is_some() {
            set_clauses.push("device_id = ?".to_string());
        }
        if set_clauses.is_empty() {
            return Ok(0);
        }

        let placeholders: Vec<&str> = scan_ids.iter().map(|_| "?").collect();
        let query = format!(
            "UPDATE scans SET {} WHERE id IN ({})",
            set_clauses.join(", "),
            placeholders.join(",")
        );

        let mut q = sqlx::query(&query);
        if let Some(jid) = job_id {
            q = q.bind(jid);
        }
        if let Some(did) = device_id {
            q = q.bind(did);
        }
        for id in scan_ids {
            q = q.bind(id);
        }

        let result = q.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Get total scan count
    pub async fn get_scan_count(&self) -> anyhow::Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scans")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0)
    }

    /// Delete a scan by its database ID
    pub async fn delete_scan(&self, id: i64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM scans WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ==================== JOBS ====================

    /// Insert or update a job
    pub async fn upsert_job(&self, input: &JobInput) -> anyhow::Result<i64> {
        let now = Utc::now().to_rfc3339();

        let created_at = input.created_at
            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| now.clone());

        let started_at = input.started_at
            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts))
            .map(|dt| dt.to_rfc3339());

        let completed_at = input.completed_at
            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts))
            .map(|dt| dt.to_rfc3339());

        let due_date = input.due_date
            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts))
            .map(|dt| dt.to_rfc3339());

        let result = sqlx::query(
            r#"
            INSERT INTO jobs (uuid, name, description, reference_number, customer_name,
                             expected_count, status, device_id, created_by, notes, 
                             priority, due_date, created_at, started_at, completed_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(uuid) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                reference_number = excluded.reference_number,
                customer_name = excluded.customer_name,
                expected_count = excluded.expected_count,
                status = excluded.status,
                notes = excluded.notes,
                priority = excluded.priority,
                due_date = excluded.due_date,
                started_at = excluded.started_at,
                completed_at = excluded.completed_at,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&input.id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.reference_number)
        .bind(&input.customer_name)
        .bind(input.expected_count.unwrap_or(0))
        .bind(input.status.as_deref().unwrap_or("PENDING"))
        .bind(&input.device_id)
        .bind(&input.created_by)
        .bind(&input.notes)
        .bind(input.priority.as_deref().unwrap_or("NORMAL"))
        .bind(&due_date)
        .bind(&created_at)
        .bind(&started_at)
        .bind(&completed_at)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get a job by its UUID (used to resolve Android job_id references)
    pub async fn get_job_by_uuid(&self, uuid: &str) -> anyhow::Result<Option<Job>> {
        Ok(sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE uuid = ?")
            .bind(uuid)
            .fetch_optional(&self.pool)
            .await?)
    }

    /// Get a job by its reference number
    pub async fn get_job_by_reference(&self, reference: &str) -> anyhow::Result<Option<Job>> {
        Ok(sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE reference_number = ?")
            .bind(reference)
            .fetch_optional(&self.pool)
            .await?)
    }

    /// Set a job's reference number (from barcode_job_ref during scan sync)
    pub async fn set_job_reference_number(&self, job_id: i64, reference: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET reference_number = ?, updated_at = ? WHERE id = ? AND (reference_number IS NULL OR reference_number = '')")
            .bind(reference)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update scan's job_id
    pub async fn set_scan_job_id(&self, scan_id: i64, job_id: i64) -> anyhow::Result<()> {
        sqlx::query("UPDATE scans SET job_id = ? WHERE id = ?")
            .bind(job_id)
            .bind(scan_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all jobs
    pub async fn get_jobs(&self) -> anyhow::Result<Vec<Job>> {
        Ok(
            sqlx::query_as::<_, Job>("SELECT * FROM jobs ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    /// Get jobs with scan counts
    pub async fn get_jobs_with_counts(&self) -> anyhow::Result<Vec<JobWithCount>> {
        // Get jobs first
        let jobs = self.get_jobs().await?;
        let mut result = Vec::new();

        for job in jobs {
            let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scans WHERE job_id = ?")
                .bind(job.id)
                .fetch_one(&self.pool)
                .await?;
            result.push(JobWithCount {
                job,
                scan_count: count.0 as i32,
            });
        }

        Ok(result)
    }

    /// Get job by ID
    pub async fn get_job(&self, id: i64) -> anyhow::Result<Option<Job>> {
        Ok(sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    /// Update job with all editable fields
    pub async fn update_job_details(
        &self,
        id: i64,
        name: Option<&str>,
        status: Option<&str>,
        reference_number: Option<Option<&str>>,
        description: Option<Option<&str>>,
        customer_name: Option<Option<&str>>,
        notes: Option<Option<&str>>,
        priority: Option<&str>,
        expected_count: Option<i32>,
        due_date: Option<Option<&str>>,
    ) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        let mut set_parts = vec!["updated_at = ?".to_string()];
        if name.is_some() { set_parts.push("name = ?".to_string()); }
        if status.is_some() { set_parts.push("status = ?".to_string()); }
        if reference_number.is_some() { set_parts.push("reference_number = ?".to_string()); }
        if description.is_some() { set_parts.push("description = ?".to_string()); }
        if customer_name.is_some() { set_parts.push("customer_name = ?".to_string()); }
        if notes.is_some() { set_parts.push("notes = ?".to_string()); }
        if priority.is_some() { set_parts.push("priority = ?".to_string()); }
        if expected_count.is_some() { set_parts.push("expected_count = ?".to_string()); }
        if due_date.is_some() { set_parts.push("due_date = ?".to_string()); }

        let query = format!("UPDATE jobs SET {} WHERE id = ?", set_parts.join(", "));
        let mut q = sqlx::query(&query);
        q = q.bind(&now);
        if let Some(v) = name { q = q.bind(v); }
        if let Some(v) = status { q = q.bind(v); }
        if let Some(v) = reference_number { q = q.bind(v); }
        if let Some(v) = description { q = q.bind(v); }
        if let Some(v) = customer_name { q = q.bind(v); }
        if let Some(v) = notes { q = q.bind(v); }
        if let Some(v) = priority { q = q.bind(v); }
        if let Some(v) = expected_count { q = q.bind(v); }
        if let Some(v) = due_date { q = q.bind(v); }
        q = q.bind(id);
        q.execute(&self.pool).await?;
        Ok(())
    }

    /// Update job (legacy compat)
    pub async fn update_job(&self, id: i64, name: &str, status: &str) -> anyhow::Result<()> {
        self.update_job_details(id, Some(name), Some(status), None, None, None, None, None, None, None).await
    }

    /// Get scan count for a specific job
    pub async fn get_job_scan_count(&self, job_id: i64) -> anyhow::Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scans WHERE job_id = ?")
            .bind(job_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0)
    }

    /// Delete job
    pub async fn delete_job(&self, id: i64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM jobs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Link any unmatched scans to a job by matching barcode_job_ref to the job's reference_number.
    /// This handles the case where scans were synced before the job's reference_number was set.
    pub async fn link_scans_by_reference(&self, job_id: i64, reference: &str) -> anyhow::Result<u64> {
        if reference.is_empty() {
            return Ok(0);
        }
        let result = sqlx::query(
            "UPDATE scans SET job_id = ? WHERE barcode_job_ref = ? AND (job_id IS NULL OR job_id = 0)"
        )
        .bind(job_id)
        .bind(reference)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ==================== DEVICES ====================

    /// Register or update a device with security validation
    pub async fn upsert_device(&self, input: &DeviceInput) -> anyhow::Result<i64> {
        let now = Utc::now().to_rfc3339();

        // Generate fingerprint hash if provided
        let fingerprint_hash = if let Some(fingerprint) = &input.fingerprint {
            Some(format!("{:x}", md5::compute(fingerprint.as_bytes())))
        } else {
            None
        };

        let result = sqlx::query(
            r#"
            INSERT INTO devices (
                device_id, device_name, model, os_version, app_version, last_seen_at,
                registration_code, fingerprint_hash, user_agent, registration_attempts,
                last_registration_attempt_at, ip_address
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)
            ON CONFLICT(device_id) DO UPDATE SET
                device_name = COALESCE(excluded.device_name, devices.device_name),
                model = COALESCE(excluded.model, devices.model),
                os_version = COALESCE(excluded.os_version, devices.os_version),
                app_version = COALESCE(excluded.app_version, devices.app_version),
                last_seen_at = excluded.last_seen_at,
                fingerprint_hash = COALESCE(excluded.fingerprint_hash, devices.fingerprint_hash),
                user_agent = COALESCE(excluded.user_agent, devices.user_agent),
                registration_attempts = devices.registration_attempts + 1,
                last_registration_attempt_at = excluded.last_registration_attempt_at,
                ip_address = COALESCE(excluded.ip_address, devices.ip_address)
            "#,
        )
        .bind(&input.device_id)
        .bind(&input.device_name)
        .bind(&input.model)
        .bind(&input.os_version)
        .bind(&input.app_version)
        .bind(&now)
        .bind(&input.registration_code)
        .bind(&fingerprint_hash)
        .bind(&input.user_agent)
        .bind(&now)
        .bind("unknown") // ip_address placeholder - will be updated by handler
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Update device last seen time (creates device if not exists)
    pub async fn update_device_last_seen(&self, device_id: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        
        // Use INSERT OR REPLACE to create device if it doesn't exist
        sqlx::query(
            r#"
            INSERT INTO devices (device_id, last_seen_at)
            VALUES (?, ?)
            ON CONFLICT(device_id) DO UPDATE SET
                last_seen_at = excluded.last_seen_at
            "#
        )
        .bind(device_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Clear device last_seen_at to immediately mark it as offline
    pub async fn clear_device_last_seen(&self, device_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE devices SET last_seen_at = NULL WHERE device_id = ?")
            .bind(device_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all devices with enhanced status information
    pub async fn get_devices_with_status(&self) -> anyhow::Result<Vec<Device>> {
        Ok(sqlx::query_as::<_, Device>(
            r#"SELECT * FROM devices ORDER BY last_seen_at DESC NULLS LAST"#,
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Update device heartbeat with enhanced status tracking
    pub async fn update_device_heartbeat(&self, device_id: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        
        // First, get current device status
        let device: Option<Device> = sqlx::query_as::<_, Device>(
            r#"SELECT * FROM devices WHERE device_id = ?"#,
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(mut device) = device {
            // Calculate uptime if this is a reconnection
            let uptime_increment = if let (Some(last_heartbeat), Some(last_seen)) = 
                (device.last_heartbeat_at, device.last_seen_at) {
                // If last_heartbeat is recent (within 5 minutes), this is a continuation
                let last_heartbeat_time = chrono::DateTime::parse_from_rfc3339(&last_heartbeat)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or(Utc::now());
                let _last_seen_time = chrono::DateTime::parse_from_rfc3339(&last_seen)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or(Utc::now());
                
                let time_diff = Utc::now().signed_duration_since(last_heartbeat_time);
                if time_diff.num_minutes() <= 5 {
                    // Continuation of existing session
                    time_diff.num_seconds().max(0) as i64
                } else {
                    // New session, increment connection count
                    device.connection_count = Some(device.connection_count.unwrap_or(0) + 1);
                    0
                }
            } else {
                // First heartbeat or reconnection
                device.connection_count = Some(device.connection_count.unwrap_or(0) + 1);
                0
            };

            // Update uptime
            let current_uptime = device.total_uptime_seconds.unwrap_or(0);
            let new_uptime = current_uptime + uptime_increment;

            // Update device status to online
            sqlx::query(
                r#"UPDATE devices SET 
                    last_seen_at = ?, 
                    last_heartbeat_at = ?,
                    status = 'online',
                    connection_count = ?,
                    total_uptime_seconds = ?
                   WHERE device_id = ?"#,
            )
            .bind(&now)
            .bind(&now)
            .bind(device.connection_count)
            .bind(new_uptime)
            .bind(device_id)
            .execute(&self.pool)
            .await?;
        } else {
            // Device doesn't exist, create it with initial heartbeat
            sqlx::query(
                r#"INSERT INTO devices (device_id, last_seen_at, last_heartbeat_at, status, connection_count, total_uptime_seconds)
                   VALUES (?, ?, ?, 'online', 1, 0)"#,
            )
            .bind(device_id)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }
        
        Ok(())
    }

    /// Update device heartbeat data (battery, memory, network info)
    pub async fn update_device_heartbeat_data(
        &self,
        device_id: &str,
        battery_level: Option<i32>,
        is_charging: Option<bool>,
        available_memory_mb: Option<i32>,
        total_memory_mb: Option<i32>,
        app_version: Option<&str>,
        network_type: Option<&str>,
        connection_quality: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"UPDATE devices SET 
                battery_level = ?,
                is_charging = ?,
                available_memory_mb = ?,
                total_memory_mb = ?,
                app_version = ?,
                network_type = ?,
                connection_quality = ?
               WHERE device_id = ?"#,
        )
        .bind(battery_level)
        .bind(is_charging.map(|b| if b { 1 } else { 0 })) // SQLite stores booleans as integers
        .bind(available_memory_mb)
        .bind(total_memory_mb)
        .bind(app_version)
        .bind(network_type)
        .bind(connection_quality)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update device status (online/offline/inactive)
    pub async fn update_device_status(&self, device_id: &str, status: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        
        sqlx::query(
            "UPDATE devices SET status = ?, last_seen_at = ? WHERE device_id = ?"
        )
        .bind(status)
        .bind(&now)
        .bind(device_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Record device error
    pub async fn record_device_error(&self, device_id: &str, error_message: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        
        // Get current error count
        let current_count: Option<i64> = sqlx::query_scalar(
            "SELECT error_count FROM devices WHERE device_id = ?"
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;
        
        let new_count = current_count.unwrap_or(0) + 1;
        
        sqlx::query(
            r#"UPDATE devices SET 
                error_count = ?, 
                last_error_at = ?, 
                last_error_message = ?,
                last_seen_at = ?
               WHERE device_id = ?"#,
        )
        .bind(new_count)
        .bind(&now)
        .bind(error_message)
        .bind(&now)
        .bind(device_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Clean up inactive devices (mark as inactive if no heartbeat for extended period)
    pub async fn cleanup_inactive_devices(&self, inactive_threshold_minutes: i64) -> anyhow::Result<i64> {
        let threshold_time = Utc::now() - chrono::Duration::minutes(inactive_threshold_minutes);
        let threshold_str = threshold_time.to_rfc3339();
        
        let result = sqlx::query(
            "UPDATE devices SET status = 'inactive' WHERE status = 'online' AND last_heartbeat_at < ?"
        )
        .bind(&threshold_str)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() as i64)
    }

    /// Get all devices
    pub async fn get_devices(&self) -> anyhow::Result<Vec<Device>> {
        Ok(
            sqlx::query_as::<_, Device>("SELECT * FROM devices ORDER BY last_seen_at DESC")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    /// Get devices with statistics
    pub async fn get_devices_with_stats(&self) -> anyhow::Result<Vec<DeviceWithStats>> {
        let devices = self.get_devices().await?;
        let mut result = Vec::new();

        for device in devices {
            let count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM scans WHERE device_id = ?")
                    .bind(&device.device_id)
                    .fetch_one(&self.pool)
                    .await?;

            // Consider device online if seen in last 5 minutes
            let is_online = device
                .last_seen_at
                .as_ref()
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| {
                    Utc::now().signed_duration_since(dt.with_timezone(&Utc))
                        < chrono::Duration::minutes(5)
                })
                .unwrap_or(false);

            result.push(DeviceWithStats {
                device,
                total_scans: count.0 as i32,
                is_online,
            });
        }

        Ok(result)
    }

    /// Delete a device by ID
    pub async fn delete_device(&self, device_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM devices WHERE device_id = ?")
            .bind(device_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ==================== EMAIL ====================

    /// Get email configuration
    pub async fn get_email_config(&self) -> anyhow::Result<Option<EmailConfigRecord>> {
        Ok(
            sqlx::query_as::<_, EmailConfigRecord>("SELECT * FROM email_config WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Update email configuration
    pub async fn update_email_config(
        &self,
        smtp_host: &str,
        smtp_port: i32,
        smtp_username: &str,
        smtp_password: &str,
        email_from: &str,
        email_to: &str,
        auto_send_enabled: bool,
        auto_send_delay_minutes: i32,
    ) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE email_config SET
                smtp_host = ?,
                smtp_port = ?,
                smtp_username = ?,
                smtp_password = ?,
                email_from = ?,
                email_to = ?,
                auto_send_enabled = ?,
                auto_send_delay_minutes = ?,
                updated_at = ?
            WHERE id = 1
            "#,
        )
        .bind(smtp_host)
        .bind(smtp_port)
        .bind(smtp_username)
        .bind(smtp_password)
        .bind(email_from)
        .bind(email_to)
        .bind(auto_send_enabled as i32)
        .bind(auto_send_delay_minutes)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Add email history record
    pub async fn add_email_history(
        &self,
        recipients: &str,
        subject: &str,
        scan_count: i32,
        status: &str,
        error_message: Option<&str>,
    ) -> anyhow::Result<i64> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO email_history (sent_at, recipients, subject, scan_count, status, error_message)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&now)
        .bind(recipients)
        .bind(subject)
        .bind(scan_count)
        .bind(status)
        .bind(error_message)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    /// Get email history
    pub async fn get_email_history(&self, limit: i64) -> anyhow::Result<Vec<EmailHistory>> {
        Ok(sqlx::query_as::<_, EmailHistory>(
            "SELECT * FROM email_history ORDER BY sent_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    // ==================== SETTINGS ====================

    /// Get a setting value
    pub async fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>> {
        let setting: Option<(Option<String>,)> =
            sqlx::query_as("SELECT value FROM settings WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(setting.and_then(|s| s.0))
    }

    /// Set a setting value
    pub async fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ==================== WEB CLIENTS ====================

    /// Register or update a web client
    pub async fn upsert_web_client(
        &self,
        client_id: &str,
        client_name: Option<&str>,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> anyhow::Result<WebClient> {
        let now = Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT INTO web_clients (client_id, client_name, user_agent, ip_address, last_seen_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(client_id) DO UPDATE SET
                client_name = COALESCE(excluded.client_name, web_clients.client_name),
                user_agent = COALESCE(excluded.user_agent, web_clients.user_agent),
                ip_address = COALESCE(excluded.ip_address, web_clients.ip_address),
                last_seen_at = excluded.last_seen_at
            "#,
        )
        .bind(client_id)
        .bind(client_name)
        .bind(user_agent)
        .bind(ip_address)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_web_client(client_id).await?.ok_or_else(|| anyhow::anyhow!("Failed to get web client"))
    }

    /// Get a web client by ID
    pub async fn get_web_client(&self, client_id: &str) -> anyhow::Result<Option<WebClient>> {
        Ok(
            sqlx::query_as::<_, WebClient>("SELECT * FROM web_clients WHERE client_id = ?")
                .bind(client_id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Get all web clients
    pub async fn get_web_clients(&self) -> anyhow::Result<Vec<WebClient>> {
        Ok(
            sqlx::query_as::<_, WebClient>("SELECT * FROM web_clients ORDER BY last_seen_at DESC")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    /// Set web client trust status
    pub async fn set_web_client_trust(&self, client_id: &str, is_trusted: bool) -> anyhow::Result<()> {
        sqlx::query("UPDATE web_clients SET is_trusted = ? WHERE client_id = ?")
            .bind(is_trusted as i32)
            .bind(client_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update web client name
    pub async fn update_web_client_name(&self, client_id: &str, name: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE web_clients SET client_name = ? WHERE client_id = ?")
            .bind(name)
            .bind(client_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a web client
    pub async fn delete_web_client(&self, client_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM web_clients WHERE client_id = ?")
            .bind(client_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ==================== PENDING CHANGES ====================

    /// Add a pending change
    pub async fn add_pending_change(
        &self,
        client_id: &str,
        change_type: &str,
        entity_type: &str,
        entity_id: Option<&str>,
        change_data: &str,
    ) -> anyhow::Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO pending_changes (client_id, change_type, entity_type, entity_id, change_data)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(client_id)
        .bind(change_type)
        .bind(entity_type)
        .bind(entity_id)
        .bind(change_data)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get pending changes (status = 'pending')
    pub async fn get_pending_changes(&self) -> anyhow::Result<Vec<PendingChange>> {
        Ok(
            sqlx::query_as::<_, PendingChange>(
                "SELECT * FROM pending_changes WHERE status = 'pending' ORDER BY created_at DESC"
            )
            .fetch_all(&self.pool)
            .await?,
        )
    }

    /// Get all changes (for history)
    pub async fn get_all_changes(&self, limit: i64) -> anyhow::Result<Vec<PendingChange>> {
        Ok(
            sqlx::query_as::<_, PendingChange>(
                "SELECT * FROM pending_changes ORDER BY created_at DESC LIMIT ?"
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    /// Get a pending change by ID
    pub async fn get_pending_change(&self, id: i64) -> anyhow::Result<Option<PendingChange>> {
        Ok(
            sqlx::query_as::<_, PendingChange>("SELECT * FROM pending_changes WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Approve a pending change
    pub async fn approve_change(&self, id: i64, reviewed_by: Option<&str>) -> anyhow::Result<Option<PendingChange>> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE pending_changes SET status = 'approved', reviewed_at = ?, reviewed_by = ? WHERE id = ?")
            .bind(&now)
            .bind(reviewed_by)
            .bind(id)
            .execute(&self.pool)
            .await?;
        
        self.get_pending_change(id).await
    }

    /// Reject a pending change
    pub async fn reject_change(&self, id: i64, reviewed_by: Option<&str>) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE pending_changes SET status = 'rejected', reviewed_at = ?, reviewed_by = ? WHERE id = ?")
            .bind(&now)
            .bind(reviewed_by)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get pending change count
    pub async fn get_pending_change_count(&self) -> anyhow::Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pending_changes WHERE status = 'pending'")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0)
    }

    // ==================== SECURITY METHODS ====================

    /// Update device approval status
    pub async fn update_device_approval(&self, device_id: &str, approved: bool, notes: Option<&str>) -> anyhow::Result<()> {
        let status = if approved { "approved" } else { "rejected" };
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"UPDATE devices SET
                is_approved = ?,
                status = ?,
                last_seen_at = ?,
                registered_at = COALESCE(registered_at, ?)
                WHERE device_id = ?"#
        )
        .bind(approved as i32)
        .bind(status)
        .bind(&now)
        .bind(&now)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        // Log the approval/rejection
        if let Some(notes) = notes {
            sqlx::query(
                "INSERT INTO pending_changes (change_type, entity_type, entity_id, data, status, created_by, notes)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(if approved { "device_approval" } else { "device_rejection" })
            .bind("device")
            .bind(device_id)
            .bind("{}")
            .bind("completed")
            .bind("admin")
            .bind(notes)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Authenticate device with token
    pub async fn authenticate_device(&self, device_id: &str, auth_token: &str) -> anyhow::Result<Option<Device>> {
        let device: Option<Device> = sqlx::query_as(
            r#"SELECT * FROM devices
               WHERE device_id = ? AND auth_token = ? AND auth_token_expires_at > datetime('now')
               AND is_active = 1"#
        )
        .bind(device_id)
        .bind(auth_token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(device)
    }

    /// Block device temporarily
    pub async fn block_device(&self, device_id: &str, minutes: i32) -> anyhow::Result<()> {
        let blocked_until = Utc::now()
            .checked_add_signed(chrono::Duration::minutes(minutes as i64))
            .unwrap()
            .to_rfc3339();

        sqlx::query(
            "UPDATE devices SET blocked_until = ?, status = 'blocked' WHERE device_id = ?"
        )
        .bind(blocked_until)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Unblock device
    pub async fn unblock_device(&self, device_id: &str) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE devices SET blocked_until = NULL, status = 'approved' WHERE device_id = ?"
        )
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get security settings
    pub async fn get_security_settings(&self) -> anyhow::Result<SecuritySettings> {
        let settings = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM settings WHERE key LIKE 'security.%'"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut security_settings = SecuritySettings {
            max_registration_attempts_per_device: 5,
            max_registration_attempts_per_ip: 10,
            rate_limit_window_minutes: 15,
            block_duration_minutes: 60,
            token_expiry_hours: 24,
            require_registration_code: true,
        };

        for (key, value) in settings {
            match key.as_str() {
                "security.require_registration_codes" => security_settings.require_registration_code = value == "true",
                "security.max_registration_attempts_per_device" => security_settings.max_registration_attempts_per_device = value.parse().unwrap_or(5),
                "security.max_registration_attempts_per_ip" => security_settings.max_registration_attempts_per_ip = value.parse().unwrap_or(10),
                "security.rate_limit_window_minutes" => security_settings.rate_limit_window_minutes = value.parse().unwrap_or(15),
                "security.block_duration_minutes" => security_settings.block_duration_minutes = value.parse().unwrap_or(60),
                "security.token_expiry_hours" => security_settings.token_expiry_hours = value.parse().unwrap_or(24),
                _ => {}
            }
        }

        Ok(security_settings)
    }

    /// Update security settings
    pub async fn update_security_settings(&self, settings: &SecuritySettings) -> anyhow::Result<()> {
        let updates = vec![
            ("security.require_registration_codes", settings.require_registration_code.to_string()),
            ("security.max_registration_attempts_per_device", settings.max_registration_attempts_per_device.to_string()),
            ("security.max_registration_attempts_per_ip", settings.max_registration_attempts_per_ip.to_string()),
            ("security.rate_limit_window_minutes", settings.rate_limit_window_minutes.to_string()),
            ("security.block_duration_minutes", settings.block_duration_minutes.to_string()),
            ("security.token_expiry_hours", settings.token_expiry_hours.to_string()),
        ];

        for (key, value) in updates {
            sqlx::query(
                "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?, ?, datetime('now'))"
            )
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Generate registration code
    pub async fn generate_registration_code(&self, description: Option<&str>) -> anyhow::Result<String> {
        use rand::Rng;
        let code: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, datetime('now'))"
        )
        .bind(format!("reg_code.{}", code))
        .bind(description.unwrap_or("Generated registration code"))
        .execute(&self.pool)
        .await?;

        Ok(code)
    }

    /// Get active registration codes
    pub async fn get_registration_codes(&self) -> anyhow::Result<Vec<RegistrationCodeInfo>> {
        let codes: Vec<(String, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"SELECT
                REPLACE(key, 'reg_code.', '') as code,
                value as description,
                updated_at as created_at,
                NULL as used_by,
                NULL as used_at
               FROM settings
               WHERE key LIKE 'reg_code.%'
               ORDER BY updated_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(codes.into_iter().map(|(code, description, created_at, used_by, used_at)| {
            RegistrationCodeInfo {
                code,
                description: Some(description),
                created_at,
                used_by,
                used_at,
            }
        }).collect())
    }

    /// Validate registration code
    pub async fn validate_registration_code(&self, code: &str) -> anyhow::Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM settings WHERE key = ?"
        )
        .bind(format!("reg_code.{}", code))
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// Use registration code (mark as used)
    pub async fn use_registration_code(&self, code: &str, device_id: &str) -> anyhow::Result<()> {
        let new_key = format!("used_reg_code.{}", code);
        let description = format!("Used by device: {}", device_id);

        // Move to used codes
        sqlx::query(
            r#"INSERT INTO settings (key, value, updated_at)
               SELECT ?, ?, datetime('now')
               FROM settings WHERE key = ?"#
        )
        .bind(&new_key)
        .bind(&description)
        .bind(format!("reg_code.{}", code))
        .execute(&self.pool)
        .await?;

        // Remove from active codes
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(format!("reg_code.{}", code))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Check rate limiting for device registration
    pub async fn check_registration_rate_limit(&self, fingerprint: &str, ip_address: &str) -> anyhow::Result<bool> {
        let settings = self.get_security_settings().await?;
        let max_attempts = settings.max_registration_attempts_per_ip as i64;
        let lockout_minutes = settings.block_duration_minutes;

        // Check attempts by fingerprint
        let fingerprint_attempts: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM devices
               WHERE fingerprint_hash = ? AND last_registration_attempt_at > datetime('now', '-{} minutes')"#,
        )
        .bind(fingerprint)
        .bind(lockout_minutes)
        .fetch_one(&self.pool)
        .await?;

        // Check attempts by IP
        let ip_attempts: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM devices
               WHERE ip_address = ? AND last_registration_attempt_at > datetime('now', '-{} minutes')"#,
        )
        .bind(ip_address)
        .bind(lockout_minutes)
        .fetch_one(&self.pool)
        .await?;

        Ok(fingerprint_attempts.0 < max_attempts && ip_attempts.0 < max_attempts)
    }

    /// Record registration attempt
    pub async fn record_registration_attempt(&self, device_id: &str, fingerprint: Option<&str>, ip_address: Option<&str>) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"UPDATE devices SET
                registration_attempts = registration_attempts + 1,
                last_registration_attempt_at = ?,
                fingerprint_hash = COALESCE(?, fingerprint_hash),
                ip_address = COALESCE(?, ip_address)
                WHERE device_id = ?"#
        )
        .bind(&now)
        .bind(fingerprint)
        .bind(ip_address)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Generate device auth token
    pub async fn generate_device_token(&self, device_id: &str) -> anyhow::Result<String> {
        use rand::Rng;
        let token: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let settings = self.get_security_settings().await?;
        let expires_at = Utc::now()
            .checked_add_signed(chrono::Duration::hours(settings.token_expiry_hours as i64))
            .unwrap()
            .to_rfc3339();

        sqlx::query(
            "UPDATE devices SET auth_token = ?, auth_token_expires_at = ? WHERE device_id = ?"
        )
        .bind(&token)
        .bind(expires_at)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    /// Check if device is rate limited for registration
    pub async fn check_device_rate_limit(&self, device_id: &str, ip_address: &str) -> anyhow::Result<bool> {
        let settings = self.get_security_settings().await?;
        let now = Utc::now();
        let window_start = now
            .checked_sub_signed(chrono::Duration::minutes(settings.rate_limit_window_minutes as i64))
            .unwrap()
            .to_rfc3339();

        // Check device-specific rate limit
        let device_attempts: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM devices WHERE device_id = ? AND last_registration_attempt_at > ?"
        )
        .bind(device_id)
        .bind(&window_start)
        .fetch_one(&self.pool)
        .await?;

        // Check IP-specific rate limit
        let ip_attempts: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM devices WHERE ip_address = ? AND last_registration_attempt_at > ?"
        )
        .bind(ip_address)
        .bind(&window_start)
        .fetch_one(&self.pool)
        .await?;

        Ok(device_attempts.0 >= settings.max_registration_attempts_per_device as i64 ||
           ip_attempts.0 >= settings.max_registration_attempts_per_ip as i64)
    }

    /// Update device rate limit counters
    pub async fn update_device_rate_limit(&self, device_id: &str, ip_address: &str) -> anyhow::Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE devices SET last_registration_attempt_at = ?, ip_address = ? WHERE device_id = ?"
        )
        .bind(&now)
        .bind(ip_address)
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if device is blocked
    pub async fn is_device_blocked(&self, device_id: &str) -> anyhow::Result<bool> {
        let device: Option<(String,)> = sqlx::query_as(
            "SELECT blocked_until FROM devices WHERE device_id = ?"
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((blocked_until,)) = device {
            if let Ok(blocked_time) = chrono::DateTime::parse_from_rfc3339(&blocked_until) {
                let blocked_time_utc = blocked_time.with_timezone(&Utc);
                return Ok(Utc::now() < blocked_time_utc);
            }
        }

        Ok(false)
    }

    /// Get a single device by ID
    pub async fn get_device(&self, device_id: &str) -> anyhow::Result<Option<Device>> {
        Ok(sqlx::query_as::<_, Device>(
            r#"SELECT * FROM devices WHERE device_id = ?"#,
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Check if heartbeat is rate limited for device/IP
    pub async fn check_heartbeat_rate_limit(&self, device_id: &str, ip_address: &str) -> anyhow::Result<bool> {
        let now = Utc::now();
        let hour_ago = now
            .checked_sub_signed(chrono::Duration::hours(1))
            .unwrap()
            .to_rfc3339();

        let heartbeat_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM devices WHERE (device_id = ? OR ip_address = ?) AND last_heartbeat_at > ?"
        )
        .bind(device_id)
        .bind(ip_address)
        .bind(&hour_ago)
        .fetch_one(&self.pool)
        .await?;

        // Allow up to 60 heartbeats per hour
        Ok(heartbeat_count.0 > 60)
    }

    // ==================== TIME ENTRIES ====================

    /// Upsert a time entry
    pub async fn upsert_time_entry(&self, input: &TimeEntryInput) -> anyhow::Result<i64> {
        let now = Utc::now().to_rfc3339();

        // Check if time rounding is enabled
        let rounding = self.get_time_rounding_config().await;

        let clock_in_dt = chrono::DateTime::from_timestamp_millis(input.clock_in)
            .unwrap_or_else(|| Utc::now());
        let clock_in_dt = if rounding.0 { round_time(clock_in_dt, rounding.1) } else { clock_in_dt };
        let clock_in = clock_in_dt.to_rfc3339();

        let clock_out = input.clock_out.and_then(|ts| {
            chrono::DateTime::from_timestamp_millis(ts).map(|dt| {
                if rounding.0 { round_time(dt, rounding.1).to_rfc3339() } else { dt.to_rfc3339() }
            })
        });

        let result = sqlx::query(
            r#"
            INSERT INTO time_entries (uuid, device_id, customer_name, job_name, job_id, 
                                      clock_in, clock_out, note, is_break, is_paid, synced_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(uuid) DO UPDATE SET
                clock_out = excluded.clock_out,
                note = excluded.note,
                synced_at = excluded.synced_at
            "#,
        )
        .bind(&input.id)
        .bind(&input.device_id)
        .bind(&input.customer_name)
        .bind(&input.job_name)
        .bind(&input.job_id)
        .bind(&clock_in)
        .bind(&clock_out)
        .bind(&input.note)
        .bind(input.is_break.unwrap_or(false) as i32)
        .bind(input.is_paid.unwrap_or(true) as i32)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get all time entries with optional filtering
    pub async fn get_time_entries(
        &self,
        device_id: Option<&str>,
        job_id: Option<&str>,
        since: Option<i64>,
        limit: i64,
    ) -> anyhow::Result<Vec<TimeEntryRecord>> {
        let mut query = String::from("SELECT * FROM time_entries WHERE 1=1");
        if device_id.is_some() {
            query.push_str(" AND device_id = ?");
        }
        if job_id.is_some() {
            query.push_str(" AND job_id = ?");
        }
        if since.is_some() {
            query.push_str(" AND clock_in > ?");
        }
        query.push_str(" ORDER BY clock_in DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, TimeEntryRecord>(&query);
        if let Some(did) = device_id {
            q = q.bind(did);
        }
        if let Some(jid) = job_id {
            q = q.bind(jid);
        }
        if let Some(since_ts) = since {
            let since_dt = chrono::DateTime::from_timestamp_millis(since_ts)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            q = q.bind(since_dt);
        }
        q = q.bind(limit);

        Ok(q.fetch_all(&self.pool).await?)
    }

    /// Get time entry count
    pub async fn get_time_entry_count(&self) -> anyhow::Result<i64> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM time_entries")
            .fetch_one(&self.pool)
            .await?;
        Ok(result.0)
    }

    /// Get active time entries (currently clocked in)
    pub async fn get_active_time_entries(&self) -> anyhow::Result<Vec<TimeEntryRecord>> {
        let entries = sqlx::query_as::<_, TimeEntryRecord>(
            "SELECT * FROM time_entries WHERE clock_out IS NULL AND is_break = 0 ORDER BY clock_in DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(entries)
    }

    /// Get active breaks
    pub async fn get_active_breaks(&self) -> anyhow::Result<Vec<TimeEntryRecord>> {
        let entries = sqlx::query_as::<_, TimeEntryRecord>(
            "SELECT * FROM time_entries WHERE clock_out IS NULL AND is_break = 1 ORDER BY clock_in DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(entries)
    }

    // ==================== REPORTS ====================

    /// Upsert a report
    pub async fn upsert_report(&self, input: &ReportInput) -> anyhow::Result<i64> {
        let now = Utc::now().to_rfc3339();
        let created_at = input.created_at
            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339()))
            .unwrap_or_else(|| now.clone());
        let updated_at = input.updated_at
            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339()))
            .unwrap_or_else(|| now.clone());

        let result = sqlx::query(
            r#"
            INSERT INTO reports (uuid, job_id, title, room_name, notes, status, 
                                 author_device_id, author_name, assigned_to_device_id, assigned_to_name,
                                 cabinet_count, is_complete, has_fillers, has_handles, has_fast_caps,
                                 has_set_boxes, has_caulking, punch_list, synced_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(uuid) DO UPDATE SET
                title = excluded.title,
                room_name = excluded.room_name,
                notes = excluded.notes,
                status = excluded.status,
                assigned_to_device_id = excluded.assigned_to_device_id,
                assigned_to_name = excluded.assigned_to_name,
                cabinet_count = excluded.cabinet_count,
                is_complete = excluded.is_complete,
                has_fillers = excluded.has_fillers,
                has_handles = excluded.has_handles,
                has_fast_caps = excluded.has_fast_caps,
                has_set_boxes = excluded.has_set_boxes,
                has_caulking = excluded.has_caulking,
                punch_list = excluded.punch_list,
                synced_at = excluded.synced_at,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&input.id)
        .bind(&input.job_id)
        .bind(&input.title)
        .bind(&input.room_name)
        .bind(&input.notes)
        .bind(input.status.as_deref().unwrap_or("draft"))
        .bind(&input.author_device_id)
        .bind(&input.author_name)
        .bind(&input.assigned_to_device_id)
        .bind(&input.assigned_to_name)
        .bind(input.cabinet_count.unwrap_or(0))
        .bind(input.is_complete.unwrap_or(false) as i32)
        .bind(input.has_fillers.unwrap_or(false) as i32)
        .bind(input.has_handles.unwrap_or(false) as i32)
        .bind(input.has_fast_caps.unwrap_or(false) as i32)
        .bind(input.has_set_boxes.unwrap_or(false) as i32)
        .bind(input.has_caulking.unwrap_or(false) as i32)
        .bind(&input.punch_list)
        .bind(&now)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get all reports with optional filtering
    pub async fn get_reports(
        &self,
        job_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<ReportRecord>> {
        let mut query = String::from("SELECT * FROM reports WHERE 1=1");
        if job_id.is_some() {
            query.push_str(" AND job_id = ?");
        }
        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        query.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, ReportRecord>(&query);
        if let Some(jid) = job_id {
            q = q.bind(jid);
        }
        if let Some(s) = status {
            q = q.bind(s);
        }
        q = q.bind(limit);

        Ok(q.fetch_all(&self.pool).await?)
    }

    /// Get report count
    pub async fn get_report_count(&self) -> anyhow::Result<i64> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reports")
            .fetch_one(&self.pool)
            .await?;
        Ok(result.0)
    }

    /// Save a report photo (metadata only, file_data stored separately)
    pub async fn upsert_report_photo(
        &self,
        uuid: &str,
        report_id: &str,
        caption: Option<&str>,
        file_name: &str,
        file_data: Option<&[u8]>,
    ) -> anyhow::Result<i64> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO report_photos (uuid, report_id, caption, file_name, file_data, synced_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(uuid) DO UPDATE SET
                caption = excluded.caption,
                synced_at = excluded.synced_at
            "#,
        )
        .bind(uuid)
        .bind(report_id)
        .bind(caption)
        .bind(file_name)
        .bind(file_data)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get photos for a report
    pub async fn get_report_photos(&self, report_id: &str) -> anyhow::Result<Vec<ReportPhotoRecord>> {
        let photos = sqlx::query_as::<_, ReportPhotoRecord>(
            "SELECT id, uuid, report_id, caption, file_name, synced_at, created_at FROM report_photos WHERE report_id = ? ORDER BY created_at ASC"
        )
        .bind(report_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(photos)
    }

    /// Get photo file data by UUID
    pub async fn get_report_photo_data(&self, photo_uuid: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let result: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT file_data FROM report_photos WHERE uuid = ? AND file_data IS NOT NULL"
        )
        .bind(photo_uuid)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.map(|(data,)| data))
    }

    // ─── Location Pings ────────────────────────────────────────

    /// Upsert a location ping
    pub async fn upsert_location_ping(&self, device_id: &str, input: &LocationPingInput) -> anyhow::Result<i64> {
        let now = Utc::now().to_rfc3339();
        let ts = chrono::DateTime::from_timestamp_millis(input.timestamp)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| now.clone());
        let is_moving = input.is_moving.unwrap_or(false) as i32;

        let result = sqlx::query(
            r#"INSERT INTO location_pings (uuid, device_id, time_entry_id, latitude, longitude, accuracy, altitude, speed, heading, timestamp, battery_level, is_moving, synced_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(uuid) DO UPDATE SET
                   latitude = excluded.latitude,
                   longitude = excluded.longitude,
                   accuracy = excluded.accuracy,
                   altitude = excluded.altitude,
                   speed = excluded.speed,
                   heading = excluded.heading,
                   timestamp = excluded.timestamp,
                   battery_level = excluded.battery_level,
                   is_moving = excluded.is_moving,
                   synced_at = excluded.synced_at"#,
        )
        .bind(&input.id)
        .bind(device_id)
        .bind(&input.time_entry_id)
        .bind(input.latitude)
        .bind(input.longitude)
        .bind(input.accuracy)
        .bind(input.altitude)
        .bind(input.speed)
        .bind(input.heading)
        .bind(&ts)
        .bind(input.battery_level)
        .bind(is_moving)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get location pings with filters
    pub async fn get_location_pings(
        &self,
        device_id: Option<&str>,
        time_entry_id: Option<&str>,
        since: Option<i64>,
        active_only: bool,
        limit: i64,
    ) -> anyhow::Result<Vec<LocationPingRecord>> {
        let mut sql = String::from(
            "SELECT id, uuid, device_id, time_entry_id, latitude, longitude, accuracy, altitude, speed, heading, timestamp, battery_level, is_moving, synced_at, created_at FROM location_pings WHERE 1=1"
        );
        if device_id.is_some() {
            sql.push_str(" AND device_id = ?");
        }
        if time_entry_id.is_some() {
            sql.push_str(" AND time_entry_id = ?");
        }
        if since.is_some() {
            sql.push_str(" AND synced_at > ?");
        }
        if active_only {
            sql.push_str(" AND time_entry_id IN (SELECT uuid FROM time_entries WHERE clock_out IS NULL)");
        }
        sql.push_str(" ORDER BY timestamp DESC LIMIT ?");

        let mut query = sqlx::query_as::<_, LocationPingRecord>(&sql);
        if let Some(d) = device_id {
            query = query.bind(d);
        }
        if let Some(t) = time_entry_id {
            query = query.bind(t);
        }
        if let Some(s) = since {
            let ts = chrono::DateTime::from_timestamp_millis(s)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            query = query.bind(ts);
        }
        query = query.bind(limit);

        let pings = query.fetch_all(&self.pool).await?;
        Ok(pings)
    }

    /// Get the latest location ping for each active worker
    pub async fn get_latest_active_positions(&self) -> anyhow::Result<Vec<ActiveWorkerPosition>> {
        // Get latest ping per device for workers currently clocked in
        let rows = sqlx::query_as::<_, (String, Option<String>, Option<String>, f64, f64, Option<f64>, Option<f64>, bool, Option<i32>, String, String, bool)>(
            r#"SELECT
                lp.device_id,
                te.customer_name,
                te.job_name,
                lp.latitude,
                lp.longitude,
                lp.accuracy,
                lp.speed,
                CASE WHEN lp.is_moving THEN 1 ELSE 0 END,
                lp.battery_level,
                lp.timestamp,
                te.clock_in,
                CASE WHEN te.is_break THEN 1 ELSE 0 END
            FROM location_pings lp
            INNER JOIN time_entries te ON lp.time_entry_id = te.uuid
            WHERE te.clock_out IS NULL
            AND lp.id = (
                SELECT lp2.id FROM location_pings lp2
                WHERE lp2.device_id = lp.device_id
                ORDER BY lp2.timestamp DESC LIMIT 1
            )
            ORDER BY lp.timestamp DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        let positions = rows.into_iter().map(|r| ActiveWorkerPosition {
            device_id: r.0,
            customer_name: r.1,
            job_name: r.2,
            latitude: r.3,
            longitude: r.4,
            accuracy: r.5,
            speed: r.6,
            is_moving: r.7,
            battery_level: r.8,
            timestamp: r.9,
            clock_in: r.10,
            is_on_break: r.11,
        }).collect();

        Ok(positions)
    }

    /// Get total location ping count
    pub async fn get_location_ping_count(&self) -> anyhow::Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM location_pings")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    // ==================== TEAM MEMBERS ====================

    /// Upsert a team member (auto-create from device_id if needed)
    pub async fn upsert_team_member(&self, input: &TeamMemberInput) -> anyhow::Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let role = input.role.clone().unwrap_or_else(|| "worker".to_string());
        let is_admin_insert = input.is_admin.unwrap_or(false) as i32;
        let is_admin_update: Option<i32> = input.is_admin.map(|v| v as i32);

        let result = sqlx::query(
            r#"INSERT INTO team_members (device_id, display_name, phone_number, role, is_admin, avatar_color, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(device_id) DO UPDATE SET
                display_name = excluded.display_name,
                phone_number = COALESCE(excluded.phone_number, team_members.phone_number),
                role = excluded.role,
                is_admin = COALESCE(?, team_members.is_admin),
                avatar_color = COALESCE(excluded.avatar_color, team_members.avatar_color),
                updated_at = excluded.updated_at"#,
        )
        .bind(&input.device_id)
        .bind(&input.display_name)
        .bind(&input.phone_number)
        .bind(&role)
        .bind(is_admin_insert)
        .bind(&input.avatar_color)
        .bind(&now)
        .bind(is_admin_update) // 8th param: nullable for ON CONFLICT update
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get a team member by device ID
    pub async fn get_team_member_by_device(&self, device_id: &str) -> anyhow::Result<Option<TeamMember>> {
        let member = sqlx::query_as::<_, TeamMember>("SELECT * FROM team_members WHERE device_id = ?")
            .bind(device_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(member)
    }

    /// Get all team members
    pub async fn get_team_members(&self) -> anyhow::Result<Vec<TeamMember>> {
        let members = sqlx::query_as::<_, TeamMember>("SELECT * FROM team_members ORDER BY display_name")
            .fetch_all(&self.pool)
            .await?;
        Ok(members)
    }

    /// Delete a team member
    pub async fn delete_team_member(&self, device_id: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("DELETE FROM team_members WHERE device_id = ?")
            .bind(device_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update admin status for a team member
    pub async fn set_team_member_admin(&self, device_id: &str, is_admin: bool) -> anyhow::Result<bool> {
        let result = sqlx::query("UPDATE team_members SET is_admin = ? WHERE device_id = ?")
            .bind(is_admin as i32)
            .bind(device_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get team members with their current status (working/break/offline)
    pub async fn get_team_status(&self) -> anyhow::Result<Vec<TeamMemberStatus>> {
        // Get all registered team members
        let members = self.get_team_members().await?;

        // Get all active time entries (work + breaks with no clock_out)
        let active_work = self.get_active_time_entries().await?;
        let active_breaks = self.get_active_breaks().await?;

        // Get today's date for scan counts
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let mut statuses = Vec::new();
        for member in &members {
            // Find active entries for this device
            let on_break = active_breaks.iter().find(|e| e.device_id == member.device_id);
            let working = active_work.iter().find(|e| e.device_id == member.device_id);

            let (status, current_job, clock_in) = if let Some(entry) = on_break {
                ("break".to_string(), entry.job_name.clone().or(entry.customer_name.clone()), Some(entry.clock_in.clone()))
            } else if let Some(entry) = working {
                ("working".to_string(), entry.job_name.clone().or(entry.customer_name.clone()), Some(entry.clock_in.clone()))
            } else {
                ("offline".to_string(), None, None)
            };

            // Get device last_seen
            let last_seen: Option<String> = sqlx::query_scalar(
                "SELECT last_seen_at FROM devices WHERE device_id = ?"
            )
            .bind(&member.device_id)
            .fetch_optional(&self.pool)
            .await?
            .flatten();

            // Get today's scan count for this device
            let (scan_count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM scans WHERE device_id = ? AND scanned_at >= ?"
            )
            .bind(&member.device_id)
            .bind(&format!("{}T00:00:00", today))
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));

            statuses.push(TeamMemberStatus {
                device_id: member.device_id.clone(),
                display_name: member.display_name.clone(),
                phone_number: member.phone_number.clone(),
                role: member.role.clone(),
                is_admin: member.is_admin != 0,
                avatar_color: member.avatar_color.clone(),
                status,
                current_job,
                clock_in,
                last_seen,
                total_scans_today: scan_count,
            });
        }

        Ok(statuses)
    }

    // ==================== TIMESHEETS ====================

    /// Get timesheet data grouped by day for a specific device (or all devices if None)
    pub async fn get_timesheets(
        &self,
        device_id: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<TimesheetDay>> {
        // Fetch time entries with filters
        let entries = if let Some(did) = device_id {
            sqlx::query_as::<_, TimeEntryRecord>(
                "SELECT * FROM time_entries WHERE device_id = ? ORDER BY clock_in DESC LIMIT ?"
            )
            .bind(did)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, TimeEntryRecord>(
                "SELECT * FROM time_entries ORDER BY clock_in DESC LIMIT ?"
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        // Get team member names lookup
        let members = self.get_team_members().await?;
        let name_map: std::collections::HashMap<String, String> = members
            .into_iter()
            .map(|m| (m.device_id, m.display_name))
            .collect();

        // Group by (device_id, date)
        let mut day_map: std::collections::HashMap<(String, String), Vec<TimeEntryRecord>> =
            std::collections::HashMap::new();

        for entry in entries {
            let date = entry.clock_in.split('T').next().unwrap_or(&entry.clock_in).to_string();

            // Apply date filters
            if let Some(from) = date_from {
                if date < from.to_string() { continue; }
            }
            if let Some(to) = date_to {
                if date > to.to_string() { continue; }
            }

            day_map
                .entry((entry.device_id.clone(), date))
                .or_default()
                .push(entry);
        }

        // Build TimesheetDay structs
        let mut result: Vec<TimesheetDay> = Vec::new();
        for ((did, date), day_entries) in &day_map {
            let display_name = name_map.get(did).cloned().unwrap_or_else(|| did.clone());

            let mut total_work: i64 = 0;
            let mut total_break: i64 = 0;

            for entry in day_entries {
                if let Some(ref clock_out) = entry.clock_out {
                    let start = chrono::DateTime::parse_from_rfc3339(&entry.clock_in).ok();
                    let end = chrono::DateTime::parse_from_rfc3339(clock_out).ok();
                    if let (Some(s), Some(e)) = (start, end) {
                        let secs = (e - s).num_seconds().max(0);
                        if entry.is_break != 0 {
                            total_break += secs;
                        } else {
                            total_work += secs;
                        }
                    }
                }
            }

            // Check if any GPS pings exist for this day's entries
            let entry_uuids: Vec<String> = day_entries.iter().map(|e| e.uuid.clone()).collect();
            let has_gps = if !entry_uuids.is_empty() {
                let placeholders: String = entry_uuids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                let query_str = format!("SELECT COUNT(*) FROM location_pings WHERE time_entry_id IN ({})", placeholders);
                let mut query = sqlx::query_scalar::<_, i64>(&query_str);
                for uuid in &entry_uuids {
                    query = query.bind(uuid);
                }
                let count: i64 = query.fetch_one(&self.pool).await.unwrap_or(0);
                count > 0
            } else {
                false
            };

            result.push(TimesheetDay {
                date: date.clone(),
                device_id: did.clone(),
                display_name,
                entries: day_entries.clone(),
                total_work_seconds: total_work,
                total_break_seconds: total_break,
                has_gps,
            });
        }

        // Sort by date descending, then by name
        result.sort_by(|a, b| b.date.cmp(&a.date).then(a.display_name.cmp(&b.display_name)));

        Ok(result)
    }

    // ==================== ROOM PROGRESS ====================

    /// Upsert room progress — updates existing or inserts new
    pub async fn upsert_room_progress(
        &self,
        job_id: &str,
        room_name: &str,
        cabinet_count: i32,
        is_complete: bool,
        has_fillers: bool,
        has_handles: bool,
        has_fast_caps: bool,
        has_set_boxes: bool,
        has_caulking: bool,
        punch_list: Option<&str>,
        notes: Option<&str>,
        report_id: Option<&str>,
        updated_by: Option<&str>,
    ) -> anyhow::Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"INSERT INTO room_progress (job_id, room_name, cabinet_count, is_complete, has_fillers, has_handles, has_fast_caps, has_set_boxes, has_caulking, punch_list, notes, last_report_id, last_updated_by, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(job_id, room_name) DO UPDATE SET
                cabinet_count = excluded.cabinet_count,
                is_complete = excluded.is_complete,
                has_fillers = excluded.has_fillers,
                has_handles = excluded.has_handles,
                has_fast_caps = excluded.has_fast_caps,
                has_set_boxes = excluded.has_set_boxes,
                has_caulking = excluded.has_caulking,
                punch_list = excluded.punch_list,
                notes = excluded.notes,
                last_report_id = excluded.last_report_id,
                last_updated_by = excluded.last_updated_by,
                updated_at = excluded.updated_at"#,
        )
        .bind(job_id)
        .bind(room_name)
        .bind(cabinet_count)
        .bind(is_complete as i32)
        .bind(has_fillers as i32)
        .bind(has_handles as i32)
        .bind(has_fast_caps as i32)
        .bind(has_set_boxes as i32)
        .bind(has_caulking as i32)
        .bind(punch_list)
        .bind(notes)
        .bind(report_id)
        .bind(updated_by)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Update room progress from a report (auto-called when reports are synced)
    pub async fn update_room_progress_from_report(&self, report: &ReportInput) -> anyhow::Result<()> {
        let room_name = match &report.room_name {
            Some(name) if !name.is_empty() => name.clone(),
            _ => return Ok(()), // No room name, skip
        };
        let job_id = match &report.job_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return Ok(()), // No job id, skip
        };

        self.upsert_room_progress(
            &job_id,
            &room_name,
            report.cabinet_count.unwrap_or(0),
            report.is_complete.unwrap_or(false),
            report.has_fillers.unwrap_or(false),
            report.has_handles.unwrap_or(false),
            report.has_fast_caps.unwrap_or(false),
            report.has_set_boxes.unwrap_or(false),
            report.has_caulking.unwrap_or(false),
            report.punch_list.as_deref(),
            report.notes.as_deref(),
            Some(&report.id),
            report.author_name.as_deref(),
        )
        .await?;

        Ok(())
    }

    /// Get all room progress for a job
    pub async fn get_room_progress_for_job(&self, job_id: &str) -> anyhow::Result<Vec<RoomProgress>> {
        let rooms = sqlx::query_as::<_, RoomProgress>(
            "SELECT * FROM room_progress WHERE job_id = ? ORDER BY room_name ASC",
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rooms)
    }

    /// Search rooms by name within a job
    pub async fn search_room_progress(
        &self,
        job_id: &str,
        query: &str,
    ) -> anyhow::Result<Vec<RoomProgress>> {
        let pattern = format!("%{}%", query);
        let rooms = sqlx::query_as::<_, RoomProgress>(
            "SELECT * FROM room_progress WHERE job_id = ? AND room_name LIKE ? ORDER BY room_name ASC",
        )
        .bind(job_id)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(rooms)
    }

    /// Get a single room's progress
    pub async fn get_room_progress(
        &self,
        job_id: &str,
        room_name: &str,
    ) -> anyhow::Result<Option<RoomProgress>> {
        let room = sqlx::query_as::<_, RoomProgress>(
            "SELECT * FROM room_progress WHERE job_id = ? AND room_name = ?",
        )
        .bind(job_id)
        .bind(room_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(room)
    }

    /// Get summary stats for a job's rooms
    pub async fn get_room_progress_summary(&self, job_id: &str) -> anyhow::Result<RoomProgressSummary> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM room_progress WHERE job_id = ?",
        )
        .bind(job_id)
        .fetch_one(&self.pool)
        .await?;

        let complete: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM room_progress WHERE job_id = ? AND is_complete = 1",
        )
        .bind(job_id)
        .fetch_one(&self.pool)
        .await?;

        let total_cabinets: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(cabinet_count), 0) FROM room_progress WHERE job_id = ?",
        )
        .bind(job_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(RoomProgressSummary {
            total_rooms: total.0,
            completed_rooms: complete.0,
            total_cabinets: total_cabinets.0,
        })
    }

    /// Get time rounding config from settings (returns (enabled, interval_minutes))
    async fn get_time_rounding_config(&self) -> (bool, i32) {
        match self.get_setting("time_rounding").await {
            Ok(Some(json_str)) => {
                #[derive(Deserialize)]
                struct Cfg { enabled: bool, interval_minutes: i32 }
                match serde_json::from_str::<Cfg>(&json_str) {
                    Ok(cfg) => (cfg.enabled, cfg.interval_minutes.max(1)),
                    Err(_) => (false, 15),
                }
            }
            _ => (false, 15),
        }
    }
}

/// Round a DateTime to the nearest N minutes
fn round_time(dt: chrono::DateTime<Utc>, interval_minutes: i32) -> chrono::DateTime<Utc> {
    let secs = interval_minutes as i64 * 60;
    let ts = dt.timestamp();
    let rounded = ((ts + secs / 2) / secs) * secs;
    chrono::DateTime::from_timestamp(rounded, 0).unwrap_or(dt)
}

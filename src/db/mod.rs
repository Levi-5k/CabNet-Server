pub mod models;
pub mod repository;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

/// Initialize the database connection pool and run migrations
pub async fn init_pool(database_path: &Path) -> anyhow::Result<SqlitePool> {
    // Ensure the data directory exists
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let database_url = format!("sqlite:{}?mode=rwc", database_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

    Ok(pool)
}

/// Run database migrations to create tables
async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS scans (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            barcode TEXT NOT NULL,
            barcode_type TEXT,
            ticket_number TEXT,
            barcode_job_ref TEXT,
            job_id INTEGER,
            device_id TEXT NOT NULL,
            user_id TEXT,
            location TEXT,
            latitude REAL,
            longitude REAL,
            scanned_at TEXT NOT NULL,
            synced_at TEXT NOT NULL,
            is_printed INTEGER DEFAULT 0,
            notes TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            
            FOREIGN KEY (job_id) REFERENCES jobs(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            reference_number TEXT,
            customer_name TEXT,
            expected_count INTEGER DEFAULT 0,
            status TEXT DEFAULT 'PENDING',
            device_id TEXT,
            created_by TEXT,
            notes TEXT,
            priority TEXT DEFAULT 'NORMAL',
            due_date TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            started_at TEXT,
            completed_at TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS devices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT UNIQUE NOT NULL,
            device_name TEXT,
            model TEXT,
            os_version TEXT,
            app_version TEXT,
            last_seen_at TEXT,
            registered_at TEXT DEFAULT CURRENT_TIMESTAMP,
            is_active INTEGER DEFAULT 1,
            -- Security fields
            registration_code TEXT, -- Admin-generated code for device approval
            is_approved INTEGER DEFAULT 0, -- Requires admin approval
            auth_token TEXT, -- Device authentication token
            auth_token_expires_at TEXT, -- Token expiration
            fingerprint_hash TEXT, -- Device fingerprint for impersonation detection
            registration_attempts INTEGER DEFAULT 0, -- Rate limiting
            last_registration_attempt_at TEXT, -- Rate limiting
            blocked_until TEXT, -- Temporary blocking for abuse
            ip_address TEXT, -- Registration IP for security tracking
            user_agent TEXT, -- Device user agent
            -- Enhanced status tracking
            status TEXT DEFAULT 'pending_approval',
            last_heartbeat_at TEXT,
            connection_count INTEGER DEFAULT 0,
            total_uptime_seconds INTEGER DEFAULT 0,
            error_count INTEGER DEFAULT 0,
            last_error_at TEXT,
            last_error_message TEXT,
            -- Latest heartbeat data
            battery_level INTEGER,
            is_charging INTEGER,
            available_memory_mb INTEGER,
            total_memory_mb INTEGER,
            network_type TEXT,
            connection_quality TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS email_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            smtp_host TEXT,
            smtp_port INTEGER DEFAULT 587,
            smtp_username TEXT,
            smtp_password TEXT,
            email_from TEXT,
            email_to TEXT,
            email_subject_template TEXT DEFAULT 'Cabinet Scan Report - {date}',
            auto_send_enabled INTEGER DEFAULT 0,
            auto_send_delay_minutes INTEGER DEFAULT 30,
            last_auto_send_at TEXT,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS email_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sent_at TEXT NOT NULL,
            recipients TEXT NOT NULL,
            subject TEXT NOT NULL,
            scan_count INTEGER,
            status TEXT NOT NULL,
            error_message TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Web clients table (for dashboard trust system)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS web_clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_id TEXT UNIQUE NOT NULL,
            client_name TEXT,
            user_agent TEXT,
            ip_address TEXT,
            is_trusted INTEGER DEFAULT 0,
            last_seen_at TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Pending changes table (for approval workflow)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS pending_changes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_id TEXT NOT NULL,
            change_type TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            change_data TEXT NOT NULL,
            status TEXT DEFAULT 'pending',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            reviewed_at TEXT,
            reviewed_by TEXT,
            
            FOREIGN KEY (client_id) REFERENCES web_clients(client_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_scans_job_id ON scans(job_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_scans_device_id ON scans(device_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_scans_scanned_at ON scans(scanned_at)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_scans_barcode ON scans(barcode)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_devices_device_id ON devices(device_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pending_changes_status ON pending_changes(status)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_web_clients_client_id ON web_clients(client_id)")
        .execute(pool)
        .await?;

    // Migration: Add local_id column to scans table for Android sync tracking
    // This column stores the local Room database ID from the Android device
    let _ = sqlx::query("ALTER TABLE scans ADD COLUMN local_id INTEGER")
        .execute(pool)
        .await;

    // Create index on local_id for faster sync lookups
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_scans_local_id ON scans(local_id)")
        .execute(pool)
        .await?;

    // Unique index on (local_id, device_id) to prevent duplicate scans from retried syncs
    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_scans_local_device ON scans(local_id, device_id) WHERE local_id IS NOT NULL")
        .execute(pool)
        .await
        .ok(); // ok() because it may fail if duplicates already exist — we clean those up next

    // Clean up existing duplicate scans: keep only the newest row per (local_id, device_id)
    let cleaned = sqlx::query(
        r#"DELETE FROM scans WHERE id NOT IN (
            SELECT MAX(id) FROM scans WHERE local_id IS NOT NULL GROUP BY local_id, device_id
        ) AND local_id IS NOT NULL"#
    )
    .execute(pool)
    .await;
    if let Ok(result) = &cleaned {
        if result.rows_affected() > 0 {
            tracing::info!("Cleaned up {} duplicate scans", result.rows_affected());
        }
    }

    // Now create the unique index if it didn't exist yet (after cleanup)
    let _ = sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_scans_local_device ON scans(local_id, device_id) WHERE local_id IS NOT NULL")
        .execute(pool)
        .await;

    // Insert default email config if not exists
    sqlx::query("INSERT OR IGNORE INTO email_config (id) VALUES (1)")
        .execute(pool)
        .await?;

    // Insert default settings
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO settings (key, value) VALUES 
        ('server_port', '8080'),
        ('server_host', '0.0.0.0'),
        ('auto_backup_enabled', 'true')
        "#,
    )
    .execute(pool)
    .await?;

    // Migration: Add security fields to devices table
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN registration_code TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN is_approved INTEGER DEFAULT 0")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN auth_token TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN auth_token_expires_at TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN fingerprint_hash TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN registration_attempts INTEGER DEFAULT 0")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN last_registration_attempt_at TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN blocked_until TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN ip_address TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN user_agent TEXT")
        .execute(pool).await;

    // Migration: Add enhanced device status fields
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN status TEXT DEFAULT 'offline'")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN last_heartbeat_at TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN connection_count INTEGER DEFAULT 0")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN total_uptime_seconds INTEGER DEFAULT 0")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN error_count INTEGER DEFAULT 0")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN last_error_at TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN last_error_message TEXT")
        .execute(pool).await;

    // Migration: Add heartbeat data columns
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN battery_level INTEGER")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN is_charging INTEGER")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN available_memory_mb INTEGER")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN total_memory_mb INTEGER")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN network_type TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE devices ADD COLUMN connection_quality TEXT")
        .execute(pool).await;

    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_devices_last_heartbeat ON devices(last_heartbeat_at)")
        .execute(pool).await;

    // Time entries table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS time_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            device_id TEXT NOT NULL,
            customer_name TEXT,
            job_name TEXT,
            job_id TEXT,
            clock_in TEXT NOT NULL,
            clock_out TEXT,
            note TEXT,
            is_break INTEGER DEFAULT 0,
            is_paid INTEGER DEFAULT 1,
            synced_at TEXT DEFAULT CURRENT_TIMESTAMP,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_time_entries_device ON time_entries(device_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_time_entries_clock_in ON time_entries(clock_in)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_time_entries_job_id ON time_entries(job_id)")
        .execute(pool)
        .await?;

    // Reports table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            job_id TEXT,
            title TEXT NOT NULL,
            room_name TEXT,
            notes TEXT,
            status TEXT DEFAULT 'draft',
            author_device_id TEXT,
            author_name TEXT,
            assigned_to_device_id TEXT,
            assigned_to_name TEXT,
            cabinet_count INTEGER DEFAULT 0,
            is_complete INTEGER DEFAULT 0,
            has_fillers INTEGER DEFAULT 0,
            has_handles INTEGER DEFAULT 0,
            has_fast_caps INTEGER DEFAULT 0,
            has_set_boxes INTEGER DEFAULT 0,
            has_caulking INTEGER DEFAULT 0,
            punch_list TEXT,
            synced_at TEXT DEFAULT CURRENT_TIMESTAMP,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_reports_job_id ON reports(job_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_reports_created_at ON reports(created_at)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_reports_status ON reports(status)")
        .execute(pool)
        .await?;

    // Report photos table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS report_photos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            report_id TEXT NOT NULL,
            caption TEXT,
            file_name TEXT NOT NULL,
            file_data BLOB,
            synced_at TEXT DEFAULT CURRENT_TIMESTAMP,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_report_photos_report_id ON report_photos(report_id)")
        .execute(pool)
        .await?;

    // Location pings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS location_pings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            device_id TEXT NOT NULL,
            time_entry_id TEXT NOT NULL,
            latitude REAL NOT NULL,
            longitude REAL NOT NULL,
            accuracy REAL,
            altitude REAL,
            speed REAL,
            heading REAL,
            timestamp TEXT NOT NULL,
            battery_level INTEGER,
            is_moving INTEGER DEFAULT 0,
            synced_at TEXT DEFAULT CURRENT_TIMESTAMP,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_location_pings_device_id ON location_pings(device_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_location_pings_time_entry_id ON location_pings(time_entry_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_location_pings_timestamp ON location_pings(timestamp)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_location_pings_entry_time ON location_pings(time_entry_id, timestamp)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_location_pings_device_time_desc ON location_pings(device_id, timestamp DESC)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_location_pings_entry_time_desc ON location_pings(time_entry_id, timestamp DESC)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_time_entries_active_device ON time_entries(device_id, clock_out, clock_in DESC)")
        .execute(pool)
        .await?;

    // Team members table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS team_members (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT UNIQUE NOT NULL,
            display_name TEXT NOT NULL,
            phone_number TEXT,
            role TEXT DEFAULT 'worker',
            is_admin INTEGER DEFAULT 0,
            avatar_color TEXT,
            active_hours_enabled INTEGER DEFAULT 1,
            active_hours_start_minutes INTEGER DEFAULT 300,
            active_hours_end_minutes INTEGER DEFAULT 1020,
            active_hours_utc_offset_minutes INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Add phone_number column if it doesn't exist (migration for existing DBs)
    let _ = sqlx::query("ALTER TABLE team_members ADD COLUMN phone_number TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE team_members ADD COLUMN active_hours_enabled INTEGER DEFAULT 1")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE team_members ADD COLUMN active_hours_start_minutes INTEGER DEFAULT 300")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE team_members ADD COLUMN active_hours_end_minutes INTEGER DEFAULT 1020")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE team_members ADD COLUMN active_hours_utc_offset_minutes INTEGER DEFAULT 0")
        .execute(pool)
        .await;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_team_members_device_id ON team_members(device_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_team_members_role ON team_members(role)")
        .execute(pool)
        .await?;

    // Deleted app accounts are kept as tombstones so reconnect cannot recreate access.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS deleted_device_accounts (
            device_id TEXT PRIMARY KEY NOT NULL,
            deleted_at TEXT NOT NULL,
            reason TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Team messaging threads
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS message_threads (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            kind TEXT NOT NULL,
            title TEXT,
            job_id TEXT,
            job_name TEXT,
            created_by_device_id TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            last_message_at TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_threads_kind ON message_threads(kind)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_threads_job_id ON message_threads(job_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_threads_updated_at ON message_threads(updated_at)")
        .execute(pool)
        .await?;

    // Team messaging membership and read state
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS message_thread_members (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            thread_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            display_name TEXT,
            joined_at TEXT DEFAULT CURRENT_TIMESTAMP,
            last_read_message_id TEXT,
            last_read_at TEXT,
            is_muted INTEGER DEFAULT 0,
            UNIQUE(thread_id, device_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_thread_members_device ON message_thread_members(device_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_thread_members_thread ON message_thread_members(thread_id)")
        .execute(pool)
        .await?;

    // Team messages
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            thread_id TEXT NOT NULL,
            sender_device_id TEXT NOT NULL,
            sender_name TEXT,
            body TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT,
            deleted_at TEXT,
            delivery_status TEXT DEFAULT 'delivered'
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_thread_created ON messages(thread_id, created_at DESC)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender_device_id)")
        .execute(pool)
        .await?;

    // Per-recipient unread tracking for messages
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS message_unread_status (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id TEXT NOT NULL,
            thread_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            is_read INTEGER DEFAULT 0,
            read_at TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(message_id, device_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_unread_device_thread ON message_unread_status(device_id, thread_id, is_read)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_unread_thread ON message_unread_status(thread_id)")
        .execute(pool)
        .await?;

    // Job-specific message channels
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS job_channels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            job_id TEXT NOT NULL,
            channel_name TEXT NOT NULL,
            thread_id TEXT NOT NULL,
            created_by_device_id TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(job_id, channel_name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_job_channels_job_id ON job_channels(job_id)")
        .execute(pool)
        .await?;

    // APNs tokens and delivery log. Push can be disabled by config while unread polling still works.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS push_tokens (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            platform TEXT DEFAULT 'apns',
            environment TEXT DEFAULT 'production',
            bundle_id TEXT,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            disabled_at TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_push_tokens_device ON push_tokens(device_id)")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS push_delivery_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id TEXT,
            thread_id TEXT,
            device_id TEXT NOT NULL,
            token_id INTEGER,
            status TEXT NOT NULL,
            error_message TEXT,
            attempted_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_push_delivery_device ON push_delivery_log(device_id, attempted_at DESC)")
        .execute(pool)
        .await?;

    // Room progress tracking table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS room_progress (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL,
            room_name TEXT NOT NULL,
            cabinet_count INTEGER DEFAULT 0,
            is_complete INTEGER DEFAULT 0,
            has_fillers INTEGER DEFAULT 0,
            has_handles INTEGER DEFAULT 0,
            has_fast_caps INTEGER DEFAULT 0,
            has_set_boxes INTEGER DEFAULT 0,
            has_caulking INTEGER DEFAULT 0,
            punch_list TEXT,
            notes TEXT,
            last_report_id TEXT,
            last_updated_by TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(job_id, room_name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_room_progress_job_id ON room_progress(job_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_room_progress_room_name ON room_progress(room_name)")
        .execute(pool)
        .await?;

    // Job files table (PDFs / documents attached to jobs)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS job_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            job_id TEXT NOT NULL,
            file_name TEXT NOT NULL,
            content_type TEXT DEFAULT 'application/pdf',
            file_data BLOB,
            extracted_text TEXT,
            file_size INTEGER DEFAULT 0,
            uploaded_by_device_id TEXT,
            uploaded_by_name TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_job_files_job_id ON job_files(job_id)")
        .execute(pool)
        .await?;

    // Lading tickets table (parsed from shipping/packing PDFs)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS lading_tickets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL,
            job_file_uuid TEXT NOT NULL,
            ticket_number TEXT NOT NULL,
            description TEXT,
            room TEXT,
            qty INTEGER DEFAULT 1,
            section TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_lading_tickets_job_id ON lading_tickets(job_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_lading_tickets_ticket ON lading_tickets(ticket_number)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_lading_tickets_file ON lading_tickets(job_file_uuid)")
        .execute(pool)
        .await?;

    tracing::info!("Database migrations completed");

    // Migration: Add address column to jobs table
    let _ = sqlx::query("ALTER TABLE jobs ADD COLUMN address TEXT")
        .execute(pool)
        .await;

    // Migration: Add user_id column to web_clients for linking to team members
    let _ = sqlx::query("ALTER TABLE web_clients ADD COLUMN user_id TEXT")
        .execute(pool)
        .await;

    // User backgrounds table (background images uploaded from iOS app)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_backgrounds (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT UNIQUE NOT NULL,
            image_data BLOB,
            content_type TEXT DEFAULT 'image/jpeg',
            background_type TEXT DEFAULT 'image',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Migration: Add background_type column
    let _ = sqlx::query("ALTER TABLE user_backgrounds ADD COLUMN background_type TEXT DEFAULT 'image'")
        .execute(pool)
        .await;

    // Migration: Allow NULL image_data for shader-only entries
    // SQLite doesn't support ALTER COLUMN, but new inserts can have NULL image_data

    Ok(())
}

# Database Schema

This document describes the SQLite database schema for CodeBar Server.

## Tables

### scans

Stores all barcode scans received from Android devices.

```sql
CREATE TABLE scans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    barcode TEXT NOT NULL,
    job_id INTEGER,
    timestamp TEXT NOT NULL,           -- ISO 8601 format
    latitude REAL,
    longitude REAL,
    device_id TEXT NOT NULL,
    synced_at TEXT NOT NULL,           -- When received by server
    notes TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (job_id) REFERENCES jobs(id)
);

-- Indexes for common queries
CREATE INDEX idx_scans_job_id ON scans(job_id);
CREATE INDEX idx_scans_device_id ON scans(device_id);
CREATE INDEX idx_scans_timestamp ON scans(timestamp);
CREATE INDEX idx_scans_barcode ON scans(barcode);
```

### jobs

Stores job definitions.

```sql
CREATE TABLE jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    job_reference TEXT UNIQUE,         -- External reference ID
    status TEXT DEFAULT 'active',      -- active, completed, cancelled
    expected_count INTEGER,            -- Expected number of scans
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    completed_at TEXT
);

CREATE INDEX idx_jobs_status ON jobs(status);
CREATE INDEX idx_jobs_job_reference ON jobs(job_reference);
```

### devices

Tracks connected Android devices.

```sql
CREATE TABLE devices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT UNIQUE NOT NULL,    -- Android device identifier
    device_name TEXT,                  -- User-friendly name
    model TEXT,                        -- Device model
    android_version TEXT,              -- Android OS version
    app_version TEXT,                  -- CodeBar app version
    last_seen_at TEXT,                 -- Last communication time
    registered_at TEXT DEFAULT CURRENT_TIMESTAMP,
    is_active INTEGER DEFAULT 1
);

CREATE INDEX idx_devices_device_id ON devices(device_id);
CREATE INDEX idx_devices_last_seen_at ON devices(last_seen_at);
```

### email_config

Stores email/SMTP configuration.

```sql
CREATE TABLE email_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Singleton row
    smtp_host TEXT,
    smtp_port INTEGER DEFAULT 587,
    smtp_username TEXT,
    smtp_password TEXT,                -- Should be encrypted
    email_from TEXT,
    email_to TEXT,                     -- Comma-separated list
    email_subject_template TEXT DEFAULT 'CodeBar Scan Report - {date}',
    auto_send_enabled INTEGER DEFAULT 0,
    auto_send_delay_minutes INTEGER DEFAULT 30,  -- Send after X minutes of inactivity
    last_auto_send_at TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### email_history

Tracks sent emails.

```sql
CREATE TABLE email_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sent_at TEXT NOT NULL,
    recipients TEXT NOT NULL,
    subject TEXT NOT NULL,
    scan_count INTEGER,
    status TEXT NOT NULL,              -- sent, failed
    error_message TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_email_history_sent_at ON email_history(sent_at);
```

### settings

General application settings (key-value store).

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

## Initial Data

```sql
-- Default settings
INSERT INTO settings (key, value) VALUES ('server_port', '8080');
INSERT INTO settings (key, value) VALUES ('server_host', '0.0.0.0');
INSERT INTO settings (key, value) VALUES ('database_path', './data/codebar.db');
INSERT INTO settings (key, value) VALUES ('auto_backup_enabled', 'true');
INSERT INTO settings (key, value) VALUES ('auto_backup_interval_hours', '24');

-- Default email config (empty)
INSERT INTO email_config (id) VALUES (1);
```

## Queries

### Common Queries

```sql
-- Get all scans for a job
SELECT * FROM scans WHERE job_id = ? ORDER BY timestamp DESC;

-- Get scan count per job
SELECT j.id, j.name, COUNT(s.id) as scan_count, j.expected_count
FROM jobs j
LEFT JOIN scans s ON j.id = s.job_id
GROUP BY j.id;

-- Get recent scans
SELECT s.*, j.name as job_name
FROM scans s
LEFT JOIN jobs j ON s.job_id = j.id
ORDER BY s.synced_at DESC
LIMIT 100;

-- Get active devices (seen in last 24 hours)
SELECT * FROM devices
WHERE last_seen_at > datetime('now', '-24 hours')
AND is_active = 1;

-- Get scans with GPS coordinates (for map)
SELECT id, barcode, latitude, longitude, timestamp, device_id
FROM scans
WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

-- Search scans by barcode
SELECT s.*, j.name as job_name
FROM scans s
LEFT JOIN jobs j ON s.job_id = j.id
WHERE s.barcode LIKE '%' || ? || '%'
ORDER BY s.timestamp DESC;
```

### Statistics Queries

```sql
-- Scans per day (last 30 days)
SELECT DATE(timestamp) as date, COUNT(*) as count
FROM scans
WHERE timestamp > datetime('now', '-30 days')
GROUP BY DATE(timestamp)
ORDER BY date;

-- Scans per device
SELECT d.device_id, d.device_name, COUNT(s.id) as scan_count
FROM devices d
LEFT JOIN scans s ON d.device_id = s.device_id
GROUP BY d.device_id;

-- Job completion percentage
SELECT 
    j.id,
    j.name,
    COUNT(s.id) as actual_count,
    j.expected_count,
    CASE 
        WHEN j.expected_count > 0 
        THEN ROUND(COUNT(s.id) * 100.0 / j.expected_count, 1)
        ELSE NULL 
    END as completion_percent
FROM jobs j
LEFT JOIN scans s ON j.id = s.job_id
WHERE j.status = 'active'
GROUP BY j.id;
```

## Migrations

SQLx handles migrations automatically. Place migration files in `migrations/` folder:

```
migrations/
├── 001_initial.sql
├── 002_add_email_history.sql
└── ...
```

Each migration file should be idempotent and include both the schema changes.

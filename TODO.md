# Development TODO

Implementation checklist for CodeBar Server.

## Phase 1: Core Infrastructure ✅

- [ ] Set up Cargo.toml with dependencies
  - axum 0.7 (HTTP server)
  - sqlx 0.7 with sqlite feature (database)
  - tokio (async runtime)
  - serde + serde_json (serialization)
  - eframe + egui 0.28 (GUI)
  - lettre (email)
  - chrono (datetime)
  - tracing + tracing-subscriber (logging)
  - anyhow + thiserror (error handling)
  - dotenvy (env config)

- [ ] Create project structure:
  ```
  src/
  ├── main.rs           # Entry point, starts both server and GUI
  ├── lib.rs            # Re-exports
  ├── config.rs         # Configuration loading
  ├── db/
  │   ├── mod.rs
  │   ├── schema.rs     # SQLx queries and models
  │   └── migrations.rs # Database migrations
  ├── api/
  │   ├── mod.rs
  │   ├── routes.rs     # Axum routes
  │   ├── handlers.rs   # Request handlers
  │   └── models.rs     # API request/response types
  ├── gui/
  │   ├── mod.rs
  │   ├── app.rs        # Main eframe App
  │   ├── tabs/
  │   │   ├── mod.rs
  │   │   ├── map.rs
  │   │   ├── jobs.rs
  │   │   ├── scans.rs
  │   │   ├── devices.rs
  │   │   ├── settings.rs
  │   │   └── email.rs
  │   └── widgets/      # Reusable UI components
  └── services/
      ├── mod.rs
      └── email.rs      # Email sending logic
  ```

## Phase 2: Database Layer

- [ ] Create SQLite database file
- [ ] Implement migrations
- [ ] Create Rust models:
  - Scan
  - Job
  - Device
  - EmailConfig
  - EmailHistory
  - Setting
- [ ] Implement CRUD operations for each model
- [ ] Add connection pool (SQLx Pool)

## Phase 3: HTTP API

- [ ] Set up Axum router
- [ ] Implement endpoints:
  - [ ] GET /api/health
  - [ ] POST /api/scans (batch upload)
  - [ ] POST /api/scans/verify (verify upload)
  - [ ] GET /api/scans (list with pagination)
  - [ ] POST /api/jobs
  - [ ] GET /api/jobs
  - [ ] PUT /api/jobs/{id}
  - [ ] DELETE /api/jobs/{id}
  - [ ] POST /api/devices/register
  - [ ] GET /api/devices
- [ ] Add JSON error handling
- [ ] Add request logging
- [ ] Add CORS support

## Phase 4: GUI - Basic Framework

- [ ] Set up eframe application
- [ ] Create tab navigation
- [ ] Implement shared state between tabs
- [ ] Add status bar with server status
- [ ] Style with custom theme

## Phase 5: GUI - Tabs

### Map Tab
- [ ] Integrate map widget (egui-map or custom)
- [ ] Display scan markers with GPS coordinates
- [ ] Marker click shows scan details
- [ ] Filter controls (job, device, date range)
- [ ] Zoom and pan controls

### Jobs Tab
- [ ] List view with all jobs
- [ ] Create job form
- [ ] Edit job dialog
- [ ] Delete confirmation
- [ ] Job progress indicator
- [ ] Status badges (active/completed/cancelled)

### Scans Tab
- [ ] Paginated scan list
- [ ] Search by barcode
- [ ] Filter by job, device, date
- [ ] Sort by timestamp
- [ ] Export to CSV button
- [ ] Real-time updates

### Devices Tab
- [ ] List connected devices
- [ ] Device status indicator (online/offline)
- [ ] Last seen time
- [ ] Per-device scan count
- [ ] Edit device name

### Settings Tab
- [ ] Server settings (port, host)
- [ ] Database path
- [ ] Auto-backup toggle
- [ ] Theme selection (light/dark)
- [ ] Save/Apply buttons

### Email Tab
- [ ] SMTP configuration form
- [ ] Test connection button
- [ ] Email template editor
- [ ] Auto-send toggle and delay setting
- [ ] Recipients list management
- [ ] Send history table
- [ ] Manual send button

## Phase 6: Email Integration

- [ ] SMTP connection with lettre
- [ ] Email template rendering
- [ ] Attach CSV report
- [ ] Auto-send timer (trigger after X minutes of no new scans)
- [ ] Send history logging
- [ ] Error handling and retry logic

## Phase 7: Real-time Updates

- [ ] Shared state between API and GUI threads
- [ ] GUI polls for updates or uses channels
- [ ] Toast notifications for new scans
- [ ] Sound alerts (optional)

## Phase 8: Polish

- [ ] Error handling throughout
- [ ] Loading indicators
- [ ] Empty state messages
- [ ] Keyboard shortcuts
- [ ] Window size/position persistence
- [ ] System tray icon (optional)
- [ ] Installer/distribution

## Dependencies (Cargo.toml)

```toml
[package]
name = "codebar-server"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# HTTP server
axum = { version = "0.7", features = ["json"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# GUI
eframe = "0.28"
egui = "0.28"
egui_extras = { version = "0.28", features = ["image"] }

# Email
lettre = { version = "0.11", features = ["tokio1-native-tls"] }

# Utilities
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1.0"
thiserror = "1.0"
dotenvy = "0.15"
uuid = { version = "1.6", features = ["v4", "serde"] }

# CSV export
csv = "1.3"
```

## Phase 9: Device Syncing Improvements

### Enhanced Device Status Management ✅
- [x] Add device status indicators: "Online", "Offline", "Inactive"
- [x] Track device connection history and uptime statistics
- [x] Add device health dashboard with connection status, last sync times, error rates

### Improved Heartbeat System ✅
- [x] Implement adaptive heartbeat intervals (start at 90s, increase to 5-15 minutes for stable connections)
- [x] Add smart heartbeat logic: skip heartbeat if recent sync occurred
- [x] Include device status, battery level, pending sync count in heartbeat payload
- [x] Add battery optimization: reduce frequency when device is on battery/standby

### Enhanced Sync Efficiency ✅
- [x] Implement delta syncing: server tracks what client has vs what server has
- [x] Enable gzip compression for large sync payloads
- [x] Add pagination for very large datasets with cursor-based pagination
- [x] Add sync priorities: sync critical data (jobs) before bulk data (scans)

### Better Error Handling & Recovery ✅
- [x] Implement exponential backoff for failed syncs with jitter to prevent thundering herd
- [x] Add partial sync recovery: retry individual items if batch fails
- [x] Add sync checkpoints: save progress so failed syncs can resume
- [x] Implement conflict resolution for concurrent edits from multiple devices

### Device Authentication & Security
- [ ] Add device registration codes: require admin approval for new devices
- [ ] Implement device certificates/keys for authentication
- [ ] Add device fingerprinting: track device characteristics to detect impersonation
- [ ] Implement rate limiting: prevent brute force device registration attempts

### Connection State Management
- [ ] Add persistent sessions: maintain server-side session state
- [ ] Implement graceful reconnection: resume sync from last checkpoint on reconnection
- [ ] Add connection quality monitoring: track latency, packet loss, connection stability

### Data Consistency & Validation
- [ ] Add checksums: validate data integrity during sync
- [ ] Implement versioning: handle schema changes gracefully
- [ ] Improve duplicate detection: better deduplication logic for scans
- [ ] Add data validation: server-side validation of incoming data

### Monitoring & Diagnostics
- [ ] Add sync metrics: track sync duration, success rates, data volumes
- [ ] Implement debug logging: enhanced logging for troubleshooting sync issues
- [ ] Add alerting: notify admins of sync failures or device issues
- [ ] Create sync status indicators: show what data is synced/pending

### Battery & Performance Optimization
- [ ] Implement adaptive sync frequency: based on device activity and battery status
- [ ] Improve background sync: use Android's JobScheduler/WorkManager more effectively
- [ ] Add data compression: reduce network usage
- [ ] Implement selective syncing: only sync relevant data for each device

### Memory Management & Manual Cleanup
- [ ] Add device memory monitoring: devices notify server when storage is almost full
- [ ] Add server-side device cleanup button: manual cleanup triggered by admin
- [ ] Implement cleanup policies: delete old scans, archive completed jobs, etc.
- [ ] Add cleanup confirmation dialogs with data size estimates

### Offline Support
- [ ] Implement offline queue: queue operations when offline, sync when reconnected
- [ ] Add conflict resolution: handle edits made while offline
- [ ] Implement local caching: cache server data for offline access
- [ ] Add sync status indicators: show what data is synced/pending

## Notes

- The GUI and HTTP server run in the same process but different threads
- Use `Arc<Mutex<AppState>>` or channels for shared state
- SQLx queries should be async
- GUI updates should be smooth (don't block on database)
- Test with the CodeBar Android app for real-world validation

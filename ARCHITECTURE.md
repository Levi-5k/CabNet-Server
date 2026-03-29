# CodeBar Server Architecture

## Overview

CodeBar Server is a Rust desktop application that serves as the control center for the CodeBar Android barcode scanning system. It provides:

1. **HTTP API Server** - Receives scans from Android devices via Axum
2. **Desktop GUI** - Native UI using egui/eframe with tabbed interface
3. **SQLite Database** - Local storage for scans, jobs, devices, and settings
4. **Email Integration** - SMTP support for auto-sending scan reports

## Project Structure

```
codebar-server/
├── Cargo.toml              # Dependencies and project config
├── src/
│   ├── main.rs             # Application entry point
│   ├── app.rs              # Main application state and GUI
│   ├── api/                # HTTP API handlers
│   │   ├── mod.rs
│   │   ├── scans.rs        # Scan endpoints
│   │   ├── jobs.rs         # Job endpoints
│   │   └── health.rs       # Health check
│   ├── db/                 # Database layer
│   │   ├── mod.rs
│   │   ├── models.rs       # Data structures
│   │   ├── schema.sql      # SQLite schema
│   │   └── repository.rs   # Database operations
│   ├── ui/                 # GUI components
│   │   ├── mod.rs
│   │   ├── tabs/
│   │   │   ├── map.rs      # Map view with scan locations
│   │   │   ├── jobs.rs     # Job management
│   │   │   ├── scans.rs    # Scan list and details
│   │   │   ├── devices.rs  # Connected devices
│   │   │   ├── settings.rs # App configuration
│   │   │   └── email.rs    # Email settings and sending
│   │   └── components/     # Reusable UI components
│   ├── email/              # Email/SMTP functionality
│   │   ├── mod.rs
│   │   └── sender.rs
│   └── config.rs           # Configuration management
├── data/                   # Runtime data (created automatically)
│   └── codebar.db          # SQLite database
└── .env                    # Environment configuration
```

## Technology Stack

- **Rust 1.75+** - Core language
- **Axum 0.7** - HTTP web framework
- **SQLx 0.7** - Async SQLite with compile-time checked queries
- **eframe/egui 0.28** - Immediate mode GUI
- **Tokio** - Async runtime
- **lettre** - Email/SMTP client
- **Serde** - JSON serialization (snake_case for Android compatibility)

## Data Flow

```
Android Device                    CodeBar Server
     │                                  │
     │  POST /api/scans                 │
     │ ─────────────────────────────────>│
     │                                  │ Store in SQLite
     │                                  │ Update GUI in real-time
     │  { success: true, synced_ids }   │
     │ <─────────────────────────────────│
     │                                  │
     │  POST /api/scans/verify          │
     │ ─────────────────────────────────>│
     │                                  │ Verify stored scans
     │  { verified: [...], missing: [] }│
     │ <─────────────────────────────────│
```

## GUI Tabs

1. **Map Tab** - Shows all scan locations on an OpenStreetMap-based map
2. **Jobs Tab** - Create, edit, delete jobs; assign to devices
3. **Scans Tab** - View all scans with filtering and search
4. **Devices Tab** - Monitor connected devices, last seen, sync status
5. **Settings Tab** - Server port, database path, auto-sync options
6. **Email Tab** - SMTP configuration, auto-send rules, send history

## Threading Model

- Main thread: GUI rendering (egui)
- Tokio runtime: HTTP server and async operations
- Background tasks: Email sending, periodic sync checks

## Web Pages

The server hosts web pages accessible via browser:

### Ticket Search (`/` on scans subdomain)
Public-facing search interface at `scans.cabnetx.com`:
- **Job Dropdown** - Select from all available jobs
- **Search Input** - Enter ticket number to search
- **Live Search** - Results filter as you type
- **Results List** - Shows matching tickets with barcode, timestamp, and status

### Dashboard / Control Center (`/dashboard` or `/` on main domain)
Full-featured control center at `cabnetx.com` or local access:
- **Stats Cards** - Device count, scan count, job count, uptime
- **Jobs Management** - Create, edit, delete jobs
- **Devices View** - Monitor and manage connected devices
- **Scans Browser** - View all scans with filtering
- **Trust System** - Changes from untrusted clients are queued for approval

### Host-Based Routing
The server uses host-based routing to serve different pages:

| Host | Page |
|------|------|
| `cabnetx.com` | Dashboard |
| `scans.cabnetx.com` | Ticket Search |
| `localhost:8080` | Dashboard |
| `192.168.x.x:8080` | Dashboard |

Logic in `src/api/web.rs`:
- Subdomains (e.g., `scans.`) → Dashboard
- Local IPs and localhost → Dashboard  
- Main domain → Ticket Search

## Web Client Trust System

The dashboard implements a trust-based approval workflow for remote management:

### How It Works

1. **Web Client Registration**: When a browser accesses the dashboard, it registers as a web client and receives a unique `client_id` stored in localStorage.

2. **Trust Levels**:
   - **Untrusted** (default): All changes (create, update, delete) are queued as "pending changes"
   - **Trusted**: Changes are applied immediately without approval

3. **Approval Workflow**:
   - Untrusted clients see "Changes need approval" badge in dashboard
   - Changes are stored in `pending_changes` table
   - Desktop app's "Approvals" tab shows pending changes
   - Admin can approve (applies change) or reject (discards change)

4. **Trusting a Device**:
   - In desktop app → Approvals → Web Clients tab
   - Click "Trust Device" to allow direct changes
   - Click "Revoke Trust" to require approval again

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/web/register` | POST | Register new web client |
| `/api/web/status` | GET | Check client trust status |
| `/api/web/clients` | GET | List all web clients |
| `/api/web/clients/:id/trust` | PUT | Set trust status |
| `/api/web/changes` | POST | Submit a change |
| `/api/web/changes/pending` | GET | Get pending changes |
| `/api/web/changes/:id/approve` | POST | Approve a change |
| `/api/web/changes/:id/reject` | POST | Reject a change |

### Database Tables

```sql
-- Web clients (browser sessions)
CREATE TABLE web_clients (
    id INTEGER PRIMARY KEY,
    client_id TEXT UNIQUE NOT NULL,
    client_name TEXT,
    user_agent TEXT,
    ip_address TEXT,
    is_trusted INTEGER DEFAULT 0,
    last_seen_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Pending changes awaiting approval
CREATE TABLE pending_changes (
    id INTEGER PRIMARY KEY,
    client_id TEXT NOT NULL,
    change_type TEXT NOT NULL,      -- 'create', 'update', 'delete'
    entity_type TEXT NOT NULL,      -- 'job', 'device', 'scan'
    entity_id TEXT,
    change_data TEXT NOT NULL,      -- JSON
    status TEXT DEFAULT 'pending',  -- 'pending', 'approved', 'rejected'
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    reviewed_at TEXT,
    reviewed_by TEXT
);
```

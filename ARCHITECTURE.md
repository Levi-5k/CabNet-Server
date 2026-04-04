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
│   ├── lib.rs              # Re-exports
│   ├── config.rs           # Configuration management
│   ├── api/                # HTTP API layer
│   │   ├── mod.rs
│   │   ├── routes.rs       # Axum router + SharedState type
│   │   ├── handlers.rs     # All endpoint logic + ApiResponse<T>
│   │   └── web.rs          # HTML dashboard/search pages (inline raw strings)
│   ├── db/                 # Database layer
│   │   ├── mod.rs          # Pool init + inline migrations
│   │   ├── models.rs       # DB row structs (FromRow) + API input structs
│   │   └── repository.rs   # All CRUD operations
│   ├── gui/                # egui immediate-mode GUI
│   │   ├── mod.rs
│   │   ├── app.rs          # CodeBarApp + CachedData + TabStates + Theme
│   │   └── tabs/
│   │       ├── mod.rs
│   │       ├── jobs.rs     # Job management
│   │       ├── scans.rs    # Scan list (with 📍 map pin buttons)
│   │       ├── devices.rs  # Connected devices (card grid)
│   │       ├── approvals.rs # Pending changes + web client management
│   │       ├── email.rs    # Email settings and sending
│   │       ├── settings.rs # App configuration
│   │       └── team.rs     # Team member cards
│   └── services/
│       ├── mod.rs
│       ├── email.rs        # SMTP via lettre
│       └── tunnel.rs       # Cloudflare Tunnel management
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

1. **Jobs Tab** - Create, edit, delete jobs; assign to devices
2. **Scans Tab** - View all scans with filtering and search; 📍 pin button opens scan location in browser map
3. **Devices Tab** - Monitor connected devices in a card grid layout, last seen, sync status
4. **Team Tab** - Team member profiles linked to devices
5. **Approvals Tab** - Manage pending changes from untrusted web clients; link web clients to team members
6. **Settings Tab** - Server port, database path, auto-sync options
7. **Email Tab** - SMTP configuration, auto-send rules, send history

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

5. **Linking Web Clients to Team Members**:
   - Each web client can be linked to a team member via `user_id` (which stores the team member's `device_id`)
   - In desktop app → Approvals → Web Clients tab → Click "🔗 Link" → Select a team member
   - This associates browser sessions with specific people/devices
   - Linked member name is displayed alongside the web client

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/web/register` | POST | Register new web client |
| `/api/web/status` | GET | Check client trust status |
| `/api/web/clients` | GET | List all web clients |
| `/api/web/clients/:id/trust` | PUT | Set trust status |
| `/api/web/clients/:id/name` | PUT | Update display name |
| `/api/web/clients/:id/link` | PUT | Link/unlink to team member |
| `/api/web/clients/:id` | DELETE | Delete a web client |
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
    user_id TEXT,                    -- Links to team_members.device_id
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

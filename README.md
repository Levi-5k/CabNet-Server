# CabNet Server

A Rust desktop application for managing barcode scans from an app or Android scanner.

## Features

- **HTTP API Server** - Receives scans from CabNet Android app
- **Real-time Dashboard** - View scans, jobs, and devices as they update
- **Job Management** - Create and manage scanning jobs
- **Device Tracking** - Monitor connected Android devices
- **Team Management** - Team member profiles, roles, and time tracking
- **Web Client Trust System** - Approve/reject changes from remote browsers; link web clients to team members
- **Email Reports** - Auto-send scan reports via SMTP
- **Cloudflare Tunnel** - Secure remote access without port forwarding
- **CSV Export** - Export data for analysis

## Requirements

- Rust 1.75 or later
- Windows 10/11 (for GUI)

## Quick Start

```bash
# Clone and build
cd codebar-server
cargo build --release

# Run
cargo run --release
```

The server will start on `http://0.0.0.0:8080` by default.

## Configuration

Create a `.env` file or use the Settings tab:

```env
# Server
SERVER_PORT=8080
SERVER_HOST=0.0.0.0

# Database
DATABASE_PATH=./data/codebar.db

# Email (optional)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
EMAIL_FROM=your-email@gmail.com
```

## Android App Configuration

In the CabNet Android app settings, set the server URL to:
```
http://{your-pc-ip}:8080
```

Make sure your PC and Android device are on the same network.

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

## Tabs Overview

### Jobs
- Create, edit, and delete jobs
- View job progress and completion status
- Assign jobs to devices

### Scans
- Real-time list of all scans
- Search and filter capabilities
- 📍 Pin button opens scan location in browser map (for GPS-enabled scans)
- Export to CSV

### Devices
- Card grid of connected Android devices
- Device status, battery, memory, and last sync time
- Per-device scan statistics

### Team
- Team member profiles linked to devices
- Role management (worker, lead, admin)
- Current status (working, break, offline)

### Approvals
- Pending changes from untrusted web clients
- Approve or reject queued changes
- Web client management (trust/revoke, link to team members)

### Settings
- Server configuration (port, host)
- Database settings
- Auto-sync options

### Email
- SMTP server configuration
- Email templates
- Auto-send rules (send report after X minutes of inactivity)
- Send history

## API Documentation

See [API.md](API.md) for full API documentation.

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for technical details.

## License

MIT

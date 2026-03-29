# CabNet Server

A Rust desktop application for managing barcode scans from Android devices.

## Features

- **HTTP API Server** - Receives scans from CodeBar Android app
- **Real-time Dashboard** - View scans, jobs, and devices as they update
- **Map View** - See all scan locations on an interactive map
- **Job Management** - Create and manage scanning jobs
- **Device Tracking** - Monitor connected Android devices
- **Email Reports** - Auto-send scan reports via SMTP
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

### Map
- Interactive map showing all scan locations
- Click markers to see scan details
- Filter by job, device, or date range

### Jobs
- Create, edit, and delete jobs
- View job progress and completion status
- Assign jobs to devices

### Scans
- Real-time list of all scans
- Search and filter capabilities
- Export to CSV

### Devices
- List of connected Android devices
- Device status and last sync time
- Per-device scan statistics

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

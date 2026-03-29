# CabNet Server API Documentation

## Overview

CabNet Server is a Rust-based backend for the CabNet cabinet installation management system. It provides REST APIs for barcode scanning, job management, time tracking, reporting, device management, GPS tracking, and a web dashboard.

## Base URL

```
http://{server_ip}:3000/api
```

## Conventions

- All request/response bodies use **JSON** with **snake_case** field names
- All responses are wrapped in a standard envelope:

```json
{
  "success": true,
  "data": { ... },
  "message": "Optional message",
  "error": "Optional error string"
}
```

- Timestamps are **milliseconds since Unix epoch** (Int64) unless noted
- UUIDs are **string** format (client-generated)
- All `POST` endpoints accept `Content-Type: application/json` unless noted

---

## Table of Contents

1. [Health](#health)
2. [Scans](#scans)
3. [Jobs](#jobs)
4. [Devices](#devices)
5. [Time Entries](#time-entries)
6. [Reports](#reports)
7. [GPS Location Tracking](#gps-location-tracking)
8. [Web Clients](#web-clients)
9. [Pending Changes (Approval Workflow)](#pending-changes)
10. [Security](#security)
11. [Web Dashboard](#web-dashboard)

---

## Health

### `GET /api/health`

Server health check. Use for connectivity testing.

**Response:**
```json
{
  "success": true,
  "data": {
    "status": "ok",
    "version": "0.1.0",
    "timestamp": 1711728000000,
    "uptime_seconds": 3600
  }
}
```

---

## Scans

### `POST /api/scans`

Upload barcode scans from a device. Supports batch upsert — existing scans (matched by UUID) are updated.

**Request:**
```json
{
  "device_id": "device-uuid",
  "batch_id": "optional-batch-uuid",
  "scans": [
    {
      "id": "scan-uuid",
      "barcode_data": "ABC123",
      "barcode_type": "QR_CODE",
      "scanned_at": 1711728000000,
      "device_id": "device-uuid",
      "user_id": "user-name",
      "location": "Site A",
      "local_id": "local-uuid-string",
      "latitude": 37.7749,
      "longitude": -122.4194,
      "job_id": "job-uuid-or-null",
      "ticket_number": "T-001",
      "barcode_job_ref": "JOB-REF",
      "is_printed": false
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "synced_ids": ["scan-uuid-1", "scan-uuid-2"]
  }
}
```

### `GET /api/scans`

Retrieve scans with optional filters.

**Query Parameters:**
| Parameter   | Type   | Description                          |
|-------------|--------|--------------------------------------|
| `device_id` | string | Filter by device                     |
| `job_id`    | int    | Filter by server-side job ID         |
| `since`     | int64  | Only scans after this timestamp (ms) |
| `limit`     | int    | Max results (default: 5000)          |

**Response:**
```json
{
  "success": true,
  "data": {
    "scans": [ ... ],
    "total": 150
  }
}
```

### `POST /api/scans/verify`

Verify which scans exist on the server.

**Request:**
```json
{
  "device_id": "device-uuid",
  "scan_ids": ["uuid-1", "uuid-2"],
  "batch_id": "optional"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "verified": ["uuid-1"],
    "missing": ["uuid-2"]
  }
}
```

### `GET /api/scans/locations`

Get unique scan locations for map display.

**Response:**
```json
{
  "success": true,
  "data": {
    "locations": [
      {
        "location": "Site A",
        "latitude": 37.7749,
        "longitude": -122.4194,
        "scan_count": 25,
        "last_scanned_at": "2026-03-29T12:00:00Z"
      }
    ]
  }
}
```

### `PUT /api/scans/assign`

Bulk reassign scans to a different job or device.

**Request:**
```json
{
  "scan_ids": [1, 2, 3],
  "job_id": 5,
  "device_id": "new-device-id"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "updated": 3
  }
}
```

### `DELETE /api/scans/:id`

Delete a scan by server-side ID.

**Response:**
```json
{
  "success": true,
  "message": "Scan deleted"
}
```

---

## Jobs

### `POST /api/jobs`

Upload/sync jobs from a device. Upserts by UUID.

**Request:**
```json
{
  "device_id": "device-uuid",
  "jobs": [
    {
      "uuid": "job-uuid",
      "name": "Kitchen Remodel",
      "description": "Install upper and lower cabinets",
      "reference_number": "JOB-2026-001",
      "customer_name": "Smith Residence",
      "expected_count": 24,
      "status": "active",
      "device_id": "device-uuid",
      "created_by": "user-name",
      "notes": "Access from back door",
      "priority": "HIGH",
      "due_date": "2026-04-15",
      "started_at": 1711728000000,
      "completed_at": null
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "synced_job_ids": ["job-uuid"]
  }
}
```

### `GET /api/jobs`

Download jobs from the server.

**Query Parameters:**
| Parameter   | Type   | Description                     |
|-------------|--------|---------------------------------|
| `device_id` | string | Filter by originating device    |
| `since`     | int64  | Only jobs updated after this ms |

**Response:**
```json
{
  "success": true,
  "data": {
    "jobs": [
      {
        "id": 1,
        "server_id": 1,
        "uuid": "job-uuid",
        "name": "Kitchen Remodel",
        "description": "Install upper and lower cabinets",
        "reference_number": "JOB-2026-001",
        "customer_name": "Smith Residence",
        "expected_count": 24,
        "scan_count": 12,
        "status": "active",
        "device_id": "device-uuid",
        "created_by": "user-name",
        "notes": "Access from back door",
        "priority": "HIGH",
        "due_date": "2026-04-15",
        "created_at": "2026-03-29T10:00:00Z",
        "updated_at": "2026-03-29T12:00:00Z",
        "started_at": "2026-03-29T10:00:00Z",
        "completed_at": null
      }
    ]
  }
}
```

### `PUT /api/jobs/:id`

Update a job by server-side ID. All fields optional.

**Request:**
```json
{
  "name": "Updated Name",
  "status": "completed",
  "completed_at": "2026-03-29T18:00:00Z",
  "reference_number": "JOB-2026-001",
  "description": "Updated description",
  "customer_name": "Smith Residence",
  "notes": "Completed successfully",
  "priority": "NORMAL",
  "expected_count": 30,
  "due_date": "2026-04-20"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "job": { ... }
  }
}
```

### `DELETE /api/jobs/:id`

Delete a job by server-side ID. Associated scans are unlinked (not deleted).

---

## Devices

### `POST /api/devices/register`

Register a new device. Subject to rate limiting and optional registration code requirement.

**Request:**
```json
{
  "device_id": "unique-device-id",
  "device_name": "iPhone 17 Pro Max",
  "model": "iPhone",
  "os_version": "iOS 26.4",
  "app_version": "1.0.0",
  "registration_code": "ABC123"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "device_id": "unique-device-id"
  }
}
```

### `POST /api/devices/approve`

Approve a registered device (admin action).

**Request:**
```json
{
  "device_id": "device-uuid",
  "approved": true,
  "notes": "Approved by admin"
}
```

### `POST /api/devices/authenticate`

Authenticate a device with its token.

**Request:**
```json
{
  "device_id": "device-uuid",
  "auth_token": "token-string"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "authenticated": true,
    "device_id": "device-uuid",
    "device_name": "iPhone 17 Pro Max"
  }
}
```

### `GET /api/devices`

List all registered devices with status information.

**Response:**
```json
{
  "success": true,
  "data": {
    "devices": [
      {
        "id": 1,
        "device_id": "device-uuid",
        "device_name": "iPhone 17 Pro Max",
        "model": "iPhone",
        "os_version": "iOS 26.4",
        "app_version": "1.0.0",
        "status": "active",
        "is_approved": true,
        "last_seen_at": "2026-03-29T12:00:00Z",
        "last_heartbeat_at": "2026-03-29T12:05:00Z",
        "battery_level": 85,
        "is_charging": false,
        "network_type": "WiFi",
        "connection_quality": "Excellent"
      }
    ],
    "total": 1
  }
}
```

### `POST /api/connect`

Lightweight device connection (used by iOS app on launch).

**Request:**
```json
{
  "device_id": "device-uuid",
  "device_name": "iPhone 17 Pro Max",
  "model": "iPhone",
  "os_version": "iOS 26.4",
  "app_version": "1.0.0"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "connected": true,
    "device_id": "device-uuid",
    "server_version": "0.1.0",
    "server_time": 1711728000000
  }
}
```

### `POST /api/devices/heartbeat`

Standard heartbeat to maintain device online status.

**Request:**
```json
{
  "device_id": "device-uuid",
  "pending_scans": 5,
  "pending_jobs": 1
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "acknowledged": true,
    "server_time": 1711728000000
  }
}
```

### `POST /api/devices/heartbeat/enhanced`

Enhanced heartbeat with device telemetry. Server responds with adaptive interval suggestions.

**Request:**
```json
{
  "device_id": "device-uuid",
  "pending_scans": 5,
  "pending_jobs": 1,
  "battery_level": 85,
  "is_charging": false,
  "total_scans": 150,
  "memory_usage": 256,
  "app_version": "1.0.0",
  "network_type": "WiFi",
  "connection_quality": "Excellent",
  "timestamp": 1711728000000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "acknowledged": true,
    "server_time": 1711728000000,
    "suggested_interval_seconds": 90
  }
}
```

### `POST /api/devices/disconnect`

Mark a device as disconnected.

**Request:**
```json
{
  "device_id": "device-uuid"
}
```

### `POST /api/devices/block`

Temporarily block a device.

**Request:**
```json
{
  "device_id": "device-uuid",
  "minutes": 60
}
```

### `POST /api/devices/unblock`

Unblock a previously blocked device.

**Request:**
```json
{
  "device_id": "device-uuid"
}
```

---

## Time Entries

Tracks employee clock-in/clock-out and breaks. Entries are client-generated with UUIDs and synced to the server.

### `POST /api/time-entries`

Sync time entries from a device. Upserts by UUID — can be called multiple times safely (e.g., to update clock_out time).

**Request:**
```json
{
  "device_id": "device-uuid",
  "entries": [
    {
      "id": "entry-uuid",
      "device_id": "device-uuid",
      "customer_name": "John Smith",
      "job_name": "Kitchen Remodel",
      "job_id": "job-uuid",
      "clock_in": 1711699200000,
      "clock_out": 1711728000000,
      "note": "",
      "is_break": false,
      "is_paid": true
    },
    {
      "id": "break-uuid",
      "device_id": "device-uuid",
      "customer_name": "John Smith",
      "job_name": "Kitchen Remodel",
      "job_id": "job-uuid",
      "clock_in": 1711713600000,
      "clock_out": 1711717200000,
      "note": "Lunch",
      "is_break": true,
      "is_paid": false
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "synced_ids": ["entry-uuid", "break-uuid"]
  }
}
```

### `GET /api/time-entries`

Retrieve time entries with optional filters.

**Query Parameters:**
| Parameter   | Type   | Description                            |
|-------------|--------|----------------------------------------|
| `device_id` | string | Filter by device                       |
| `job_id`    | string | Filter by job UUID                     |
| `since`     | int64  | Only entries synced after this ms      |
| `limit`     | int    | Max results (default: 1000)            |

**Response:**
```json
{
  "success": true,
  "data": {
    "entries": [
      {
        "id": 1,
        "uuid": "entry-uuid",
        "device_id": "device-uuid",
        "customer_name": "John Smith",
        "job_name": "Kitchen Remodel",
        "job_id": "job-uuid",
        "clock_in": "2026-03-29T08:00:00Z",
        "clock_out": "2026-03-29T16:00:00Z",
        "note": "",
        "is_break": false,
        "is_paid": true,
        "synced_at": "2026-03-29T16:05:00Z",
        "created_at": "2026-03-29T08:00:00Z"
      }
    ],
    "total": 1
  }
}
```

**Active Workers:** Entries where `clock_out` is `null` and `is_break` is `false` represent currently working employees. Entries where `clock_out` is `null` and `is_break` is `true` represent employees on break.

---

## Reports

Cabinet installation reports with photo attachments. Tracks room-level progress with checklist items.

### `POST /api/reports`

Sync reports from a device. Upserts by UUID.

**Request:**
```json
{
  "device_id": "device-uuid",
  "reports": [
    {
      "uuid": "report-uuid",
      "job_id": "job-uuid",
      "title": "Kitchen Upper Cabinets",
      "room_name": "Kitchen",
      "notes": "All uppers installed and leveled",
      "status": "complete",
      "author_device_id": "device-uuid",
      "author_name": "John Smith",
      "assigned_to_device_id": null,
      "assigned_to_name": null,
      "cabinet_count": 8,
      "is_complete": true,
      "has_fillers": true,
      "has_handles": true,
      "has_fast_caps": true,
      "has_set_boxes": false,
      "has_caulking": true,
      "punch_list": "Touch up paint on panel 3",
      "created_at": 1711728000000,
      "updated_at": 1711728000000
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "synced_report_ids": ["report-uuid"]
  }
}
```

### `GET /api/reports`

Retrieve reports with optional filters.

**Query Parameters:**
| Parameter | Type   | Description                     |
|-----------|--------|---------------------------------|
| `job_id`  | string | Filter by job UUID              |
| `status`  | string | Filter by status (draft/in_progress/complete) |
| `limit`   | int    | Max results (default: 500)      |

**Response:**
```json
{
  "success": true,
  "data": {
    "reports": [ ... ],
    "total": 10
  }
}
```

### `POST /api/reports/:report_id/photos`

Upload a photo for a report. Uses multipart form data.

**Content-Type:** `multipart/form-data`

**Form Field:**
| Field   | Type | Description      |
|---------|------|------------------|
| `photo` | file | JPEG image data  |

**Response:**
```json
{
  "success": true,
  "data": {
    "synced_report_ids": ["photo-uuid"]
  }
}
```

---

## GPS Location Tracking

Tracks employee GPS location while clocked in. The mobile app sends periodic location updates (every 30–120 seconds depending on movement and battery). Enables live map view on the web dashboard showing where workers are.

### Design

- **When tracking starts:** Automatically when a time entry begins (`clock_in`)
- **When tracking stops:** Automatically when the time entry ends (`clock_out`)
- **Update frequency:** Adaptive — 30s while moving, 120s while stationary
- **Battery optimization:** Uses `CLLocationManager` significant change monitoring + timer-based GPS pings
- **Privacy:** Tracking only occurs while clocked in; location data is tied to time entries
- **Storage:** Location pings are batched and synced to the server every 60 seconds or on clock out

### `POST /api/location-pings`

Batch upload location pings from a device. Each ping is tied to a time entry.

**Request:**
```json
{
  "device_id": "device-uuid",
  "pings": [
    {
      "id": "ping-uuid",
      "time_entry_id": "time-entry-uuid",
      "latitude": 37.7749,
      "longitude": -122.4194,
      "accuracy": 10.5,
      "altitude": 15.2,
      "speed": 0.0,
      "heading": -1.0,
      "timestamp": 1711728000000,
      "battery_level": 85,
      "is_moving": false
    },
    {
      "id": "ping-uuid-2",
      "time_entry_id": "time-entry-uuid",
      "latitude": 37.7751,
      "longitude": -122.4190,
      "accuracy": 8.0,
      "altitude": 15.5,
      "speed": 1.2,
      "heading": 45.0,
      "timestamp": 1711728030000,
      "battery_level": 84,
      "is_moving": true
    }
  ]
}
```

**Fields:**
| Field           | Type    | Required | Description                                    |
|-----------------|---------|----------|------------------------------------------------|
| `id`            | string  | Yes      | Client-generated UUID                          |
| `time_entry_id` | string  | Yes      | UUID of the associated time entry              |
| `latitude`      | float   | Yes      | GPS latitude                                   |
| `longitude`     | float   | Yes      | GPS longitude                                  |
| `accuracy`      | float   | No       | Horizontal accuracy in meters                  |
| `altitude`      | float   | No       | Altitude in meters                             |
| `speed`         | float   | No       | Speed in m/s (-1 if unavailable)               |
| `heading`       | float   | No       | Course heading in degrees (-1 if unavailable)  |
| `timestamp`     | int64   | Yes      | Milliseconds since epoch                       |
| `battery_level` | int     | No       | Battery percentage (0–100)                     |
| `is_moving`     | bool    | No       | Whether significant movement was detected      |

**Response:**
```json
{
  "success": true,
  "data": {
    "synced_ids": ["ping-uuid", "ping-uuid-2"]
  }
}
```

### `GET /api/location-pings`

Retrieve location pings. Primarily used by the web dashboard for map display.

**Query Parameters:**
| Parameter       | Type   | Description                              |
|-----------------|--------|------------------------------------------|
| `device_id`     | string | Filter by device                         |
| `time_entry_id` | string | Filter by time entry UUID                |
| `since`         | int64  | Only pings after this timestamp (ms)     |
| `limit`         | int    | Max results (default: 5000)              |
| `active_only`   | bool   | Only pings for currently active entries  |

**Response:**
```json
{
  "success": true,
  "data": {
    "pings": [
      {
        "id": 1,
        "uuid": "ping-uuid",
        "device_id": "device-uuid",
        "time_entry_id": "time-entry-uuid",
        "latitude": 37.7749,
        "longitude": -122.4194,
        "accuracy": 10.5,
        "altitude": 15.2,
        "speed": 0.0,
        "heading": -1.0,
        "timestamp": "2026-03-29T12:00:00Z",
        "battery_level": 85,
        "is_moving": false,
        "synced_at": "2026-03-29T12:01:00Z"
      }
    ],
    "total": 50
  }
}
```

### `GET /api/location-pings/latest`

Get the most recent location ping for each active worker. Used by the dashboard map for live positions.

**Response:**
```json
{
  "success": true,
  "data": {
    "positions": [
      {
        "device_id": "device-uuid",
        "device_name": "iPhone 17 Pro Max",
        "customer_name": "John Smith",
        "job_name": "Kitchen Remodel",
        "latitude": 37.7749,
        "longitude": -122.4194,
        "accuracy": 10.5,
        "speed": 0.0,
        "is_moving": false,
        "battery_level": 85,
        "timestamp": "2026-03-29T12:00:00Z",
        "clock_in": "2026-03-29T08:00:00Z",
        "is_on_break": false
      }
    ]
  }
}
```

---

## Web Clients

Web dashboard client management with trust-based access control.

### `POST /api/web/register`

Register a new web client session.

**Request:**
```json
{
  "client_name": "Office Dashboard"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "client_id": "generated-client-id",
    "is_trusted": false,
    "client_name": "Office Dashboard"
  }
}
```

### `GET /api/web/status`

Check a web client's trust status.

**Query Parameters:**
| Parameter   | Type   | Description   |
|-------------|--------|---------------|
| `client_id` | string | The client ID |

### `GET /api/web/clients`

List all registered web clients (admin).

### `PUT /api/web/clients/:client_id/trust`

Set a web client's trusted status.

**Request:**
```json
{
  "is_trusted": true
}
```

### `PUT /api/web/clients/:client_id/name`

Update a web client's display name.

**Request:**
```json
{
  "name": "Office Dashboard"
}
```

### `DELETE /api/web/clients/:client_id`

Remove a web client.

---

## Pending Changes

Approval workflow for untrusted web clients. Changes from untrusted clients are queued for review.

### `POST /api/changes`

Submit a change request. If the client is trusted, the change is applied immediately. Otherwise, it's queued for approval.

**Request:**
```json
{
  "client_id": "web-client-id",
  "change_type": "update",
  "entity_type": "job",
  "entity_id": "5",
  "change_data": {
    "status": "completed",
    "completed_at": "2026-03-29T18:00:00Z"
  }
}
```

**Response (trusted — applied immediately):**
```json
{
  "success": true,
  "data": {
    "applied": true,
    "message": "Change applied"
  }
}
```

**Response (untrusted — queued):**
```json
{
  "success": true,
  "data": {
    "applied": false,
    "pending_id": 1,
    "message": "Change queued for approval"
  }
}
```

### `GET /api/changes`

Get all pending changes awaiting approval.

### `POST /api/changes/:id/approve`

Approve a pending change. Applies the queued modification.

### `POST /api/changes/:id/reject`

Reject a pending change. Discards it.

---

## Security

### `GET /api/security/settings`

Get current security configuration.

**Response:**
```json
{
  "success": true,
  "data": {
    "max_registration_attempts_per_device": 5,
    "max_registration_attempts_per_ip": 10,
    "rate_limit_window_minutes": 60,
    "block_duration_minutes": 1440,
    "token_expiry_hours": 720,
    "require_registration_code": false
  }
}
```

### `PUT /api/security/settings`

Update security settings. All fields optional.

### `POST /api/security/codes`

Generate a new device registration code.

**Request:**
```json
{
  "description": "For field team"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "code": "ABC123"
  }
}
```

### `GET /api/security/codes`

List all registration codes with usage status.

---

## Web Dashboard

### `GET /`

Serves the single-page web dashboard with the following tabs:

| Tab          | Description                                                  |
|--------------|--------------------------------------------------------------|
| **Dashboard** | Overview stats: total scans, jobs, devices, active workers, reports |
| **Scans**    | Browse/search scans, view by job, assign to jobs             |
| **Jobs**     | Job list with scan counts, status management, detail view    |
| **Devices**  | Connected devices with battery, heartbeat, telemetry         |
| **Time Clock** | Active workers (live), completed timesheets, break tracking |
| **Reports**  | Installation reports by room, checklist progress, photos     |
| **Map**      | Live GPS positions of clocked-in workers                     |
| **Approvals** | Pending change review queue from untrusted web clients      |

### Static Assets

| Route          | Description          |
|----------------|----------------------|
| `GET /favicon.ico` | App favicon       |
| `GET /logo.png`    | Logo image        |
| `GET /dashboard`   | Alias for `/`     |

---

## Database Schema

### Tables

| Table              | Description                                      |
|--------------------|--------------------------------------------------|
| `scans`            | Barcode scan records                             |
| `jobs`             | Installation jobs                                |
| `devices`          | Registered mobile devices                        |
| `time_entries`     | Clock in/out and break records                   |
| `reports`          | Room-level installation reports                  |
| `report_photos`    | Photo attachments for reports (BLOB)             |
| `location_pings`   | GPS location tracking pings                      |
| `email_config`     | SMTP configuration (singleton)                   |
| `email_history`    | Email send log                                   |
| `settings`         | Key-value server settings                        |
| `web_clients`      | Dashboard client sessions                        |
| `pending_changes`  | Approval workflow queue                          |

### Indexes

```sql
-- Scans
CREATE INDEX idx_scans_job_id ON scans(job_id);
CREATE INDEX idx_scans_device_id ON scans(device_id);
CREATE INDEX idx_scans_scanned_at ON scans(scanned_at);
CREATE INDEX idx_scans_barcode ON scans(barcode);
CREATE INDEX idx_scans_local_id ON scans(local_id);
CREATE UNIQUE INDEX idx_scans_local_device ON scans(local_id, device_id);

-- Jobs
CREATE INDEX idx_jobs_status ON jobs(status);

-- Devices
CREATE INDEX idx_devices_device_id ON devices(device_id);
CREATE INDEX idx_devices_status ON devices(status);

-- Time Entries
CREATE INDEX idx_time_entries_device_id ON time_entries(device_id);
CREATE INDEX idx_time_entries_job_id ON time_entries(job_id);

-- Location Pings
CREATE INDEX idx_location_pings_device_id ON location_pings(device_id);
CREATE INDEX idx_location_pings_time_entry_id ON location_pings(time_entry_id);
CREATE INDEX idx_location_pings_timestamp ON location_pings(timestamp);
```

---

## Error Handling

All errors return the standard envelope with `success: false`:

```json
{
  "success": false,
  "error": "Description of what went wrong"
}
```

Common HTTP status codes:
| Code | Meaning                          |
|------|----------------------------------|
| 200  | Success                          |
| 400  | Bad request / validation error   |
| 403  | Device blocked or not approved   |
| 404  | Resource not found               |
| 429  | Rate limited                     |
| 500  | Internal server error            |

---

## iOS App Sync Behavior

The CabNet iOS app syncs data to the server through multiple mechanisms:

1. **Foreground sync** — Tests server connection and syncs all data (scans, jobs, time entries) when the app comes to foreground
2. **Background processing** — `BGProcessingTask` runs periodically (configurable 1–60 min interval)
3. **Background refresh** — `BGAppRefreshTask` runs lightweight sync every ~15 min
4. **Event-driven sync** — Time entries sync immediately on clock out
5. **Network restore** — Full sync triggers when network connectivity is restored
6. **GPS pings** — Batched every 60 seconds while clocked in, plus flush on clock out

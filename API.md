# CodeBar Server API Documentation

## Base URL

```
http://{server_ip}:{port}/api
```

Default port: `8080`

## JSON Format

All requests and responses use **snake_case** JSON for Android/Rust compatibility.

---

## Health Check

### GET /api/health

Check if the server is running.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "timestamp": 1706620800000,
  "uptime_seconds": 3600
}
```

---

## Scans

### POST /api/scans

Sync scans from Android device to server.

**Request:**
```json
{
  "device_id": "abc123",
  "scans": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "barcode_data": "1234567890",
      "barcode_type": "CODE_128",
      "ticket_number": "7890",
      "barcode_job_ref": "JOB001",
      "job_id": "job-uuid-here",
      "device_id": "abc123",
      "user_id": "user1",
      "location": "Warehouse A",
      "latitude": 40.7128,
      "longitude": -74.0060,
      "scanned_at": 1706620800000,
      "is_printed": false
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "synced_ids": ["550e8400-e29b-41d4-a716-446655440000"],
  "message": "Synced 1 scan(s)"
}
```

### POST /api/scans/verify

Verify which scans are stored on the server.

**Request:**
```json
{
  "device_id": "abc123",
  "scan_ids": [
    "550e8400-e29b-41d4-a716-446655440000",
    "550e8400-e29b-41d4-a716-446655440001"
  ]
}
```

**Response:**
```json
{
  "verified": ["550e8400-e29b-41d4-a716-446655440000"],
  "missing": ["550e8400-e29b-41d4-a716-446655440001"]
}
```

### GET /api/scans

Get all scans (with optional filtering).

**Query Parameters:**
- `job_id` (optional) - Filter by job
- `device_id` (optional) - Filter by device
- `since` (optional) - Unix timestamp, get scans after this time
- `limit` (optional) - Max results (default: 1000)

**Response:**
```json
{
  "success": true,
  "scans": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "barcode_data": "1234567890",
      "barcode_type": "CODE_128",
      "ticket_number": "7890",
      "barcode_job_ref": "JOB001",
      "job_id": "job-uuid-here",
      "device_id": "abc123",
      "user_id": "user1",
      "location": "Warehouse A",
      "latitude": 40.7128,
      "longitude": -74.0060,
      "scanned_at": 1706620800000,
      "synced_at": 1706620805000,
      "is_printed": false
    }
  ],
  "total": 1
}
```

---

## Jobs

### POST /api/jobs

Upload jobs from Android or create new jobs.

**Request:**
```json
{
  "device_id": "abc123",
  "jobs": [
    {
      "id": "job-uuid-here",
      "name": "Morning Route",
      "description": "Deliveries for morning shift",
      "reference_number": "JOB001",
      "customer_name": "ACME Corp",
      "expected_count": 50,
      "scan_count": 15,
      "status": "ACTIVE",
      "created_at": 1706620800000,
      "updated_at": 1706620800000,
      "started_at": 1706620900000,
      "completed_at": null,
      "device_id": "abc123",
      "created_by": "user1",
      "notes": "Handle with care",
      "priority": "HIGH",
      "due_date": 1706707200000
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "synced_job_ids": ["job-uuid-here"],
  "message": "Synced 1 job(s)"
}
```

### GET /api/jobs

Download jobs for a device.

**Query Parameters:**
- `device_id` (required) - Device requesting jobs
- `since` (optional) - Unix timestamp, get jobs updated after this time

**Response:**
```json
{
  "success": true,
  "jobs": [
    {
      "id": "job-uuid-here",
      "name": "Morning Route",
      "description": "Deliveries for morning shift",
      "reference_number": "JOB001",
      "customer_name": "ACME Corp",
      "expected_count": 50,
      "scan_count": 15,
      "status": "ACTIVE",
      "created_at": 1706620800000,
      "updated_at": 1706620800000,
      "started_at": 1706620900000,
      "completed_at": null,
      "device_id": "abc123",
      "created_by": "user1",
      "notes": "Handle with care",
      "priority": "HIGH",
      "due_date": 1706707200000
    }
  ],
  "message": null
}
```

### PUT /api/jobs/{job_id}

Update a job.

**Request:**
```json
{
  "name": "Morning Route - Updated",
  "status": "COMPLETED",
  "completed_at": 1706707200000
}
```

**Response:**
```json
{
  "success": true,
  "job": { ... },
  "message": "Job updated"
}
```

### DELETE /api/jobs/{job_id}

Delete a job.

**Response:**
```json
{
  "success": true,
  "message": "Job deleted"
}
```

---

## Devices

### POST /api/connect

**Primary endpoint for device connection.** When an Android device connects to the server, call this endpoint first to register the device and establish the connection.

**Request:**
```json
{
  "device_id": "abc123",
  "device_name": "TC56 Warehouse 1",
  "model": "Zebra TC56",
  "os_version": "Android 11",
  "app_version": "1.0.0"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "connected": true,
    "device_id": "abc123",
    "server_version": "0.1.0",
    "server_time": 1706620800000
  },
  "message": "Device connected successfully"
}
```

**Android Usage:**
```kotlin
// Call when app starts or reconnects
suspend fun connectToServer() {
    val request = ConnectRequest(
        deviceId = getDeviceId(),
        deviceName = Build.MODEL,
        model = Build.DEVICE,
        osVersion = "Android ${Build.VERSION.RELEASE}",
        appVersion = BuildConfig.VERSION_NAME
    )
    val response = api.connect(request)
    if (response.data?.connected == true) {
        // Connection successful, save server info
        serverVersion = response.data.serverVersion
    }
}
```

---

### POST /api/devices/heartbeat

**Lightweight keepalive endpoint.** Call periodically to let the server know the device is still connected. This also auto-registers the device if it doesn't exist.

**Request:**
```json
{
  "device_id": "abc123"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "acknowledged": true,
    "server_time": 1706620800000
  }
}
```

**Android Usage:**
```kotlin
// Call every 30-60 seconds while app is active
private val heartbeatJob = viewModelScope.launch {
    while (isActive) {
        api.heartbeat(HeartbeatRequest(deviceId = getDeviceId()))
        delay(30_000) // 30 seconds
    }
}
```

---

### POST /api/devices/register

Register or update a device. (Alternative to `/api/connect`)

**Request:**
```json
{
  "device_id": "abc123",
  "device_name": "TC56 Warehouse 1",
  "model": "Zebra TC56",
  "os_version": "Android 11",
  "app_version": "1.0.0"
}
```

**Response:**
```json
{
  "success": true,
  "device_id": "abc123",
  "message": "Device registered"
}
```

### GET /api/devices

Get all registered devices.

**Response:**
```json
{
  "success": true,
  "devices": [
    {
      "device_id": "abc123",
      "device_name": "TC56 Warehouse 1",
      "model": "Zebra TC56",
      "os_version": "Android 11",
      "app_version": "1.0.0",
      "last_seen": 1706620800000,
      "total_scans": 150,
      "is_online": true
    }
  ]
}
```

---

## Error Responses

All endpoints return errors in this format:

```json
{
  "success": false,
  "error": "Error description",
  "code": "ERROR_CODE"
}
```

**Error Codes:**
- `INVALID_REQUEST` - Malformed JSON or missing fields
- `NOT_FOUND` - Resource not found
- `DATABASE_ERROR` - Database operation failed
- `INTERNAL_ERROR` - Unexpected server error

---

## Data Models

### Scan

| Field | Type | Description |
|-------|------|-------------|
| id | string (UUID) | Unique scan ID |
| barcode_data | string | Raw barcode content |
| barcode_type | string | Barcode symbology (CODE_128, QR_CODE, etc.) |
| ticket_number | string | Extracted ticket number |
| barcode_job_ref | string | Job reference from barcode |
| job_id | string? | Associated job UUID |
| device_id | string | Device that scanned |
| user_id | string | User who scanned |
| location | string | Location name/description |
| latitude | float? | GPS latitude |
| longitude | float? | GPS longitude |
| scanned_at | int64 | Unix timestamp (ms) |
| synced_at | int64 | When synced to server |
| is_printed | bool | Whether label was printed |

### Job

| Field | Type | Description |
|-------|------|-------------|
| id | string (UUID) | Unique job ID |
| name | string | Job name |
| description | string | Job description |
| reference_number | string | External reference |
| customer_name | string | Customer name |
| expected_count | int | Expected scan count |
| scan_count | int | Current scan count |
| status | string | PENDING, ACTIVE, COMPLETED, CANCELLED |
| created_at | int64 | Unix timestamp (ms) |
| updated_at | int64 | Last update timestamp |
| started_at | int64? | When job started |
| completed_at | int64? | When job completed |
| device_id | string | Assigned device |
| created_by | string | Creator user ID |
| notes | string | Additional notes |
| priority | string | LOW, NORMAL, HIGH, URGENT |
| due_date | int64? | Due date timestamp |

### Device

| Field | Type | Description |
|-------|------|-------------|
| device_id | string | Unique device identifier |
| device_name | string | Friendly name |
| model | string | Device model |
| os_version | string | Android version |
| app_version | string | App version |
| last_seen | int64 | Last activity timestamp |
| total_scans | int | Total scans from device |
| is_online | bool | Currently connected |

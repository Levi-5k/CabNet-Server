# CabNet Server Connection & Auto-Sync Documentation

This document explains how the CabNet app connects to the server and manages automatic synchronization.

## Table of Contents

1. [Overview](#overview)
2. [QR Code Connection](#qr-code-connection)
3. [Cloudflare Tunnel (Internet Access)](#cloudflare-tunnel-internet-access)
4. [Manual Connection](#manual-connection)
5. [Network Monitoring](#network-monitoring)
6. [Auto-Sync Behavior](#auto-sync-behavior)
7. [Architecture](#architecture)
8. [Troubleshooting](#troubleshooting)

---

## Overview

The CabNet app uses a multi-layered approach to ensure reliable server connectivity and data synchronization:

1. **QR Code Scanning** - Quick connection setup by scanning a server QR code
2. **Cloudflare Tunnel** - Secure internet access without port forwarding
3. **Manual Entry** - IP:Port or domain input for manual configuration
4. **Network Monitoring** - Automatic detection of network availability
5. **Auto-Sync** - Automatic data synchronization when connection is restored

---

## QR Code Connection

### Supported QR Code Formats

The app supports multiple QR code formats for maximum flexibility:

#### 1. CabNet Protocol URL (Recommended)
```
cabnet://192.168.1.100:8080
cabnet://cabnet.example.com
```
- Prefix: `cabnet://`
- For local network: `cabnet://IP:PORT`
- For Cloudflare tunnel: `cabnet://domain` (no port needed)

#### 2. HTTP/HTTPS URL
```
http://192.168.1.100:8080
https://cabnet.example.com
```
- Standard URL format
- Use `http://` for local network connections
- Use `https://` for Cloudflare tunnel connections (secure)

#### 3. Simple Address
```
192.168.1.100:8080
cabnet.example.com
```
- Direct IP:port or domain
- No protocol prefix required

#### 4. JSON Configuration
```json
{"host":"192.168.1.100","port":8080,"secure":false}
{"host":"cabnet.example.com","secure":true}
```
- JSON object with connection details
- `host`: IP address or domain name
- `port`: Server port (optional for tunnel connections)
- `secure`: `true` for HTTPS/tunnel, `false` for HTTP/local

### Connection Types

| Type | URL Format | Example |
|------|------------|---------|
| Local Network | `cabnet://IP:PORT` | `cabnet://192.168.1.100:8080` |
| Cloudflare Tunnel | `cabnet://DOMAIN` | `cabnet://cabnet.example.com` |

### Scan-to-Connect Mode

1. Navigate to **Connection** screen
2. Tap the **Scan QR** button
3. The app enters "Scan-to-Connect" mode
4. Scan a server QR code
5. Connection is established automatically

### QR Code Parsing Flow

```
┌─────────────────┐
│  Scan QR Code   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Parse QR Data   │
│                 │
│ Try in order:   │
│ 1. cabnet://    │
│ 2. http(s)://   │
│ 3. IP:Port/Domain│
│ 4. JSON         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Validate Address│
│ (IP:Port or URL)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Connect to      │
│ Server          │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Save Connection │
│ to SharedPrefs  │
└─────────────────┘
```

---

## Cloudflare Tunnel (Internet Access)

Cloudflare Tunnel allows devices to connect to the CabNet server from anywhere on the internet without exposing ports or configuring firewalls.

### How It Works

```
┌──────────────┐     ┌───────────────────┐     ┌─────────────────┐
│   Android    │────▶│  Cloudflare Edge  │────▶│  CabNet Server  │
│   Device     │     │  (Global CDN)     │     │  (Your PC)      │
└──────────────┘     └───────────────────┘     └─────────────────┘
     HTTPS              Secure Tunnel            localhost:8080
```

1. **CabNet Server** runs a `cloudflared` tunnel to Cloudflare
2. **Cloudflare** provides a public URL (e.g., `cabnet.example.com`)
3. **Android devices** connect via HTTPS to the Cloudflare URL
4. **Traffic** is securely routed to your local CabNet server

### Benefits

- ✅ No port forwarding required
- ✅ Automatic HTTPS encryption
- ✅ DDoS protection from Cloudflare
- ✅ Works from any network (WiFi, mobile data, etc.)
- ✅ No static IP needed

### Server Setup

1. Go to **Settings** tab in CabNet Server
2. Click **Setup Cloudflare Tunnel**
3. Follow the 5-step wizard:
   - Install cloudflared CLI
   - Authenticate with Cloudflare
   - Create a tunnel
   - Configure DNS for your domain
   - Start the tunnel

### Device Connection via Tunnel

1. On the server, go to **Devices** tab
2. Click **📲 Connect Device**
3. Select **☁️ Internet (Tunnel)**
4. Scan the QR code with CabNet Android app

The QR code will contain your tunnel URL:
```
cabnet://cabnet.example.com
```

### Tunnel URL Format

The tunnel URL is constructed from your configuration:

```
https://{subdomain}.{domain}
```

Example:
- Domain: `example.com`
- Subdomain: `cabnet`
- Result: `https://cabnet.example.com`

---

## Manual Connection

### Steps

1. Navigate to **Connection** screen
2. Enter the server **IP Address** or **Domain**
3. Enter the server **Port** (default: 8080)
4. Tap **Connect**

### Validation

- IP address is required
- Port must be between 1-65535
- Connection test is performed before saving

---

## Network Monitoring

### NetworkMonitor Component

The `NetworkMonitor` class provides real-time network connectivity monitoring using Android's `ConnectivityManager.NetworkCallback`.

### Features

| Feature | Description |
|---------|-------------|
| **Network Detection** | Detects when network becomes available/unavailable |
| **Internet Validation** | Validates actual internet capability (not just WiFi connection) |
| **Server Reachability** | Tests if the configured server is reachable |
| **Auto-Sync Trigger** | Triggers sync when network is restored |

### Network States

```kotlin
// StateFlow properties available for UI observation
val isConnected: StateFlow<Boolean>      // Network available
val isServerReachable: StateFlow<Boolean> // Server is reachable
val lastSyncTime: StateFlow<Long?>       // Last successful sync timestamp
```

### Callback Events

| Event | Action |
|-------|--------|
| `onAvailable` | Network connected, test server, trigger sync |
| `onLost` | Network disconnected, update UI |
| `onCapabilitiesChanged` | Validate internet capability |

---

## Auto-Sync Behavior

### When Auto-Sync Occurs

1. **Network Restored** - When network connectivity is restored after being lost
2. **App Launch** - If auto-sync is enabled and network is available
3. **Manual Trigger** - User taps "Sync Now" button
4. **Periodic Sync** - WorkManager schedules periodic sync (every 15 minutes)

### Sync Process

```
┌─────────────────┐
│ Network Restored│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Check Auto-Sync │
│ Enabled?        │
└────────┬────────┘
         │ Yes
         ▼
┌─────────────────┐
│ Test Server     │
│ Reachability    │
└────────┬────────┘
         │ Success
         ▼
┌─────────────────┐
│ Sync Unsynced   │
│ Scans           │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Fetch Jobs from │
│ Server          │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Update Last     │
│ Sync Time       │
└─────────────────┘
```

### What Gets Synced

| Data Type | Direction | Condition |
|-----------|-----------|-----------|
| **Scans** | App → Server | Scans marked as `synced = false` |
| **Jobs** | Server → App | All available jobs |
| **Barcodes** | Server → App | PDF barcodes for active job |

### SyncWorker Configuration

```kotlin
// Periodic sync constraints
PeriodicWorkRequestBuilder<SyncWorker>(15, TimeUnit.MINUTES)
    .setConstraints(
        Constraints.Builder()
            .setRequiredNetworkType(NetworkType.CONNECTED)  // Requires network
            .build()
    )
```

---

## Architecture

### Component Relationships

```
┌──────────────────────────────────────────────────────┐
│                    MainActivity                       │
│                                                      │
│  ┌─────────────────┐    ┌─────────────────┐         │
│  │ NetworkMonitor  │───▶│ SyncWorker      │         │
│  │ (Singleton)     │    │ (WorkManager)   │         │
│  └────────┬────────┘    └────────┬────────┘         │
│           │                      │                   │
│           ▼                      ▼                   │
│  ┌─────────────────┐    ┌─────────────────┐         │
│  │ ConnectivityMgr │    │ SyncRepository  │         │
│  │ NetworkCallback │    │                 │         │
│  └─────────────────┘    └────────┬────────┘         │
│                                  │                   │
│                                  ▼                   │
│  ┌─────────────────┐    ┌─────────────────┐         │
│  │ ConnectionVM    │───▶│ ApiService      │         │
│  │                 │    │ (Retrofit)      │         │
│  └────────┬────────┘    └─────────────────┘         │
│           │                                          │
│           ▼                                          │
│  ┌─────────────────┐                                │
│  │ ConnectionFrag  │                                │
│  │ (UI)            │                                │
│  └─────────────────┘                                │
└──────────────────────────────────────────────────────┘
```

### Key Classes

| Class | Responsibility |
|-------|----------------|
| `NetworkMonitor` | Network connectivity monitoring, auto-sync triggering |
| `ConnectionViewModel` | QR parsing, connection management, UI state |
| `ConnectionFragment` | Connection UI, user interactions |
| `SyncWorker` | Background sync using WorkManager |
| `SyncRepository` | Data sync logic, API calls |
| `ApiService` | Retrofit API interface |

---

## Troubleshooting

### Connection Issues

#### "Unable to connect to server"

**Possible Causes:**
1. Server not running
2. Wrong IP address or port
3. Firewall blocking connection
4. Device not on same network as server

**Solutions:**
1. Verify server is running and accessible
2. Double-check IP and port
3. Ensure device and server are on the same network
4. Check firewall settings

#### QR Code Not Recognized

**Possible Causes:**
1. QR code format not supported
2. Camera focus issues
3. Poor lighting

**Solutions:**
1. Use one of the supported QR formats
2. Clean camera lens
3. Improve lighting conditions
4. Try manual connection as fallback

### Sync Issues

#### Scans Not Syncing

**Possible Causes:**
1. Auto-sync disabled
2. No network connection
3. Server unreachable
4. Authentication issues

**Solutions:**
1. Enable auto-sync in settings
2. Check network connection
3. Verify server is running
4. Test connection manually

#### Sync Stuck

**Possible Causes:**
1. Large number of unsynced scans
2. Network instability
3. Server timeout

**Solutions:**
1. Wait for sync to complete
2. Check network stability
3. Try manual sync

### Debug Information

To check current connection status:

1. Navigate to **Connection** screen
2. View the status indicator:
   - 🟢 **Green** - Connected
   - 🟡 **Yellow** - Connecting
   - 🔴 **Red** - Disconnected/Error

To check sync status:

1. Navigate to **Settings** screen
2. View last sync time
3. Check pending sync count

---

## Server QR Code Generation

The CabNet server automatically generates QR codes in the **Devices** tab.

### Using CabNet Server UI

1. Open CabNet Server
2. Go to **Devices** tab
3. Click **📲 Connect Device**
4. Choose connection type:
   - **🏠 Local Network** - For devices on the same network
   - **☁️ Internet (Tunnel)** - For remote access via Cloudflare
5. Choose QR format (cabnet://, https://, JSON, or Address)
6. Scan the QR code with the CabNet Android app

### QR Code Content Examples

#### Local Network
```
cabnet://192.168.1.100:8080
http://192.168.1.100:8080
{"host":"192.168.1.100","port":8080,"secure":false}
192.168.1.100:8080
```

#### Cloudflare Tunnel
```
cabnet://cabnet.example.com
https://cabnet.example.com
{"host":"cabnet.example.com","secure":true}
cabnet.example.com
```

### Manual QR Generation (Optional)

#### Using qrencode (Linux/macOS)

```bash
# Local network
qrencode -o server_qr.png "cabnet://192.168.1.100:8080"

# Cloudflare tunnel
qrencode -o server_qr.png "cabnet://cabnet.example.com"
```

#### Using Python

```python
import qrcode

# For local network
qr = qrcode.QRCode(version=1, box_size=10, border=5)
qr.add_data("cabnet://192.168.1.100:8080")
qr.make(fit=True)
img = qr.make_image(fill_color="black", back_color="white")
img.save("server_qr_local.png")

# For Cloudflare tunnel
qr = qrcode.QRCode(version=1, box_size=10, border=5)
qr.add_data("cabnet://cabnet.example.com")
qr.make(fit=True)
img = qr.make_image(fill_color="black", back_color="white")
img.save("server_qr_tunnel.png")
```

---

## Settings Reference

| Setting | Default | Description |
|---------|---------|-------------|
| `auto_sync` | `true` | Enable automatic sync when network available |
| `server_ip` | - | Server IP address |
| `server_port` | `8080` | Server port |
| `sync_interval` | `15 min` | WorkManager periodic sync interval |

---

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Server health check |
| `/api/connect` | POST | **Device connection** - Call when device connects |
| `/api/devices/heartbeat` | POST | **Keep-alive** - Call every 30-60 seconds |
| `/api/devices/register` | POST | Register/update device info |
| `/api/scans` | POST | Upload scans |
| `/api/jobs` | GET | Fetch jobs |
| `/api/jobs/{id}/barcodes` | GET | Fetch job barcodes |

### Recommended Connection Flow

```
┌─────────────────┐
│  App Starts     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│ POST /api/connect│────▶│ Server registers│
│                 │     │ device, returns │
│ {               │     │ server info     │
│   device_id,    │     └─────────────────┘
│   device_name,  │
│   model,        │
│   os_version,   │
│   app_version   │
│ }               │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     
│ Connection OK!  │     
│ Start heartbeat │     
└────────┬────────┘     
         │
         ▼
┌─────────────────────────────────────────────┐
│ Every 30 seconds: POST /api/devices/heartbeat│
│ { device_id: "abc123" }                      │
└─────────────────────────────────────────────┘
```

---

*Document Version: 1.1*

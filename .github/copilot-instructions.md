# CabNet Server – Copilot Instructions

## Project Overview

CabNet Server is a **Rust desktop application** that pairs with the CabNet Android barcode-scanning app. It runs an Axum HTTP API server and an egui/eframe native GUI **simultaneously** – the API on a Tokio runtime, the GUI on the main thread (see `src/main.rs`).

## Architecture (key mental model)

```
main.rs → spawns Tokio runtime
        → starts Axum HTTP server on runtime (background)
        → starts eframe/egui GUI on main thread
        → both share SharedState = Arc<RwLock<AppState>> (contains Repository + start_time)
```

- **`src/api/`** – Axum HTTP layer: `routes.rs` (router + `SharedState` type), `handlers.rs` (all endpoint logic + `ApiResponse<T>` pattern), `web.rs` (HTML dashboard/search pages served inline as raw strings).
- **`src/db/`** – SQLite via SQLx: `mod.rs` (pool init + inline migrations in `run_migrations()`), `models.rs` (DB row structs `FromRow` + API input structs), `repository.rs` (all CRUD, ~1800 lines).
- **`src/gui/`** – egui immediate-mode GUI: `app.rs` (`CodeBarApp` with `CachedData`, `TabStates`, `Theme`), `tabs/` (one file per tab: map, jobs, scans, devices, approvals, email, settings).
- **`src/services/`** – `email.rs` (SMTP via lettre), `tunnel.rs` (Cloudflare Tunnel management via `cloudflared` CLI subprocess).
- **`src/config.rs`** – `Config::from_env()` loads from `.env`/env vars; `get_data_dir()` resolves to `Documents/CabNet` on Windows.

## Build & Run

```powershell
# Prerequisites (one-time)
.\install-requirements.ps1          # Installs Rust toolchain + updates

# Build
cargo build --release               # or .\build.ps1 / .\build.bat
# Output: target\release\codebar-server.exe (also copied to root as CabNet-Server.exe by build.ps1)

# Run
cargo run --release                 # Starts API server on 0.0.0.0:8080 + GUI window
```

The VS Code task **"Build Release"** runs `cargo build --release`. There are no automated tests yet.

## Critical Patterns & Conventions

### Shared state access
All API handlers receive `State(state): State<SharedState>` and must acquire the `RwLock`:
```rust
let state = state.read().await;   // read-only access
let state = state.write().await;  // mutations
```

### API response pattern
Use the generic `ApiResponse<T>` wrapper from `handlers.rs`:
```rust
Ok(Json(ApiResponse::success(MyData { ... })))           // 200 success
Ok(Json(ApiResponse::success_with_message(data, "msg"))) // 200 + message
Err((StatusCode::BAD_REQUEST, Json(ApiResponse::<()>::error("reason"))))
```

### Database migrations
Migrations are **not** managed by sqlx-cli. They are inline `CREATE TABLE IF NOT EXISTS` statements in `src/db/mod.rs::run_migrations()`. When adding a table or column, add an `ALTER TABLE ... ADD COLUMN` block with error suppression (already-exists is OK).

### Model pair convention
Each entity has two structs in `src/db/models.rs`:
- `Foo` (derives `FromRow`) – represents a DB row, fields use `String` for dates, `i32`/`i64` for ints.
- `FooInput` (derives `Deserialize`) – represents incoming API JSON, timestamps are `i64` (Unix millis from Android), IDs are `String` (UUIDs).

### GUI tab pattern
Each tab in `src/gui/tabs/` is a struct implementing a `render(&mut self, ui: &mut egui::Ui, ...)` method. Tab state lives in `TabStates` in `app.rs`. Data displayed comes from `CachedData` which is periodically refreshed from the DB.

### Web pages are inline HTML
The dashboard, search page, email page, and map page in `src/api/web.rs` are **large raw HTML strings** (file is ~5700 lines). They use `Html(r##"..."##)` responses with embedded CSS/JS. No template engine.

### Host-based routing
`web::root_handler` inspects the `Host` header to decide between the public search page (main domain like `cabnetx.com`) and the dashboard (subdomains, localhost, or LAN IPs).

### Cloudflare Tunnel
Managed by `src/services/tunnel.rs` which spawns/kills `cloudflared` as a child process. Config persisted at `{data_dir}/tunnel_config.json`.

## JSON Conventions

- All API JSON uses **snake_case** (Android/Rust Serde compatibility).
- Timestamps from Android arrive as **Unix milliseconds** (`i64`), stored as **ISO 8601 / RFC 3339** strings in SQLite.
- UUIDs are strings; SQLite uses `INTEGER PRIMARY KEY AUTOINCREMENT` for row IDs and `TEXT UNIQUE` for UUIDs.

## Key File Reference

| Purpose | File |
|---------|------|
| App entry point + threading | `src/main.rs` |
| All API routes | `src/api/routes.rs` |
| All API handler logic | `src/api/handlers.rs` |
| DB schema + migrations | `src/db/mod.rs` |
| Data models (DB + API) | `src/db/models.rs` |
| All CRUD operations | `src/db/repository.rs` |
| GUI main loop + state | `src/gui/app.rs` |
| Config + data dir logic | `src/config.rs` |
| Web dashboard HTML | `src/api/web.rs` |

# PDF Logic Documentation

## Overview

CabNet supports uploading PDF files to jobs, extracting searchable text from them, and automatically parsing lading/shipping PDFs to associate ticket numbers with room numbers and descriptions. This enables users to scan a barcode ticket and instantly see what item it corresponds to and which room it belongs to.

---

## Architecture

```
┌──────────────┐    multipart/form-data    ┌──────────────────┐
│   iOS App    │ ─────────────────────────▶ │   Rust Server    │
│              │                            │                  │
│  • Pick PDF  │                            │  • Store in DB   │
│  • Upload    │                            │  • Extract text  │
│  • Search    │                            │  • Parse tickets │
│  • Enrich    │                            │  • Serve search  │
│    scans     │ ◀───────────────────────── │    results       │
└──────────────┘       JSON responses       └──────────────────┘
```

### Data Flow

1. **Upload**: User selects a PDF → iOS sends it as multipart → server stores the file blob, extracts text, and (for lading PDFs) parses ticket data.
2. **Search**: User types a room number or keyword → iOS queries the server → server does a `LIKE` search against extracted text → returns matching file names and text snippets.
3. **Lading Enrichment**: After uploading a lading PDF, the iOS app fetches parsed tickets and updates any local `ScanEntry` records that have matching ticket numbers with descriptions and room info.
4. **Scan Detail**: When viewing a scan, the detail view lazy-loads the ticket description from the server (if not already cached locally) and persists it to the local SwiftData model.

---

## Database Schema

### `job_files`

Stores uploaded PDFs (or any document) as blobs with extracted text for full-text search.

| Column                 | Type    | Description                                    |
|------------------------|---------|------------------------------------------------|
| `id`                   | INTEGER | Auto-increment primary key                     |
| `uuid`                 | TEXT    | Unique file identifier (UUID v4)               |
| `job_id`               | TEXT    | Foreign key to the job                         |
| `file_name`            | TEXT    | Original filename (e.g. `lading_123.pdf`)      |
| `content_type`         | TEXT    | MIME type, defaults to `application/pdf`       |
| `file_data`            | BLOB    | Raw file bytes                                 |
| `extracted_text`       | TEXT    | Text extracted from the PDF for search         |
| `file_size`            | INTEGER | File size in bytes                             |
| `uploaded_by_device_id`| TEXT    | Device that uploaded the file                  |
| `uploaded_by_name`     | TEXT    | User who uploaded the file                     |
| `created_at`           | TEXT    | ISO 8601 timestamp                             |

**Indexes**: `job_id`

### `lading_tickets`

Stores parsed ticket data from shipping/packing/lading PDFs. Each row represents one ticket line from the PDF.

| Column          | Type    | Description                                         |
|-----------------|---------|-----------------------------------------------------|
| `id`            | INTEGER | Auto-increment primary key                          |
| `job_id`        | TEXT    | Foreign key to the job                              |
| `job_file_uuid` | TEXT    | Foreign key to the `job_files` entry it came from   |
| `ticket_number` | TEXT    | The parsed ticket number (e.g. `000115`)            |
| `description`   | TEXT    | Item description (e.g. `SS Counter Top`)            |
| `room`          | TEXT    | Room number or name (e.g. `103`)                    |
| `qty`           | INTEGER | Quantity, defaults to 1                             |
| `section`       | TEXT    | Section header from the PDF (e.g. `HARDWARE TICKETS`)|
| `created_at`    | TEXT    | ISO 8601 timestamp                                  |

**Indexes**: `job_id`, `ticket_number`, `job_file_uuid`

---

## API Endpoints

### File Management

| Method   | Path                                      | Description                          |
|----------|-------------------------------------------|--------------------------------------|
| `POST`   | `/api/jobs/:job_id/files`                 | Upload a PDF (multipart/form-data)   |
| `GET`    | `/api/jobs/:job_id/files`                 | List all files for a job             |
| `GET`    | `/api/jobs/:job_id/files/:file_uuid`      | Download a file (returns raw bytes)  |
| `DELETE` | `/api/jobs/:job_id/files/:file_uuid`      | Delete a file                        |
| `GET`    | `/api/jobs/files/search?q=...&job_id=...` | Search file contents                 |

### Lading Tickets

| Method | Path                                          | Description                              |
|--------|-----------------------------------------------|------------------------------------------|
| `GET`  | `/api/jobs/:job_id/lading-tickets`            | List all parsed tickets for a job        |
| `GET`  | `/api/lading-tickets/:ticket_number?job_id=…` | Look up a single ticket by number        |

### Upload Request Format

```
POST /api/jobs/{job_id}/files
Content-Type: multipart/form-data; boundary={boundary}

--{boundary}
Content-Disposition: form-data; name="file"; filename="lading_report.pdf"
Content-Type: application/pdf

{PDF bytes}
--{boundary}
Content-Disposition: form-data; name="device_id"

{device_uuid}
--{boundary}
Content-Disposition: form-data; name="uploader_name"

{user_name}
--{boundary}--
```

### Upload Response

```json
{
  "success": true,
  "data": {
    "uuid": "a1b2c3d4-...",
    "job_id": "job_123",
    "file_name": "lading_report.pdf",
    "content_type": "application/pdf",
    "file_size": 245760,
    "uploaded_by_name": "John",
    "created_at": "2026-03-30T12:00:00Z"
  }
}
```

### Search Response

```json
{
  "success": true,
  "data": {
    "results": [
      {
        "file_uuid": "a1b2c3d4-...",
        "file_name": "lading_report.pdf",
        "job_id": "job_123",
        "snippet": "...SS Counter Top 103 1..."
      }
    ]
  }
}
```

### Lading Tickets Response

```json
{
  "success": true,
  "data": {
    "tickets": [
      {
        "id": 1,
        "job_id": "job_123",
        "job_file_uuid": "a1b2c3d4-...",
        "ticket_number": "000115",
        "description": "SS Counter Top",
        "room": "103",
        "qty": 1,
        "section": "HARDWARE TICKETS",
        "created_at": "2026-03-30T12:00:00Z"
      }
    ]
  }
}
```

---

## Server-Side Logic

### PDF Text Extraction (`extract_pdf_text`)

A lightweight, dependency-free text extractor that works directly on PDF byte streams. It uses two strategies:

1. **BT/ET Block Parsing** (primary): Scans the raw PDF bytes for `BT` (begin text) and `ET` (end text) operators, then extracts text from parenthesized string literals within those blocks. Handles escape sequences (`\n`, `\r`, `\t`, `\\`) and nested parentheses.

2. **ASCII Fallback**: If no text blocks are found, falls back to extracting any printable ASCII sequences longer than 8 characters, filtering out PDF structural keywords (`<<`, `/`, `stream`, `endstream`, `obj`, `endobj`).

**Limitations**:
- Does not decode compressed streams (FlateDecode, etc.) — only works on uncompressed PDF text.
- Does not handle CID fonts, Unicode CMaps, or ToUnicode mappings.
- Hex-encoded strings (`<48656C6C6F>`) are not decoded.

### Lading Ticket Parser (`parse_lading_tickets`)

Parses extracted PDF text into structured ticket records. Designed for shipping/packing/bill-of-lading documents with tabular data.

**Expected PDF format** (columns: Priority/Ticket, Description, Room, Qty):
```
HARDWARE TICKETS
Priority  Ticket  Description              Room  Qty
000115           SS Counter Top             103   1
000116           Base Cabinet 24in          103   2
000200           Wall Cabinet 30in          SINKS 1
```

**Parsing rules**:

1. **Section detection**: Lines containing `TICKETS`, `HARDWARE`, or `SHIPPING` (case-insensitive) are treated as section headers and stored in the `section` field of subsequent tickets.

2. **Header skip**: Lines containing both `PRIORITY` and `TICKET`, or containing `REPORT REF` or `RUN DATE`, are skipped.

3. **Ticket line recognition**: The first whitespace-separated token must be 3–8 digits (the ticket number). Lines starting with anything else are ignored.

4. **Multi-ticket skip**: If the second token starts with `*`, the line is treated as a multi-ticket grouping header and skipped.

5. **Field extraction** (right to left):
   - **Qty**: If the last token parses as an integer 0–9999, it's the quantity. Otherwise qty defaults to 1.
   - **Room**: If the second-to-last remaining token is ≤10 characters, purely alphanumeric, doesn't contain `.` or `-`, and there are still tokens left for a description, it's treated as the room.
   - **Description**: All remaining tokens between the ticket number and room/qty.

6. **Noise filter**: Lines where the description contains `"see inside"` are skipped.

**Auto-detection**: The upload handler triggers ticket parsing when the filename (case-insensitive) contains any of: `lading`, `shipping`, `packing`, `bol`.

### Content Search (`search_job_files`)

Performs a case-insensitive `LIKE` query against the `extracted_text` column. Returns the filename, file UUID, and a ~160-character snippet centered around the first match. Optionally filtered by `job_id`.

---

## iOS-Side Logic

### Upload Flow (`JobDetailView.uploadFile`)

1. User taps "Upload PDF" → `DocumentPicker` presents the system file picker (accepts PDF and generic data types).
2. Security-scoped resource access is obtained for the selected file.
3. File bytes are read into `Data` and sent via `APIService.uploadJobFile()` as a multipart request.
4. On success, the file list is refreshed.
5. **Lading enrichment**: If the filename contains `lading`, `shipping`, `packing`, or `bol`, `enrichScansFromLadingTickets()` is called.

### Scan Enrichment (`JobDetailView.enrichScansFromLadingTickets`)

After a lading PDF upload:

1. Fetches all parsed lading tickets for the job from the server (`GET /api/jobs/:job_id/lading-tickets`).
2. Builds an in-memory dictionary keyed by ticket number.
3. Fetches all local `ScanEntry` records for the job from SwiftData.
4. For each scan that has a ticket number matching a parsed ticket, writes the `ticketDescription` and `ticketRoom` fields (if not already populated).
5. Saves the SwiftData context.

This is a best-effort operation — errors are silently ignored.

### Lazy Ticket Lookup (`ScanDetailView.fetchTicketDescription`)

When the user opens a scan detail view:

1. If `scan.ticketDescription` is already populated (from prior enrichment or a previous view), the cached value is displayed immediately.
2. Otherwise, if the scan has a non-empty `ticketNumber`, the view calls `GET /api/lading-tickets/:ticket_number?job_id=…`.
3. If a matching ticket is found, the description and room are displayed and persisted to the local `ScanEntry` model so future views are instant.

### ScanEntry Model Fields

| Field              | Type   | Description                                     |
|--------------------|--------|-------------------------------------------------|
| `ticketDescription`| String | Cached item description from lading PDF         |
| `ticketRoom`       | String | Cached room number/name from lading PDF         |

Both default to `""` and are populated either by bulk enrichment after upload or by lazy fetch on scan detail view.

### Display

- **ScanRowView** (scan list): Shows `ticketDescription` below the ticket number when available, and a door icon with `ticketRoom` if present.
- **ScanDetailView** (scan detail): Shows a "Ticket Info" section with the full description and room, with a loading spinner while fetching from the server.

---

## File Locations

### Server (CabNet-Server)

| File | Contains |
|------|----------|
| `src/api/handlers.rs` | `upload_job_file`, `get_job_files`, `download_job_file`, `delete_job_file`, `search_job_files`, `get_lading_tickets`, `get_ticket_description`, `extract_pdf_text`, `parse_lading_tickets` |
| `src/api/routes.rs` | Route definitions for all file and ticket endpoints |
| `src/db/mod.rs` | `CREATE TABLE` for `job_files` and `lading_tickets` |
| `src/db/models.rs` | `JobFileRecord`, `JobFileMeta`, `JobFileSearchResult`, `LadingTicket` structs |
| `src/db/repository.rs` | `insert_job_file`, `get_job_files`, `get_job_file_data`, `delete_job_file`, `search_job_files`, `insert_lading_tickets`, `get_lading_tickets_for_job`, `get_lading_ticket_description`, `delete_lading_tickets_for_file` |

### iOS (gonk / CabNet)

| File | Contains |
|------|----------|
| `CabNet/Services/APIService.swift` | `uploadJobFile`, `getJobFiles`, `downloadJobFile`, `deleteJobFile`, `searchJobFiles`, `getLadingTickets`, `getTicketDescription` |
| `CabNet/Models/APIModels.swift` | `JobFileMeta`, `JobFileUploadResponse`, `JobFilesResponse`, `JobFileSearchResult`, `JobFileSearchResponse`, `LadingTicket`, `LadingTicketsResponse`, `TicketDescriptionResponse` |
| `CabNet/Models/ScanEntry.swift` | `ticketDescription`, `ticketRoom` fields |
| `CabNet/Views/Jobs/JobDetailView.swift` | `uploadFile`, `enrichScansFromLadingTickets`, file picker integration |
| `CabNet/Views/History/ScanDetailView.swift` | `fetchTicketDescription`, ticket info display |
| `CabNet/Views/History/ScanHistoryView.swift` | `ScanRowView` with description/room display |

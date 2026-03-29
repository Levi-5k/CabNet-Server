//! Web dashboard for CabNet Server
//! Serves a responsive web interface on the root domain

use axum::{
    extract::{Host, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use chrono::{DateTime, Utc};

use super::routes::SharedState;

/// GET / - Route handler that serves dashboard on main domain, search page on subdomain
pub async fn root_handler(
    Host(host): Host,
    State(state): State<SharedState>,
) -> Response {
    // Check if this is a search subdomain (e.g., scans.cabnetx.com)
    let is_search_subdomain = host.starts_with("scans.");
    
    if is_search_subdomain {
        // scans.* subdomain - serve search page
        search_page().await.into_response()
    } else {
        // Main domain, localhost, LAN IPs, other subdomains - serve dashboard
        dashboard_inner(&state).await.into_response()
    }
}

/// GET / - Ticket search page for the public domain
pub async fn search_page() -> impl IntoResponse {
    Html(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CabNet - Ticket Search</title>
    <link rel="icon" type="image/png" href="/logo.png">
    <style>
        :root {
            --bg: #0f172a;
            --card: #1e293b;
            --card-hover: #334155;
            --accent: #6366f1;
            --accent-glow: rgba(99, 102, 241, 0.3);
            --green: #22c55e;
            --red: #ef4444;
            --orange: #fb923c;
            --text: #f8fafc;
            --text-muted: #94a3b8;
            --border: #334155;
        }
        
        * { box-sizing: border-box; margin: 0; padding: 0; }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg);
            color: var(--text);
            min-height: 100vh;
            line-height: 1.6;
        }
        
        .container {
            max-width: 900px;
            margin: 0 auto;
            padding: 2rem;
        }
        
        header {
            text-align: center;
            margin-bottom: 2rem;
            padding-top: 2rem;
        }
        
        .logo {
            font-size: 3rem;
            margin-bottom: 0.5rem;
        }
        
        h1 {
            font-size: 2rem;
            font-weight: 700;
            margin-bottom: 0.5rem;
        }
        
        .subtitle {
            color: var(--text-muted);
        }
        
        /* Search Card */
        .search-card {
            background: var(--card);
            border: 1px solid var(--border);
            border-radius: 1rem;
            padding: 2rem;
            margin-bottom: 2rem;
        }
        
        .form-group {
            margin-bottom: 1.5rem;
        }
        
        label {
            display: block;
            margin-bottom: 0.5rem;
            font-weight: 500;
            color: var(--text-muted);
        }
        
        select, input {
            width: 100%;
            padding: 0.875rem 1rem;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 0.5rem;
            color: var(--text);
            font-size: 1rem;
            transition: border-color 0.2s, box-shadow 0.2s;
        }
        
        select:focus, input:focus {
            outline: none;
            border-color: var(--accent);
            box-shadow: 0 0 0 3px var(--accent-glow);
        }
        
        select option {
            background: var(--card);
            color: var(--text);
        }
        
        .search-row {
            display: flex;
            gap: 1rem;
        }
        
        .search-row .form-group {
            flex: 1;
            margin-bottom: 0;
        }
        
        .btn {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
            padding: 0.875rem 1.5rem;
            border-radius: 0.5rem;
            font-weight: 600;
            font-size: 1rem;
            border: none;
            cursor: pointer;
            transition: all 0.2s;
        }
        
        .btn-primary {
            background: var(--accent);
            color: white;
            width: 100%;
            margin-top: 1rem;
        }
        
        .btn-primary:hover {
            background: #4f46e5;
            transform: translateY(-1px);
        }
        
        .btn-primary:disabled {
            opacity: 0.6;
            cursor: not-allowed;
            transform: none;
        }
        
        /* Results */
        .results {
            background: var(--card);
            border: 1px solid var(--border);
            border-radius: 1rem;
            overflow: hidden;
        }
        
        .results-header {
            padding: 1rem 1.5rem;
            background: var(--card-hover);
            border-bottom: 1px solid var(--border);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .results-header h2 {
            font-size: 1.1rem;
            font-weight: 600;
        }
        
        .results-count {
            color: var(--text-muted);
            font-size: 0.9rem;
        }
        
        .results-list {
            max-height: 500px;
            overflow-y: auto;
        }
        
        .result-item {
            padding: 1rem 1.5rem;
            border-bottom: 1px solid var(--border);
            display: flex;
            justify-content: space-between;
            align-items: center;
            transition: background 0.2s;
        }
        
        .result-item:last-child {
            border-bottom: none;
        }
        
        .result-item:hover {
            background: var(--card-hover);
        }
        
        .result-info {
            flex: 1;
        }
        
        .ticket-number {
            font-size: 1.1rem;
            font-weight: 600;
            font-family: 'Monaco', 'Menlo', monospace;
            color: var(--accent);
        }
        
        .barcode {
            color: var(--text-muted);
            font-size: 0.85rem;
            font-family: 'Monaco', 'Menlo', monospace;
        }
        
        .result-meta {
            text-align: right;
            color: var(--text-muted);
            font-size: 0.85rem;
        }
        
        .status-badge {
            display: inline-block;
            padding: 0.25rem 0.75rem;
            border-radius: 9999px;
            font-size: 0.75rem;
            font-weight: 600;
        }
        
        .status-found {
            background: rgba(34, 197, 94, 0.2);
            color: var(--green);
        }
        
        .status-not-found {
            background: rgba(239, 68, 68, 0.2);
            color: var(--red);
        }
        
        /* Empty/Loading States */
        .empty-state {
            text-align: center;
            padding: 3rem 2rem;
            color: var(--text-muted);
        }
        
        .empty-state .icon {
            font-size: 3rem;
            margin-bottom: 1rem;
        }
        
        .loading {
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
            padding: 2rem;
            color: var(--text-muted);
        }
        
        .spinner {
            width: 20px;
            height: 20px;
            border: 2px solid var(--border);
            border-top-color: var(--accent);
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
        }
        
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
        
        /* No results message */
        .not-found-message {
            background: rgba(239, 68, 68, 0.1);
            border: 1px solid rgba(239, 68, 68, 0.3);
            border-radius: 0.5rem;
            padding: 1rem 1.5rem;
            margin: 1rem 1.5rem;
            color: var(--red);
            text-align: center;
        }
        
        /* Footer */
        footer {
            text-align: center;
            padding: 2rem;
            color: var(--text-muted);
            font-size: 0.85rem;
        }
        
        @media (max-width: 640px) {
            .container { padding: 1rem; }
            .search-row { flex-direction: column; }
            .search-card { padding: 1.5rem; }
            .result-item { flex-direction: column; align-items: flex-start; gap: 0.5rem; }
            .result-meta { text-align: left; }
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="logo">🔍</div>
            <h1>Ticket Search</h1>
            <p class="subtitle">Search for scanned tickets by job</p>
        </header>
        
        <div class="search-card">
            <div class="form-group">
                <label for="job-select">Select Job</label>
                <select id="job-select">
                    <option value="">Loading jobs...</option>
                </select>
            </div>
            
            <div class="search-row">
                <div class="form-group">
                    <label for="search-input">Ticket Number</label>
                    <input type="text" id="search-input" placeholder="Enter ticket number to search...">
                </div>
            </div>
            
            <button class="btn btn-primary" id="search-btn" disabled>
                🔍 Search
            </button>
        </div>
        
        <div class="results" id="results" style="display: none;">
            <div class="results-header">
                <h2>Search Results</h2>
                <span class="results-count" id="results-count"></span>
            </div>
            <div class="results-list" id="results-list"></div>
        </div>
        
        <footer>
            CabNet Ticket Search
        </footer>
    </div>
    
    <script>
        const jobSelect = document.getElementById('job-select');
        const searchInput = document.getElementById('search-input');
        const searchBtn = document.getElementById('search-btn');
        const resultsDiv = document.getElementById('results');
        const resultsList = document.getElementById('results-list');
        const resultsCount = document.getElementById('results-count');
        
        let allScans = [];
        let currentJobId = null;
        
        // Load jobs on page load
        async function loadJobs() {
            try {
                const response = await fetch('/api/jobs');
                const data = await response.json();
                
                if (data.jobs && data.jobs.length > 0) {
                    jobSelect.innerHTML = '<option value="">Select a job...</option>';
                    data.jobs.forEach(job => {
                        const option = document.createElement('option');
                        option.value = job.id;
                        option.textContent = job.name + (job.reference_number ? ` (${job.reference_number})` : '');
                        jobSelect.appendChild(option);
                    });
                    searchBtn.disabled = true;
                } else {
                    jobSelect.innerHTML = '<option value="">No jobs found</option>';
                }
            } catch (error) {
                console.error('Failed to load jobs:', error);
                jobSelect.innerHTML = '<option value="">Error loading jobs</option>';
            }
        }
        
        // Load scans for selected job
        async function loadScans(jobId) {
            try {
                resultsList.innerHTML = '<div class="loading"><div class="spinner"></div>Loading scans...</div>';
                resultsDiv.style.display = 'block';
                
                const response = await fetch(`/api/scans?job_id=${jobId}`);
                const data = await response.json();
                
                allScans = data.scans || [];
                currentJobId = jobId;
                
                if (allScans.length > 0) {
                    showScans(allScans);
                    searchBtn.disabled = false;
                } else {
                    resultsList.innerHTML = '<div class="empty-state"><div class="icon">📭</div><p>No scans in this job yet</p></div>';
                    searchBtn.disabled = true;
                }
            } catch (error) {
                console.error('Failed to load scans:', error);
                resultsList.innerHTML = '<div class="empty-state"><div class="icon">❌</div><p>Error loading scans</p></div>';
            }
        }
        
        // Show scans in list
        function showScans(scans) {
            resultsCount.textContent = `${scans.length} ticket${scans.length !== 1 ? 's' : ''}`;
            
            if (scans.length === 0) {
                resultsList.innerHTML = '<div class="not-found-message">❌ No matching tickets found</div>';
                return;
            }
            
            resultsList.innerHTML = scans.map(scan => `
                <div class="result-item">
                    <div class="result-info">
                        <div class="ticket-number">${scan.ticket_number || scan.barcode}</div>
                        ${scan.ticket_number ? `<div class="barcode">Barcode: ${scan.barcode}</div>` : ''}
                    </div>
                    <div class="result-meta">
                        <span class="status-badge status-found">✓ Found</span>
                        <div style="margin-top: 0.25rem">${formatDate(scan.scanned_at)}</div>
                    </div>
                </div>
            `).join('');
        }
        
        // Search within loaded scans
        function searchScans(query) {
            if (!query.trim()) {
                showScans(allScans);
                return;
            }
            
            const q = query.toLowerCase().trim();
            const filtered = allScans.filter(scan => 
                (scan.ticket_number && scan.ticket_number.toLowerCase().includes(q)) ||
                scan.barcode.toLowerCase().includes(q)
            );
            
            showScans(filtered);
            
            if (filtered.length === 0 && query.trim()) {
                resultsCount.textContent = '0 tickets';
                resultsList.innerHTML = `
                    <div class="not-found-message">
                        ❌ Ticket "${query}" not found in this job
                    </div>
                `;
            }
        }
        
        // Format date (MM/DD/YYYY)
        function formatDate(dateStr) {
            try {
                const date = new Date(dateStr);
                return date.toLocaleDateString('en-US') + ' ' + date.toLocaleTimeString('en-US', {hour: '2-digit', minute:'2-digit'});
            } catch {
                return dateStr;
            }
        }
        
        // Helper for date only (MM/DD/YYYY)
        function formatDateOnly(date) {
            return date.toLocaleDateString('en-US', {month: '2-digit', day: '2-digit', year: 'numeric'});
        }
        
        // Helper for time only (12h format)
        function formatTimeOnly(date) {
            return date.toLocaleTimeString('en-US', {hour: '2-digit', minute:'2-digit'});
        }
        
        // Helper for full date+time (MM/DD/YYYY HH:MM AM/PM)
        function formatDateTime(date) {
            return formatDateOnly(date) + ' ' + formatTimeOnly(date);
        }
        
        // Event listeners
        jobSelect.addEventListener('change', (e) => {
            if (e.target.value) {
                loadScans(e.target.value);
                searchInput.value = '';
            } else {
                resultsDiv.style.display = 'none';
                allScans = [];
                searchBtn.disabled = true;
            }
        });
        
        searchBtn.addEventListener('click', () => {
            searchScans(searchInput.value);
        });
        
        searchInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                searchScans(searchInput.value);
            }
        });
        
        searchInput.addEventListener('input', (e) => {
            // Live search as user types
            if (allScans.length > 0) {
                searchScans(e.target.value);
            }
        });
        
        // Initialize
        loadJobs();
    </script>
</body>
</html>
"##.to_string())
}

/// GET /dashboard - Server dashboard with stats
pub async fn dashboard(State(state): State<SharedState>) -> impl IntoResponse {
    dashboard_inner(&state).await
}

/// Inner dashboard function that can be called without extractors
async fn dashboard_inner(state: &SharedState) -> Html<String> {
    let state = state.read().await;
    
    // Get stats
    let device_count = state.repo.get_devices().await.map(|d| d.len()).unwrap_or(0);
    let scan_count = state.repo.get_scan_count().await.unwrap_or(0) as usize;
    let job_count = state.repo.get_jobs().await.map(|j| j.len()).unwrap_or(0);
    let uptime_secs = state.start_time.elapsed().as_secs();
    let recent_scans = state.repo.get_scans(None, None, None, 5).await.unwrap_or_default();
    let jobs_with_counts = state.repo.get_jobs_with_counts().await.unwrap_or_default();
    let active_workers = state.repo.get_active_time_entries().await.unwrap_or_default();
    let report_count = state.repo.get_report_count().await.unwrap_or(0);
    let active_worker_count = active_workers.len();
    let uptime = format_uptime(uptime_secs);
    let version = env!("CARGO_PKG_VERSION");
    
    // Generate recent scans HTML
    let recent_scans_html = if recent_scans.is_empty() {
        "<p style='color: var(--text-muted); font-style: italic;'>No recent scans</p>".to_string()
    } else {
        let mut scan_htmls = Vec::new();
        for scan in &recent_scans {
            let time_ago = format_time_ago(&scan.scanned_at);
            let barcode_short = if scan.barcode.len() > 20 {
                format!("{}...", &scan.barcode[..17])
            } else {
                scan.barcode.clone()
            };
            // Resolve job name: first try job_id, then barcode_job_ref → reference_number
            let job_name = if let Some(job_id) = scan.job_id {
                state.repo.get_job(job_id).await.ok().flatten().map(|j| j.name)
            } else {
                None
            };
            let job_name = job_name.or_else(|| {
                scan.barcode_job_ref.as_deref()
                    .filter(|r| !r.is_empty())
                    .and_then(|ref_num| {
                        jobs_with_counts.iter().find(|j| j.job.reference_number.as_deref() == Some(ref_num)).map(|j| j.job.name.clone())
                    })
            });
            let job_label = job_name.unwrap_or_else(|| "Unassigned".to_string());
            scan_htmls.push(format!(r##"<div style="display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0; border-bottom: 1px solid var(--border);">
                <div>
                    <div style="font-weight: 500;">{}</div>
                    <div style="color: var(--text-muted); font-size: 0.8rem;">Device: {} • {}</div>
                </div>
                <div style="color: var(--text-muted); font-size: 0.8rem;">{}</div>
            </div>"##, barcode_short, scan.device_id, job_label, time_ago));
        }
        scan_htmls.join("")
    };
    
    // Generate jobs status HTML
    let jobs_status_html = if jobs_with_counts.is_empty() {
        "<p style='color: var(--text-muted); font-style: italic;'>No jobs created yet</p>".to_string()
    } else {
        jobs_with_counts.iter().take(5).map(|job| {
            let progress_percent = if job.job.expected_count > 0 {
                (job.scan_count as f32 / job.job.expected_count as f32 * 100.0) as i32
            } else {
                0
            };
            let status_color = if progress_percent >= 100 { "var(--success)" } else if progress_percent > 50 { "var(--warning)" } else { "var(--text-muted)" };
            format!(r##"<div style="margin-bottom: 1rem;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.25rem;">
                    <span style="font-weight: 500; font-size: 0.9rem;">{}</span>
                    <span style="color: {}; font-size: 0.8rem;">{} / {}</span>
                </div>
                <div style="width: 100%; height: 6px; background: var(--border); border-radius: 3px; overflow: hidden;">
                    <div style="width: {}%; height: 100%; background: {}; border-radius: 3px;"></div>
                </div>
            </div>"##, job.job.name, status_color, job.scan_count, job.job.expected_count, progress_percent, status_color)
        }).collect::<Vec<_>>().join("")
    };
    
    Html(format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CabNet Control Center</title>
    <link rel="icon" type="image/png" href="/logo.png">
    <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
    <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
    <style>
        :root {{
            --bg: #0a0e1a;
            --bg-gradient: linear-gradient(135deg, #0a0e1a 0%, #111827 50%, #0f172a 100%);
            --card: rgba(30, 41, 59, 0.45);
            --card-solid: rgba(30, 41, 59, 0.85);
            --card-hover: rgba(51, 65, 85, 0.4);
            --accent: #6366f1;
            --accent-hover: #818cf8;
            --accent-glow: rgba(99, 102, 241, 0.25);
            --accent-subtle: rgba(99, 102, 241, 0.08);
            --text: #f1f5f9;
            --text-muted: #94a3b8;
            --success: #22c55e;
            --warning: #f59e0b;
            --danger: #ef4444;
            --border: rgba(255, 255, 255, 0.08);
            --radius: 1rem;
            --glass: rgba(15, 23, 42, 0.3);
            --shadow: 0 1px 3px rgba(0, 0, 0, 0.08), 0 4px 16px rgba(0, 0, 0, 0.06);
            --shadow-lg: 0 2px 8px rgba(0, 0, 0, 0.08), 0 12px 40px rgba(0, 0, 0, 0.1);
            --font-size: 1rem;
            --transition: 0.2s cubic-bezier(0.4, 0, 0.2, 1);
            /* Liquid Glass (iOS 26 style) */
            --glass-edge: rgba(255, 255, 255, 0.06);
            --glass-edge-strong: rgba(255, 255, 255, 0.1);
            --glass-inner: inset 0 0.5px 0 0 rgba(255, 255, 255, 0.1);
            --glass-shine: none;
            --glass-border: 0.5px solid var(--glass-edge);
            --glass-blur: blur(40px);
        }}
        
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        
        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg);
            background-image: var(--bg-gradient);
            background-attachment: fixed;
            color: var(--text);
            min-height: 100vh;
            line-height: 1.6;
            font-size: var(--font-size);
        }}
        
        .layout {{
            display: flex;
            min-height: 100vh;
        }}
        
        /* Sidebar */
        .sidebar {{
            width: 260px;
            background: rgba(15, 23, 42, 0.35);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border-right: 0.5px solid var(--glass-edge);
            padding: 1.5rem;
            display: flex;
            flex-direction: column;
            position: fixed;
            height: 100vh;
            overflow-y: auto;
            z-index: 100;
            box-shadow: var(--glass-inner), 1px 0 30px rgba(0, 0, 0, 0.06);
        }}
        
        .logo {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
            margin-bottom: 2rem;
            padding-bottom: 1rem;
            border-bottom: 1px solid var(--border);
        }}
        
        .logo-icon {{ width: 36px; height: 36px; border-radius: 8px; filter: drop-shadow(0 0 8px var(--accent-glow)); object-fit: contain; }}
        .logo h1 {{ font-size: 1.25rem; font-weight: 700; background: linear-gradient(135deg, var(--accent), #a78bfa); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }}
        
        .nav {{ flex: 1; }}
        
        .nav-item {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.5rem 0.75rem;
            border-radius: var(--radius);
            color: var(--text-muted);
            cursor: pointer;
            transition: all var(--transition);
            margin-bottom: 0.125rem;
            font-size: 0.875rem;
        }}
        
        .nav-item:hover {{ background: rgba(255, 255, 255, 0.03); color: var(--text); }}
        .nav-item.active {{ background: linear-gradient(135deg, var(--accent), var(--accent-hover)); color: white; box-shadow: 0 2px 12px var(--accent-glow); position: relative; overflow: hidden; }}
        
        .nav-badge {{
            margin-left: auto;
            background: var(--danger);
            color: white;
            font-size: 0.7rem;
            padding: 0.15rem 0.5rem;
            border-radius: 9999px;
            font-weight: 600;
        }}
        
        .sidebar-footer {{
            padding-top: 1rem;
            border-top: 1px solid var(--border);
            font-size: 0.8rem;
            color: var(--text-muted);
        }}
        .settings-btn {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.5rem 0.75rem;
            border-radius: var(--radius);
            color: var(--text-muted);
            cursor: pointer;
            transition: all var(--transition);
            font-size: 0.85rem;
            margin-bottom: 0.75rem;
            border: 1px solid transparent;
            background: none;
            width: 100%;
            text-align: left;
        }}
        .settings-btn:hover {{ background: rgba(255, 255, 255, 0.03); color: var(--text); }}
        
        .trust-status {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.75rem;
            border-radius: 0.5rem;
            margin-bottom: 1rem;
            font-size: 0.85rem;
        }}
        
        .trust-status.trusted {{ background: rgba(34, 197, 94, 0.08); color: var(--success); border: 0.5px solid rgba(34, 197, 94, 0.1); }}
        .trust-status.untrusted {{ background: rgba(251, 146, 60, 0.08); color: var(--warning); border: 0.5px solid rgba(251, 146, 60, 0.1); }}
        
        /* Main Content */
        .main {{
            flex: 1;
            margin-left: 260px;
            padding: 1rem 1.5rem;
        }}
        
        .page {{ display: none; }}
        .page.active {{ display: block; }}
        
        .page-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1rem;
        }}
        
        .page-header h2 {{
            font-size: 1.25rem;
            font-weight: 700;
        }}
        
        /* Stats Grid - Compact */
        .stats {{
            display: grid;
            grid-template-columns: repeat(6, 1fr);
            gap: 0.75rem;
            margin-bottom: 1rem;
        }}
        
        .stat-card {{
            background: rgba(30, 41, 59, 0.3);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            padding: 0.75rem;
            text-align: center;
            transition: all var(--transition);
            position: relative;
            overflow: hidden;
            box-shadow: var(--glass-inner), 0 1px 3px rgba(0,0,0,0.06), 0 4px 16px rgba(0,0,0,0.04);
        }}
        .stat-card:hover {{ transform: translateY(-2px); box-shadow: var(--glass-inner), 0 2px 8px rgba(0,0,0,0.08), 0 12px 40px rgba(0,0,0,0.08); }}
        .stat-icon {{ font-size: 1.25rem; margin-bottom: 0.125rem; }}
        .stat-value {{ font-size: 1.25rem; font-weight: 700; color: var(--accent); }}
        .stat-label {{ color: var(--text-muted); font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.5px; }}
        
        /* Cards Grid - Compact Layout */
        .cards-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 0.75rem;
            margin-bottom: 1rem;
        }}
        
        @media (max-width: 1200px) {{
            .cards-grid {{
                grid-template-columns: 1fr;
            }}
        }}
        
        /* Cards and Tables - Compact */
        .card {{
            background: rgba(30, 41, 59, 0.3);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            overflow: hidden;
            margin-bottom: 0.75rem;
            transition: border-color var(--transition), box-shadow var(--transition), transform var(--transition);
            box-shadow: var(--glass-inner), 0 1px 3px rgba(0,0,0,0.06), 0 4px 16px rgba(0,0,0,0.04);
            position: relative;
        }}
        .card:hover {{
            border-color: var(--glass-edge-strong);
            box-shadow: var(--glass-inner), 0 2px 8px rgba(0,0,0,0.08), 0 12px 40px rgba(0,0,0,0.08);
        }}
        
        .card-header {{
            padding: 0.5rem 0.75rem;
            background: rgba(51, 65, 85, 0.12);
            border-bottom: 0.5px solid rgba(255, 255, 255, 0.04);
            display: flex;
            justify-content: space-between;
            align-items: center;
            position: relative;
            z-index: 1;
        }}
        
        .card-header h3 {{ font-size: 0.8rem; font-weight: 600; }}
        
        .card-body {{ padding: 0.75rem; position: relative; z-index: 1; }}
        
        table {{
            width: 100%;
            border-collapse: collapse;
            font-size: 0.8rem;
        }}
        
        th, td {{
            padding: 0.5rem 0.625rem;
            text-align: left;
            border-bottom: 1px solid rgba(255, 255, 255, 0.04);
        }}
        
        th {{
            background: rgba(51, 65, 85, 0.1);
            font-weight: 600;
            font-size: 0.7rem;
            text-transform: uppercase;
            color: var(--text-muted);
        }}
        
        tr:hover td {{ background: rgba(99, 102, 241, 0.04); }}
        
        /* Buttons - Compact */
        .btn {{
            display: inline-flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
            padding: 0.625rem 1rem;
            border-radius: var(--radius);
            font-weight: 600;
            font-size: 0.875rem;
            border: none;
            cursor: pointer;
            transition: all var(--transition);
        }}
        
        .btn-primary {{ background: linear-gradient(135deg, var(--accent), var(--accent-hover)); color: white; box-shadow: 0 2px 8px var(--accent-glow); position: relative; overflow: hidden; }}
        .btn-primary:hover {{ box-shadow: 0 4px 20px var(--accent-glow); transform: translateY(-1px); }}
        .btn-success {{ background: var(--success); color: white; }}
        .btn-success:hover {{ opacity: 0.9; }}
        .btn-danger {{ background: var(--danger); color: white; }}
        .btn-danger:hover {{ opacity: 0.9; }}
        .btn-accent {{ background: var(--accent); color: white; border: none; cursor: pointer; border-radius: 4px; }}
        .btn-accent:hover {{ opacity: 0.9; }}
        .btn-outline {{
            background: rgba(255, 255, 255, 0.03);
            border: 0.5px solid var(--glass-edge);
            color: var(--text);
            backdrop-filter: blur(20px);
            -webkit-backdrop-filter: blur(20px);
        }}
        .btn-outline:hover {{ background: rgba(255, 255, 255, 0.06); border-color: var(--glass-edge-strong); }}
        .btn-sm {{ padding: 0.375rem 0.75rem; font-size: 0.8rem; }}
        
        /* Forms */
        .form-group {{ margin-bottom: 1rem; }}
        
        .form-group label {{
            display: block;
            margin-bottom: 0.5rem;
            font-weight: 500;
            color: var(--text-muted);
            font-size: 0.9rem;
        }}
        
        input, select, textarea {{
            width: 100%;
            padding: 0.75rem 1rem;
            background: rgba(15, 23, 42, 0.2);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            color: var(--text);
            font-size: 0.9rem;
            transition: all var(--transition);
            backdrop-filter: blur(20px);
            -webkit-backdrop-filter: blur(20px);
        }}
        
        input:focus, select:focus, textarea:focus {{
            outline: none;
            border-color: var(--accent);
            box-shadow: 0 0 0 3px var(--accent-glow);
        }}
        
        /* Modal */
        .modal {{
            display: none;
            position: fixed;
            inset: 0;
            background: rgba(0, 0, 0, 0.3);
            backdrop-filter: blur(8px);
            -webkit-backdrop-filter: blur(8px);
            z-index: 1000;
            align-items: center;
            justify-content: center;
            animation: modalBgIn 0.2s ease;
        }}
        
        .modal.active {{ display: flex; }}
        
        @keyframes modalBgIn {{ from {{ opacity: 0; }} to {{ opacity: 1; }} }}
        @keyframes modalContentIn {{ from {{ opacity: 0; transform: scale(0.95) translateY(10px); }} to {{ opacity: 1; transform: scale(1) translateY(0); }} }}
        
        .modal-content {{
            background: rgba(30, 41, 59, 0.5);
            backdrop-filter: blur(50px);
            -webkit-backdrop-filter: blur(50px);
            border: 0.5px solid var(--glass-edge-strong);
            border-radius: 1.5rem;
            width: 90%;
            max-width: 500px;
            max-height: 90vh;
            overflow-y: auto;
            box-shadow: var(--glass-inner), 0 8px 40px rgba(0,0,0,0.12), 0 24px 80px rgba(0,0,0,0.08);
            animation: modalContentIn 0.25s ease;
            position: relative;
        }}
        
        .modal-header {{
            padding: 1.25rem 1.5rem;
            border-bottom: 0.5px solid rgba(255, 255, 255, 0.05);
            display: flex;
            justify-content: space-between;
            align-items: center;
            position: relative;
            z-index: 1;
        }}
        
        .modal-header h3 {{ font-size: 1.1rem; font-weight: 600; }}
        
        .modal-close {{
            background: none;
            border: none;
            color: var(--text-muted);
            font-size: 1.5rem;
            cursor: pointer;
            padding: 0;
            line-height: 1;
        }}
        
        .modal-close:hover {{ color: var(--text); }}
        
        .modal-body {{ padding: 1.5rem; }}
        .modal-footer {{
            padding: 1rem 1.5rem;
            border-top: 1px solid var(--border);
            display: flex;
            justify-content: flex-end;
            gap: 0.75rem;
        }}
        
        /* Status badges */
        .badge {{
            display: inline-block;
            padding: 0.25rem 0.625rem;
            border-radius: 9999px;
            font-size: 0.75rem;
            font-weight: 600;
        }}
        
        .badge-success {{ background: rgba(34, 197, 94, 0.2); color: var(--success); }}
        .badge-warning {{ background: rgba(251, 146, 60, 0.2); color: var(--warning); }}
        .badge-danger {{ background: rgba(239, 68, 68, 0.2); color: var(--danger); }}
        .badge-info {{ background: rgba(99, 102, 241, 0.2); color: var(--accent); }}
        
        /* Toast notifications */
        .toast-container {{
            position: fixed;
            bottom: 1.5rem;
            right: 1.5rem;
            z-index: 2000;
        }}
        
        .toast {{
            background: rgba(30, 41, 59, 0.4);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            padding: 1rem 1.25rem;
            margin-top: 0.5rem;
            display: flex;
            align-items: center;
            gap: 0.75rem;
            animation: slideIn 0.3s ease;
            box-shadow: var(--glass-inner), 0 2px 8px rgba(0,0,0,0.08), 0 8px 32px rgba(0,0,0,0.06);
            position: relative;
            overflow: hidden;
        }}
        
        .toast.success {{ border-left: 4px solid var(--success); }}
        .toast.error {{ border-left: 4px solid var(--danger); }}
        .toast.warning {{ border-left: 4px solid var(--warning); }}
        
        @keyframes slideIn {{
            from {{ transform: translateX(100%); opacity: 0; }}
            to {{ transform: translateX(0); opacity: 1; }}
        }}
        
        /* Empty state */
        .empty-state {{
            text-align: center;
            padding: 3rem 2rem;
            color: var(--text-muted);
        }}
        
        .empty-state .icon {{ font-size: 3rem; margin-bottom: 1rem; }}
        
        /* Loading */
        .loading {{
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.75rem;
            padding: 2rem;
            color: var(--text-muted);
        }}
        
        .spinner {{
            width: 20px;
            height: 20px;
            border: 2px solid var(--border);
            border-top-color: var(--accent);
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
        }}
        
        @keyframes spin {{ to {{ transform: rotate(360deg); }} }}
        
        /* Pending change card */
        .change-card {{
            background: rgba(10, 14, 26, 0.3);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            padding: 1rem;
            margin-bottom: 0.75rem;
            box-shadow: var(--glass-inner);
        }}
        
        .change-header {{
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            margin-bottom: 0.75rem;
        }}
        
        .change-type {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        
        .change-data {{
            background: var(--card);
            padding: 0.75rem;
            border-radius: 0.5rem;
            font-family: monospace;
            font-size: 0.8rem;
            white-space: pre-wrap;
            word-break: break-all;
            margin-bottom: 0.75rem;
        }}
        
        /* Live Scans */
        .live-scan-item {{
            background: rgba(30, 41, 59, 0.25);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            padding: 1rem;
            margin-bottom: 0.75rem;
            animation: fadeIn 0.5s ease-in;
            box-shadow: var(--glass-inner);
        }}
        
        .live-scan-item.new {{
            border-left: 4px solid var(--success);
            background: rgba(34, 197, 94, 0.05);
        }}
        
        .live-scan-header {{
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            margin-bottom: 0.5rem;
        }}
        
        .live-scan-time {{
            color: var(--text-muted);
            font-size: 0.8rem;
        }}
        
        .live-scan-barcode {{
            font-family: monospace;
            font-weight: 600;
            font-size: 1.1rem;
            color: var(--accent);
            margin-bottom: 0.25rem;
        }}
        
        .live-scan-details {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 0.5rem;
            font-size: 0.85rem;
            color: var(--text-muted);
        }}
        
        @keyframes fadeIn {{
            from {{ opacity: 0; transform: translateY(-10px); }}
            to {{ opacity: 1; transform: translateY(0); }}
        }}
        
        /* Scan Groups */
        .scan-group {{
            background: rgba(30, 41, 59, 0.25);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            margin-bottom: 0.75rem;
            overflow: hidden;
            transition: border-color 0.2s, box-shadow 0.2s;
            box-shadow: var(--glass-inner), 0 1px 3px rgba(0,0,0,0.06), 0 4px 16px rgba(0,0,0,0.04);
            position: relative;
        }}
        
        .scan-group:hover {{
            border-color: var(--glass-edge-strong);
            box-shadow: var(--glass-inner), 0 2px 8px rgba(0,0,0,0.08), 0 12px 40px rgba(0,0,0,0.08);
        }}
        
        .group-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 1rem 1.25rem;
            cursor: pointer;
            transition: background 0.2s;
            user-select: none;
            position: relative;
            z-index: 1;
        }}
        
        .group-header:hover {{
            background: rgba(255, 255, 255, 0.04);
        }}
        
        .group-title {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}
        
        .group-arrow {{
            font-size: 0.65rem;
            transition: transform 0.2s;
            color: var(--text-muted);
        }}
        
        .scan-group.expanded .group-arrow {{
            transform: rotate(90deg);
        }}
        
        .group-time {{
            font-weight: 600;
            font-size: 0.95rem;
        }}
        
        .group-job {{
            font-weight: 700;
            font-size: 0.95rem;
            color: var(--accent);
        }}
        
        .group-meta {{
            display: flex;
            align-items: center;
            gap: 1rem;
            font-size: 0.8rem;
            color: var(--text-muted);
        }}
        
        .group-count {{
            background: var(--accent);
            color: white;
            padding: 0.2rem 0.65rem;
            border-radius: 9999px;
            font-size: 0.75rem;
            font-weight: 600;
        }}
        
        .group-scans {{
            display: none;
            border-top: 1px solid var(--border);
            padding: 0;
        }}
        
        .scan-group.expanded .group-scans {{
            display: block;
        }}
        
        .group-scans table {{
            margin: 0;
            width: 100%;
        }}
        
        .group-scans th {{
            background: var(--bg);
            font-size: 0.75rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--text-muted);
        }}
        
        .group-scans td {{
            font-size: 0.85rem;
        }}
        
        /* Swipe-to-delete scan rows */
        .scan-row-wrapper {{
            position: relative;
            overflow: hidden;
            border-bottom: 1px solid var(--border);
        }}
        .scan-row-wrapper:last-child {{
            border-bottom: none;
        }}
        .scan-row-delete {{
            position: absolute;
            right: 0;
            top: 0;
            bottom: 0;
            width: 80px;
            background: #ef4444;
            color: white;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 0.8rem;
            font-weight: 600;
            cursor: pointer;
            user-select: none;
            z-index: 1;
        }}
        .scan-row-delete:hover {{
            background: #dc2626;
        }}
        .scan-row-content {{
            position: relative;
            z-index: 2;
            display: grid;
            grid-template-columns: 1fr 1fr 0.7fr;
            gap: 0.75rem;
            padding: 0.5rem 0.75rem;
            background: var(--card-solid);
            transition: transform 0.25s ease;
            touch-action: pan-y;
            user-select: none;
        }}
        .scan-row-content.swiped {{
            transform: translateX(-80px);
        }}
        .scan-row-content.swiping {{
            transition: none;
        }}
        .scan-row-header {{
            display: grid;
            grid-template-columns: 1fr 1fr 0.7fr;
            gap: 0.75rem;
            padding: 0.4rem 0.75rem;
            background: var(--bg);
            font-size: 0.75rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--text-muted);
            font-weight: 600;
        }}
        .scan-row-content span {{
            font-size: 0.85rem;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }}
        .scan-row-removing {{
            max-height: 50px;
            overflow: hidden;
            transition: max-height 0.3s ease, opacity 0.3s ease;
        }}
        .scan-row-removing.removed {{
            max-height: 0;
            opacity: 0;
            border-bottom: none;
            padding: 0;
            margin: 0;
        }}
        
        .change-actions {{
            display: flex;
            gap: 0.5rem;
        }}
        
        /* Job Cards */
        .job-card {{
            background: rgba(30, 41, 59, 0.3);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            padding: 0;
            cursor: pointer;
            transition: all var(--transition);
            user-select: none;
            overflow: hidden;
            box-shadow: var(--glass-inner), 0 1px 3px rgba(0,0,0,0.06), 0 4px 16px rgba(0,0,0,0.04);
            position: relative;
        }}
        .job-card:hover {{
            border-color: var(--glass-edge-strong);
            box-shadow: var(--glass-inner), 0 2px 8px rgba(0,0,0,0.08), 0 12px 40px rgba(0,0,0,0.08);
            transform: translateY(-2px);
        }}
        .job-card.expanded {{
            border-color: rgba(99, 102, 241, 0.12);
            box-shadow: var(--glass-inner), 0 4px 20px var(--accent-glow);
        }}
        .job-card-summary {{
            padding: 1rem 1.25rem;
            position: relative;
            z-index: 1;
        }}
        .job-card-top {{
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            gap: 0.5rem;
        }}
        .job-card .job-card-name {{
            font-weight: 700;
            font-size: 0.95rem;
            color: var(--accent);
            margin-bottom: 0.25rem;
        }}
        .job-card .job-card-ref {{
            font-size: 0.8rem;
            color: var(--text-muted);
            margin-bottom: 0.25rem;
        }}
        .job-card .job-card-meta {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.8rem;
            color: var(--text-muted);
            margin-bottom: 0.25rem;
            flex-wrap: wrap;
        }}
        .job-card .job-card-date {{
            font-size: 0.75rem;
            color: var(--text-muted);
            opacity: 0.7;
        }}
        .job-card-expand-icon {{
            color: var(--text-muted);
            font-size: 0.9rem;
            transition: transform 0.2s;
            flex-shrink: 0;
            margin-top: 2px;
        }}
        .job-card.expanded .job-card-expand-icon {{
            transform: rotate(180deg);
        }}
        .job-card-progress {{
            margin-top: 0.5rem;
        }}
        .job-card-progress-bar {{
            width: 100%;
            height: 6px;
            background: var(--border);
            border-radius: 3px;
            overflow: hidden;
            margin-bottom: 0.25rem;
        }}
        .job-card-progress-fill {{
            height: 100%;
            border-radius: 3px;
            transition: width 0.3s ease;
        }}
        .job-card-progress-text {{
            display: flex;
            justify-content: space-between;
            font-size: 0.7rem;
            color: var(--text-muted);
        }}
        .job-card .job-card-details {{
            display: none;
            border-top: 1px solid var(--border);
        }}
        .job-card.expanded .job-card-details {{
            display: block;
        }}
        .job-card-details-inner {{
            padding: 1rem 1.25rem;
        }}
        .job-card-tabs {{
            display: flex;
            border-bottom: 1px solid var(--border);
            padding: 0 1.25rem;
            background: rgba(0,0,0,0.03);
        }}
        .job-card-tab {{
            padding: 0.5rem 0.75rem;
            font-size: 0.8rem;
            cursor: pointer;
            border-bottom: 2px solid transparent;
            color: var(--text-muted);
            transition: color 0.2s, border-color 0.2s;
            white-space: nowrap;
        }}
        .job-card-tab:hover {{
            color: var(--text);
        }}
        .job-card-tab.active {{
            color: var(--accent);
            border-bottom-color: var(--accent);
            font-weight: 600;
        }}
        .job-tab-content {{
            display: none;
        }}
        .job-tab-content.active {{
            display: block;
        }}
        .job-scan-row {{
            display: flex;
            align-items: center;
            padding: 0.35rem 0;
            border-bottom: 1px solid rgba(128,128,128,0.1);
            transition: background 0.15s;
        }}
        .job-scan-row:hover {{
            background: rgba(128,128,128,0.06);
        }}
        .job-card .detail-row {{
            display: flex;
            gap: 0.5rem;
            font-size: 0.85rem;
            margin-bottom: 0.5rem;
            align-items: flex-start;
        }}
        .job-card .detail-label {{
            color: var(--text-muted);
            min-width: 90px;
            font-size: 0.8rem;
            flex-shrink: 0;
        }}
        .job-card .detail-value {{
            word-break: break-word;
        }}
        .job-card .job-card-actions {{
            display: flex;
            gap: 0.5rem;
            padding: 0.75rem 1.25rem;
            border-top: 1px solid var(--border);
            background: rgba(0,0,0,0.02);
        }}
        .job-card .job-edit-panel {{
            display: none;
            border-top: 1px solid var(--border);
            padding: 1rem 1.25rem;
        }}
        .job-card .job-edit-panel.active {{
            display: block;
        }}
        .job-card .edit-grid {{
            display: grid;
            grid-template-columns: auto 1fr;
            gap: 0.4rem 0.75rem;
            align-items: center;
            font-size: 0.85rem;
        }}
        .job-card .edit-grid label {{
            color: var(--text-muted);
            font-size: 0.8rem;
        }}
        .job-card .edit-grid input,
        .job-card .edit-grid select,
        .job-card .edit-grid textarea {{
            width: 100%;
            padding: 0.3rem 0.5rem;
            font-size: 0.85rem;
        }}
        .job-stat-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 0.5rem;
            margin-bottom: 0.75rem;
        }}
        .job-stat-box {{
            background: var(--background);
            border: 1px solid var(--border);
            border-radius: 0.5rem;
            padding: 0.5rem 0.75rem;
            text-align: center;
        }}
        .job-stat-box .stat-val {{
            font-size: 1.1rem;
            font-weight: 700;
            color: var(--accent);
        }}
        .job-stat-box .stat-lbl {{
            font-size: 0.7rem;
            color: var(--text-muted);
        }}
        .priority-badge {{
            display: inline-block;
            padding: 0.1rem 0.4rem;
            border-radius: 4px;
            font-size: 0.7rem;
            font-weight: 600;
            text-transform: uppercase;
        }}
        .priority-LOW {{ background: #e0e0e0; color: #555; }}
        .priority-NORMAL {{ background: #e3f2fd; color: #1565c0; }}
        .priority-HIGH {{ background: #fff3e0; color: #e65100; }}
        .priority-URGENT {{ background: #ffebee; color: #c62828; }}

        /* Mobile Bottom Navigation — Floating Glassmorphic Pill */
        .mobile-nav {{
            display: none;
            position: fixed;
            bottom: 0.6rem;
            left: 0.75rem;
            right: 0.75rem;
            background: var(--glass);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: 1.5rem;
            z-index: 1000;
            padding: 0.4rem 0.25rem;
            padding-bottom: calc(0.4rem + env(safe-area-inset-bottom, 0px));
            box-shadow: var(--glass-inner), 0 4px 24px rgba(0, 0, 0, 0.12), 0 1px 3px rgba(0, 0, 0, 0.08);
        }}
        .mobile-nav-inner {{
            display: flex;
            justify-content: space-around;
            align-items: center;
        }}
        .mobile-nav-item {{
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 0.1rem;
            padding: 0.4rem 0.75rem;
            border-radius: 1rem;
            color: var(--text-muted);
            cursor: pointer;
            font-size: 0.6rem;
            transition: all 0.2s ease;
            position: relative;
            -webkit-tap-highlight-color: transparent;
        }}
        .mobile-nav-item span.mobile-nav-icon {{ font-size: 1.2rem; line-height: 1; }}
        .mobile-nav-item.active {{ color: var(--accent); background: rgba(99, 102, 241, 0.1); }}
        .mobile-nav-item .mobile-badge {{
            position: absolute;
            top: 0; right: 0;
            background: var(--danger);
            color: white;
            font-size: 0.55rem;
            min-width: 14px;
            height: 14px;
            line-height: 14px;
            text-align: center;
            border-radius: 9999px;
            font-weight: 700;
        }}
        .mobile-more-menu {{
            display: none;
            position: fixed;
            bottom: 5.5rem;
            left: 0.75rem;
            right: 0.75rem;
            background: rgba(15, 23, 42, 0.4);
            backdrop-filter: blur(40px);
            -webkit-backdrop-filter: blur(40px);
            border: 0.5px solid var(--glass-edge);
            border-radius: var(--radius);
            padding: 0.5rem;
            z-index: 1001;
            box-shadow: var(--glass-inner), 0 -4px 24px rgba(0, 0, 0, 0.08);
        }}
        .mobile-more-menu.open {{ display: block; }}
        .mobile-more-item {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
            padding: 0.75rem;
            border-radius: 0.5rem;
            color: var(--text-muted);
            cursor: pointer;
            font-size: 0.9rem;
        }}
        .mobile-more-item:hover {{ background: var(--card-hover); }}
        .mobile-more-item.active {{ color: var(--accent); background: rgba(56,189,248,0.1); }}
        .mobile-more-overlay {{
            display: none;
            position: fixed;
            inset: 0;
            z-index: 1000;
        }}
        .mobile-more-overlay.open {{ display: block; }}

        /* Settings Controls */
        .theme-mode-btn {{
            flex: 1;
            padding: 0.5rem 0.75rem;
            border-radius: var(--radius);
            border: 0.5px solid var(--glass-edge);
            background: rgba(255, 255, 255, 0.03);
            backdrop-filter: blur(20px);
            -webkit-backdrop-filter: blur(20px);
            color: var(--text-muted);
            cursor: pointer;
            font-size: 0.8rem;
            transition: all var(--transition);
            text-align: center;
        }}
        .theme-mode-btn:hover {{ border-color: var(--glass-edge-strong); color: var(--text); background: rgba(255, 255, 255, 0.04); }}
        .theme-mode-btn.active {{ background: linear-gradient(135deg, var(--accent), var(--accent-hover)); color: white; border-color: var(--accent); box-shadow: 0 2px 12px var(--accent-glow); }}

        .color-swatch {{
            width: 32px;
            height: 32px;
            border-radius: 50%;
            border: 2px solid transparent;
            cursor: pointer;
            transition: all var(--transition);
            position: relative;
        }}
        .color-swatch:hover {{ transform: scale(1.15); }}
        .color-swatch.active {{ border-color: var(--text); box-shadow: 0 0 0 3px var(--accent-glow); transform: scale(1.1); }}
        .color-swatch.active::after {{
            content: '✓';
            position: absolute;
            inset: 0;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-size: 0.8rem;
            font-weight: 700;
            text-shadow: 0 1px 2px rgba(0,0,0,0.3);
        }}

        /* Settings range slider */
        input[type="range"] {{
            -webkit-appearance: none;
            appearance: none;
            height: 6px;
            border-radius: 3px;
            background: var(--border);
            outline: none;
            border: none;
            box-shadow: none;
        }}
        input[type="range"]::-webkit-slider-thumb {{
            -webkit-appearance: none;
            appearance: none;
            width: 18px;
            height: 18px;
            border-radius: 50%;
            background: linear-gradient(135deg, var(--accent), var(--accent-hover));
            cursor: pointer;
            box-shadow: 0 2px 6px var(--accent-glow);
        }}
        input[type="range"]::-moz-range-thumb {{
            width: 18px;
            height: 18px;
            border-radius: 50%;
            background: linear-gradient(135deg, var(--accent), var(--accent-hover));
            cursor: pointer;
            border: none;
            box-shadow: 0 2px 6px var(--accent-glow);
        }}

        /* No-animations mode */
        body.no-animations * {{
            animation-duration: 0s !important;
            transition-duration: 0s !important;
        }}
        /* Compact mode */
        body.compact .card-body {{ padding: 0.4rem; }}
        body.compact .stat-card {{ padding: 0.5rem; }}
        body.compact .page-header {{ margin-bottom: 0.5rem; }}
        body.compact .cards-grid {{ gap: 0.5rem; }}
        body.compact .stats {{ gap: 0.5rem; }}

        /* Responsive */
        @media (max-width: 900px) {{
            .sidebar {{ display: none; }}
            .main {{ margin-left: 0; padding: 0.75rem; padding-bottom: 6rem; }}
            .mobile-nav {{ display: block; }}

            /* Page headers: stack on mobile */
            .page-header {{
                flex-direction: column;
                align-items: flex-start;
                gap: 0.75rem;
            }}
            .page-header > div {{
                width: 100%;
                flex-wrap: wrap;
            }}
            .page-header h2 {{
                font-size: 1.15rem;
            }}

            /* Dashboard stats: 3 columns on tablet, 2 on phone */
            .stats {{
                grid-template-columns: repeat(3, 1fr);
            }}

            /* Cards grid: single column */
            .cards-grid {{
                grid-template-columns: 1fr;
            }}

            /* Jobs grid: single column */
            #jobs-cards-container {{
                grid-template-columns: 1fr !important;
            }}

            /* Scans page: full-width filters */
            #page-scans .page-header > div {{
                display: flex;
                flex-wrap: wrap;
                gap: 0.5rem;
            }}
            #page-scans .page-header select {{
                flex: 1;
                min-width: 0;
            }}

            /* Devices table: horizontal scroll */
            #page-devices .card {{
                overflow-x: auto;
                -webkit-overflow-scrolling: touch;
            }}
            #page-devices table {{
                min-width: 600px;
                font-size: 0.85rem;
            }}
            #page-devices table th {{
                position: sticky;
                top: 0;
                background: var(--card);
                z-index: 1;
            }}

            /* Map: shorter on mobile */
            #dashboard-map {{
                height: 400px !important;
            }}

            /* Buttons: full width on very small */
            .btn-primary, .btn-outline {{
                font-size: 0.8rem;
                padding: 0.5rem 0.75rem;
            }}

            /* Scan groups: tighter padding */
            .group-header {{
                padding: 0.75rem;
            }}
            .group-meta {{
                flex-wrap: wrap;
                gap: 0.5rem;
            }}

            /* Job cards: tighter */
            .job-card-summary {{
                padding: 0.75rem;
            }}
            .job-card-top {{
                flex-direction: column;
                gap: 0.25rem;
            }}

            /* Job cards: tighter */
            .job-card-summary {{
                padding: 0.75rem;
            }}
            .job-card-top {{
                flex-direction: column;
                gap: 0.25rem;
            }}
        }}

        @media (max-width: 480px) {{
            .main {{
                padding: 0.5rem;
                padding-bottom: 6rem;
            }}
            .stats {{
                grid-template-columns: repeat(2, 1fr);
                gap: 0.5rem;
            }}
            .stat-card {{
                padding: 0.5rem;
            }}
            .stat-value {{
                font-size: 1.1rem;
            }}
            .stat-label {{
                font-size: 0.6rem;
            }}

            /* Scan row content: stack for readability */
            .scan-row-content {{
                grid-template-columns: 1fr !important;
                gap: 0.25rem;
            }}
            .scan-row-header {{
                display: none;
            }}

            /* Live scan details: single column */
            .live-scan-details {{
                grid-template-columns: 1fr;
            }}

            /* Page header buttons: stack */
            .page-header > div {{
                display: flex;
                flex-direction: column;
                gap: 0.5rem;
            }}
            .page-header > div > select,
            .page-header > div > input,
            .page-header > div > button {{
                width: 100%;
            }}

            /* Modal: almost full width */
            .modal-content {{
                width: 95%;
                border-radius: 1.25rem;
            }}

            /* Devices table: hide less important columns on very small screens */
            #page-devices table th:nth-child(3),
            #page-devices table td:nth-child(3),
            #page-devices table th:nth-child(4),
            #page-devices table td:nth-child(4) {{
                display: none;
            }}
            #page-devices table {{
                font-size: 0.8rem;
            }}
            #page-devices table th,
            #page-devices table td {{
                padding: 0.5rem 0.25rem;
            }}
        }}
    </style>
</head>
<body>
    <div class="layout">
        <aside class="sidebar">
            <div class="logo">
                <img class="logo-icon" src="/logo.png" alt="CabNet">
                <h1>CabNet</h1>
            </div>
            
            <div id="trust-badge" class="trust-status untrusted">
                <span>⏳</span>
                <span>Checking status...</span>
            </div>
            
            <nav class="nav">
                <div class="nav-item active" data-page="dashboard">
                    <span>🏠</span>
                    <span>Dashboard</span>
                </div>
                <div class="nav-item" data-page="jobs">
                    <span>🗂️</span>
                    <span>Jobs</span>
                </div>
                <div class="nav-item" data-page="devices">
                    <span>📡</span>
                    <span>Devices</span>
                </div>
                <div class="nav-item" data-page="scans">
                    <span>🏷️</span>
                    <span>Scans</span>
                </div>
                <div class="nav-item" data-page="map">
                    <span>🗺️</span>
                    <span>Map</span>
                </div>
                <div class="nav-item" data-page="live">
                    <span>⚡</span>
                    <span>Live</span>
                </div>
                <div class="nav-item" data-page="timeclock">
                    <span>⏱️</span>
                    <span>Time Clock</span>
                </div>
                <div class="nav-item" data-page="reports">
                    <span>📋</span>
                    <span>Reports</span>
                </div>
                <div class="nav-item" data-page="team">
                    <span>👥</span>
                    <span>Team</span>
                </div>
                <div class="nav-item" data-page="timesheets">
                    <span>📋</span>
                    <span>Timesheets</span>
                </div>
                <div class="nav-item" data-page="approvals" id="nav-approvals" style="display: none;">
                    <span>✅</span>
                    <span>Approvals <span id="nav-approvals-badge" style="background: var(--accent); color: white; border-radius: 9999px; padding: 0.1rem 0.4rem; font-size: 0.65rem; margin-left: 0.25rem; display: none;">0</span></span>
                </div>
            </nav>
            
            <button class="settings-btn" onclick="openSettings()">⚙️ Settings</button>
            <div class="sidebar-footer">
                <p>v{version} · <span id="uptime">{uptime}</span></p>
                <p id="client-id-display" style="margin-top: 0.5rem; font-size: 0.7rem; opacity: 0.6;"></p>
            </div>
        </aside>
        
        <main class="main">
            <!-- Dashboard Page -->
            <div class="page active" id="page-dashboard">
                <div class="page-header">
                    <h2>Control Center</h2>
                    <button class="btn btn-outline" onclick="refreshAll()">🔄 Refresh</button>
                </div>
                
                <!-- Top Stats Row -->
                <div class="stats">
                    <div class="stat-card" onclick="showPage('devices')" style="cursor:pointer;">
                        <div class="stat-icon">📱</div>
                        <div class="stat-value" id="stat-devices">{device_count}</div>
                        <div class="stat-label">Devices</div>
                    </div>
                    <div class="stat-card" onclick="showPage('scans')" style="cursor:pointer;">
                        <div class="stat-icon">🏷️</div>
                        <div class="stat-value" id="stat-scans">{scan_count}</div>
                        <div class="stat-label">Scans</div>
                    </div>
                    <div class="stat-card" onclick="showPage('jobs')" style="cursor:pointer;">
                        <div class="stat-icon">🗂️</div>
                        <div class="stat-value" id="stat-jobs">{job_count}</div>
                        <div class="stat-label">Jobs</div>
                    </div>
                    <div class="stat-card" onclick="showPage('timeclock')" style="cursor:pointer;">
                        <div class="stat-icon">⏱️</div>
                        <div class="stat-value" id="stat-workers">{active_worker_count}</div>
                        <div class="stat-label">Working Now</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-icon">⏱️</div>
                        <div class="stat-value" id="stat-uptime">{uptime}</div>
                        <div class="stat-label">Uptime</div>
                    </div>
                    <div class="stat-card" onclick="showPage('reports')" style="cursor:pointer;">
                        <div class="stat-icon">📋</div>
                        <div class="stat-value" id="stat-reports">{report_count}</div>
                        <div class="stat-label">Reports</div>
                    </div>
                </div>

                <!-- Team Status (condensed) -->
                <div class="card" style="margin-bottom: 1rem;">
                    <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
                        <h3>👥 Team Status</h3>
                        <button class="btn btn-outline" style="font-size: 0.8rem; padding: 0.3rem 0.8rem;" onclick="showPage('team')">View All →</button>
                    </div>
                    <div class="card-body" id="dash-team-status">
                        <div style="text-align: center; color: var(--text-muted); padding: 1rem;"><div class="spinner"></div>Loading team...</div>
                    </div>
                </div>

                <!-- Two column grid: Today's Reports + Job Progress -->
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 1rem; margin-bottom: 1rem;">
                    <!-- Today's Reports -->
                    <div class="card">
                        <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
                            <h3>📋 Today's Reports</h3>
                            <button class="btn btn-outline" style="font-size: 0.8rem; padding: 0.3rem 0.8rem;" onclick="showPage('reports')">View All →</button>
                        </div>
                        <div class="card-body" id="dash-reports">
                            <div style="text-align: center; color: var(--text-muted); padding: 1rem;"><div class="spinner"></div>Loading...</div>
                        </div>
                    </div>

                    <!-- Job Progress -->
                    <div class="card">
                        <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
                            <h3>🗂️ Job Progress</h3>
                            <button class="btn btn-outline" style="font-size: 0.8rem; padding: 0.3rem 0.8rem;" onclick="showPage('jobs')">View All →</button>
                        </div>
                        <div class="card-body">
                            {jobs_status_html}
                        </div>
                    </div>
                </div>

                <!-- Two column grid: Recent Activity + Quick Actions -->
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 1rem; margin-bottom: 1rem;">
                    <!-- Recent Activity -->
                    <div class="card">
                        <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
                            <h3>📡 Recent Activity</h3>
                            <button class="btn btn-outline" style="font-size: 0.8rem; padding: 0.3rem 0.8rem;" onclick="showPage('scans')">View All →</button>
                        </div>
                        <div class="card-body">
                            {recent_scans_html}
                        </div>
                    </div>

                    <!-- Quick Actions + Server Info -->
                    <div class="card">
                        <div class="card-header">
                            <h3>🚀 Quick Actions</h3>
                        </div>
                        <div class="card-body">
                            <div style="display: flex; gap: 0.75rem; flex-wrap: wrap; margin-bottom: 1rem;">
                                <button class="btn btn-primary" onclick="showPage('jobs'); openJobModal()">+ New Job</button>
                                <button class="btn btn-outline" onclick="showPage('scans')">View Scans</button>
                                <button class="btn btn-outline" onclick="showPage('devices')">Manage Devices</button>
                                <button class="btn btn-outline" onclick="showPage('timesheets')">📋 Timesheets</button>
                            </div>
                            <div style="font-size: 0.85rem; color: var(--text-muted);">
                                <strong>Version:</strong> {version} · <span class="badge badge-success">Online</span>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Admin Section (only for trusted devices) -->
                <div id="dash-admin-section" style="display: none;">
                    <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.75rem; color: var(--text-muted); font-size: 0.85rem;">
                        <span style="background: var(--accent); color: white; padding: 0.15rem 0.5rem; border-radius: 4px; font-size: 0.7rem; font-weight: 600;">ADMIN</span>
                        <span>Administrator View</span>
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 1rem;">
                        <!-- Pending Approvals -->
                        <div class="card">
                            <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
                                <h3>⚠️ Pending Approvals</h3>
                                <button class="btn btn-outline" style="font-size: 0.8rem; padding: 0.3rem 0.8rem;" onclick="showPage('approvals')">Manage →</button>
                            </div>
                            <div class="card-body" id="dash-pending-approvals">
                                <p style="color: var(--text-muted); font-style: italic;">Loading...</p>
                            </div>
                        </div>

                        <!-- Today's Hours (all workers) -->
                        <div class="card">
                            <div class="card-header" style="display: flex; justify-content: space-between; align-items: center;">
                                <h3>⏱️ Today's Hours</h3>
                                <button class="btn btn-outline" style="font-size: 0.8rem; padding: 0.3rem 0.8rem;" onclick="showPage('timesheets')">Timesheets →</button>
                            </div>
                            <div class="card-body" id="dash-today-hours">
                                <p style="color: var(--text-muted); font-style: italic;">Loading...</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Jobs Page -->
            <div class="page" id="page-jobs">
                <div class="page-header">
                    <h2>Jobs</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center; flex-wrap: wrap;">
                        <input type="text" id="jobs-search" placeholder="🔍 Search jobs..." style="width: 180px;" oninput="filterJobCards()">
                        <select id="jobs-status-filter" style="width: 140px;" onchange="filterJobCards()">
                            <option value="">All Statuses</option>
                            <option value="PENDING">Pending</option>
                            <option value="ACTIVE">Active</option>
                            <option value="COMPLETED">Completed</option>
                            <option value="CANCELLED">Cancelled</option>
                        </select>
                        <select id="jobs-sort" style="width: 140px;" onchange="sortAndRenderJobs()">
                            <option value="newest">Newest First</option>
                            <option value="oldest">Oldest First</option>
                            <option value="name">Name A–Z</option>
                            <option value="progress">Progress</option>
                            <option value="priority">Priority</option>
                        </select>
                        <button class="btn btn-outline" onclick="loadJobs()">🔄 Refresh</button>
                        <button class="btn btn-primary" onclick="openJobModal()">+ New Job</button>
                    </div>
                </div>
                
                <div style="display: flex; gap: 1rem; font-size: 0.8rem; color: var(--text-muted); margin-bottom: 0.75rem; flex-wrap: wrap;">
                    <span><strong id="jobs-count">0</strong> jobs</span>
                    <span>·</span>
                    <span id="jobs-active-count">0</span> active
                    <span>·</span>
                    <span id="jobs-pending-count">0</span> pending
                    <span>·</span>
                    <span id="jobs-completed-count">0</span> completed
                </div>
                <div id="jobs-cards-container" style="display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 0.75rem;">
                    <div style="text-align: center; padding: 2rem; color: var(--text-muted); grid-column: 1 / -1;">
                        <div class="spinner"></div>Loading jobs...
                    </div>
                </div>
            </div>
            
            <!-- Devices Page -->
            <div class="page" id="page-devices">
                <div class="page-header">
                    <h2>Devices</h2>
                    <button class="btn btn-outline" onclick="loadDevices()">🔄 Refresh</button>
                </div>
                
                <div class="card">
                    <table>
                        <thead>
                            <tr>
                                <th>Name</th>
                                <th>Device ID</th>
                                <th>Model</th>
                                <th>App Version</th>
                                <th>Last Seen</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody id="devices-table">
                            <tr><td colspan="6" class="loading"><div class="spinner"></div>Loading...</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
            
            <!-- Scans Page -->
            <div class="page" id="page-scans">
                <div class="page-header">
                    <h2>Scans</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center; flex-wrap: wrap;">
                        <select id="scans-job-filter" style="min-width: 140px; flex: 1; max-width: 200px;" onchange="loadScans()">
                            <option value="">All Jobs</option>
                        </select>
                        <select id="scans-time-gap" style="min-width: 130px; flex: 1; max-width: 180px;" onchange="regroupScansPage()">
                            <option value="1">1 hour groups</option>
                            <option value="2">2 hour groups</option>
                            <option value="4" selected>4 hour groups</option>
                            <option value="8">8 hour groups</option>
                            <option value="24">24 hour groups</option>
                        </select>
                        <button class="btn btn-outline" onclick="loadScans()">🔄 Refresh</button>
                        <button class="btn btn-primary" onclick="openAddScansModal()">➕ Add Scans</button>
                    </div>
                </div>
                
                <div class="card">
                    <div class="card-header">
                        <h3>🏷️ Scan Groups</h3>
                        <div style="font-size: 0.8rem; color: var(--text-muted);">
                            <span id="scans-total-count">0</span> total scans in <span id="scans-group-count">0</span> groups
                        </div>
                    </div>
                    <div class="card-body" style="padding: 0.5rem;">
                        <div id="scans-groups-container">
                            <div style="text-align: center; padding: 2rem; color: var(--text-muted);">
                                <div class="spinner"></div>Loading scans...
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Map Page -->
            <div class="page" id="page-map">
                <div class="page-header">
                    <h2>Scan Map</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center; flex-wrap: wrap;">
                        <select id="map-provider" style="min-width: 140px; max-width: 200px; flex: 1;" onchange="changeMapProvider()">
                            <option value="osm">OpenStreetMap</option>
                            <option value="esri">Esri Satellite</option>
                            <option value="esri-street">Esri Streets</option>
                            <option value="esri-topo">Esri Topographic</option>
                        </select>
                        <span class="badge" id="map-scan-count" style="background: var(--accent); padding: 0.35rem 0.75rem; border-radius: 9999px;">0 locations</span>
                        <button class="btn btn-outline" onclick="loadMapData()">🔄 Refresh</button>
                    </div>
                </div>
                
                <div class="card" style="padding: 0; overflow: hidden;">
                    <div id="dashboard-map" style="height: 600px; width: 100%;"></div>
                </div>
            </div>
            
            <!-- Live Scans Page -->
            <div class="page" id="page-live">
                <div class="page-header">
                    <h2>Live Scans</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center; flex-wrap: wrap;">
                        <span class="badge" id="live-status" style="background: var(--success); padding: 0.35rem 0.75rem; border-radius: 9999px;">🔴 Stopped</span>
                        <button class="btn btn-success" id="live-toggle" onclick="toggleLiveMode()">▶️ Start Live</button>
                        <button class="btn btn-outline" onclick="clearLiveScans()">🗑️ Clear</button>
                    </div>
                </div>
                
                <div class="card">
                    <div class="card-header">
                        <h3>⚡ Live Scan Feed</h3>
                        <div style="font-size: 0.8rem; color: var(--text-muted);">
                            Auto-refresh: <span id="live-refresh-rate">5s</span> | 
                            Last update: <span id="live-last-update">Never</span>
                        </div>
                    </div>
                    <div class="card-body">
                        <div id="live-scans-container" style="max-height: 600px; overflow-y: auto;">
                            <div class="live-scan-item" style="text-align: center; color: var(--text-muted); padding: 2rem;">
                                <div style="font-size: 2rem; margin-bottom: 1rem;">⚡</div>
                                <div>Click "Start Live" to begin monitoring new scans in real-time</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Time Clock Page -->
            <div class="page" id="page-timeclock">
                <div class="page-header">
                    <h2>Time Clock</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                        <select id="tc-filter" onchange="loadTimeEntries()" style="background: var(--card); color: var(--text); border: 1px solid var(--border); border-radius: 8px; padding: 0.4rem 0.75rem; font-size: 0.85rem;">
                            <option value="all">All Workers</option>
                        </select>
                        <button class="btn btn-outline" onclick="loadTimeEntries()">🔄 Refresh</button>
                    </div>
                </div>

                <!-- Active Workers -->
                <div id="active-workers" style="margin-bottom: 1.5rem;">
                    <h3 style="font-size: 1rem; margin-bottom: 0.75rem; color: var(--text-muted);">Currently Working</h3>
                    <div id="active-workers-list" style="display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 1rem;"></div>
                </div>

                <!-- Timesheet Table -->
                <div class="card" style="overflow-x: auto;">
                    <table id="timeclock-table" style="width: 100%;">
                        <thead>
                            <tr>
                                <th>Worker</th>
                                <th>Job</th>
                                <th>Clock In</th>
                                <th>Clock Out</th>
                                <th>Duration</th>
                                <th>Type</th>
                            </tr>
                        </thead>
                        <tbody id="timeclock-tbody"></tbody>
                    </table>
                </div>
            </div>

            <!-- Reports Page -->
            <div class="page" id="page-reports">
                <div class="page-header">
                    <h2>Daily Reports</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                        <select id="report-sort" onchange="loadReports()" style="background: var(--card); color: var(--text); border: 1px solid var(--border); border-radius: 8px; padding: 0.4rem 0.75rem; font-size: 0.85rem;">
                            <option value="recent">Most Recent</option>
                            <option value="job">By Job</option>
                        </select>
                        <select id="report-filter-job" onchange="loadReports()" style="background: var(--card); color: var(--text); border: 1px solid var(--border); border-radius: 8px; padding: 0.4rem 0.75rem; font-size: 0.85rem;">
                            <option value="all">All Jobs</option>
                        </select>
                        <button class="btn btn-outline" onclick="loadReports()">🔄 Refresh</button>
                    </div>
                </div>

                <div id="reports-container" style="display: grid; grid-template-columns: repeat(auto-fill, minmax(350px, 1fr)); gap: 1rem;"></div>
            </div>

            <!-- Approvals Page -->
            <div class="page" id="page-approvals">
                <div class="page-header">
                    <h2>Pending Approvals</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                        <button class="btn btn-outline" onclick="loadApprovals()">🔄 Refresh</button>
                    </div>
                </div>
                
                <div style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 0.75rem;">
                    Changes submitted by untrusted web devices appear here for you to approve or reject.
                </div>
                
                <div id="approvals-container">
                    <div style="text-align: center; padding: 2rem; color: var(--text-muted);">
                        <div class="spinner"></div>Loading...
                    </div>
                </div>
            </div>

            <!-- Team Page -->
            <div class="page" id="page-team">
                <div class="page-header">
                    <h2>👥 Team</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                        <button class="btn btn-primary" onclick="showAddMemberModal()">➕ Add Member</button>
                        <button class="btn btn-outline" onclick="loadTeamStatus()">🔄 Refresh</button>
                    </div>
                </div>

                <div style="display: flex; gap: 1rem; margin-bottom: 1.5rem; flex-wrap: wrap;" id="team-stats"></div>

                <div style="display: flex; gap: 0.75rem; margin-bottom: 1rem; align-items: center;">
                    <input type="text" id="team-search" placeholder="Search team..." oninput="filterTeam()" style="background: var(--card); border: 1px solid var(--border); color: var(--text); padding: 0.5rem 0.75rem; border-radius: 8px; width: 200px;">
                    <select id="team-filter" onchange="filterTeam()" style="background: var(--card); border: 1px solid var(--border); color: var(--text); padding: 0.5rem; border-radius: 8px;">
                        <option value="all">All</option>
                        <option value="working">🟢 Working</option>
                        <option value="break">☕ Break</option>
                        <option value="offline">⚫ Offline</option>
                    </select>
                </div>

                <div id="team-container">
                    <div style="text-align: center; padding: 2rem; color: var(--text-muted);"><div class="spinner"></div>Loading...</div>
                </div>
            </div>

            <!-- Timesheets Page -->
            <div class="page" id="page-timesheets">
                <div class="page-header">
                    <h2>📋 Timesheets</h2>
                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                        <button class="btn btn-outline" onclick="loadTimesheets()">🔄 Refresh</button>
                    </div>
                </div>

                <div style="display: flex; gap: 1rem; margin-bottom: 1.5rem; flex-wrap: wrap;" id="ts-stats"></div>

                <div style="display: flex; gap: 0.75rem; margin-bottom: 1rem; align-items: center; flex-wrap: wrap;">
                    <select id="ts-worker-filter" onchange="loadTimesheets()" style="background: var(--card); border: 1px solid var(--border); color: var(--text); padding: 0.5rem; border-radius: 8px;">
                        <option value="all">All Workers</option>
                    </select>
                    <input type="date" id="ts-date-from" onchange="loadTimesheets()" style="background: var(--card); border: 1px solid var(--border); color: var(--text); padding: 0.5rem; border-radius: 8px;">
                    <span style="color: var(--text-muted);">to</span>
                    <input type="date" id="ts-date-to" onchange="loadTimesheets()" style="background: var(--card); border: 1px solid var(--border); color: var(--text); padding: 0.5rem; border-radius: 8px;">
                </div>

                <div id="timesheets-container">
                    <div style="text-align: center; padding: 2rem; color: var(--text-muted);"><div class="spinner"></div>Loading...</div>
                </div>
            </div>
        </main>
    </div>
    
    <!-- Job Modal -->
    <div class="modal" id="job-modal">
        <div class="modal-content">
            <div class="modal-header">
                <h3 id="job-modal-title">New Job</h3>
                <button class="modal-close" onclick="closeJobModal()">&times;</button>
            </div>
            <div class="modal-body">
                <input type="hidden" id="job-id">
                <div class="form-group">
                    <label>Job Name *</label>
                    <input type="text" id="job-name" placeholder="Enter job name">
                </div>
                <div class="form-group">
                    <label>Reference Number</label>
                    <input type="text" id="job-reference" placeholder="Optional reference">
                </div>
                <div class="form-group">
                    <label>Description</label>
                    <textarea id="job-description" rows="3" placeholder="Optional description"></textarea>
                </div>
                <div class="form-group">
                    <label>Customer Name</label>
                    <input type="text" id="job-customer" placeholder="Optional customer name">
                </div>
                <div class="form-group">
                    <label>Status</label>
                    <select id="job-status">
                        <option value="PENDING">Pending</option>
                        <option value="ACTIVE">Active</option>
                        <option value="COMPLETED">Completed</option>
                        <option value="CANCELLED">Cancelled</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Priority</label>
                    <select id="job-priority">
                        <option value="LOW">Low</option>
                        <option value="NORMAL" selected>Normal</option>
                        <option value="HIGH">High</option>
                        <option value="URGENT">Urgent</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Expected Scan Count</label>
                    <input type="number" id="job-expected" min="0" value="0" placeholder="0">
                </div>
                <div class="form-group">
                    <label>Due Date</label>
                    <input type="date" id="job-due-date">
                </div>
                <div class="form-group">
                    <label>Notes</label>
                    <textarea id="job-notes" rows="2" placeholder="Optional notes"></textarea>
                </div>
            </div>
            <div class="modal-footer">
                <button class="btn btn-outline" onclick="closeJobModal()">Cancel</button>
                <button class="btn btn-primary" onclick="saveJob()">Save Job</button>
            </div>
        </div>
    </div>
    
    <!-- Add Scans Modal -->
    <div class="modal" id="scan-modal">
        <div class="modal-content" style="max-width:560px;">
            <div class="modal-header">
                <h3>➕ Add Scans</h3>
                <button class="modal-close" onclick="closeAddScansModal()">&times;</button>
            </div>
            <div class="modal-body" style="padding: 1rem 1.5rem;">
                <div style="display:flex; gap:0.75rem; margin-bottom:1rem; flex-wrap:wrap;">
                    <div class="form-group" style="flex:1; min-width:160px; margin:0;">
                        <label style="font-size:0.8rem;">Job (optional)</label>
                        <select id="scan-modal-job" style="width:100%;">
                            <option value="">No Job</option>
                        </select>
                    </div>
                    <div class="form-group" style="flex:1; min-width:160px; margin:0;">
                        <label style="font-size:0.8rem;">📍 Location</label>
                        <div id="scan-modal-location" style="font-size:0.8rem; padding:0.4rem; border-radius:0.5rem; background:rgba(128,128,128,0.08); color:var(--text-muted); min-height:2rem; display:flex; align-items:center;">
                            Requesting…
                        </div>
                    </div>
                </div>
                <div style="display:flex; gap:0.5rem; margin-bottom:0.75rem;">
                    <input type="text" id="scan-modal-input" placeholder="Enter ticket number and press Enter" style="flex:1;" onkeydown="if(event.key==='Enter')addScanRow()">
                    <button class="btn btn-primary" onclick="addScanRow()" style="white-space:nowrap;">+ Add</button>
                </div>
                <div id="scan-modal-list" style="max-height:280px; overflow-y:auto; border:1px solid var(--border); border-radius:0.75rem;">
                    <div style="text-align:center; padding:1.5rem; color:var(--text-muted); font-size:0.85rem;">No tickets added yet</div>
                </div>
                <div style="margin-top:0.5rem; font-size:0.8rem; color:var(--text-muted);">
                    <span id="scan-modal-count">0</span> ticket(s) ready to submit
                </div>
            </div>
            <div class="modal-footer">
                <button class="btn btn-outline" onclick="closeAddScansModal()">Cancel</button>
                <button class="btn btn-primary" id="scan-modal-submit" onclick="submitAddedScans()" disabled>Submit Scans</button>
            </div>
        </div>
    </div>

    <!-- Settings Modal -->
    <div class="modal" id="settings-modal">
        <div class="modal-content" style="max-width:580px;">
            <div class="modal-header">
                <h3>⚙️ Settings</h3>
                <button class="modal-close" onclick="closeSettings()">&times;</button>
            </div>
            <div class="modal-body" style="padding: 1rem 1.5rem;">
                <!-- Theme Mode -->
                <div style="margin-bottom:1.25rem;">
                    <label style="display:block; font-size:0.8rem; font-weight:600; color:var(--text-muted); margin-bottom:0.5rem; text-transform:uppercase; letter-spacing:0.05em;">Theme Mode</label>
                    <div id="settings-theme-mode" style="display:flex; gap:0.5rem;">
                        <button class="theme-mode-btn active" data-mode="dark" onclick="setThemeMode('dark')">🌙 Dark</button>
                        <button class="theme-mode-btn" data-mode="light" onclick="setThemeMode('light')">☀️ Light</button>
                        <button class="theme-mode-btn" data-mode="midnight" onclick="setThemeMode('midnight')">🌌 Midnight</button>
                        <button class="theme-mode-btn" data-mode="sunset" onclick="setThemeMode('sunset')">🌅 Sunset</button>
                    </div>
                </div>

                <!-- Accent Color -->
                <div style="margin-bottom:1.25rem;">
                    <label style="display:block; font-size:0.8rem; font-weight:600; color:var(--text-muted); margin-bottom:0.5rem; text-transform:uppercase; letter-spacing:0.05em;">Accent Color</label>
                    <div id="settings-accent" style="display:flex; gap:0.5rem; flex-wrap:wrap;">
                        <button class="color-swatch active" data-color="indigo" onclick="setAccentColor('indigo')" style="background:linear-gradient(135deg,#6366f1,#818cf8);" title="Indigo"></button>
                        <button class="color-swatch" data-color="blue" onclick="setAccentColor('blue')" style="background:linear-gradient(135deg,#3b82f6,#60a5fa);" title="Blue"></button>
                        <button class="color-swatch" data-color="cyan" onclick="setAccentColor('cyan')" style="background:linear-gradient(135deg,#06b6d4,#22d3ee);" title="Cyan"></button>
                        <button class="color-swatch" data-color="teal" onclick="setAccentColor('teal')" style="background:linear-gradient(135deg,#14b8a6,#2dd4bf);" title="Teal"></button>
                        <button class="color-swatch" data-color="green" onclick="setAccentColor('green')" style="background:linear-gradient(135deg,#22c55e,#4ade80);" title="Green"></button>
                        <button class="color-swatch" data-color="amber" onclick="setAccentColor('amber')" style="background:linear-gradient(135deg,#f59e0b,#fbbf24);" title="Amber"></button>
                        <button class="color-swatch" data-color="orange" onclick="setAccentColor('orange')" style="background:linear-gradient(135deg,#f97316,#fb923c);" title="Orange"></button>
                        <button class="color-swatch" data-color="rose" onclick="setAccentColor('rose')" style="background:linear-gradient(135deg,#f43f5e,#fb7185);" title="Rose"></button>
                        <button class="color-swatch" data-color="purple" onclick="setAccentColor('purple')" style="background:linear-gradient(135deg,#a855f7,#c084fc);" title="Purple"></button>
                        <button class="color-swatch" data-color="pink" onclick="setAccentColor('pink')" style="background:linear-gradient(135deg,#ec4899,#f472b6);" title="Pink"></button>
                    </div>
                </div>

                <!-- Font Size -->
                <div style="margin-bottom:1.25rem;">
                    <label style="display:block; font-size:0.8rem; font-weight:600; color:var(--text-muted); margin-bottom:0.5rem; text-transform:uppercase; letter-spacing:0.05em;">Font Size</label>
                    <div style="display:flex; align-items:center; gap:0.75rem;">
                        <span style="font-size:0.75rem; color:var(--text-muted);">A</span>
                        <input type="range" id="settings-font-size" min="12" max="20" value="16" step="1" style="flex:1; padding:0;" oninput="setFontSize(this.value)">
                        <span style="font-size:1.1rem; color:var(--text-muted);">A</span>
                        <span id="font-size-label" style="font-size:0.8rem; color:var(--text-muted); min-width:2.5rem; text-align:right;">16px</span>
                    </div>
                </div>

                <!-- Border Radius -->
                <div style="margin-bottom:1.25rem;">
                    <label style="display:block; font-size:0.8rem; font-weight:600; color:var(--text-muted); margin-bottom:0.5rem; text-transform:uppercase; letter-spacing:0.05em;">Corner Roundness</label>
                    <div style="display:flex; align-items:center; gap:0.75rem;">
                        <span style="font-size:0.75rem; color:var(--text-muted);">▪</span>
                        <input type="range" id="settings-radius" min="0" max="20" value="12" step="2" style="flex:1; padding:0;" oninput="setBorderRadius(this.value)">
                        <span style="font-size:0.75rem; color:var(--text-muted);">⬮</span>
                        <span id="radius-label" style="font-size:0.8rem; color:var(--text-muted); min-width:2.5rem; text-align:right;">12px</span>
                    </div>
                </div>

                <!-- Glassmorphism Toggle -->
                <div style="margin-bottom:1.25rem;">
                    <label style="display:flex; align-items:center; gap:0.75rem; cursor:pointer; font-size:0.85rem;">
                        <input type="checkbox" id="settings-glass" checked onchange="toggleGlass(this.checked)" style="width:auto; padding:0;">
                        <span>Enable glassmorphism (blur effects)</span>
                    </label>
                </div>

                <!-- Compact Mode -->
                <div style="margin-bottom:1.25rem;">
                    <label style="display:flex; align-items:center; gap:0.75rem; cursor:pointer; font-size:0.85rem;">
                        <input type="checkbox" id="settings-compact" onchange="toggleCompact(this.checked)" style="width:auto; padding:0;">
                        <span>Compact mode (tighter spacing)</span>
                    </label>
                </div>

                <!-- Gradient Background Toggle -->
                <div style="margin-bottom:1.25rem;">
                    <label style="display:flex; align-items:center; gap:0.75rem; cursor:pointer; font-size:0.85rem;">
                        <input type="checkbox" id="settings-gradient" checked onchange="toggleGradient(this.checked)" style="width:auto; padding:0;">
                        <span>Gradient background</span>
                    </label>
                </div>

                <!-- Animations Toggle -->
                <div style="margin-bottom:0.5rem;">
                    <label style="display:flex; align-items:center; gap:0.75rem; cursor:pointer; font-size:0.85rem;">
                        <input type="checkbox" id="settings-animations" checked onchange="toggleAnimations(this.checked)" style="width:auto; padding:0;">
                        <span>Enable animations</span>
                    </label>
                </div>
            </div>
            <div class="modal-footer" style="justify-content:space-between;">
                <button class="btn btn-outline btn-sm" onclick="resetSettings()">↩️ Reset to Defaults</button>
                <button class="btn btn-primary btn-sm" onclick="closeSettings()">Done</button>
            </div>
        </div>
    </div>

    <!-- Mobile Bottom Nav -->
    <div class="mobile-more-overlay" id="mobile-more-overlay" onclick="closeMobileMore()"></div>
    <div class="mobile-more-menu" id="mobile-more-menu">
        <div class="mobile-more-item" data-page="map" onclick="mobileNav('map')">
            <span>🗺️</span><span>Map</span>
        </div>
        <div class="mobile-more-item" data-page="live" onclick="mobileNav('live')">
            <span>⚡</span><span>Live</span>
        </div>
        <div class="mobile-more-item" data-page="timeclock" onclick="mobileNav('timeclock')">
            <span>⏱️</span><span>Time Clock</span>
        </div>
        <div class="mobile-more-item" data-page="reports" onclick="mobileNav('reports')">
            <span>📋</span><span>Reports</span>
        </div>
        <div class="mobile-more-item" data-page="team" onclick="mobileNav('team')">
            <span>👥</span><span>Team</span>
        </div>
        <div class="mobile-more-item" data-page="timesheets" onclick="mobileNav('timesheets')">
            <span>📋</span><span>Timesheets</span>
        </div>
        <div class="mobile-more-item" data-page="approvals" id="mobile-more-approvals" style="display:none;" onclick="mobileNav('approvals')">
            <span>✅</span><span>Approvals</span>
        </div>
        <div class="mobile-more-item" onclick="closeMobileMore(); openSettings();">
            <span>⚙️</span><span>Settings</span>
        </div>
    </div>
    <nav class="mobile-nav" id="mobile-nav">
        <div class="mobile-nav-inner">
            <div class="mobile-nav-item active" data-page="dashboard" onclick="mobileNav('dashboard')">
                <span class="mobile-nav-icon">🏠</span>
                <span>Home</span>
            </div>
            <div class="mobile-nav-item" data-page="jobs" onclick="mobileNav('jobs')">
                <span class="mobile-nav-icon">�️</span>
                <span>Jobs</span>
            </div>
            <div class="mobile-nav-item" data-page="scans" onclick="mobileNav('scans')">
                <span class="mobile-nav-icon">🏷️</span>
                <span>Scans</span>
            </div>
            <div class="mobile-nav-item" data-page="devices" onclick="mobileNav('devices')">
                <span class="mobile-nav-icon">�</span>
                <span>Devices</span>
            </div>
            <div class="mobile-nav-item" id="mobile-more-btn" onclick="toggleMobileMore()">
                <span class="mobile-nav-icon">⋯</span>
                <span>More</span>
            </div>
        </div>
    </nav>

    <!-- Toast Container -->
    <div class="toast-container" id="toast-container"></div>
    
    <script>
        // ========== Theme Engine ==========
        const ACCENT_COLORS = {{
            indigo:  {{ main: '#6366f1', hover: '#818cf8', glow: 'rgba(99,102,241,0.25)', subtle: 'rgba(99,102,241,0.08)' }},
            blue:    {{ main: '#3b82f6', hover: '#60a5fa', glow: 'rgba(59,130,246,0.25)', subtle: 'rgba(59,130,246,0.08)' }},
            cyan:    {{ main: '#06b6d4', hover: '#22d3ee', glow: 'rgba(6,182,212,0.25)', subtle: 'rgba(6,182,212,0.08)' }},
            teal:    {{ main: '#14b8a6', hover: '#2dd4bf', glow: 'rgba(20,184,166,0.25)', subtle: 'rgba(20,184,166,0.08)' }},
            green:   {{ main: '#22c55e', hover: '#4ade80', glow: 'rgba(34,197,94,0.25)', subtle: 'rgba(34,197,94,0.08)' }},
            amber:   {{ main: '#f59e0b', hover: '#fbbf24', glow: 'rgba(245,158,11,0.25)', subtle: 'rgba(245,158,11,0.08)' }},
            orange:  {{ main: '#f97316', hover: '#fb923c', glow: 'rgba(249,115,22,0.25)', subtle: 'rgba(249,115,22,0.08)' }},
            rose:    {{ main: '#f43f5e', hover: '#fb7185', glow: 'rgba(244,63,94,0.25)', subtle: 'rgba(244,63,94,0.08)' }},
            purple:  {{ main: '#a855f7', hover: '#c084fc', glow: 'rgba(168,85,247,0.25)', subtle: 'rgba(168,85,247,0.08)' }},
            pink:    {{ main: '#ec4899', hover: '#f472b6', glow: 'rgba(236,72,153,0.25)', subtle: 'rgba(236,72,153,0.08)' }},
        }};

        const THEME_MODES = {{
            dark: {{
                bg: '#0a0e1a',
                bgGradient: 'linear-gradient(135deg, #0a0e1a 0%, #111827 50%, #0f172a 100%)',
                card: 'rgba(30,41,59,0.45)',
                cardSolid: '#1e293b',
                cardHover: 'rgba(51,65,85,0.4)',
                text: '#f1f5f9',
                textMuted: '#94a3b8',
                border: 'rgba(255,255,255,0.08)',
                glass: 'rgba(15,23,42,0.3)',
                glassEdge: 'rgba(255,255,255,0.06)',
                glassEdgeStrong: 'rgba(255,255,255,0.1)',
            }},
            light: {{
                bg: '#f1f5f9',
                bgGradient: 'linear-gradient(135deg, #f1f5f9 0%, #e2e8f0 50%, #f8fafc 100%)',
                card: 'rgba(255,255,255,0.55)',
                cardSolid: '#ffffff',
                cardHover: 'rgba(241,245,249,0.5)',
                text: '#0f172a',
                textMuted: '#64748b',
                border: 'rgba(0,0,0,0.08)',
                glass: 'rgba(255,255,255,0.35)',
                glassEdge: 'rgba(255,255,255,0.35)',
                glassEdgeStrong: 'rgba(255,255,255,0.5)',
            }},
            midnight: {{
                bg: '#020617',
                bgGradient: 'linear-gradient(135deg, #020617 0%, #0c0a24 50%, #0a0517 100%)',
                card: 'rgba(15,23,42,0.5)',
                cardSolid: '#0f172a',
                cardHover: 'rgba(30,41,59,0.45)',
                text: '#e2e8f0',
                textMuted: '#64748b',
                border: 'rgba(255,255,255,0.06)',
                glass: 'rgba(2,6,23,0.35)',
                glassEdge: 'rgba(255,255,255,0.05)',
                glassEdgeStrong: 'rgba(255,255,255,0.09)',
            }},
            sunset: {{
                bg: '#1a0e0e',
                bgGradient: 'linear-gradient(135deg, #1a0e0e 0%, #1c1412 50%, #1a1018 100%)',
                card: 'rgba(42,24,24,0.45)',
                cardSolid: '#2a1818',
                cardHover: 'rgba(62,36,36,0.4)',
                text: '#fde8e8',
                textMuted: '#b89898',
                border: 'rgba(255,255,255,0.07)',
                glass: 'rgba(26,14,14,0.3)',
                glassEdge: 'rgba(255,200,200,0.06)',
                glassEdgeStrong: 'rgba(255,200,200,0.1)',
            }},
        }};

        const DEFAULT_SETTINGS = {{
            themeMode: 'dark',
            accentColor: 'indigo',
            fontSize: 16,
            borderRadius: 12,
            glass: true,
            compact: false,
            gradient: true,
            animations: true,
        }};

        function loadSettings() {{
            try {{
                return JSON.parse(localStorage.getItem('cabnet_settings')) || {{ ...DEFAULT_SETTINGS }};
            }} catch {{
                return {{ ...DEFAULT_SETTINGS }};
            }}
        }}

        function saveSettings(settings) {{
            localStorage.setItem('cabnet_settings', JSON.stringify(settings));
        }}

        function applySettings(settings) {{
            const r = document.documentElement.style;
            // Theme mode
            const theme = THEME_MODES[settings.themeMode] || THEME_MODES.dark;
            r.setProperty('--bg', theme.bg);
            r.setProperty('--bg-gradient', settings.gradient ? theme.bgGradient : 'none');
            r.setProperty('--card', settings.glass ? theme.card : theme.cardSolid);
            r.setProperty('--card-solid', theme.cardSolid);
            r.setProperty('--card-hover', theme.cardHover);
            r.setProperty('--text', theme.text);
            r.setProperty('--text-muted', theme.textMuted);
            r.setProperty('--border', theme.border);
            r.setProperty('--glass', theme.glass);
            // Glass edge effects
            r.setProperty('--glass-edge', settings.glass ? (theme.glassEdge || 'rgba(255,255,255,0.12)') : theme.border);
            r.setProperty('--glass-edge-strong', settings.glass ? (theme.glassEdgeStrong || 'rgba(255,255,255,0.18)') : theme.border);
            if (settings.glass) {{
                r.setProperty('--glass-inner', 'inset 0 0.5px 0 0 rgba(255,255,255,0.1)');
                r.setProperty('--glass-shine', 'none');
            }} else {{
                r.setProperty('--glass-inner', 'none');
                r.setProperty('--glass-shine', 'none');
            }}
            // Accent
            const accent = ACCENT_COLORS[settings.accentColor] || ACCENT_COLORS.indigo;
            r.setProperty('--accent', accent.main);
            r.setProperty('--accent-hover', accent.hover);
            r.setProperty('--accent-glow', accent.glow);
            r.setProperty('--accent-subtle', accent.subtle);
            // Font size
            r.setProperty('--font-size', settings.fontSize + 'px');
            // Border radius
            r.setProperty('--radius', settings.borderRadius + 'px');
            // Glass
            if (!settings.glass) {{
                r.setProperty('--shadow', '0 1px 3px rgba(0,0,0,0.08)');
                r.setProperty('--shadow-lg', '0 4px 12px rgba(0,0,0,0.1)');
            }} else {{
                r.setProperty('--shadow', '0 1px 3px rgba(0,0,0,0.08), 0 4px 16px rgba(0,0,0,0.06)');
                r.setProperty('--shadow-lg', '0 2px 8px rgba(0,0,0,0.08), 0 12px 40px rgba(0,0,0,0.1)');
            }}
            // Compact
            document.body.classList.toggle('compact', !!settings.compact);
            // Animations
            document.body.classList.toggle('no-animations', !settings.animations);
            // Gradient bg
            document.body.style.backgroundImage = settings.gradient ? theme.bgGradient : 'none';
        }}

        function syncSettingsUI(settings) {{
            // Theme mode buttons
            document.querySelectorAll('.theme-mode-btn').forEach(b => {{
                b.classList.toggle('active', b.dataset.mode === settings.themeMode);
            }});
            // Accent color swatches
            document.querySelectorAll('.color-swatch').forEach(b => {{
                b.classList.toggle('active', b.dataset.color === settings.accentColor);
            }});
            // Sliders
            const fs = document.getElementById('settings-font-size');
            if (fs) {{ fs.value = settings.fontSize; document.getElementById('font-size-label').textContent = settings.fontSize + 'px'; }}
            const rd = document.getElementById('settings-radius');
            if (rd) {{ rd.value = settings.borderRadius; document.getElementById('radius-label').textContent = settings.borderRadius + 'px'; }}
            // Checkboxes
            const glassEl = document.getElementById('settings-glass');
            if (glassEl) glassEl.checked = settings.glass;
            const compactEl = document.getElementById('settings-compact');
            if (compactEl) compactEl.checked = settings.compact;
            const gradEl = document.getElementById('settings-gradient');
            if (gradEl) gradEl.checked = settings.gradient;
            const animEl = document.getElementById('settings-animations');
            if (animEl) animEl.checked = settings.animations;
        }}

        let currentSettings = loadSettings();
        applySettings(currentSettings);

        function openSettings() {{
            syncSettingsUI(currentSettings);
            document.getElementById('settings-modal').classList.add('active');
        }}
        function closeSettings() {{
            document.getElementById('settings-modal').classList.remove('active');
        }}
        function setThemeMode(mode) {{
            currentSettings.themeMode = mode;
            saveSettings(currentSettings);
            applySettings(currentSettings);
            syncSettingsUI(currentSettings);
        }}
        function setAccentColor(color) {{
            currentSettings.accentColor = color;
            saveSettings(currentSettings);
            applySettings(currentSettings);
            syncSettingsUI(currentSettings);
        }}
        function setFontSize(val) {{
            currentSettings.fontSize = parseInt(val);
            document.getElementById('font-size-label').textContent = val + 'px';
            saveSettings(currentSettings);
            applySettings(currentSettings);
        }}
        function setBorderRadius(val) {{
            currentSettings.borderRadius = parseInt(val);
            document.getElementById('radius-label').textContent = val + 'px';
            saveSettings(currentSettings);
            applySettings(currentSettings);
        }}
        function toggleGlass(on) {{
            currentSettings.glass = on;
            saveSettings(currentSettings);
            applySettings(currentSettings);
        }}
        function toggleCompact(on) {{
            currentSettings.compact = on;
            saveSettings(currentSettings);
            applySettings(currentSettings);
        }}
        function toggleGradient(on) {{
            currentSettings.gradient = on;
            saveSettings(currentSettings);
            applySettings(currentSettings);
        }}
        function toggleAnimations(on) {{
            currentSettings.animations = on;
            saveSettings(currentSettings);
            applySettings(currentSettings);
        }}
        function resetSettings() {{
            currentSettings = {{ ...DEFAULT_SETTINGS }};
            saveSettings(currentSettings);
            applySettings(currentSettings);
            syncSettingsUI(currentSettings);
            showToast('success', 'Settings reset to defaults');
        }}

        // ========== App State ==========
        // State
        let clientId = localStorage.getItem('cabnet_client_id');
        let clientName = localStorage.getItem('cabnet_client_name') || '';
        let isTrusted = false;
        let jobs = [];
        let devices = [];
        let scans = [];
        
        // Initialize
        async function init() {{
            if (!clientId) {{
                // First time — ask for their name
                let name = '';
                while (!name.trim()) {{
                    name = prompt('Welcome to CabNet! Please enter your name:') || '';
                }}
                await registerClient(name.trim());
            }} else {{
                await checkClientStatus();
            }}
            
            loadJobs();
            loadDevices();
            loadDashboardHome();
            
            updateClientDisplay();
        }}
        
        // Register as new web client
        async function registerClient(name) {{
            try {{
                const response = await fetch('/api/web/register', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ client_name: name }})
                }});
                const data = await response.json();
                if (data.success) {{
                    clientId = data.client_id;
                    clientName = data.client_name || name;
                    isTrusted = data.is_trusted;
                    localStorage.setItem('cabnet_client_id', clientId);
                    localStorage.setItem('cabnet_client_name', clientName);
                    updateTrustBadge();
                    updateClientDisplay();
                }}
            }} catch (e) {{
                console.error('Failed to register client:', e);
            }}
        }}

        function updateClientDisplay() {{
            const display = document.getElementById('client-id-display');
            if (clientName) {{
                display.textContent = '👤 ' + clientName;
            }} else {{
                display.textContent = 'Client: ' + (clientId || 'unregistered').substring(0, 8) + '...';
            }}
        }}
        
        // Check client status
        async function checkClientStatus() {{
            try {{
                const response = await fetch(`/api/web/status?client_id=${{clientId}}`);
                const data = await response.json();
                if (data.success) {{
                    isTrusted = data.is_trusted;
                    if (data.client_name) {{
                        clientName = data.client_name;
                        localStorage.setItem('cabnet_client_name', clientName);
                    }}
                    // If no name on server or locally, ask for it now
                    if (!clientName) {{
                        let name = '';
                        while (!name.trim()) {{
                            name = prompt('Welcome to CabNet! Please enter your name:') || '';
                        }}
                        clientName = name.trim();
                        localStorage.setItem('cabnet_client_name', clientName);
                        // Update the server
                        try {{
                            await fetch(`/api/web/clients/${{clientId}}/name`, {{
                                method: 'PUT',
                                headers: {{ 'Content-Type': 'application/json' }},
                                body: JSON.stringify({{ name: clientName }})
                            }});
                        }} catch (e) {{}}
                    }}
                    updateTrustBadge();
                    updateClientDisplay();
                }} else {{
                    // Client not found, re-register
                    localStorage.removeItem('cabnet_client_id');
                    localStorage.removeItem('cabnet_client_name');
                    clientId = null;
                    clientName = '';
                    let name = '';
                    while (!name.trim()) {{
                        name = prompt('Your session expired. Please enter your name:') || '';
                    }}
                    await registerClient(name.trim());
                }}
            }} catch (e) {{
                console.error('Failed to check status:', e);
            }}
        }}
        
        // Update trust badge
        function updateTrustBadge() {{
            const badge = document.getElementById('trust-badge');
            const approvalsNav = document.getElementById('nav-approvals');
            const mobileApprovals = document.getElementById('mobile-more-approvals');
            if (isTrusted) {{
                badge.className = 'trust-status trusted';
                badge.innerHTML = '<span>✓</span><span>Trusted Device</span>';
                if (approvalsNav) approvalsNav.style.display = '';
                if (mobileApprovals) mobileApprovals.style.display = '';
                loadApprovalsBadge();
            }} else {{
                badge.className = 'trust-status untrusted';
                badge.innerHTML = '<span>⏳</span><span>Changes need approval</span>';
                if (approvalsNav) approvalsNav.style.display = 'none';
                if (mobileApprovals) mobileApprovals.style.display = 'none';
            }}
        }}
        
        // Submit a change (handles trust status)
        async function submitChange(changeType, entityType, entityId, changeData) {{
            try {{
                const response = await fetch('/api/web/changes', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        client_id: clientId,
                        change_type: changeType,
                        entity_type: entityType,
                        entity_id: entityId,
                        change_data: changeData
                    }})
                }});
                const data = await response.json();
                if (data.success) {{
                    if (data.applied) {{
                        showToast('success', data.message);
                    }} else {{
                        showToast('warning', 'Change queued for approval');
                    }}
                    return true;
                }} else {{
                    showToast('error', data.error || 'Failed to submit change');
                    return false;
                }}
            }} catch (e) {{
                showToast('error', 'Network error: ' + e.message);
                return false;
            }}
        }}
        
        // ========== Approvals Page ==========
        async function loadApprovalsBadge() {{
            try {{
                const response = await fetch('/api/web/changes/pending');
                const data = await response.json();
                const badge = document.getElementById('nav-approvals-badge');
                if (badge && data.success) {{
                    const count = data.count || 0;
                    badge.textContent = count;
                    badge.style.display = count > 0 ? 'inline-block' : 'none';
                }}
            }} catch (e) {{
                console.error('Failed to load approvals badge:', e);
            }}
        }}

        async function loadApprovals() {{
            const container = document.getElementById('approvals-container');
            container.innerHTML = '<div style="text-align:center; padding:2rem; color:var(--text-muted);"><div class="spinner"></div>Loading...</div>';
            try {{
                // Fetch pending changes and web clients in parallel
                const [changesRes, clientsRes] = await Promise.all([
                    fetch('/api/web/changes/pending'),
                    fetch('/api/web/clients')
                ]);
                const data = await changesRes.json();
                let clientMap = {{}};
                try {{
                    const clientsData = await clientsRes.json();
                    if (clientsData.success && clientsData.clients) {{
                        clientsData.clients.forEach(cl => {{ clientMap[cl.client_id] = cl.client_name || cl.client_id.substring(0, 8) + '...'; }});
                    }}
                }} catch(e) {{}}
                if (!data.success) {{
                    container.innerHTML = '<div style="text-align:center; padding:2rem; color:var(--text-muted);">Failed to load approvals.</div>';
                    return;
                }}
                const changes = data.changes || [];
                if (changes.length === 0) {{
                    container.innerHTML = '<div style="text-align:center; padding:2rem; color:var(--text-muted);"><div style="font-size:2rem; margin-bottom:1rem;">✅</div><div>No pending approvals</div></div>';
                    return;
                }}
                container.innerHTML = changes.map(c => {{
                    let details = '';
                    try {{
                        const d = typeof c.change_data === 'string' ? JSON.parse(c.change_data) : c.change_data;
                        details = Object.entries(d).map(([k, v]) =>
                            `<div style="display:flex; justify-content:space-between; padding:0.25rem 0; border-bottom:1px solid var(--border-color);">` +
                            `<span style="color:var(--text-muted); font-size:0.8rem;">${{k}}</span>` +
                            `<span style="font-size:0.8rem;">${{typeof v === 'object' ? JSON.stringify(v) : v}}</span></div>`
                        ).join('');
                    }} catch (e) {{
                        details = `<div style="font-size:0.8rem; color:var(--text-muted);">${{c.change_data}}</div>`;
                    }}
                    const typeIcon = c.change_type === 'delete' ? '🗑️' : c.change_type === 'create' ? '➕' : '✏️';
                    const entityLabel = (c.entity_type || '').replace(/_/g, ' ');
                    const timeAgo = c.created_at ? new Date(c.created_at).toLocaleString() : 'Unknown';
                    return `<div class="card" style="margin-bottom:0.75rem; border-left:3px solid var(--warning-color);">` +
                        `<div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:0.5rem;">` +
                            `<div style="display:flex; align-items:center; gap:0.5rem;">` +
                                `<span style="font-size:1.2rem;">${{typeIcon}}</span>` +
                                `<strong style="text-transform:capitalize;">${{c.change_type}} ${{entityLabel}}</strong>` +
                            `</div>` +
                            `<span class="badge" style="background:var(--warning-color); color:#000;">#${{c.id}}</span>` +
                        `</div>` +
                        (c.entity_id ? `<div style="font-size:0.8rem; color:var(--text-muted); margin-bottom:0.5rem;">Entity ID: ${{c.entity_id}}</div>` : '') +
                        `<div style="background:var(--bg-color); border-radius:6px; padding:0.5rem; margin-bottom:0.75rem;">${{details}}</div>` +
                        `<div style="display:flex; justify-content:space-between; align-items:center;">` +
                            `<div style="font-size:0.75rem; color:var(--text-muted);">From: ${{clientMap[c.client_id] || c.client_id.substring(0,8) + '...'}} · ${{timeAgo}}</div>` +
                            `<div style="display:flex; gap:0.5rem;">` +
                                `<button class="btn" style="background:var(--success-color); color:#fff; padding:0.4rem 1rem; font-size:0.85rem;" onclick="approveChange(${{c.id}})">✓ Approve</button>` +
                                `<button class="btn" style="background:var(--danger-color); color:#fff; padding:0.4rem 1rem; font-size:0.85rem;" onclick="rejectChange(${{c.id}})">✗ Reject</button>` +
                            `</div>` +
                        `</div>` +
                    `</div>`;
                }}).join('');
            }} catch (e) {{
                container.innerHTML = '<div style="text-align:center; padding:2rem; color:var(--danger-color);">Error loading approvals: ' + e.message + '</div>';
            }}
            loadApprovalsBadge();
        }}

        async function approveChange(id) {{
            try {{
                const response = await fetch(`/api/web/changes/${{id}}/approve`, {{ method: 'POST' }});
                const data = await response.json();
                if (data.success) {{
                    showToast('success', 'Change approved and applied');
                    loadApprovals();
                }} else {{
                    showToast('error', data.error || 'Failed to approve change');
                }}
            }} catch (e) {{
                showToast('error', 'Network error: ' + e.message);
            }}
        }}

        async function rejectChange(id) {{
            try {{
                const response = await fetch(`/api/web/changes/${{id}}/reject`, {{ method: 'POST' }});
                const data = await response.json();
                if (data.success) {{
                    showToast('success', 'Change rejected');
                    loadApprovals();
                }} else {{
                    showToast('error', data.error || 'Failed to reject change');
                }}
            }} catch (e) {{
                showToast('error', 'Network error: ' + e.message);
            }}
        }}

        // Navigation
        function showPage(pageId) {{
            document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
            // Update sidebar nav
            document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
            // Update mobile bottom nav
            document.querySelectorAll('.mobile-nav-item').forEach(n => n.classList.remove('active'));
            document.querySelectorAll('.mobile-more-item').forEach(n => n.classList.remove('active'));
            
            document.getElementById('page-' + pageId).classList.add('active');
            // Highlight in sidebar
            const sidebarEl = document.querySelector(`.sidebar [data-page="${{pageId}}"]`);
            if (sidebarEl) sidebarEl.classList.add('active');
            // Highlight in mobile nav
            const mobileEl = document.querySelector(`.mobile-nav-item[data-page="${{pageId}}"]`);
            if (mobileEl) mobileEl.classList.add('active');
            // Highlight in mobile more menu
            const moreEl = document.querySelector(`.mobile-more-item[data-page="${{pageId}}"]`);
            if (moreEl) {{
                moreEl.classList.add('active');
                // Also highlight the More button
                document.getElementById('mobile-more-btn').classList.add('active');
            }}
            
            // Stop live mode when leaving live page
            if (pageId !== 'live' && liveMode) {{
                toggleLiveMode();
            }}
            
            // Load data for page
            if (pageId === 'dashboard') loadDashboardHome();
            if (pageId === 'jobs') loadJobs();
            if (pageId === 'devices') loadDevices();
            if (pageId === 'scans') loadScans();
            if (pageId === 'approvals') loadApprovals();
            if (pageId === 'timeclock') loadTimeEntries();
            if (pageId === 'reports') loadReports();
            if (pageId === 'team') loadTeamStatus();
            if (pageId === 'timesheets') loadTimesheets();
        }}
        
        document.querySelectorAll('.nav-item').forEach(item => {{
            item.addEventListener('click', () => showPage(item.dataset.page));
        }});

        // Mobile nav helpers
        function mobileNav(pageId) {{
            closeMobileMore();
            showPage(pageId);
        }}
        function toggleMobileMore() {{
            document.getElementById('mobile-more-menu').classList.toggle('open');
            document.getElementById('mobile-more-overlay').classList.toggle('open');
        }}
        function closeMobileMore() {{
            document.getElementById('mobile-more-menu').classList.remove('open');
            document.getElementById('mobile-more-overlay').classList.remove('open');
        }}
        
        // Jobs
        async function loadJobs() {{
            try {{
                const response = await fetch('/api/jobs');
                const data = await response.json();
                jobs = data.jobs || [];
                
                // Update scans filter
                const filter = document.getElementById('scans-job-filter');
                filter.innerHTML = '<option value="">All Jobs</option>' + 
                    jobs.map(j => `<option value="${{j.server_id}}">${{j.name}}</option>`).join('');
                
                // Update status counts
                const active = jobs.filter(j => j.status === 'ACTIVE').length;
                const pending = jobs.filter(j => j.status === 'PENDING').length;
                const completed = jobs.filter(j => j.status === 'COMPLETED').length;
                const ael = document.getElementById('jobs-active-count');
                const pel = document.getElementById('jobs-pending-count');
                const cel = document.getElementById('jobs-completed-count');
                if (ael) ael.textContent = active;
                if (pel) pel.textContent = pending;
                if (cel) cel.textContent = completed;
                
                sortAndRenderJobs();
                document.getElementById('stat-jobs').textContent = jobs.length;
            }} catch (e) {{
                console.error('Failed to load jobs:', e);
            }}
        }}
        
        function sortAndRenderJobs() {{
            const sortBy = document.getElementById('jobs-sort')?.value || 'newest';
            const priorityOrder = {{ 'URGENT': 0, 'HIGH': 1, 'NORMAL': 2, 'LOW': 3 }};
            
            jobs.sort((a, b) => {{
                switch (sortBy) {{
                    case 'oldest': return (a.created_at || 0) - (b.created_at || 0);
                    case 'name': return (a.name || '').localeCompare(b.name || '');
                    case 'progress':
                        const pctA = a.expected_count > 0 ? a.scan_count / a.expected_count : 0;
                        const pctB = b.expected_count > 0 ? b.scan_count / b.expected_count : 0;
                        return pctB - pctA;
                    case 'priority':
                        return (priorityOrder[a.priority] ?? 2) - (priorityOrder[b.priority] ?? 2);
                    default: // newest
                        return (b.created_at || 0) - (a.created_at || 0);
                }}
            }});
            
            renderJobCards();
        }}
        
        function statusColor(status) {{
            switch (status) {{
                case 'ACTIVE': return '#4caf50';
                case 'COMPLETED': return '#2196f3';
                case 'CANCELLED': return '#f44336';
                default: return '#ffc107';
            }}
        }}
        
        function renderJobCards() {{
            const container = document.getElementById('jobs-cards-container');
            const searchText = (document.getElementById('jobs-search')?.value || '').toLowerCase();
            const statusFilter = document.getElementById('jobs-status-filter')?.value || '';
            
            const filtered = jobs.filter(j => {{
                const matchStatus = !statusFilter || j.status === statusFilter;
                const matchSearch = !searchText 
                    || j.name.toLowerCase().includes(searchText)
                    || (j.reference_number || '').toLowerCase().includes(searchText)
                    || (j.customer_name || '').toLowerCase().includes(searchText);
                return matchStatus && matchSearch;
            }});
            
            document.getElementById('jobs-count').textContent = filtered.length;
            
            if (filtered.length === 0) {{
                container.innerHTML = '<div style="text-align: center; padding: 2rem; color: var(--text-muted); grid-column: 1 / -1;"><div class="icon" style="font-size: 2rem; margin-bottom: 0.5rem;">�️</div><p>No jobs found</p></div>';
                return;
            }}
            
            container.innerHTML = filtered.map(job => {{
                const progressPct = job.expected_count > 0 ? Math.min(100, Math.round(job.scan_count / job.expected_count * 100)) : 0;
                const progressColor = progressPct >= 100 ? 'var(--success, #4caf50)' : progressPct > 50 ? 'var(--warning, #ff9800)' : 'var(--accent)';
                const dueDateStr = job.due_date ? new Date(job.due_date).toLocaleDateString() : '';
                const isOverdue = job.due_date && job.due_date < Date.now() && job.status !== 'COMPLETED' && job.status !== 'CANCELLED';
                const createdStr = job.created_at ? new Date(job.created_at).toLocaleDateString() : '-';
                const updatedStr = job.updated_at ? new Date(job.updated_at).toLocaleDateString() : '-';
                const startedStr = job.started_at ? new Date(job.started_at).toLocaleString() : '-';
                const completedStr = job.completed_at ? new Date(job.completed_at).toLocaleString() : '-';
                
                return `
                <div class="job-card" id="job-card-${{job.id}}" onclick="toggleJobCard('${{job.id}}', event)">
                    <div class="job-card-summary">
                        <div class="job-card-top">
                            <div style="flex: 1; min-width: 0;">
                                <div class="job-card-name">${{escapeHtml(job.name)}}</div>
                                ${{job.reference_number ? `<div class="job-card-ref">Ref: ${{escapeHtml(job.reference_number)}}</div>` : ''}}
                                <div class="job-card-meta">
                                    <span class="badge" style="background: ${{statusColor(job.status)}}; color: white; padding: 0.15rem 0.5rem; border-radius: 9999px; font-size: 0.7rem;">${{job.status}}</span>
                                    <span class="priority-badge priority-${{job.priority || 'NORMAL'}}">${{job.priority || 'NORMAL'}}</span>
                                    ${{job.customer_name ? `<span style="font-size: 0.75rem;">👤 ${{escapeHtml(job.customer_name)}}</span>` : ''}}
                                </div>
                                ${{isOverdue ? '<div style="color: #f44336; font-size: 0.75rem; margin-top: 0.15rem;">⚠ Overdue</div>' : ''}}
                            </div>
                            <span class="job-card-expand-icon">▼</span>
                        </div>
                        
                        ${{job.expected_count > 0 ? `
                        <div class="job-card-progress">
                            <div class="job-card-progress-bar">
                                <div class="job-card-progress-fill" style="width: ${{progressPct}}%; background: ${{progressColor}};"></div>
                            </div>
                            <div class="job-card-progress-text">
                                <span>${{job.scan_count}} / ${{job.expected_count}} scans</span>
                                <span>${{progressPct}}%</span>
                            </div>
                        </div>` : `
                        <div style="font-size: 0.75rem; color: var(--text-muted); margin-top: 0.3rem;">${{job.scan_count}} scans</div>
                        `}}
                    </div>
                    
                    <div class="job-card-details">
                        <div class="job-card-tabs">
                            <div class="job-card-tab active" onclick="switchJobTab('${{job.id}}', 'info', event)">�️ Details</div>
                            <div class="job-card-tab" onclick="switchJobTab('${{job.id}}', 'scans', event)">🏷️ Scans (${{job.scan_count}})</div>
                            <div class="job-card-tab" onclick="switchJobTab('${{job.id}}', 'edit', event)">✏️ Edit</div>
                        </div>
                        
                        <div class="job-tab-content active" id="job-tab-info-${{job.id}}">
                            <div class="job-card-details-inner">
                                <div class="job-stat-grid">
                                    <div class="job-stat-box">
                                        <div class="stat-val">${{job.scan_count}}</div>
                                        <div class="stat-lbl">Scans</div>
                                    </div>
                                    <div class="job-stat-box">
                                        <div class="stat-val">${{job.expected_count || '—'}}</div>
                                        <div class="stat-lbl">Expected</div>
                                    </div>
                                    <div class="job-stat-box">
                                        <div class="stat-val" style="font-size: 0.95rem;">${{createdStr}}</div>
                                        <div class="stat-lbl">Created</div>
                                    </div>
                                    <div class="job-stat-box">
                                        <div class="stat-val" style="font-size: 0.95rem;">${{updatedStr}}</div>
                                        <div class="stat-lbl">Updated</div>
                                    </div>
                                </div>
                                
                                ${{job.description ? `<div class="detail-row"><span class="detail-label">Description:</span><span class="detail-value">${{escapeHtml(job.description)}}</span></div>` : ''}}
                                ${{job.customer_name ? `<div class="detail-row"><span class="detail-label">Customer:</span><span class="detail-value">${{escapeHtml(job.customer_name)}}</span></div>` : ''}}
                                ${{job.reference_number ? `<div class="detail-row"><span class="detail-label">Reference:</span><span class="detail-value">${{escapeHtml(job.reference_number)}}</span></div>` : ''}}
                                ${{dueDateStr ? `<div class="detail-row"><span class="detail-label">Due Date:</span><span class="detail-value" style="${{isOverdue ? 'color:#f44336;font-weight:600;' : ''}}">${{dueDateStr}}${{isOverdue ? ' (overdue)' : ''}}</span></div>` : ''}}
                                ${{job.notes ? `<div class="detail-row"><span class="detail-label">Notes:</span><span class="detail-value">${{escapeHtml(job.notes)}}</span></div>` : ''}}
                                ${{job.started_at ? `<div class="detail-row"><span class="detail-label">Started:</span><span class="detail-value">${{startedStr}}</span></div>` : ''}}
                                ${{job.completed_at ? `<div class="detail-row"><span class="detail-label">Completed:</span><span class="detail-value">${{completedStr}}</span></div>` : ''}}
                                ${{job.device_id ? `<div class="detail-row"><span class="detail-label">Device:</span><span class="detail-value" style="font-family:monospace;font-size:0.8rem;">${{job.device_id.substring(0,16)}}${{job.device_id.length > 16 ? '...' : ''}}</span></div>` : ''}}
                                ${{job.created_by ? `<div class="detail-row"><span class="detail-label">Created by:</span><span class="detail-value">${{escapeHtml(job.created_by)}}</span></div>` : ''}}
                            </div>
                        </div>
                        
                        <div class="job-tab-content" id="job-tab-scans-${{job.id}}">
                            <div class="job-card-details-inner">
                                <input type="text" class="job-scans-search" placeholder="🔍 Search scans..." oninput="filterJobScansTab('${{job.id}}', this.value)" onclick="event.stopPropagation()"
                                       style="width:100%; padding:0.4rem 0.6rem; font-size:0.8rem; margin-bottom:0.5rem; background:var(--bg); color:var(--text); border:1px solid var(--border); border-radius:6px; box-sizing:border-box;">
                                <div id="job-scans-list-${{job.id}}" style="max-height:350px; overflow-y:auto;">
                                    <div style="text-align:center; padding:1rem; color:var(--text-muted); font-size:0.8rem;">Click to load scans...</div>
                                </div>
                                <div style="margin-top:0.5rem; text-align:right;">
                                    <button class="btn btn-outline btn-sm" onclick="viewJobScans('${{job.id}}', event)">View All in Scans Page →</button>
                                </div>
                            </div>
                        </div>
                        
                        <div class="job-tab-content" id="job-tab-edit-${{job.id}}">
                            <div class="job-card-details-inner">
                                <div class="edit-grid">
                                    <label>Name:</label>
                                    <input type="text" id="edit-job-name-${{job.id}}" value="${{escapeHtml(job.name)}}">
                                    <label>Reference:</label>
                                    <input type="text" id="edit-job-ref-${{job.id}}" value="${{escapeHtml(job.reference_number || '')}}">
                                    <label>Status:</label>
                                    <select id="edit-job-status-${{job.id}}">
                                        ${{['PENDING','ACTIVE','COMPLETED','CANCELLED'].map(s => `<option value="${{s}}" ${{job.status === s ? 'selected' : ''}}>${{s}}</option>`).join('')}}
                                    </select>
                                    <label>Priority:</label>
                                    <select id="edit-job-priority-${{job.id}}">
                                        ${{['LOW','NORMAL','HIGH','URGENT'].map(p => `<option value="${{p}}" ${{job.priority === p ? 'selected' : ''}}>${{p}}</option>`).join('')}}
                                    </select>
                                    <label>Customer:</label>
                                    <input type="text" id="edit-job-customer-${{job.id}}" value="${{escapeHtml(job.customer_name || '')}}">
                                    <label>Expected:</label>
                                    <input type="number" id="edit-job-expected-${{job.id}}" value="${{job.expected_count || 0}}" min="0">
                                    <label>Due Date:</label>
                                    <input type="date" id="edit-job-due-${{job.id}}" value="${{job.due_date ? new Date(job.due_date).toISOString().split('T')[0] : ''}}">
                                    <label>Description:</label>
                                    <textarea id="edit-job-desc-${{job.id}}" rows="2">${{escapeHtml(job.description || '')}}</textarea>
                                    <label>Notes:</label>
                                    <textarea id="edit-job-notes-${{job.id}}" rows="2">${{escapeHtml(job.notes || '')}}</textarea>
                                </div>
                                <div style="display: flex; gap: 0.5rem; margin-top: 0.75rem;">
                                    <button class="btn btn-primary btn-sm" onclick="saveJobEdit('${{job.id}}', event)">💾 Save</button>
                                    <button class="btn btn-outline btn-sm" onclick="switchJobTab('${{job.id}}', 'info', event)">Cancel</button>
                                </div>
                            </div>
                        </div>
                        
                        <div class="job-card-actions">
                            <button class="btn btn-outline btn-sm" onclick="viewJobScans('${{job.id}}', event)">🏷️ View Scans</button>
                            ${{job.status === 'PENDING' ? `<button class="btn btn-primary btn-sm" onclick="quickStatusChange('${{job.id}}', 'ACTIVE', event)">▶ Start</button>` : ''}}
                            ${{job.status === 'ACTIVE' ? `<button class="btn btn-primary btn-sm" onclick="quickStatusChange('${{job.id}}', 'COMPLETED', event)">✓ Complete</button>` : ''}}
                            ${{job.status !== 'CANCELLED' && job.status !== 'COMPLETED' ? `<button class="btn btn-outline btn-sm" style="color:#f44336;border-color:#f44336;" onclick="quickStatusChange('${{job.id}}', 'CANCELLED', event)">✕ Cancel</button>` : ''}}
                            <button class="btn btn-danger btn-sm" onclick="deleteJob('${{job.id}}', event)" style="margin-left: auto;">🗑 Delete</button>
                        </div>
                    </div>
                </div>`;
            }}).join('');
        }}
        
        function filterJobCards() {{
            renderJobCards();
        }}
        
        function toggleJobCard(id, event) {{
            // Don't toggle if clicking on buttons/inputs inside
            if (event.target.closest('button') || event.target.closest('input') || event.target.closest('select') || event.target.closest('textarea') || event.target.closest('.job-card-tab')) return;
            const card = document.getElementById(`job-card-${{id}}`);
            if (!card) return;
            card.classList.toggle('expanded');
        }}
        
        function switchJobTab(id, tab, event) {{
            if (event) event.stopPropagation();
            const card = document.getElementById(`job-card-${{id}}`);
            if (!card) return;
            // Ensure expanded
            card.classList.add('expanded');
            // Switch tab buttons
            card.querySelectorAll('.job-card-tab').forEach(t => t.classList.remove('active'));
            event.target.classList.add('active');
            // Switch tab content
            card.querySelectorAll('.job-tab-content').forEach(c => c.classList.remove('active'));
            const target = document.getElementById(`job-tab-${{tab}}-${{id}}`);
            if (target) target.classList.add('active');
            // Load scans when switching to scans tab
            if (tab === 'scans') {{
                loadJobScansTab(id);
            }}
        }}

        // Cache for loaded job scans
        const jobScansCache = {{}};

        async function loadJobScansTab(jobId) {{
            const container = document.getElementById(`job-scans-list-${{jobId}}`);
            if (!container) return;

            // Find server_id for this job
            const job = jobs.find(j => j.id === jobId);
            const serverId = job ? job.server_id : null;
            if (!serverId) {{
                container.innerHTML = '<div style="text-align:center; padding:1rem; color:var(--text-muted); font-size:0.8rem;">No server ID for this job</div>';
                return;
            }}

            container.innerHTML = '<div style="text-align:center; padding:1rem; color:var(--text-muted);"><div class="spinner"></div>Loading scans...</div>';

            try {{
                const response = await fetch(`/api/scans?job_id=${{serverId}}&limit=500`);
                const data = await response.json();
                const jobScans = data.scans || [];
                jobScansCache[jobId] = jobScans;
                renderJobScansTab(jobId, jobScans);
            }} catch (e) {{
                container.innerHTML = '<div style="text-align:center; padding:1rem; color:var(--danger);">Failed to load scans</div>';
            }}
        }}

        function renderJobScansTab(jobId, scanList) {{
            const container = document.getElementById(`job-scans-list-${{jobId}}`);
            if (!container) return;

            if (scanList.length === 0) {{
                container.innerHTML = '<div style="text-align:center; padding:1rem; color:var(--text-muted); font-size:0.8rem;">\ud83d\udce6 No scans for this job yet</div>';
                return;
            }}

            container.innerHTML = `
                <div style="display:flex; padding:0.3rem 0; border-bottom:1px solid var(--border); font-size:0.7rem; color:var(--text-muted); font-weight:600;">
                    <span style="flex:1;">Ticket / Barcode</span>
                    <span style="width:80px; text-align:center;">Device</span>
                    <span style="width:100px; text-align:right;">Time</span>
                </div>
                ${{scanList.map(s => {{
                    const ticket = s.ticket_number || stripJobPrefix(s.barcode);
                    const devShort = s.device_id ? s.device_id.substring(0,8) : '-';
                    const timeStr = new Date(s.scanned_at).toLocaleString('en-US', {{month:'short', day:'numeric', hour:'2-digit', minute:'2-digit'}});
                    return `<div class="job-scan-row" data-search="${{(ticket + ' ' + s.barcode + ' ' + (s.device_id || '')).toLowerCase()}}">
                        <span style="flex:1; font-family:monospace; color:var(--accent); font-size:0.8rem; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">${{escapeHtml(ticket)}}</span>
                        <span style="width:80px; text-align:center; font-size:0.75rem; color:var(--text-muted);">${{devShort}}</span>
                        <span style="width:100px; text-align:right; font-size:0.75rem; color:var(--text-muted);">${{timeStr}}</span>
                    </div>`;
                }}).join('')}}
            `;
        }}

        function filterJobScansTab(jobId, query) {{
            const container = document.getElementById(`job-scans-list-${{jobId}}`);
            if (!container) return;
            const q = query.toLowerCase().trim();
            container.querySelectorAll('.job-scan-row').forEach(row => {{
                if (!q) {{
                    row.style.display = '';
                }} else {{
                    row.style.display = (row.dataset.search || '').includes(q) ? '' : 'none';
                }}
            }});
        }}
        
        async function saveJobEdit(id, event) {{
            if (event) event.stopPropagation();
            const name = document.getElementById(`edit-job-name-${{id}}`).value.trim();
            if (!name) {{ showToast('error', 'Job name is required'); return; }}
            
            const dueVal = document.getElementById(`edit-job-due-${{id}}`).value;
            
            const jobData = {{
                name: name,
                status: document.getElementById(`edit-job-status-${{id}}`).value,
                reference_number: document.getElementById(`edit-job-ref-${{id}}`).value.trim() || null,
                description: document.getElementById(`edit-job-desc-${{id}}`).value.trim() || null,
                customer_name: document.getElementById(`edit-job-customer-${{id}}`).value.trim() || null,
                notes: document.getElementById(`edit-job-notes-${{id}}`).value.trim() || null,
                priority: document.getElementById(`edit-job-priority-${{id}}`).value,
                expected_count: parseInt(document.getElementById(`edit-job-expected-${{id}}`).value) || 0,
                due_date: dueVal ? new Date(dueVal).toISOString() : null,
            }};
            
            const success = await submitChange('update', 'job_details', String(id), jobData);
            if (success) loadJobs();
        }}
        
        async function quickStatusChange(id, newStatus, event) {{
            if (event) event.stopPropagation();
            const success = await submitChange('update', 'job_details', String(id), {{ status: newStatus }});
            if (success) loadJobs();
        }}
        
        function viewJobScans(jobId, event) {{
            if (event) event.stopPropagation();
            // Find the server_id for this job
            const job = jobs.find(j => j.id === jobId);
            const serverId = job ? job.server_id : jobId;
            // Switch to scans page with this job pre-selected
            const filter = document.getElementById('scans-job-filter');
            if (filter) filter.value = serverId;
            showPage('scans');
        }}
        
        function openJobModal(job = null) {{
            document.getElementById('job-modal-title').textContent = job ? 'Edit Job' : 'New Job';
            document.getElementById('job-id').value = job ? job.id : '';
            document.getElementById('job-name').value = job ? job.name : '';
            document.getElementById('job-reference').value = job ? (job.reference_number || '') : '';
            document.getElementById('job-description').value = job ? (job.description || '') : '';
            document.getElementById('job-customer').value = job ? (job.customer_name || '') : '';
            document.getElementById('job-status').value = job ? job.status : 'PENDING';
            document.getElementById('job-priority').value = job ? (job.priority || 'NORMAL') : 'NORMAL';
            document.getElementById('job-expected').value = job ? (job.expected_count || 0) : 0;
            document.getElementById('job-due-date').value = job && job.due_date ? new Date(job.due_date).toISOString().split('T')[0] : '';
            document.getElementById('job-notes').value = job ? (job.notes || '') : '';
            document.getElementById('job-modal').classList.add('active');
        }}
        
        function closeJobModal() {{
            document.getElementById('job-modal').classList.remove('active');
        }}
        
        function editJob(id) {{
            const job = jobs.find(j => j.id === id);
            if (job) openJobModal(job);
        }}
        
        async function saveJob() {{
            const id = document.getElementById('job-id').value;
            const name = document.getElementById('job-name').value.trim();
            
            if (!name) {{
                showToast('error', 'Job name is required');
                return;
            }}
            
            const dueDate = document.getElementById('job-due-date').value;
            
            const jobData = {{
                job_id: id || `job_${{Date.now()}}`,
                name: name,
                reference_number: document.getElementById('job-reference').value.trim() || null,
                description: document.getElementById('job-description').value.trim() || null,
                customer_name: document.getElementById('job-customer').value.trim() || null,
                status: document.getElementById('job-status').value,
                priority: document.getElementById('job-priority').value,
                expected_count: parseInt(document.getElementById('job-expected').value) || 0,
                due_date: dueDate ? new Date(dueDate).toISOString() : null,
                notes: document.getElementById('job-notes').value.trim() || null,
            }};
            
            const changeType = id ? 'update' : 'create';
            const success = await submitChange(changeType, 'job', id || null, jobData);
            
            if (success || !isTrusted) {{
                closeJobModal();
                loadJobs();
            }}
        }}
        
        async function deleteJob(id, event) {{
            if (event) event.stopPropagation();
            if (!confirm('Are you sure you want to delete this job?')) return;
            
            const success = await submitChange('delete', 'job', String(id), {{}});
            if (success || !isTrusted) {{
                loadJobs();
            }}
        }}
        
        // Devices
        async function loadDevices() {{
            try {{
                const response = await fetch('/api/devices');
                const data = await response.json();
                devices = data.devices || [];
                
                const tbody = document.getElementById('devices-table');
                if (devices.length === 0) {{
                    tbody.innerHTML = '<tr><td colspan="6" class="empty-state"><div class="icon">�</div><p>No devices connected</p></td></tr>';
                    return;
                }}
                
                tbody.innerHTML = devices.map(device => `
                    <tr>
                        <td><strong>${{escapeHtml(device.device_name || 'Unnamed')}}</strong></td>
                        <td style="font-family: monospace; font-size: 0.8rem;">${{device.device_id.substring(0, 12)}}...</td>
                        <td>${{device.model || '-'}}</td>
                        <td>${{device.app_version || '-'}}</td>
                        <td>${{formatDate(device.last_seen_at)}}</td>
                        <td>
                            <button class="btn btn-danger btn-sm" onclick="deleteDevice('${{device.device_id}}')">Remove</button>
                        </td>
                    </tr>
                `).join('');
                
                document.getElementById('stat-devices').textContent = devices.length;
            }} catch (e) {{
                console.error('Failed to load devices:', e);
            }}
        }}
        
        async function deleteDevice(id) {{
            if (!confirm('Are you sure you want to remove this device?')) return;
            
            const success = await submitChange('delete', 'device', id, {{}});
            if (success || !isTrusted) {{
                loadDevices();
            }}
        }}
        
        // ========== Add Scans Modal ==========
        let webGeoPosition = null;
        let scanModalRows = [];

        function requestGeoLocation() {{
            const el = document.getElementById('scan-modal-location');
            if (!navigator.geolocation) {{
                el.innerHTML = '❌ Geolocation not supported';
                return;
            }}
            el.innerHTML = '⏳ Requesting…';
            navigator.geolocation.getCurrentPosition(
                (pos) => {{
                    webGeoPosition = {{
                        latitude: pos.coords.latitude,
                        longitude: pos.coords.longitude,
                        accuracy: pos.coords.accuracy
                    }};
                    el.innerHTML = `✅ ${{pos.coords.latitude.toFixed(5)}}, ${{pos.coords.longitude.toFixed(5)}} <span style="font-size:0.7rem;">(±${{Math.round(pos.coords.accuracy)}}m)</span>`;
                }},
                (err) => {{
                    webGeoPosition = null;
                    el.innerHTML = `⚠️ ${{err.message}}`;
                }},
                {{ enableHighAccuracy: true, timeout: 15000, maximumAge: 60000 }}
            );
        }}

        function openAddScansModal() {{
            scanModalRows = [];
            // Populate job dropdown
            const sel = document.getElementById('scan-modal-job');
            sel.innerHTML = '<option value="">No Job</option>';
            jobs.forEach(j => {{
                const opt = document.createElement('option');
                opt.value = j.id;
                opt.textContent = `${{j.name}}${{j.reference_number ? ' (' + j.reference_number + ')' : ''}}`;
                sel.appendChild(opt);
            }});
            document.getElementById('scan-modal-input').value = '';
            renderScanModalList();
            document.getElementById('scan-modal').classList.add('active');
            requestGeoLocation();
            setTimeout(() => document.getElementById('scan-modal-input').focus(), 100);
        }}

        function closeAddScansModal() {{
            document.getElementById('scan-modal').classList.remove('active');
            scanModalRows = [];
        }}

        function addScanRow() {{
            const inp = document.getElementById('scan-modal-input');
            const val = inp.value.trim();
            if (!val) return;
            // Allow multiple comma/space/newline separated
            const tickets = val.split(/[,\n]+/).map(t => t.trim()).filter(Boolean);
            tickets.forEach(ticket => {{
                scanModalRows.push({{ ticket, id: crypto.randomUUID() }});
            }});
            inp.value = '';
            inp.focus();
            renderScanModalList();
        }}

        function removeScanRow(rowId) {{
            scanModalRows = scanModalRows.filter(r => r.id !== rowId);
            renderScanModalList();
        }}

        function renderScanModalList() {{
            const container = document.getElementById('scan-modal-list');
            const countEl = document.getElementById('scan-modal-count');
            const submitBtn = document.getElementById('scan-modal-submit');
            countEl.textContent = scanModalRows.length;
            submitBtn.disabled = scanModalRows.length === 0;
            if (scanModalRows.length === 0) {{
                container.innerHTML = '<div style="text-align:center; padding:1.5rem; color:var(--text-muted); font-size:0.85rem;">No tickets added yet</div>';
                return;
            }}
            container.innerHTML = scanModalRows.map((row, i) => `
                <div style="display:flex; align-items:center; padding:0.4rem 0.75rem; border-bottom:1px solid var(--border); gap:0.5rem;">
                    <span style="width:1.5rem; font-size:0.75rem; color:var(--text-muted);">${{i+1}}</span>
                    <span style="flex:1; font-family:monospace; font-size:0.85rem; color:var(--accent);">${{escapeHtml(row.ticket)}}</span>
                    <button onclick="removeScanRow('${{row.id}}')" style="background:none; border:none; cursor:pointer; color:var(--danger); font-size:1rem; padding:0 0.25rem;" title="Remove">&times;</button>
                </div>
            `).join('');
        }}

        async function submitAddedScans() {{
            if (scanModalRows.length === 0) return;
            const submitBtn = document.getElementById('scan-modal-submit');
            submitBtn.disabled = true;
            submitBtn.textContent = 'Submitting…';

            const jobId = document.getElementById('scan-modal-job').value || null;
            const now = Date.now();
            let successCount = 0;
            let failCount = 0;

            for (let i = 0; i < scanModalRows.length; i++) {{
                const row = scanModalRows[i];
                const scanData = {{
                    id: crypto.randomUUID(),
                    barcode_data: row.ticket,
                    barcode_type: 'MANUAL',
                    ticket_number: row.ticket,
                    barcode_job_ref: null,
                    job_id: jobId,
                    device_id: 'web-' + (clientId || 'unknown'),
                    user_id: localStorage.getItem('cabnet_client_name') || null,
                    location: webGeoPosition ? `${{webGeoPosition.latitude.toFixed(6)}},${{webGeoPosition.longitude.toFixed(6)}}` : null,
                    latitude: webGeoPosition ? webGeoPosition.latitude : null,
                    longitude: webGeoPosition ? webGeoPosition.longitude : null,
                    scanned_at: now + i,
                    is_printed: false,
                    local_id: null
                }};

                const ok = await submitChange('create', 'scan', null, scanData);
                if (ok) successCount++; else failCount++;
            }}

            submitBtn.textContent = 'Submit Scans';
            submitBtn.disabled = false;

            if (successCount > 0) {{
                showToast('success', `${{successCount}} scan(s) submitted${{failCount > 0 ? `, ${{failCount}} failed` : ''}}`);
                closeAddScansModal();
                loadScans();
            }} else {{
                showToast('error', 'All scans failed to submit');
            }}
        }}

        // Scans
        let allScansData = [];
        let groupedScansData = [];
        
        async function loadScans() {{
            try {{
                const jobId = document.getElementById('scans-job-filter').value;
                const url = jobId ? `/api/scans?job_id=${{jobId}}&limit=500` : '/api/scans?limit=500';
                
                const response = await fetch(url);
                const data = await response.json();
                scans = data.scans || [];
                allScansData = scans;
                
                document.getElementById('stat-scans').textContent = data.total || scans.length;
                
                regroupScansPage();
            }} catch (e) {{
                console.error('Failed to load scans:', e);
                document.getElementById('scans-groups-container').innerHTML = 
                    '<div style="text-align: center; padding: 2rem; color: var(--text-muted);">Error loading scans</div>';
            }}
        }}
        
        function regroupScansPage() {{
            const gapHours = parseInt(document.getElementById('scans-time-gap').value);
            const gapMs = gapHours * 60 * 60 * 1000;
            
            // Sort scans by time (newest first)
            const sorted = [...allScansData].sort((a, b) => 
                new Date(b.scanned_at) - new Date(a.scanned_at)
            );
            
            groupedScansData = [];
            let currentGroup = null;
            
            for (const scan of sorted) {{
                const scanTime = new Date(scan.scanned_at).getTime();
                
                if (!currentGroup || (currentGroup.startTime - scanTime) > gapMs) {{
                    currentGroup = {{
                        id: groupedScansData.length,
                        startTime: scanTime,
                        endTime: scanTime,
                        scans: [scan]
                    }};
                    groupedScansData.push(currentGroup);
                }} else {{
                    currentGroup.endTime = scanTime;
                    currentGroup.scans.push(scan);
                }}
            }}
            
            renderScansGroups();
        }}
        
        function renderScansGroups() {{
            const container = document.getElementById('scans-groups-container');
            
            if (groupedScansData.length === 0) {{
                container.innerHTML = '<div style="text-align: center; padding: 2rem; color: var(--text-muted);"><div style="font-size: 2rem; margin-bottom: 0.5rem;">🏷️</div>No scans found</div>';
                document.getElementById('scans-total-count').textContent = '0';
                document.getElementById('scans-group-count').textContent = '0';
                return;
            }}
            
            document.getElementById('scans-total-count').textContent = allScansData.length;
            document.getElementById('scans-group-count').textContent = groupedScansData.length;
            
            let html = '';
            for (const group of groupedScansData) {{
                const startDate = new Date(group.startTime);
                const endDate = new Date(group.endTime);
                
                const timeRange = group.scans.length === 1 
                    ? startDate.toLocaleDateString('en-US') + ' ' + startDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})
                    : `${{endDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})}} - ${{startDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})}} on ${{startDate.toLocaleDateString('en-US')}}`;
                
                // Determine group job name: try barcode_job_ref, then job_id, then scan-level lookup
                let groupJobName = '';
                // Try barcode_job_ref → reference_number matching first
                const firstRef = group.scans.map(s => s.barcode_job_ref).find(r => r);
                if (firstRef) {{
                    const matchedJob = jobs.find(j => j.reference_number === firstRef);
                    if (matchedJob) groupJobName = matchedJob.name;
                }}
                // Fall back to job_id lookup
                if (!groupJobName) {{
                    const jobIds = [...new Set(group.scans.map(s => s.job_id).filter(Boolean))];
                    if (jobIds.length === 1) {{
                        const name = getJobName(jobIds[0]);
                        if (name !== '-') groupJobName = name;
                    }} else if (jobIds.length > 1) {{
                        groupJobName = 'Mixed';
                    }}
                }}
                // Last resort: try getJobNameForScan on first scan
                if (!groupJobName) {{
                    const firstScanName = getJobNameForScan(group.scans[0]);
                    if (firstScanName) groupJobName = firstScanName;
                }}
                
                html += `
                    <div class="scan-group" data-group-id="${{group.id}}" id="scan-card-${{group.id}}">
                        <div class="group-header" onclick="toggleScansGroupExpand(${{group.id}})">
                            <div class="group-title">
                                <span class="group-arrow">▶</span>
                                ${{groupJobName ? `<span class="group-job">${{groupJobName}}</span><span style="color: var(--text-muted);">—</span>` : ''}}
                                <span class="group-time">${{timeRange}}</span>
                            </div>
                            <div class="group-meta">
                                <span class="group-count">${{group.scans.length}} scans</span>
                            </div>
                        </div>
                        <div class="group-scans">
                            <div class="group-kebab-row" style="display: none; padding: 0.5rem 0; border-bottom: 1px solid var(--border); margin-bottom: 0.5rem;">
                                <div style="display: flex; gap: 0.75rem; align-items: center; flex-wrap: wrap;">
                                    <label style="font-size: 0.75rem; color: var(--text-muted);">Job:
                                        <select class="kebab-job-select" style="margin-left: 0.25rem; padding: 0.2rem 0.4rem; font-size: 0.75rem; background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 4px;">
                                            <option value="">None</option>
                                            ${{jobs.map(j => `<option value="${{j.id}}">${{escapeHtml(j.name)}}</option>`).join('')}}
                                        </select>
                                    </label>
                                    <label style="font-size: 0.75rem; color: var(--text-muted);">Device:
                                        <select class="kebab-device-select" style="margin-left: 0.25rem; padding: 0.2rem 0.4rem; font-size: 0.75rem; background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 4px;">
                                            ${{devices.map(d => `<option value="${{d.device_id}}">${{escapeHtml(d.device_name || d.device_id.substring(0,12))}}</option>`).join('')}}
                                        </select>
                                    </label>
                                    <button class="btn btn-accent" style="padding: 0.2rem 0.75rem; font-size: 0.75rem;" onclick="event.stopPropagation(); applyKebabScans(${{group.id}})">Apply</button>
                                </div>
                            </div>
                            <div style="display: flex; justify-content: flex-end; padding: 0.25rem 0;">
                                <button class="btn btn-outline" style="padding: 0.1rem 0.4rem; font-size: 0.8rem; border: none;" onclick="event.stopPropagation(); toggleKebabMenu(${{group.id}})">⋮</button>
                            </div>
                            <input type="text" class="card-search" placeholder="🔍 Search scans..." 
                                   oninput="filterCardScans(${{group.id}}, this.value)" onclick="event.stopPropagation()"
                                   style="width: 100%; padding: 0.3rem 0.5rem; font-size: 0.75rem; margin-bottom: 0.5rem; background: var(--bg); color: var(--text); border: 1px solid var(--border); border-radius: 4px; box-sizing: border-box;">
                            <div class="scan-row-header">
                                <span>Ticket #</span>
                                <span>Device</span>
                                <span>Time</span>
                            </div>
                            <div class="card-scan-rows">
                                ${{group.scans.map(scan => {{
                                    const ticket = scan.ticket_number || stripJobPrefix(scan.barcode);
                                    return `
                                    <div class="scan-row-wrapper" data-scan-id="${{scan.id}}" data-ticket="${{escapeHtml(ticket.toLowerCase())}}" data-barcode="${{escapeHtml(scan.barcode.toLowerCase())}}">
                                        <div class="scan-row-delete" onclick="deleteScan(${{scan.id}}, this.closest('.scan-row-wrapper'))">🗑️ Delete</div>
                                        <div class="scan-row-content" ontouchstart="swipeStart(event, this)" ontouchmove="swipeMove(event, this)" ontouchend="swipeEnd(event, this)" onmousedown="swipeStart(event, this)">
                                            <span style="font-family: monospace; color: var(--accent);">${{ticket}}</span>
                                            <span>${{scan.device_id ? scan.device_id.substring(0, 8) + '...' : '-'}}</span>
                                            <span>${{new Date(scan.scanned_at).toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})}}</span>
                                        </div>
                                    </div>`;
                                }}).join('')}}
                            </div>
                        </div>
                    </div>
                `;
            }}
            
            container.innerHTML = html;
        }}
        
        function toggleScansGroupExpand(groupId) {{
            const card = document.getElementById(`scan-card-${{groupId}}`);
            card.classList.toggle('expanded');
        }}

        function filterCardScans(groupId, query) {{
            const card = document.getElementById(`scan-card-${{groupId}}`);
            const rows = card.querySelectorAll('.card-scan-rows .scan-row-wrapper');
            const q = query.toLowerCase();
            rows.forEach(row => {{
                const ticket = row.dataset.ticket || '';
                const barcode = row.dataset.barcode || '';
                row.style.display = (!q || ticket.includes(q) || barcode.includes(q)) ? '' : 'none';
            }});
        }}

        function toggleKebabMenu(groupId) {{
            const card = document.getElementById(`scan-card-${{groupId}}`);
            const row = card.querySelector('.group-kebab-row');
            if (row.style.display === 'none') {{
                // Pre-select current values
                const group = groupedScansData.find(g => g.id === groupId);
                if (group) {{
                    const firstJob = group.scans.find(s => s.job_id)?.job_id || '';
                    const firstDevice = group.scans[0]?.device_id || '';
                    row.querySelector('.kebab-job-select').value = firstJob;
                    row.querySelector('.kebab-device-select').value = firstDevice;
                }}
                row.style.display = 'block';
            }} else {{
                row.style.display = 'none';
            }}
        }}

        async function applyKebabScans(groupId) {{
            const card = document.getElementById(`scan-card-${{groupId}}`);
            const group = groupedScansData.find(g => g.id === groupId);
            if (!group) return;

            const jobSelect = card.querySelector('.kebab-job-select');
            const deviceSelect = card.querySelector('.kebab-device-select');
            const jobId = jobSelect.value ? parseInt(jobSelect.value) : null;
            const deviceId = deviceSelect.value || null;

            const scanIds = group.scans.map(s => s.id);

            const assignData = {{ scan_ids: scanIds }};
            if (jobId !== null) assignData.job_id = jobId;
            if (deviceId) assignData.device_id = deviceId;

            const success = await submitChange('update', 'scan_assignments', null, assignData);
            if (success) loadScans();
        }}
        
        // ============ Swipe-to-delete for scan rows ============
        let swipeState = {{ startX: 0, currentX: 0, swiping: false, el: null }};
        
        function swipeStart(e, el) {{
            // Close any other open swipes first
            document.querySelectorAll('.scan-row-content.swiped').forEach(other => {{
                if (other !== el) other.classList.remove('swiped');
            }});
            
            const clientX = e.touches ? e.touches[0].clientX : e.clientX;
            swipeState = {{ startX: clientX, currentX: clientX, swiping: true, el: el }};
            el.classList.add('swiping');
            
            if (!e.touches) {{
                // Mouse events need move/up on document
                document.addEventListener('mousemove', swipeMouseMove);
                document.addEventListener('mouseup', swipeMouseUp);
            }}
        }}
        
        function swipeMove(e, el) {{
            if (!swipeState.swiping || swipeState.el !== el) return;
            const clientX = e.touches ? e.touches[0].clientX : e.clientX;
            swipeState.currentX = clientX;
            
            let dx = swipeState.currentX - swipeState.startX;
            // Only allow swiping left (negative dx), or right to close
            if (el.classList.contains('swiped')) {{
                dx = Math.max(-80, Math.min(0, -80 + (dx)));
            }} else {{
                dx = Math.max(-80, Math.min(0, dx));
            }}
            el.style.transform = `translateX(${{dx}}px)`;
        }}
        
        function swipeEnd(e, el) {{
            if (!swipeState.swiping || swipeState.el !== el) return;
            el.classList.remove('swiping');
            el.style.transform = '';
            
            const dx = swipeState.currentX - swipeState.startX;
            
            if (el.classList.contains('swiped')) {{
                // Currently open — close if swiped right enough
                if (dx > 30) {{
                    el.classList.remove('swiped');
                }}
                // else stay open
            }} else {{
                // Currently closed — open if swiped left enough
                if (dx < -40) {{
                    el.classList.add('swiped');
                }}
            }}
            
            swipeState.swiping = false;
        }}
        
        function swipeMouseMove(e) {{
            if (!swipeState.swiping || !swipeState.el) return;
            swipeMove(e, swipeState.el);
        }}
        
        function swipeMouseUp(e) {{
            if (!swipeState.swiping || !swipeState.el) return;
            swipeEnd(e, swipeState.el);
            document.removeEventListener('mousemove', swipeMouseMove);
            document.removeEventListener('mouseup', swipeMouseUp);
        }}
        
        // Close swipes when tapping elsewhere
        document.addEventListener('click', (e) => {{
            if (!e.target.closest('.scan-row-wrapper')) {{
                document.querySelectorAll('.scan-row-content.swiped').forEach(el => {{
                    el.classList.remove('swiped');
                }});
            }}
        }});
        
        async function deleteScan(scanId, wrapper) {{
            const success = await submitChange('delete', 'scan', String(scanId), {{}});
            if (success) {{
                // Animate removal
                wrapper.classList.add('scan-row-removing');
                requestAnimationFrame(() => {{
                    wrapper.classList.add('removed');
                }});
                setTimeout(() => {{
                    wrapper.remove();
                    // Update count in group header
                    const group = wrapper.closest('.scan-group');
                    if (group) {{
                        const remaining = group.querySelectorAll('.scan-row-wrapper').length;
                        const countBadge = group.querySelector('.group-count');
                        if (countBadge) countBadge.textContent = `${{remaining}} scans`;
                        if (remaining === 0) {{
                            group.remove();
                        }}
                    }}
                    // Update global count
                    const totalEl = document.getElementById('scans-total-count');
                    if (totalEl) totalEl.textContent = parseInt(totalEl.textContent) - 1;
                    // Also remove from in-memory array
                    allScansData = allScansData.filter(s => s.id !== scanId);
                    const statEl = document.getElementById('stat-scans');
                    if (statEl) statEl.textContent = allScansData.length;
                }}, 300);
            }}
        }}
        
        function getJobName(jobId) {{
            if (!jobId) return '-';
            const job = jobs.find(j => j.id === jobId || j.id === String(jobId) || j.server_id === jobId || j.server_id === Number(jobId));
            return job ? job.name : '-';
        }}

        function getJobNameForScan(scan) {{
            // First try direct job_id lookup
            if (scan.job_id) {{
                const name = getJobName(scan.job_id);
                if (name !== '-') return name;
            }}
            // Fall back to barcode_job_ref → job reference_number matching
            if (scan.barcode_job_ref) {{
                const matched = jobs.find(j => j.reference_number === scan.barcode_job_ref);
                if (matched) return matched.name;
            }}
            return '';
        }}
        
        // Strip job ID prefix from barcode (removes leading digits and zeros)
        function stripJobPrefix(barcode) {{
            const match = barcode.match(/^\\d+0+(\\d+)$/);
            return match ? match[1] : barcode;
        }}
        
        // Utilities
        function escapeHtml(text) {{
            const div = document.createElement('div');
            div.textContent = text || '';
            return div.innerHTML;
        }}
        
        function formatDate(dateStr) {{
            if (!dateStr) return '-';
            try {{
                const date = new Date(dateStr);
                const now = new Date();
                const diff = (now - date) / 1000;
                
                if (diff < 60) return 'Just now';
                if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
                if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
                if (diff < 604800) return Math.floor(diff / 86400) + 'd ago';
                
                return date.toLocaleDateString('en-US');
            }} catch {{
                return dateStr;
            }}
        }}
        
        function showToast(type, message) {{
            const container = document.getElementById('toast-container');
            const toast = document.createElement('div');
            toast.className = `toast ${{type}}`;
            toast.innerHTML = `<span>${{type === 'success' ? '✓' : type === 'error' ? '✗' : '⚠'}}</span><span>${{message}}</span>`;
            container.appendChild(toast);
            
            setTimeout(() => {{
                toast.style.opacity = '0';
                setTimeout(() => toast.remove(), 300);
            }}, 4000);
        }}
        
        function refreshAll() {{
            loadJobs();
            loadDevices();
            loadScans();
            loadDashboardHome();
            checkClientStatus();
            showToast('success', 'Data refreshed');
        }}
        
        // Close modal on outside click
        document.getElementById('job-modal').addEventListener('click', (e) => {{
            if (e.target === e.currentTarget) closeJobModal();
        }});
        
        // Map state
        let dashboardMap = null;
        let mapCurrentLayer = null;
        let mapMarkers = [];
        
        const mapTileLayers = {{
            osm: () => L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
                attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
            }}),
            esri: () => L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{{z}}/{{y}}/{{x}}', {{
                attribution: 'Tiles &copy; Esri'
            }}),
            'esri-street': () => L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Street_Map/MapServer/tile/{{z}}/{{y}}/{{x}}', {{
                attribution: 'Tiles &copy; Esri'
            }}),
            'esri-topo': () => L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Topo_Map/MapServer/tile/{{z}}/{{y}}/{{x}}', {{
                attribution: 'Tiles &copy; Esri'
            }})
        }};
        
        function initMap() {{
            if (dashboardMap) return;
            
            const mapContainer = document.getElementById('dashboard-map');
            if (!mapContainer) return;
            
            // Default to US center
            dashboardMap = L.map('dashboard-map').setView([39.8283, -98.5795], 4);
            
            mapCurrentLayer = mapTileLayers.osm();
            mapCurrentLayer.addTo(dashboardMap);
            
            // Load scan locations
            loadMapData();
        }}
        
        async function loadMapData() {{
            try {{
                // Fetch all scans and filter for those with GPS
                const response = await fetch('/api/scans?limit=1000');
                const result = await response.json();
                const allScans = result.data?.scans || result.scans || [];
                
                // Filter scans that have valid GPS coordinates
                const locations = allScans.filter(scan => {{
                    const hasLatLon = scan.latitude && scan.longitude && 
                                      (scan.latitude !== 0 || scan.longitude !== 0);
                    const hasLocation = scan.location && scan.location.includes(',');
                    return hasLatLon || hasLocation;
                }}).map(scan => {{
                    // Parse coordinates from location field if lat/lon not set
                    let lat = scan.latitude;
                    let lon = scan.longitude;
                    if ((!lat || !lon || (lat === 0 && lon === 0)) && scan.location) {{
                        const parts = scan.location.split(',');
                        if (parts.length === 2) {{
                            lat = parseFloat(parts[0].trim());
                            lon = parseFloat(parts[1].trim());
                        }}
                    }}
                    return {{ ...scan, latitude: lat, longitude: lon }};
                }}).filter(scan => scan.latitude && scan.longitude);
                
                // Clear existing markers
                mapMarkers.forEach(m => dashboardMap.removeLayer(m));
                mapMarkers = [];
                
                if (locations.length > 0) {{
                    locations.forEach(scan => {{
                        const scanDate = new Date(scan.scanned_at);
                        const formattedTime = scanDate.toLocaleDateString('en-US') + ' ' + scanDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}});
                        const marker = L.marker([scan.latitude, scan.longitude])
                            .bindPopup(`
                                <div style="min-width: 180px;">
                                    <strong>🏷️ ${{scan.barcode}}</strong><br>
                                    <small>Device: ${{scan.device_id.substring(0, 8)}}...</small><br>
                                    <small>${{formattedTime}}</small>
                                </div>
                            `);
                        marker.addTo(dashboardMap);
                        mapMarkers.push(marker);
                    }});
                    
                    // Fit to bounds
                    if (mapMarkers.length > 1) {{
                        const group = L.featureGroup(mapMarkers);
                        dashboardMap.fitBounds(group.getBounds().pad(0.1));
                    }} else {{
                        dashboardMap.setView([locations[0].latitude, locations[0].longitude], 12);
                    }}
                }}
                
                document.getElementById('map-scan-count').textContent = 
                    locations.length + ' location' + (locations.length !== 1 ? 's' : '');
            }} catch (e) {{
                console.error('Failed to load map data:', e);
            }}
        }}
        
        function changeMapProvider() {{
            if (!dashboardMap) return;
            
            const provider = document.getElementById('map-provider').value;
            if (mapCurrentLayer) {{
                dashboardMap.removeLayer(mapCurrentLayer);
            }}
            mapCurrentLayer = mapTileLayers[provider]();
            mapCurrentLayer.addTo(dashboardMap);
        }}
        
        // Initialize map when showing map page
        const originalShowPage = showPage;
        showPage = function(page) {{
            originalShowPage(page);
            if (page === 'map') {{
                setTimeout(() => {{
                    initMap();
                    if (dashboardMap) {{
                        dashboardMap.invalidateSize();
                    }}
                }}, 100);
            }}
        }};
        
        // Live scan functionality
        let liveMode = false;
        let liveInterval = null;
        let lastScanId = null;
        let liveScans = [];
        
        // Toggle live mode
        function toggleLiveMode() {{
            liveMode = !liveMode;
            const button = document.getElementById('live-toggle');
            const status = document.getElementById('live-status');
            
            if (liveMode) {{
                button.innerHTML = '⏹️ Stop Live';
                button.className = 'btn btn-danger';
                status.innerHTML = '🟢 Live';
                status.style.background = 'var(--success)';
                startLiveMode();
            }} else {{
                button.innerHTML = '▶️ Start Live';
                button.className = 'btn btn-success';
                status.innerHTML = '🔴 Stopped';
                status.style.background = 'var(--danger)';
                stopLiveMode();
            }}
        }}
        
        // Start live mode
        function startLiveMode() {{
            loadLiveScans(); // Initial load
            liveInterval = setInterval(loadLiveScans, 5000); // Check every 5 seconds
        }}
        
        // Stop live mode
        function stopLiveMode() {{
            if (liveInterval) {{
                clearInterval(liveInterval);
                liveInterval = null;
            }}
        }}
        
        // Load live scans
        async function loadLiveScans() {{
            try {{
                const response = await fetch('/api/scans?limit=50&sort=desc');
                const data = await response.json();
                
                if (data.success && data.scans) {{
                    const newScans = data.scans.filter(scan => !lastScanId || scan.id > lastScanId);
                    
                    if (newScans.length > 0) {{
                        // Update last scan ID
                        lastScanId = Math.max(...data.scans.map(s => s.id));
                        
                        // Add new scans to the top
                        newScans.reverse().forEach(scan => {{
                            addLiveScan(scan, true);
                        }});
                        
                        // Update last update time
                        document.getElementById('live-last-update').textContent = new Date().toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}});
                        
                        // Play notification sound (optional)
                        playNotificationSound();
                    }}
                }}
            }} catch (e) {{
                console.error('Failed to load live scans:', e);
            }}
        }}
        
        // Add a scan to the live feed
        function addLiveScan(scan, isNew = false) {{
            const container = document.getElementById('live-scans-container');
            
            // Remove placeholder if it exists
            const placeholder = container.querySelector('.live-scan-item[style*="text-align: center"]');
            if (placeholder) {{
                placeholder.remove();
            }}
            
            const scanElement = document.createElement('div');
            scanElement.className = `live-scan-item ${{isNew ? 'new' : ''}}`;
            
            const scanDate = new Date(scan.scanned_at);
            const time = scanDate.toLocaleDateString('en-US') + ' ' + scanDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}});
            const barcodeShort = scan.barcode.length > 30 ? scan.barcode.substring(0, 27) + '...' : scan.barcode;
            
            scanElement.innerHTML = `
                <div class="live-scan-header">
                    <div class="live-scan-time">${{time}}</div>
                    <div style="color: var(--text-muted); font-size: 0.8rem;">Device: ${{scan.device_id}}</div>
                </div>
                <div class="live-scan-barcode">${{barcodeShort}}</div>
                <div class="live-scan-details">
                    <div><strong>Type:</strong> ${{scan.barcode_type || 'Unknown'}}</div>
                    <div><strong>Job:</strong> ${{scan.job_id ? 'Yes' : 'No'}}</div>
                    <div><strong>Ticket:</strong> ${{scan.ticket_number || 'N/A'}}</div>
                    <div><strong>Location:</strong> ${{scan.location || 'N/A'}}</div>
                </div>
            `;
            
            // Insert at the top
            container.insertBefore(scanElement, container.firstChild);
            
            // Remove animation class after animation completes
            if (isNew) {{
                setTimeout(() => {{
                    scanElement.classList.remove('new');
                }}, 3000);
            }}
            
            // Limit to 100 scans to prevent memory issues
            const items = container.querySelectorAll('.live-scan-item');
            if (items.length > 100) {{
                for (let i = 100; i < items.length; i++) {{
                    items[i].remove();
                }}
            }}
        }}
        
        // Clear live scans
        function clearLiveScans() {{
            const container = document.getElementById('live-scans-container');
            container.innerHTML = `
                <div class="live-scan-item" style="text-align: center; color: var(--text-muted); padding: 2rem;">
                    <div style="font-size: 2rem; margin-bottom: 1rem;">⚡</div>
                    <div>Scan feed cleared. Click "Start Live" to begin monitoring.</div>
                </div>
            `;
            lastScanId = null;
        }}
        
        // Play notification sound (subtle)
        function playNotificationSound() {{
            // Create a subtle beep sound using Web Audio API
            try {{
                const audioContext = new (window.AudioContext || window.webkitAudioContext)();
                const oscillator = audioContext.createOscillator();
                const gainNode = audioContext.createGain();
                
                oscillator.connect(gainNode);
                gainNode.connect(audioContext.destination);
                
                oscillator.frequency.setValueAtTime(800, audioContext.currentTime);
                oscillator.frequency.setValueAtTime(600, audioContext.currentTime + 0.1);
                
                gainNode.gain.setValueAtTime(0.1, audioContext.currentTime);
                gainNode.gain.exponentialRampToValueAtTime(0.01, audioContext.currentTime + 0.2);
                
                oscillator.start(audioContext.currentTime);
                oscillator.stop(audioContext.currentTime + 0.2);
            }} catch (e) {{
                // Silently fail if Web Audio API is not available
            }}
        }}
        
        // Initialize
        init();
        
        // Auto-refresh every 60 seconds
        setInterval(() => {{
            loadJobs();
            loadDevices();
            loadTimeEntries();
            const activePage = document.querySelector('.page.active')?.id;
            if (activePage === 'page-dashboard') loadDashboardHome();
            if (activePage === 'page-team') loadTeamStatus();
            if (activePage === 'page-timesheets') loadTimesheets();
        }}, 60000);

        // ==================== TIME CLOCK ====================

        async function loadTimeEntries() {{
            try {{
                const response = await fetch('/api/time-entries?limit=500');
                const data = await response.json();
                if (!data.success) return;
                
                const entries = data.entries || [];
                const filter = document.getElementById('tc-filter').value;
                
                // Populate device filter dropdown
                const devices = [...new Set(entries.map(e => e.device_id))];
                const filterEl = document.getElementById('tc-filter');
                const currentVal = filterEl.value;
                filterEl.innerHTML = '<option value="all">All Workers</option>' + 
                    devices.map(d => `<option value="${{d}}" ${{d === currentVal ? 'selected' : ''}}>${{d}}</option>`).join('');
                
                const filtered = filter === 'all' ? entries : entries.filter(e => e.device_id === filter);
                
                // Active workers (clocked in, not on break)
                const activeWorkers = filtered.filter(e => !e.clock_out && e.is_break === 0);
                const activeBreaks = filtered.filter(e => !e.clock_out && e.is_break === 1);
                
                const activeHtml = activeWorkers.length === 0 && activeBreaks.length === 0
                    ? '<div style="color: var(--text-muted); font-style: italic; padding: 1rem;">No one is currently clocked in</div>'
                    : [...activeWorkers.map(e => {{
                        const elapsed = formatElapsed(e.clock_in);
                        return `<div style="background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 1rem; border-left: 4px solid var(--green);">
                            <div style="display: flex; justify-content: space-between; align-items: start;">
                                <div>
                                    <div style="font-weight: 600;">${{e.device_id}}</div>
                                    <div style="color: var(--text-muted); font-size: 0.85rem;">${{e.job_name || e.customer_name || 'No job'}}</div>
                                </div>
                                <div style="text-align: right;">
                                    <div style="color: var(--green); font-weight: 600;">🟢 Working</div>
                                    <div style="font-size: 0.85rem; color: var(--text-muted);">${{elapsed}}</div>
                                </div>
                            </div>
                        </div>`;
                    }}), ...activeBreaks.map(e => {{
                        const elapsed = formatElapsed(e.clock_in);
                        return `<div style="background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 1rem; border-left: 4px solid var(--orange);">
                            <div style="display: flex; justify-content: space-between; align-items: start;">
                                <div>
                                    <div style="font-weight: 600;">${{e.device_id}}</div>
                                    <div style="color: var(--text-muted); font-size: 0.85rem;">${{e.job_name || e.customer_name || 'No job'}}</div>
                                </div>
                                <div style="text-align: right;">
                                    <div style="color: var(--orange); font-weight: 600;">☕ Break</div>
                                    <div style="font-size: 0.85rem; color: var(--text-muted);">${{elapsed}}</div>
                                </div>
                            </div>
                        </div>`;
                    }})].join('');
                
                document.getElementById('active-workers-list').innerHTML = activeHtml;
                
                // Timesheet table (completed entries)
                const completed = filtered.filter(e => e.clock_out);
                const tbody = document.getElementById('timeclock-tbody');
                tbody.innerHTML = completed.length === 0
                    ? '<tr><td colspan="6" style="text-align: center; color: var(--text-muted); padding: 2rem;">No completed time entries</td></tr>'
                    : completed.map(e => {{
                        const dur = formatDuration(e.clock_in, e.clock_out);
                        const typeLabel = e.is_break ? `<span style="color: var(--orange);">☕ Break</span>` : `<span style="color: var(--green);">🟢 Work</span>`;
                        const paidLabel = e.is_paid ? '' : ' <span style="background: var(--border); padding: 0.1rem 0.4rem; border-radius: 4px; font-size: 0.7rem;">UNPAID</span>';
                        return `<tr>
                            <td>${{e.device_id}}</td>
                            <td>${{e.job_name || e.customer_name || '-'}}</td>
                            <td>${{formatDateTime(e.clock_in)}}</td>
                            <td>${{formatDateTime(e.clock_out)}}</td>
                            <td>${{dur}}</td>
                            <td>${{typeLabel}}${{paidLabel}}</td>
                        </tr>`;
                    }}).join('');
            }} catch (e) {{
                console.error('Failed to load time entries:', e);
            }}
        }}

        function formatElapsed(isoDate) {{
            const start = new Date(isoDate);
            const now = new Date();
            const diff = Math.floor((now - start) / 1000);
            const h = Math.floor(diff / 3600);
            const m = Math.floor((diff % 3600) / 60);
            return h > 0 ? `${{h}}h ${{m}}m` : `${{m}}m`;
        }}

        function formatDuration(startIso, endIso) {{
            const start = new Date(startIso);
            const end = new Date(endIso);
            const diff = Math.floor((end - start) / 1000);
            const h = Math.floor(diff / 3600);
            const m = Math.floor((diff % 3600) / 60);
            return h > 0 ? `${{h}}h ${{m}}m` : `${{m}}m`;
        }}

        function formatDateTime(isoDate) {{
            if (!isoDate) return '-';
            const d = new Date(isoDate);
            return d.toLocaleDateString() + ' ' + d.toLocaleTimeString([], {{hour: '2-digit', minute: '2-digit'}});
        }}

        // ==================== REPORTS ====================

        async function loadReports() {{
            try {{
                const sortBy = document.getElementById('report-sort').value;
                const jobFilter = document.getElementById('report-filter-job').value;
                
                let url = '/api/reports?limit=500';
                if (jobFilter !== 'all') url += `&job_id=${{jobFilter}}`;
                
                const response = await fetch(url);
                const data = await response.json();
                if (!data.success) return;
                
                let reports = data.reports || [];
                
                // Populate job filter dropdown  
                const jobIds = [...new Set(reports.map(r => r.job_id).filter(Boolean))];
                const jobFilterEl = document.getElementById('report-filter-job');
                const currentJob = jobFilterEl.value;
                // Only rebuild if needed
                if (jobFilterEl.options.length <= 1) {{
                    // Fetch job names for display
                    try {{
                        const jobsResp = await fetch('/api/jobs');
                        const jobsData = await jobsResp.json();
                        const jobs = jobsData.jobs || [];
                        jobFilterEl.innerHTML = '<option value="all">All Jobs</option>' +
                            jobs.map(j => `<option value="${{j.uuid}}" ${{j.uuid === currentJob ? 'selected' : ''}}>${{j.name}}</option>`).join('');
                    }} catch (e) {{}}
                }}
                
                // Sort
                if (sortBy === 'job') {{
                    reports.sort((a, b) => (a.job_id || '').localeCompare(b.job_id || '') || new Date(b.created_at) - new Date(a.created_at));
                }} else {{
                    reports.sort((a, b) => new Date(b.created_at) - new Date(a.created_at));
                }}
                
                const container = document.getElementById('reports-container');
                if (reports.length === 0) {{
                    container.innerHTML = '<div style="text-align: center; color: var(--text-muted); padding: 2rem; grid-column: 1 / -1;"><div style="font-size: 2rem; margin-bottom: 1rem;">\ud83d\udccb</div>No reports synced yet</div>';
                    return;
                }}
                
                container.innerHTML = reports.map(r => {{
                    const statusColor = r.is_complete ? 'var(--green)' : r.status === 'in_progress' ? 'var(--orange)' : 'var(--text-muted)';
                    const statusLabel = r.is_complete ? '\u2705 Complete' : r.status === 'in_progress' ? '\ud83d\udfe1 In Progress' : '\ud83d\udccb Draft';
                    const date = new Date(r.created_at).toLocaleDateString();
                    
                    // Checklist items
                    const checks = [
                        r.has_fillers ? '\u2705 Fillers' : '\u2b1c Fillers',
                        r.has_handles ? '\u2705 Handles' : '\u2b1c Handles',
                        r.has_fast_caps ? '\u2705 Fast Caps' : '\u2b1c Fast Caps',
                        r.has_set_boxes ? '\u2705 Set Boxes' : '\u2b1c Set Boxes',
                        r.has_caulking ? '\u2705 Caulking' : '\u2b1c Caulking',
                    ].join(' &middot; ');
                    
                    return `<div style="background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 1.25rem; cursor: pointer;" onclick="showReportDetail(this)" data-report-id="${{r.uuid}}">
                        <div style="display: flex; justify-content: space-between; align-items: start; margin-bottom: 0.75rem;">
                            <div>
                                <div style="font-weight: 600; font-size: 1.05rem;">${{r.title}}</div>
                                <div style="color: var(--text-muted); font-size: 0.85rem;">${{r.room_name || 'No room'}} &middot; ${{date}}</div>
                            </div>
                            <div style="color: ${{statusColor}}; font-size: 0.85rem; font-weight: 500;">${{statusLabel}}</div>
                        </div>
                        <div style="display: flex; gap: 1rem; margin-bottom: 0.5rem; font-size: 0.85rem; color: var(--text-muted);">
                            <span>\ud83d\udccc ${{r.author_name || 'Unknown'}}</span>
                            <span>\ud83d\uddc4\ufe0f ${{r.cabinet_count || 0}} cabinets</span>
                        </div>
                        <div style="font-size: 0.75rem; color: var(--text-muted);">${{checks}}</div>
                        ${{r.notes ? `<div style="margin-top: 0.5rem; padding: 0.5rem; background: var(--bg); border-radius: 8px; font-size: 0.85rem; color: var(--text-muted);">${{r.notes.substring(0, 200)}}${{r.notes.length > 200 ? '...' : ''}}</div>` : ''}}
                        ${{r.punch_list ? `<div style="margin-top: 0.5rem; padding: 0.5rem; background: rgba(251,146,60,0.1); border: 1px solid rgba(251,146,60,0.3); border-radius: 8px; font-size: 0.85rem;"><strong style="color: var(--orange);">Punch List:</strong> <span style="color: var(--text-muted);">${{r.punch_list.substring(0, 200)}}${{r.punch_list.length > 200 ? '...' : ''}}</span></div>` : ''}}
                    </div>`;
                }}).join('');
            }} catch (e) {{
                console.error('Failed to load reports:', e);
            }}
        }}

        function showReportDetail(el) {{
            // Future: expand or open a detail modal
            el.style.borderColor = 'var(--accent)';
            setTimeout(() => el.style.borderColor = 'var(--border)', 1500);
        }}

        // ==================== DASHBOARD HOME ====================

        async function loadDashboardHome() {{
            // 1. Team status (condensed)
            try {{
                const resp = await fetch('/api/team/status');
                const data = await resp.json();
                if (data.success) {{
                    const members = data.members || [];
                    const container = document.getElementById('dash-team-status');
                    if (!container) return;

                    if (members.length === 0) {{
                        container.innerHTML = '<p style="color: var(--text-muted); font-style: italic;">No team members yet. <a href="#" onclick="showPage(\'team\')" style="color: var(--accent);">Add team members →</a></p>';
                    }} else {{
                        const working = members.filter(m => m.status === 'working');
                        const onBreak = members.filter(m => m.status === 'break');
                        const offline = members.filter(m => m.status === 'offline');

                        // Summary bar
                        let html = `<div style="display: flex; gap: 1rem; margin-bottom: 0.75rem; font-size: 0.85rem; flex-wrap: wrap;">
                            <span style="color: var(--success); font-weight: 600;">🟢 ${{working.length}} Working</span>
                            <span style="color: var(--warning); font-weight: 600;">☕ ${{onBreak.length}} Break</span>
                            <span style="color: #6b7280; font-weight: 600;">⚫ ${{offline.length}} Offline</span>
                        </div>`;

                        // Show active members as compact chips (up to 10)
                        const active = [...working, ...onBreak].slice(0, 10);
                        if (active.length > 0) {{
                            html += '<div style="display: flex; flex-wrap: wrap; gap: 0.5rem;">';
                            active.forEach(m => {{
                                const color = m.status === 'working' ? 'var(--success)' : 'var(--warning)';
                                const icon = m.status === 'working' ? '🟢' : '☕';
                                const initials = m.display_name.split(' ').map(w => w[0]).join('').toUpperCase().slice(0,2);
                                const avatarColor = m.avatar_color || '#6366f1';
                                const jobText = m.current_job ? ` · ${{m.current_job}}` : '';
                                html += `<div style="display: flex; align-items: center; gap: 0.5rem; background: rgba(255,255,255,0.04); border: 1px solid var(--border); border-radius: 999px; padding: 0.3rem 0.75rem 0.3rem 0.3rem;">
                                    <div style="width: 28px; height: 28px; border-radius: 50%; background: ${{avatarColor}}; display: flex; align-items: center; justify-content: center; color: white; font-size: 0.7rem; font-weight: 600;">${{initials}}</div>
                                    <div>
                                        <div style="font-size: 0.8rem; font-weight: 500;">${{icon}} ${{m.display_name}}</div>
                                        <div style="font-size: 0.7rem; color: var(--text-muted);">${{m.total_scans_today || 0}} scans${{jobText}}</div>
                                    </div>
                                </div>`;
                            }});
                            const remaining = [...working, ...onBreak].length - 10;
                            if (remaining > 0) {{
                                html += `<div style="display:flex;align-items:center;padding:0 0.5rem;color:var(--text-muted);font-size:0.8rem;">+${{remaining}} more</div>`;
                            }}
                            html += '</div>';
                        }}

                        container.innerHTML = html;
                    }}
                }}
            }} catch (e) {{ console.error('Dashboard team load failed:', e); }}

            // 2. Today's reports (condensed)
            try {{
                const resp = await fetch('/api/reports?limit=500');
                const data = await resp.json();
                if (data.success) {{
                    const reports = data.reports || [];
                    const today = new Date().toISOString().split('T')[0];
                    const todayReports = reports.filter(r => r.created_at && r.created_at.startsWith(today));
                    const container = document.getElementById('dash-reports');
                    if (!container) return;

                    if (todayReports.length === 0) {{
                        container.innerHTML = '<p style="color: var(--text-muted); font-style: italic;">No reports today yet</p>';
                    }} else {{
                        const complete = todayReports.filter(r => r.is_complete).length;
                        const inProgress = todayReports.filter(r => r.status === 'in_progress').length;

                        let html = `<div style="display: flex; gap: 1rem; margin-bottom: 0.75rem; font-size: 0.85rem;">
                            <span style="font-weight: 600;">${{todayReports.length}} reports today</span>
                            <span style="color: var(--success);">✅ ${{complete}} complete</span>
                            ${{inProgress > 0 ? `<span style="color: var(--warning);">🟡 ${{inProgress}} in progress</span>` : ''}}
                        </div>`;

                        // Show latest 4 reports as condensed rows
                        todayReports.sort((a, b) => new Date(b.created_at) - new Date(a.created_at));
                        todayReports.slice(0, 4).forEach(r => {{
                            const statusColor = r.is_complete ? 'var(--success)' : r.status === 'in_progress' ? 'var(--warning)' : 'var(--text-muted)';
                            const statusIcon = r.is_complete ? '✅' : r.status === 'in_progress' ? '🟡' : '📋';
                            html += `<div style="display: flex; justify-content: space-between; align-items: center; padding: 0.4rem 0; border-bottom: 1px solid var(--border);">
                                <div>
                                    <span style="font-weight: 500; font-size: 0.9rem;">${{r.title}}</span>
                                    <span style="color: var(--text-muted); font-size: 0.8rem; margin-left: 0.5rem;">${{r.room_name || ''}}</span>
                                </div>
                                <div style="display: flex; align-items: center; gap: 0.5rem;">
                                    <span style="color: var(--text-muted); font-size: 0.75rem;">${{r.author_name || ''}}</span>
                                    <span style="color: ${{statusColor}}; font-size: 0.85rem;">${{statusIcon}}</span>
                                </div>
                            </div>`;
                        }});
                        if (todayReports.length > 4) {{
                            html += `<div style="text-align: center; padding: 0.5rem; font-size: 0.8rem;"><a href="#" onclick="showPage('reports')" style="color: var(--accent);">View all ${{todayReports.length}} reports →</a></div>`;
                        }}
                        container.innerHTML = html;
                    }}
                }}
            }} catch (e) {{ console.error('Dashboard reports load failed:', e); }}

            // 3. Admin section (only if trusted)
            if (isTrusted) {{
                const adminSection = document.getElementById('dash-admin-section');
                if (adminSection) adminSection.style.display = '';

                // Pending approvals
                try {{
                    const resp = await fetch('/api/web/changes/pending');
                    const data = await resp.json();
                    if (data.success) {{
                        const changes = data.changes || [];
                        const container = document.getElementById('dash-pending-approvals');
                        if (container) {{
                            if (changes.length === 0) {{
                                container.innerHTML = '<p style="color: var(--success);">✅ No pending approvals</p>';
                            }} else {{
                                let html = `<div style="font-size: 0.85rem; font-weight: 600; color: var(--warning); margin-bottom: 0.5rem;">⚠️ ${{changes.length}} pending</div>`;
                                changes.slice(0, 5).forEach(c => {{
                                    html += `<div style="display: flex; justify-content: space-between; align-items: center; padding: 0.35rem 0; border-bottom: 1px solid var(--border); font-size: 0.85rem;">
                                        <div>
                                            <span style="font-weight: 500;">${{c.change_type}}</span>
                                            <span style="color: var(--text-muted); margin-left: 0.5rem;">${{c.client_name || 'Unknown'}}</span>
                                        </div>
                                        <div style="display: flex; gap: 0.3rem;">
                                            <button class="btn" style="font-size: 0.7rem; padding: 0.2rem 0.5rem; background: var(--success); color: white; border: none; border-radius: 4px; cursor: pointer;" onclick="approveChange(${{c.id}}); loadDashboardHome();">✓</button>
                                            <button class="btn" style="font-size: 0.7rem; padding: 0.2rem 0.5rem; background: var(--danger); color: white; border: none; border-radius: 4px; cursor: pointer;" onclick="rejectChange(${{c.id}}); loadDashboardHome();">✕</button>
                                        </div>
                                    </div>`;
                                }});
                                if (changes.length > 5) {{
                                    html += `<div style="text-align: center; padding: 0.5rem; font-size: 0.8rem;"><a href="#" onclick="showPage('approvals')" style="color: var(--accent);">View all ${{changes.length}} →</a></div>`;
                                }}
                                container.innerHTML = html;
                            }}
                        }}
                    }}
                }} catch (e) {{ console.error('Dashboard approvals failed:', e); }}

                // Today's hours for all workers
                try {{
                    const today = new Date().toISOString().split('T')[0];
                    const resp = await fetch(`/api/timesheets?date_from=${{today}}&date_to=${{today}}&limit=500`);
                    const data = await resp.json();
                    if (data.success) {{
                        const days = data.days || [];
                        const container = document.getElementById('dash-today-hours');
                        if (container) {{
                            if (days.length === 0) {{
                                container.innerHTML = '<p style="color: var(--text-muted); font-style: italic;">No clock-ins today</p>';
                            }} else {{
                                const totalWork = days.reduce((s, d) => s + d.total_work_seconds, 0);
                                const totalBreak = days.reduce((s, d) => s + d.total_break_seconds, 0);
                                const workH = (totalWork / 3600).toFixed(1);
                                const breakH = (totalBreak / 3600).toFixed(1);

                                let html = `<div style="display: flex; gap: 1rem; margin-bottom: 0.75rem; font-size: 0.85rem;">
                                    <span style="font-weight: 600;">${{days.length}} workers</span>
                                    <span style="color: var(--success);">🟢 ${{workH}}h work</span>
                                    ${{parseFloat(breakH) > 0 ? `<span style="color: var(--warning);">☕ ${{breakH}}h break</span>` : ''}}
                                </div>`;

                                days.forEach(d => {{
                                    const wH = (d.total_work_seconds / 3600).toFixed(1);
                                    const bH = (d.total_break_seconds / 3600).toFixed(1);
                                    html += `<div style="display: flex; justify-content: space-between; align-items: center; padding: 0.35rem 0; border-bottom: 1px solid var(--border); font-size: 0.85rem;">
                                        <span style="font-weight: 500;">${{d.display_name}}</span>
                                        <div>
                                            <span style="color: var(--success);">${{wH}}h</span>
                                            ${{parseFloat(bH) > 0 ? `<span style="color: var(--warning); margin-left: 0.5rem;">${{bH}}h break</span>` : ''}}
                                            ${{d.entries.length > 0 ? `<span style="color: var(--text-muted); margin-left: 0.5rem;">${{d.entries.length}} entries</span>` : ''}}
                                        </div>
                                    </div>`;
                                }});
                                container.innerHTML = html;
                            }}
                        }}
                    }}
                }} catch (e) {{ console.error('Dashboard hours failed:', e); }}
            }} else {{
                const adminSection = document.getElementById('dash-admin-section');
                if (adminSection) adminSection.style.display = 'none';
            }}
        }}

        // ==================== TEAM ====================

        let teamData = [];

        async function loadTeamStatus() {{
            try {{
                const response = await fetch('/api/team/status');
                const data = await response.json();
                if (!data.success) return;
                teamData = data.members || [];
                renderTeam();
            }} catch (e) {{
                console.error('Failed to load team:', e);
            }}
        }}

        function renderTeam() {{
            const search = (document.getElementById('team-search')?.value || '').toLowerCase();
            const filter = document.getElementById('team-filter')?.value || 'all';

            const filtered = teamData.filter(m => {{
                const matchSearch = !search || m.display_name.toLowerCase().includes(search) || m.device_id.toLowerCase().includes(search);
                const matchFilter = filter === 'all' || m.status === filter;
                return matchSearch && matchFilter;
            }});

            const working = teamData.filter(m => m.status === 'working').length;
            const onBreak = teamData.filter(m => m.status === 'break').length;
            const offline = teamData.filter(m => m.status === 'offline').length;
            const totalScans = teamData.reduce((s, m) => s + (m.total_scans_today || 0), 0);

            document.getElementById('team-stats').innerHTML = `
                <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid var(--accent);">
                    <div style="font-size: 1.5rem; font-weight: 700;">${{teamData.length}}</div><div style="color: var(--text-muted); font-size: 0.8rem;">Total</div>
                </div>
                <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid var(--green);">
                    <div style="font-size: 1.5rem; font-weight: 700; color: var(--green);">${{working}}</div><div style="color: var(--text-muted); font-size: 0.8rem;">Working</div>
                </div>
                <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid var(--orange);">
                    <div style="font-size: 1.5rem; font-weight: 700; color: var(--orange);">${{onBreak}}</div><div style="color: var(--text-muted); font-size: 0.8rem;">On Break</div>
                </div>
                <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid #6b7280;">
                    <div style="font-size: 1.5rem; font-weight: 700; color: #6b7280;">${{offline}}</div><div style="color: var(--text-muted); font-size: 0.8rem;">Offline</div>
                </div>
                <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid var(--orange);">
                    <div style="font-size: 1.5rem; font-weight: 700;">${{totalScans}}</div><div style="color: var(--text-muted); font-size: 0.8rem;">Scans Today</div>
                </div>
            `;

            if (filtered.length === 0) {{
                document.getElementById('team-container').innerHTML = '<div style="text-align:center; padding:2rem; color:var(--text-muted);">No team members found. Add members to get started.</div>';
                return;
            }}

            document.getElementById('team-container').innerHTML = filtered.map(m => {{
                const statusColor = m.status === 'working' ? 'var(--green)' : m.status === 'break' ? 'var(--orange)' : '#6b7280';
                const statusIcon = m.status === 'working' ? '🟢' : m.status === 'break' ? '☕' : '⚫';
                const statusText = m.status === 'working' ? 'Working' : m.status === 'break' ? 'On Break' : 'Offline';
                const initials = m.display_name.split(' ').map(w => w[0]).join('').toUpperCase().slice(0,2);
                const avatarColor = m.avatar_color || '#6366f1';
                const adminBadge = m.is_admin ? '<span style="background:var(--orange); color:#000; padding:0.1rem 0.4rem; border-radius:4px; font-size:0.65rem; margin-left:0.5rem;">ADMIN</span>' : '';
                const elapsed = m.clock_in ? formatElapsed(m.clock_in) : '';

                return `<div style="background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 1rem; margin-bottom: 0.75rem; border-left: 4px solid ${{statusColor}};">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                        <div style="display: flex; align-items: center; gap: 0.75rem;">
                            <div style="width:40px; height:40px; border-radius:50%; background:${{avatarColor}}; display:flex; align-items:center; justify-content:center; color:white; font-weight:600; font-size:0.85rem;">${{initials}}</div>
                            <div>
                                <div style="font-weight: 600;">${{m.display_name}}${{adminBadge}} <span style="color:var(--text-muted); font-size:0.75rem; margin-left:0.5rem;">${{m.role}}</span></div>
                                <div style="color:var(--text-muted); font-size:0.8rem;">${{m.device_id}}</div>
                            </div>
                        </div>
                        <div style="text-align: right;">
                            <div style="color: ${{statusColor}}; font-weight: 600;">${{statusIcon}} ${{statusText}}</div>
                            ${{m.current_job ? `<div style="font-size:0.8rem; color:var(--text-muted);">${{m.current_job}}</div>` : ''}}
                            ${{elapsed ? `<div style="font-size:0.8rem; color:var(--text-muted);">${{elapsed}}</div>` : ''}}
                            <div style="font-size:0.8rem; color:var(--text-muted);">📊 ${{m.total_scans_today}} scans today</div>
                        </div>
                    </div>
                </div>`;
            }}).join('');
        }}

        function filterTeam() {{ renderTeam(); }}

        function showAddMemberModal() {{
            const name = prompt('Team member display name:');
            if (!name) return;
            const deviceId = prompt('Device ID (from the Devices tab):');
            if (!deviceId) return;
            const role = prompt('Role (worker/lead/manager):', 'worker') || 'worker';
            const isAdmin = confirm('Grant admin access?');

            fetch('/api/team/members', {{
                method: 'POST',
                headers: {{ 'Content-Type': 'application/json' }},
                body: JSON.stringify({{ device_id: deviceId, display_name: name, role: role, is_admin: isAdmin }})
            }}).then(() => loadTeamStatus());
        }}

        // ==================== TIMESHEETS ====================

        async function loadTimesheets() {{
            try {{
                const workerFilter = document.getElementById('ts-worker-filter')?.value || 'all';
                const dateFrom = document.getElementById('ts-date-from')?.value || '';
                const dateTo = document.getElementById('ts-date-to')?.value || '';

                let url = '/api/timesheets?limit=500';
                if (workerFilter !== 'all') url += `&device_id=${{encodeURIComponent(workerFilter)}}`;
                if (dateFrom) url += `&date_from=${{dateFrom}}`;
                if (dateTo) url += `&date_to=${{dateTo}}`;

                const response = await fetch(url);
                const data = await response.json();
                if (!data.success) return;
                const days = data.days || [];

                // Populate worker dropdown
                const workers = [...new Set(days.map(d => JSON.stringify({{id: d.device_id, name: d.display_name}})))];
                const filterEl = document.getElementById('ts-worker-filter');
                const curVal = filterEl.value;
                filterEl.innerHTML = '<option value="all">All Workers</option>' +
                    workers.map(w => {{ const p = JSON.parse(w); return `<option value="${{p.id}}" ${{p.id === curVal ? 'selected' : ''}}>${{p.name}}</option>`; }}).join('');

                // Stats
                const totalWorkH = days.reduce((s, d) => s + d.total_work_seconds / 3600, 0).toFixed(1);
                const totalBreakH = days.reduce((s, d) => s + d.total_break_seconds / 3600, 0).toFixed(1);
                const gpsDays = days.filter(d => d.has_gps).length;
                document.getElementById('ts-stats').innerHTML = `
                    <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid var(--accent);">
                        <div style="font-size: 1.5rem; font-weight: 700;">${{days.length}}</div><div style="color: var(--text-muted); font-size: 0.8rem;">Days</div>
                    </div>
                    <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid var(--green);">
                        <div style="font-size: 1.5rem; font-weight: 700; color: var(--green);">${{totalWorkH}}h</div><div style="color: var(--text-muted); font-size: 0.8rem;">Work</div>
                    </div>
                    <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid var(--orange);">
                        <div style="font-size: 1.5rem; font-weight: 700; color: var(--orange);">${{totalBreakH}}h</div><div style="color: var(--text-muted); font-size: 0.8rem;">Break</div>
                    </div>
                    <div style="background: var(--card); border-radius: 12px; padding: 1rem 1.25rem; min-width: 120px; border-left: 4px solid #8b5cf6;">
                        <div style="font-size: 1.5rem; font-weight: 700; color: #8b5cf6;">${{gpsDays}}</div><div style="color: var(--text-muted); font-size: 0.8rem;">GPS Days</div>
                    </div>
                `;

                if (days.length === 0) {{
                    document.getElementById('timesheets-container').innerHTML = '<div style="text-align:center; padding:2rem; color:var(--text-muted);">No timesheet entries found</div>';
                    return;
                }}

                document.getElementById('timesheets-container').innerHTML = days.map(day => {{
                    const workH = (day.total_work_seconds / 3600).toFixed(1);
                    const breakH = (day.total_break_seconds / 3600).toFixed(1);
                    const mapBtn = day.has_gps
                        ? `<button class="btn btn-sm" style="background:#8b5cf6; color:white; border:none; padding:0.25rem 0.75rem; border-radius:6px; cursor:pointer; font-size:0.75rem;" onclick="openTimesheetMap('${{day.device_id}}','${{day.date}}')">🗺️ Map</button>`
                        : '';

                    const entriesHtml = day.entries.map(e => {{
                        const type = e.is_break ? '<span style="color:var(--orange);">☕ Break</span>' : '<span style="color:var(--green);">🟢 Work</span>';
                        const dur = e.clock_out ? formatDuration(e.clock_in, e.clock_out) : '<span style="color:var(--green);">Active</span>';
                        const job = e.job_name || e.customer_name || '-';
                        return `<tr>
                            <td style="padding:0.4rem 0.75rem;">${{type}}</td>
                            <td style="padding:0.4rem 0.75rem;">${{job}}</td>
                            <td style="padding:0.4rem 0.75rem;">${{formatDateTime(e.clock_in)}}</td>
                            <td style="padding:0.4rem 0.75rem;">${{e.clock_out ? formatDateTime(e.clock_out) : '-'}}</td>
                            <td style="padding:0.4rem 0.75rem;">${{dur}}</td>
                        </tr>`;
                    }}).join('');

                    return `<details style="background: var(--card); border: 1px solid var(--border); border-radius: 12px; margin-bottom: 0.75rem;">
                        <summary style="padding: 1rem; cursor: pointer; display: flex; justify-content: space-between; align-items: center;">
                            <div style="display:flex; align-items:center; gap:1rem; flex-wrap:wrap;">
                                <strong>${{day.date}}</strong>
                                <span style="color:#8b5cf6;">${{day.display_name}}</span>
                                <span style="color:var(--green); font-size:0.85rem;">🟢 ${{workH}}h work</span>
                                ${{parseFloat(breakH) > 0 ? `<span style="color:var(--orange); font-size:0.85rem;">☕ ${{breakH}}h break</span>` : ''}}
                                <span style="color:var(--text-muted); font-size:0.8rem;">${{day.entries.length}} entries</span>
                            </div>
                            <div style="display:flex; gap:0.5rem; align-items:center;">${{mapBtn}}</div>
                        </summary>
                        <div style="padding: 0 1rem 1rem;">
                            <table style="width:100%; border-collapse:collapse; font-size:0.85rem;">
                                <thead><tr style="border-bottom:1px solid var(--border); color:var(--text-muted);">
                                    <th style="padding:0.4rem 0.75rem; text-align:left;">Type</th>
                                    <th style="padding:0.4rem 0.75rem; text-align:left;">Job</th>
                                    <th style="padding:0.4rem 0.75rem; text-align:left;">Clock In</th>
                                    <th style="padding:0.4rem 0.75rem; text-align:left;">Clock Out</th>
                                    <th style="padding:0.4rem 0.75rem; text-align:left;">Duration</th>
                                </tr></thead>
                                <tbody>${{entriesHtml}}</tbody>
                            </table>
                        </div>
                    </details>`;
                }}).join('');

            }} catch (e) {{
                console.error('Failed to load timesheets:', e);
            }}
        }}

        async function openTimesheetMap(deviceId, date) {{
            try {{
                // Fetch time entries for this device on this date
                const teResp = await fetch(`/api/time-entries?device_id=${{encodeURIComponent(deviceId)}}&limit=100`);
                const teData = await teResp.json();
                if (!teData.success) return;

                const dayEntries = (teData.entries || []).filter(e => e.clock_in && e.clock_in.startsWith(date));
                if (dayEntries.length === 0) {{ alert('No time entries found for this day'); return; }}

                // Fetch location pings for each entry
                let allPings = [];
                for (const entry of dayEntries) {{
                    const pingResp = await fetch(`/api/location-pings?time_entry_id=${{encodeURIComponent(entry.uuid)}}&limit=5000`);
                    const pingData = await pingResp.json();
                    if (pingData.success && pingData.pings) allPings = allPings.concat(pingData.pings);
                }}

                if (allPings.length === 0) {{ alert('No GPS data for this day'); return; }}

                // Open map in new window with pings plotted on OpenStreetMap
                const mapHtml = buildMapHtml(allPings, deviceId, date);
                const w = window.open('', '_blank', 'width=800,height=600');
                w.document.write(mapHtml);
                w.document.close();
            }} catch (e) {{
                console.error('Failed to open map:', e);
                alert('Failed to load GPS data');
            }}
        }}

        function buildMapHtml(pings, deviceId, date) {{
            const centerLat = pings.reduce((s, p) => s + p.latitude, 0) / pings.length;
            const centerLon = pings.reduce((s, p) => s + p.longitude, 0) / pings.length;
            const points = JSON.stringify(pings.map(p => [p.latitude, p.longitude, p.timestamp]));
            return `<!DOCTYPE html><html><head><title>GPS Track - ${{deviceId}} - ${{date}}</title>
                <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css"/>
                <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"><\/script>
                <style>body{{margin:0}} #map{{width:100vw;height:100vh}}</style></head><body>
                <div id="map"></div><script>
                const points = ${{points}};
                const map = L.map('map').setView([${{centerLat}}, ${{centerLon}}], 14);
                L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{attribution:'OSM'}}).addTo(map);
                const line = points.map(p => [p[0], p[1]]);
                L.polyline(line, {{color:'#6366f1', weight:3}}).addTo(map);
                if (points.length > 0) {{
                    L.circleMarker([points[0][0], points[0][1]], {{radius:8, color:'#22c55e', fillOpacity:1}}).addTo(map).bindPopup('Start: ' + new Date(points[0][2]).toLocaleTimeString());
                    const last = points[points.length-1];
                    L.circleMarker([last[0], last[1]], {{radius:8, color:'#ef4444', fillOpacity:1}}).addTo(map).bindPopup('End: ' + new Date(last[2]).toLocaleTimeString());
                }}
                points.forEach((p, i) => {{
                    if (i % Math.max(1, Math.floor(points.length / 20)) === 0) {{
                        L.circleMarker([p[0], p[1]], {{radius:4, color:'#6366f1', fillOpacity:0.7}}).addTo(map)
                            .bindPopup(new Date(p[2]).toLocaleTimeString());
                    }}
                }});
                map.fitBounds(L.polyline(line).getBounds().pad(0.1));
                <\/script></body></html>`;
        }}
    </script>
</body>
</html>
"##))
}

/// GET /favicon.ico - Serve logo as favicon
pub async fn favicon() -> Response {
    serve_logo().await
}

/// GET /logo.png - Serve logo PNG
pub async fn serve_logo() -> Response {
    // Try to load from static/logo.png at runtime
    let paths = [
        "static/logo.png",
        "server/codebar-server/static/logo.png",
        "../server/codebar-server/static/logo.png",
    ];
    for path in &paths {
        if let Ok(bytes) = tokio::fs::read(path).await {
            return (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "image/png"),
                    (header::CACHE_CONTROL, "public, max-age=86400"),
                ],
                bytes,
            )
                .into_response();
        }
    }
    // Fallback: simple SVG if PNG not found
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" rx="20" fill="#6366f1"/><text x="50" y="62" text-anchor="middle" font-size="48" font-weight="bold" fill="white" font-family="sans-serif">CN</text></svg>"##;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml")],
        svg,
    )
        .into_response()
}

/// GET /email - Email reports page
pub async fn email_page(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    
    // Get email config
    let email_config = state.repo.get_email_config().await.ok().flatten();
    let (email_to, email_from, smtp_host) = match email_config {
        Some(ref c) => (
            c.email_to.clone().unwrap_or_default(),
            c.email_from.clone().unwrap_or_default(),
            c.smtp_host.clone().unwrap_or_default()
        ),
        None => (String::new(), String::new(), String::new()),
    };
    
    // Get all jobs for dropdown
    let jobs = state.repo.get_jobs().await.unwrap_or_default();
    let jobs_json = serde_json::to_string(&jobs).unwrap_or_else(|_| "[]".to_string());
    
    // Get stats for email body
    let scan_count = state.repo.get_scan_count().await.unwrap_or(0);
    let job_count = jobs.len();
    let device_count = state.repo.get_devices().await.map(|d| d.len()).unwrap_or(0);
    
    Html(format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CabNet - Email Reports</title>
    <link rel="icon" type="image/png" href="/logo.png">
    <style>
        :root {{
            --bg: #0f172a;
            --card: #1e293b;
            --card-hover: #334155;
            --accent: #6366f1;
            --accent-glow: rgba(99, 102, 241, 0.3);
            --green: #22c55e;
            --text: #f8fafc;
            --text-muted: #94a3b8;
            --border: #334155;
            --warning: #f59e0b;
        }}
        
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg);
            color: var(--text);
            min-height: 100vh;
            line-height: 1.6;
        }}
        
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 1.5rem;
        }}
        
        header {{
            text-align: center;
            margin-bottom: 1.5rem;
            padding-top: 1rem;
        }}
        
        .logo {{ font-size: 2.5rem; margin-bottom: 0.5rem; }}
        h1 {{ font-size: 1.75rem; font-weight: 700; margin-bottom: 0.25rem; }}
        .subtitle {{ color: var(--text-muted); font-size: 0.9rem; }}
        
        .layout {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 1.5rem;
        }}
        
        @media (max-width: 900px) {{
            .layout {{ grid-template-columns: 1fr; }}
        }}
        
        .card {{
            background: var(--card);
            border: 1px solid var(--border);
            border-radius: 0.75rem;
            padding: 1rem;
            margin-bottom: 1rem;
        }}
        
        .card h2 {{
            font-size: 1rem;
            margin-bottom: 0.75rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        
        .filter-row {{
            display: flex;
            gap: 0.75rem;
            margin-bottom: 1rem;
            flex-wrap: wrap;
        }}
        
        .filter-group {{
            flex: 1;
            min-width: 150px;
        }}
        
        .filter-group label {{
            display: block;
            font-size: 0.75rem;
            color: var(--text-muted);
            margin-bottom: 0.25rem;
        }}
        
        select, input, textarea {{
            width: 100%;
            padding: 0.5rem 0.75rem;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 0.375rem;
            color: var(--text);
            font-size: 0.875rem;
        }}
        
        select:focus, input:focus, textarea:focus {{
            outline: none;
            border-color: var(--accent);
        }}
        
        .scan-groups {{
            max-height: 500px;
            overflow-y: auto;
        }}
        
        .scan-group {{
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 0.5rem;
            margin-bottom: 0.75rem;
            overflow: hidden;
        }}
        
        .scan-group.selected {{
            border-color: var(--accent);
            box-shadow: 0 0 0 2px var(--accent-glow);
        }}
        
        .group-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 0.75rem;
            cursor: pointer;
            transition: background 0.2s;
        }}
        
        .group-header:hover {{
            background: var(--card-hover);
        }}
        
        .group-title {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        
        .group-checkbox {{
            width: 18px;
            height: 18px;
            accent-color: var(--accent);
        }}
        
        .group-time {{
            font-weight: 600;
            font-size: 0.9rem;
        }}
        
        .group-meta {{
            display: flex;
            align-items: center;
            gap: 1rem;
            font-size: 0.8rem;
            color: var(--text-muted);
        }}
        
        .group-count {{
            background: var(--accent);
            color: white;
            padding: 0.125rem 0.5rem;
            border-radius: 9999px;
            font-size: 0.75rem;
            font-weight: 600;
        }}
        
        .group-scans {{
            display: none;
            padding: 0.5rem 0.75rem;
            border-top: 1px solid var(--border);
            max-height: 200px;
            overflow-y: auto;
        }}
        
        .group-scans.expanded {{
            display: block;
        }}
        
        .scan-item {{
            display: flex;
            justify-content: space-between;
            padding: 0.375rem 0;
            border-bottom: 1px solid var(--border);
            font-size: 0.8rem;
        }}
        
        .scan-item:last-child {{
            border-bottom: none;
        }}
        
        .scan-barcode {{
            font-family: monospace;
            color: var(--accent);
        }}
        
        .scan-time {{
            color: var(--text-muted);
        }}
        
        .form-group {{
            margin-bottom: 0.75rem;
        }}
        
        .form-group label {{
            display: block;
            margin-bottom: 0.25rem;
            font-weight: 500;
            color: var(--text-muted);
            font-size: 0.8rem;
        }}
        
        textarea {{ min-height: 120px; resize: vertical; font-family: inherit; }}
        
        .btn {{
            display: inline-flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
            padding: 0.625rem 1rem;
            border-radius: 0.375rem;
            font-weight: 600;
            font-size: 0.875rem;
            border: none;
            cursor: pointer;
            transition: all 0.2s;
        }}
        
        .btn-primary {{
            background: var(--accent);
            color: white;
            width: 100%;
        }}
        
        .btn-primary:hover {{ background: #4f46e5; }}
        
        .btn-success {{
            background: var(--green);
            color: white;
            width: 100%;
            margin-top: 0.5rem;
        }}
        
        .btn-success:hover {{ opacity: 0.9; }}
        
        .btn-outline {{
            background: transparent;
            border: 1px solid var(--border);
            color: var(--text);
        }}
        
        .btn-outline:hover {{ background: var(--card-hover); }}
        
        .stats {{
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 0.5rem;
            margin-bottom: 0.75rem;
        }}
        
        .stat {{
            background: var(--bg);
            border-radius: 0.5rem;
            padding: 0.5rem;
            text-align: center;
        }}
        
        .stat-value {{ font-size: 1.25rem; font-weight: 700; color: var(--accent); }}
        .stat-label {{ font-size: 0.7rem; color: var(--text-muted); }}
        
        .selected-info {{
            background: var(--bg);
            border-radius: 0.375rem;
            padding: 0.5rem 0.75rem;
            margin-bottom: 0.75rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            font-size: 0.85rem;
        }}
        
        .selected-count {{
            color: var(--accent);
            font-weight: 600;
        }}
        
        .loading {{
            text-align: center;
            padding: 2rem;
            color: var(--text-muted);
        }}
        
        .no-data {{
            text-align: center;
            padding: 2rem;
            color: var(--text-muted);
            font-style: italic;
        }}
        
        footer {{
            text-align: center;
            padding: 1.5rem;
            color: var(--text-muted);
            font-size: 0.8rem;
        }}
        
        footer a {{ color: var(--accent); text-decoration: none; }}
        footer a:hover {{ text-decoration: underline; }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="logo">📧</div>
            <h1>Email Reports</h1>
            <p class="subtitle">Select scan groups to include in your email report</p>
        </header>
        
        <div class="layout">
            <!-- Left Column: Scan Groups -->
            <div>
                <div class="card">
                    <h2>🏷️ Filter Scans</h2>
                    <div class="filter-row">
                        <div class="filter-group">
                            <label>Job</label>
                            <select id="job-filter" onchange="loadScans()">
                                <option value="">All Scans (Newest)</option>
                                {jobs_options}
                            </select>
                        </div>
                        <div class="filter-group">
                            <label>Group by Time Gap</label>
                            <select id="time-gap" onchange="regroupScans()">
                                <option value="1">1 hour between groups</option>
                                <option value="2">2 hours between groups</option>
                                <option value="4" selected>4 hours between groups</option>
                                <option value="8">8 hours between groups</option>
                                <option value="12">12 hours between groups</option>
                                <option value="24">24 hours between groups</option>
                            </select>
                        </div>
                    </div>
                    <button class="btn btn-outline" style="width: 100%;" onclick="loadScans()">🔄 Refresh</button>
                </div>
                
                <div class="card">
                    <h2>�️ Scan Groups</h2>
                    <div class="selected-info">
                        <span>Selected: <span class="selected-count" id="selected-count">0</span> scans in <span id="selected-groups">0</span> groups</span>
                        <button class="btn btn-outline" style="padding: 0.25rem 0.5rem; font-size: 0.75rem;" onclick="selectAll()">Select All</button>
                    </div>
                    <div class="scan-groups" id="scan-groups">
                        <div class="loading">Loading scans...</div>
                    </div>
                </div>
            </div>
            
            <!-- Right Column: Email Composer -->
            <div>
                <div class="card">
                    <h2>🏷️ Summary</h2>
                    <div class="stats">
                        <div class="stat">
                            <div class="stat-value">{scan_count}</div>
                            <div class="stat-label">Total Scans</div>
                        </div>
                        <div class="stat">
                            <div class="stat-value">{job_count}</div>
                            <div class="stat-label">Jobs</div>
                        </div>
                        <div class="stat">
                            <div class="stat-value">{device_count}</div>
                            <div class="stat-label">Devices</div>
                        </div>
                    </div>
                </div>
                
                <div class="card">
                    <h2>✉️ Compose Email</h2>
                    
                    <div class="form-group">
                        <label>Recipient Email</label>
                        <input type="email" id="email-to" placeholder="recipient@example.com" value="{email_to}">
                    </div>
                    
                    <div class="form-group">
                        <label>Subject</label>
                        <input type="text" id="email-subject" value="CabNet Scan Report - ">
                    </div>
                    
                    <div class="form-group">
                        <label>Message Body (scans will be inserted at {{{{SCANS}}}})</label>
                        <textarea id="email-body">CabNet Scan Report

Summary:
- Selected Scans: [COUNT]
- Generated: [DATE]

Scan Data:
{{{{SCANS}}}}

---
Sent from CabNet Server</textarea>
                    </div>
                    
                    <button class="btn btn-primary" onclick="insertScansAndOpen()">
                        � Insert Scans & Open in Email App
                    </button>
                    
                    <button class="btn btn-success" onclick="insertScansAndCopy()">
                        📋 Insert Scans & Copy to Clipboard
                    </button>
                    
                    <button class="btn btn-outline" style="width: 100%; margin-top: 0.5rem; background: var(--card-hover);" onclick="downloadCSV()">
                        📎 Download Scans as CSV
                    </button>
                </div>
                
                <div class="card">
                    <h2>⚙️ Email Config</h2>
                    <div style="font-size: 0.8rem; color: var(--text-muted);">
                        <strong>SMTP:</strong> {smtp_host_display}<br>
                        <strong>From:</strong> {email_from_display}
                    </div>
                </div>
            </div>
        </div>
        
        <footer>
            <a href="/">← Back to Search</a> · <a href="/dashboard">Dashboard</a> · CabNet Email Reports
        </footer>
    </div>
    
    <script>
        const jobs = {jobs_json};
        let allScans = [];
        let groupedScans = [];
        let selectedGroups = new Set();
        
        // Initialize
        document.addEventListener('DOMContentLoaded', () => {{
            const today = new Date().toLocaleDateString('en-US');
            document.getElementById('email-subject').value += today;
            loadScans();
        }});
        
        // Load scans from API
        async function loadScans() {{
            const jobFilter = document.getElementById('job-filter').value;
            const container = document.getElementById('scan-groups');
            container.innerHTML = '<div class="loading">Loading scans...</div>';
            
            try {{
                let url = '/api/scans?limit=500';
                if (jobFilter) {{
                    url += `&job_id=${{jobFilter}}`;
                }}
                
                const response = await fetch(url);
                const data = await response.json();
                
                if (data.success && data.scans) {{
                    allScans = data.scans;
                    regroupScans();
                }} else {{
                    container.innerHTML = '<div class="no-data">No scans found</div>';
                }}
            }} catch (e) {{
                container.innerHTML = '<div class="no-data">Error loading scans</div>';
                console.error(e);
            }}
        }}
        
        // Group scans by time gap
        function regroupScans() {{
            const gapHours = parseInt(document.getElementById('time-gap').value);
            const gapMs = gapHours * 60 * 60 * 1000;
            
            // Sort scans by time (newest first)
            const sorted = [...allScans].sort((a, b) => 
                new Date(b.scanned_at) - new Date(a.scanned_at)
            );
            
            groupedScans = [];
            let currentGroup = null;
            
            for (const scan of sorted) {{
                const scanTime = new Date(scan.scanned_at).getTime();
                
                if (!currentGroup || (currentGroup.startTime - scanTime) > gapMs) {{
                    // Start new group
                    currentGroup = {{
                        id: groupedScans.length,
                        startTime: scanTime,
                        endTime: scanTime,
                        scans: [scan]
                    }};
                    groupedScans.push(currentGroup);
                }} else {{
                    // Add to current group
                    currentGroup.endTime = scanTime;
                    currentGroup.scans.push(scan);
                }}
            }}
            
            selectedGroups.clear();
            renderGroups();
        }}
        
        // Render scan groups
        function renderGroups() {{
            const container = document.getElementById('scan-groups');
            
            if (groupedScans.length === 0) {{
                container.innerHTML = '<div class="no-data">No scans to display</div>';
                updateSelectedCount();
                return;
            }}
            
            let html = '';
            for (const group of groupedScans) {{
                const startDate = new Date(group.startTime);
                const endDate = new Date(group.endTime);
                const isSelected = selectedGroups.has(group.id);
                
                const timeRange = group.scans.length === 1 
                    ? startDate.toLocaleDateString('en-US') + ' ' + startDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})
                    : `${{endDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})}} - ${{startDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})}} on ${{startDate.toLocaleDateString('en-US')}}`;
                
                // Determine group job name from first barcode_job_ref
                const firstRef = group.scans.map(s => s.barcode_job_ref).find(r => r);
                let groupJobName = '';
                if (firstRef) {{
                    const matchedJob = jobs.find(j => j.reference_number === firstRef);
                    groupJobName = matchedJob ? matchedJob.name : '';
                }}
                if (!groupJobName) {{
                    const jobIds = [...new Set(group.scans.map(s => s.job_id).filter(Boolean))];
                    groupJobName = jobIds.length === 1 ? (jobs.find(j => j.id === jobIds[0])?.name || '-') : jobIds.length > 1 ? 'Mixed' : '-';
                }}
                
                html += `
                    <div class="scan-group ${{isSelected ? 'selected' : ''}}" data-group-id="${{group.id}}">
                        <div class="group-header" onclick="toggleGroupSelection(${{group.id}})">
                            <div class="group-title">
                                <input type="checkbox" class="group-checkbox" ${{isSelected ? 'checked' : ''}} 
                                       onclick="event.stopPropagation(); toggleGroupSelection(${{group.id}})">
                                <span class="group-time">${{groupJobName}} — ${{timeRange}}</span>
                            </div>
                            <div class="group-meta">
                                <span class="group-count">${{group.scans.length}} scans</span>
                                <button class="btn btn-outline" style="padding: 0.125rem 0.5rem; font-size: 0.7rem;" 
                                        onclick="event.stopPropagation(); toggleGroupExpand(${{group.id}})">
                                    Details
                                </button>
                            </div>
                        </div>
                        <div class="group-scans" id="group-scans-${{group.id}}">
                            ${{group.scans.map(s => {{
                                const ticketNum = s.ticket_number || stripJobPrefix(s.barcode);
                                return `
                                <div class="scan-item">
                                    <span class="scan-barcode">${{ticketNum}}</span>
                                    <span class="scan-time">${{new Date(s.scanned_at).toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})}}</span>
                                </div>
                            `;
                            }}).join('')}}
                        </div>
                    </div>
                `;
            }}
            
            container.innerHTML = html;
            updateSelectedCount();
        }}
        
        // Toggle group selection
        function toggleGroupSelection(groupId) {{
            if (selectedGroups.has(groupId)) {{
                selectedGroups.delete(groupId);
            }} else {{
                selectedGroups.add(groupId);
            }}
            renderGroups();
        }}
        
        // Toggle group expand
        function toggleGroupExpand(groupId) {{
            const el = document.getElementById(`group-scans-${{groupId}}`);
            el.classList.toggle('expanded');
        }}
        
        // Select all groups
        function selectAll() {{
            if (selectedGroups.size === groupedScans.length) {{
                selectedGroups.clear();
            }} else {{
                groupedScans.forEach(g => selectedGroups.add(g.id));
            }}
            renderGroups();
        }}
        
        // Update selected count display
        function updateSelectedCount() {{
            let totalScans = 0;
            for (const groupId of selectedGroups) {{
                const group = groupedScans.find(g => g.id === groupId);
                if (group) totalScans += group.scans.length;
            }}
            document.getElementById('selected-count').textContent = totalScans;
            document.getElementById('selected-groups').textContent = selectedGroups.size;
        }}
        
        // Strip job ID prefix from barcode (removes leading digits and zeros)
        function stripJobPrefix(barcode) {{
            // Match pattern: digits followed by zeros, then the actual ticket number
            const match = barcode.match(/^\d+0+(\d+)$/);
            return match ? match[1] : barcode;
        }}
        
        function getJobName(jobId) {{
            if (!jobId) return '-';
            const job = jobs.find(j => j.id === jobId || j.id === String(jobId) || j.server_id === jobId || j.server_id === Number(jobId));
            return job ? job.name : '-';
        }}
        
        // Get selected scans as formatted text
        function getSelectedScansText() {{
            let lines = [];
            for (const groupId of [...selectedGroups].sort((a, b) => a - b)) {{
                const group = groupedScans.find(g => g.id === groupId);
                if (!group) continue;
                
                const startDate = new Date(group.startTime);
                // Resolve job name from first barcode_job_ref
                const firstRef = group.scans.map(s => s.barcode_job_ref).find(r => r);
                let groupJob = '';
                if (firstRef) {{
                    const matchedJob = jobs.find(j => j.reference_number === firstRef);
                    groupJob = matchedJob ? matchedJob.name : '';
                }}
                if (!groupJob) {{
                    const jobIds = [...new Set(group.scans.map(s => s.job_id).filter(Boolean))];
                    groupJob = jobIds.length === 1 ? getJobName(jobIds[0]) : jobIds.length > 1 ? 'Mixed' : '';
                }}
                const groupLabel = groupJob ? `${{groupJob}} — ` : '';
                lines.push(`\n--- ${{groupLabel}}${{startDate.toLocaleDateString('en-US')}} ${{startDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}})}} (${{group.scans.length}} scans) ---`);
                
                for (const scan of group.scans) {{
                    const time = new Date(scan.scanned_at).toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}});
                    const ticketNum = scan.ticket_number || stripJobPrefix(scan.barcode);
                    lines.push(`${{ticketNum}} | ${{time}}`);
                }}
            }}
            return lines.join('\n');
        }}
        
        // Count selected scans
        function getSelectedScanCount() {{
            let count = 0;
            for (const groupId of selectedGroups) {{
                const group = groupedScans.find(g => g.id === groupId);
                if (group) count += group.scans.length;
            }}
            return count;
        }}
        
        // Prepare email body with scans inserted
        function prepareEmailBody() {{
            let body = document.getElementById('email-body').value;
            const scansText = getSelectedScansText();
            const count = getSelectedScanCount();
            const now = new Date();
            const date = now.toLocaleDateString('en-US') + ' ' + now.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}});
            
            body = body.replace('{{{{SCANS}}}}', scansText || '(No scans selected)');
            body = body.replace('[COUNT]', count.toString());
            body = body.replace('[DATE]', date);
            
            return body;
        }}
        
        // Insert scans and open in email app
        function insertScansAndOpen() {{
            const to = document.getElementById('email-to').value;
            const subject = encodeURIComponent(document.getElementById('email-subject').value);
            const body = encodeURIComponent(prepareEmailBody());
            
            window.location.href = `mailto:${{to}}?subject=${{subject}}&body=${{body}}`;
        }}
        
        // Insert scans and copy to clipboard
        function insertScansAndCopy() {{
            const subject = document.getElementById('email-subject').value;
            const body = prepareEmailBody();
            const fullText = `Subject: ${{subject}}\n\n${{body}}`;
            
            navigator.clipboard.writeText(fullText).then(() => {{
                alert(`Email content copied! (${{getSelectedScanCount()}} scans included)`);
            }}).catch(() => {{
                const textarea = document.createElement('textarea');
                textarea.value = fullText;
                document.body.appendChild(textarea);
                textarea.select();
                document.execCommand('copy');
                document.body.removeChild(textarea);
                alert(`Email content copied! (${{getSelectedScanCount()}} scans included)`);
            }});
        }}
        
        // Generate CSV content from selected scans
        function generateCSV() {{
            const rows = [['Job', 'Ticket #', 'Barcode', 'Device', 'Date', 'Time']];
            
            for (const groupId of [...selectedGroups].sort((a, b) => a - b)) {{
                const group = groupedScans.find(g => g.id === groupId);
                if (!group) continue;
                
                for (const scan of group.scans) {{
                    const jobName = getJobName(scan.job_id);
                    const ticketNum = scan.ticket_number || stripJobPrefix(scan.barcode);
                    const scanDate = new Date(scan.scanned_at);
                    const date = scanDate.toLocaleDateString('en-US');
                    const time = scanDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}});
                    const device = scan.device_id || '-';
                    // Escape fields that might contain commas
                    rows.push([jobName, ticketNum, scan.barcode, device, date, time].map(f => `"${{f}}"`));
                }}
            }}
            
            return rows.map(r => r.join(',')).join('\n');
        }}
        
        // Download selected scans as CSV file
        function downloadCSV() {{
            const count = getSelectedScanCount();
            if (count === 0) {{
                alert('Please select at least one scan group first.');
                return;
            }}
            
            const csv = generateCSV();
            const blob = new Blob([csv], {{ type: 'text/csv;charset=utf-8;' }});
            const url = URL.createObjectURL(blob);
            const link = document.createElement('a');
            const today = new Date().toLocaleDateString('en-US').replace(/\//g, '-');
            link.href = url;
            link.download = `CabNet_Scans_${{today}}.csv`;
            link.style.display = 'none';
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
            URL.revokeObjectURL(url);
        }}
    </script>
</body>
</html>
"##, 
        jobs_options = jobs.iter().map(|j| format!(r#"<option value="{}">{}</option>"#, j.id, j.name)).collect::<Vec<_>>().join(""),
        smtp_host_display = if smtp_host.is_empty() { "Not configured" } else { &smtp_host },
        email_from_display = if email_from.is_empty() { "Not configured" } else { &email_from },
    ))
}

/// GET /map - Map page showing scan locations
pub async fn map_page(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    
    // Get scan locations
    let scan_locations = state.repo.get_scan_locations().await.unwrap_or_default();
    let locations_json = serde_json::to_string(&scan_locations).unwrap_or_else(|_| "[]".to_string());
    
    Html(format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CabNet - Scan Map</title>
    <link rel="icon" type="image/png" href="/logo.png">
    <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
    <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
    <style>
        :root {{
            --bg: #0f172a;
            --card: #1e293b;
            --accent: #6366f1;
            --text: #f8fafc;
            --text-muted: #94a3b8;
            --border: #334155;
        }}
        
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg);
            color: var(--text);
            min-height: 100vh;
        }}
        
        .header {{
            background: var(--card);
            border-bottom: 1px solid var(--border);
            padding: 1rem 2rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        
        .header h1 {{
            font-size: 1.25rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        
        .controls {{
            display: flex;
            gap: 1rem;
            align-items: center;
        }}
        
        .control-group {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        
        .control-group label {{
            font-size: 0.85rem;
            color: var(--text-muted);
        }}
        
        select {{
            padding: 0.5rem 1rem;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 0.5rem;
            color: var(--text);
            font-size: 0.9rem;
        }}
        
        .badge {{
            background: var(--accent);
            color: white;
            padding: 0.25rem 0.75rem;
            border-radius: 9999px;
            font-size: 0.8rem;
            font-weight: 600;
        }}
        
        #map {{
            height: calc(100vh - 70px);
            width: 100%;
        }}
        
        .leaflet-popup-content {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        }}
        
        .scan-popup {{
            min-width: 200px;
        }}
        
        .scan-popup h3 {{
            margin: 0 0 0.5rem 0;
            font-size: 1rem;
            color: #1e293b;
        }}
        
        .scan-popup p {{
            margin: 0.25rem 0;
            font-size: 0.85rem;
            color: #64748b;
        }}
        
        .scan-popup .barcode {{
            font-family: monospace;
            background: #f1f5f9;
            padding: 0.25rem 0.5rem;
            border-radius: 0.25rem;
            font-size: 0.9rem;
        }}
        
        .back-link {{
            color: var(--text-muted);
            text-decoration: none;
            font-size: 0.9rem;
            display: flex;
            align-items: center;
            gap: 0.25rem;
        }}
        
        .back-link:hover {{ color: var(--text); }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🗺️ Scan Map</h1>
        <div class="controls">
            <div class="control-group">
                <label>Map Style:</label>
                <select id="map-provider" onchange="changeMapProvider()">
                    <option value="osm">OpenStreetMap</option>
                    <option value="esri">Esri Satellite</option>
                    <option value="esri-street">Esri Streets</option>
                    <option value="esri-topo">Esri Topographic</option>
                </select>
            </div>
            <span class="badge" id="scan-count">0 scans</span>
            <a href="/" class="back-link">← Back</a>
        </div>
    </div>
    
    <div id="map"></div>
    
    <script>
        const scanLocations = {locations_json};
        
        // Initialize map - default to US center or first scan location
        let defaultLat = 39.8283;
        let defaultLon = -98.5795;
        let defaultZoom = 4;
        
        if (scanLocations.length > 0) {{
            defaultLat = scanLocations[0].latitude;
            defaultLon = scanLocations[0].longitude;
            defaultZoom = 10;
        }}
        
        const map = L.map('map').setView([defaultLat, defaultLon], defaultZoom);
        
        // Tile layers
        const tileLayers = {{
            osm: L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
                attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
            }}),
            esri: L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{{z}}/{{y}}/{{x}}', {{
                attribution: 'Tiles &copy; Esri'
            }}),
            'esri-street': L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Street_Map/MapServer/tile/{{z}}/{{y}}/{{x}}', {{
                attribution: 'Tiles &copy; Esri'
            }}),
            'esri-topo': L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Topo_Map/MapServer/tile/{{z}}/{{y}}/{{x}}', {{
                attribution: 'Tiles &copy; Esri'
            }})
        }};
        
        let currentLayer = tileLayers.osm;
        currentLayer.addTo(map);
        
        function changeMapProvider() {{
            const provider = document.getElementById('map-provider').value;
            map.removeLayer(currentLayer);
            currentLayer = tileLayers[provider];
            currentLayer.addTo(map);
        }}
        
        // Add markers for scan locations
        const markers = [];
        scanLocations.forEach(scan => {{
            const scanDate = new Date(scan.scanned_at);
            const formattedTime = scanDate.toLocaleDateString('en-US') + ' ' + scanDate.toLocaleTimeString('en-US', {{hour: '2-digit', minute:'2-digit'}});
            const marker = L.marker([scan.latitude, scan.longitude])
                .bindPopup(`
                    <div class="scan-popup">
                        <h3>🏷️ Scan</h3>
                        <p><span class="barcode">${{scan.barcode}}</span></p>
                        <p><strong>Device:</strong> ${{scan.device_id.substring(0, 8)}}...</p>
                        <p><strong>Time:</strong> ${{formattedTime}}</p>
                        <p><strong>Location:</strong> ${{scan.latitude.toFixed(6)}}, ${{scan.longitude.toFixed(6)}}</p>
                    </div>
                `);
            marker.addTo(map);
            markers.push(marker);
        }});
        
        // Update scan count
        document.getElementById('scan-count').textContent = scanLocations.length + ' scan' + (scanLocations.length !== 1 ? 's' : '');
        
        // Fit bounds if we have markers
        if (markers.length > 1) {{
            const group = L.featureGroup(markers);
            map.fitBounds(group.getBounds().pad(0.1));
        }}
    </script>
</body>
</html>
"##))
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    
    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes.max(1))
    }
}

fn format_time_ago(timestamp: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
        let now = Utc::now();
        let duration = now.signed_duration_since(dt.with_timezone(&Utc));
        let seconds = duration.num_seconds() as u64;
        
        if seconds < 60 {
            "Just now".to_string()
        } else if seconds < 3600 {
            format!("{}m ago", seconds / 60)
        } else if seconds < 86400 {
            format!("{}h ago", seconds / 3600)
        } else {
            format!("{}d ago", seconds / 86400)
        }
    } else {
        "Unknown".to_string()
    }
}

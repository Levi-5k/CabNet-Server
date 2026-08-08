use codebar_server::api::routes::{AppState, SharedState};
use codebar_server::config::{Config, get_data_dir};
use codebar_server::db::{init_pool, repository::Repository};
use codebar_server::gui::CodeBarApp;
use codebar_server::gui::tabs::settings::auto_backup_on_startup;
use codebar_server::services::{create_tunnel_manager, PushNotificationService, TunnelConfig};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    // Initialize logging — stdout + file (Documents/CabNet/server.log)
    let data_dir = get_data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    let file_appender = tracing_appender::rolling::daily(&data_dir, "server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "codebar_server=debug,tower_http=info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!("Starting CabNet Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Data directory: {}", data_dir.display());

    // Load configuration
    let config = Config::from_env()?;
    let server_addr = config.server_addr();
    let port: u16 = server_addr.split(':').last()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    tracing::info!("Server will listen on http://{}", server_addr);
    tracing::info!("Database path: {}", config.database.path.display());

    // Create tunnel manager
    let tunnel_manager = create_tunnel_manager(port);

    // Cloudflare tunnel is required for app/device access. The HTTP server binds to localhost;
    // cloudflared is responsible for exposing it publicly.
    if let Some(tunnel_config) = TunnelConfig::load() {
        tracing::info!("Loaded tunnel config: tunnel_name={}, domain={}, subdomain={}, enabled={}", 
            tunnel_config.tunnel_name, tunnel_config.domain, tunnel_config.subdomain, tunnel_config.enabled);
    } else {
        tracing::info!("No tunnel configuration found at {}", data_dir.join("tunnel_config.json").display());
    }

    let tunnel_start_error = if let Ok(mut manager) = tunnel_manager.lock() {
        tracing::info!("Starting required Cloudflare tunnel...");
        manager.start().err()
    } else {
        Some("Could not acquire Cloudflare tunnel manager".to_string())
    };

    if let Some(error) = &tunnel_start_error {
        tracing::warn!("Cloudflare tunnel is not running yet: {}", error);
    }

    // Create tokio runtime
    let runtime = tokio::runtime::Runtime::new()?;
    let runtime_handle = runtime.handle().clone();

    // Initialize database and shared state
    let state: SharedState = runtime.block_on(async {
        let pool = init_pool(&config.database.path).await?;
        let repo = Repository::new(pool);
        let push_notifications = PushNotificationService::from_env();
        if push_notifications.is_configured() {
            tracing::info!("APNs push delivery configured");
        } else {
            tracing::info!("APNs push delivery disabled; unread polling remains available");
        }

        Ok::<_, anyhow::Error>(Arc::new(RwLock::new(AppState {
            repo,
            push_notifications,
            start_time: std::time::Instant::now(),
        })))
    })?;

    // Run daily auto-backup
    auto_backup_on_startup();

    // Start HTTP server in background
    let api_state = state.clone();
    let api_addr = server_addr.clone();
    runtime.spawn(async move {
        let router = codebar_server::api::create_router(api_state);
        let listener = tokio::net::TcpListener::bind(&api_addr).await.unwrap();
        tracing::info!("HTTP server listening on http://{}", api_addr);
        axum::serve(listener, router).await.unwrap();
    });

    // Run GUI on main thread (skip in headless environments)
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("CabNet Server"),
        ..Default::default()
    };

    // Check if we have a display available
    let has_display = std::env::var("DISPLAY").is_ok() || 
                     std::env::var("WAYLAND_DISPLAY").is_ok() || 
                     cfg!(windows) ||
                     cfg!(target_os = "macos");

    if has_display {
        eframe::run_native(
            "CabNet Server",
            native_options,
            Box::new(move |cc| {
                Ok(Box::new(CodeBarApp::new(
                    cc,
                    state,
                    runtime_handle,
                    server_addr,
                    tunnel_manager,
                )))
            }),
        )
        .map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;
    } else {
        if let Some(error) = tunnel_start_error {
            return Err(anyhow::anyhow!(
                "Cloudflare tunnel setup is required, but no desktop display is available to open setup: {}",
                error
            ));
        }

        tracing::info!("Running in headless mode - GUI disabled");
        // In headless mode, just keep the HTTP server running
        // Wait for shutdown signal
        runtime.block_on(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Shutdown signal received");
        });
    }

    Ok(())
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod app;
mod branding;
mod session;
mod singleton;
mod ssh;
mod tabs;
mod theme;
mod updater;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--cli") {
        std::process::exit(cli::run_cli());
    }

    // Singleton check (GUI mode only)
    match singleton::try_acquire("open") {
        Ok(singleton::SingletonResult::Secondary { .. }) => {
            eprintln!("Aineer is already running. Bringing existing window to front.");
            std::process::exit(0);
        }
        Ok(singleton::SingletonResult::Primary) => {
            singleton::start_listener(|msg| {
                tracing::info!("Received IPC message from secondary instance: {msg}");
            });
        }
        Err(e) => {
            tracing::warn!("Singleton check failed, proceeding anyway: {e}");
        }
    }

    if let Err(e) = run_gui() {
        eprintln!("Aineer GUI error: {e}");
        std::process::exit(1);
    }
}

fn run_gui() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("AINEER_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(branding::APP_NAME)
        .with_icon(branding::app_icon())
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([640.0, 400.0]);

    if cfg!(target_os = "linux") {
        viewport = viewport.with_app_id(branding::APP_ID);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        branding::APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(app::AineerApp::new(cc)))),
    )
}

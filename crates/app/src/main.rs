use anyhow::Result;

mod application;
mod bridge;
mod platform;
mod session;
mod workspace;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aineer=info".into()),
        )
        .init();

    if args.iter().any(|a| a == "--cli" || a == "-c") {
        tracing::info!("Starting Aineer CLI mode");
        println!("Aineer CLI mode — not yet implemented in new codebase");
        return Ok(());
    }

    tracing::info!(
        "Starting {} v{}",
        aineer_release_channel::ReleaseChannel::current().display_name(),
        env!("CARGO_PKG_VERSION"),
    );

    application::run_app();

    Ok(())
}

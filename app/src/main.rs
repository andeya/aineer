#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--cli" || a == "-c") {
        std::process::exit(aineer_cli::run_cli());
    }

    aineer_lib::run_desktop();
}

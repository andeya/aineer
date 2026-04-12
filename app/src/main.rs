#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|a| a == "--cli" || a == "-c") {
        aineer_lib::run_cli_with_tauri(tauri::generate_context!());
    } else {
        aineer_lib::run_desktop();
    }
}

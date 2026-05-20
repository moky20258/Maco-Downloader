#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api_types;
mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::search_music,
            commands::get_music_url,
            commands::download_music,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

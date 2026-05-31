#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api_types;
mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::search_music,
            commands::get_music_url,
            commands::download_music,
            commands::get_lyrics,
            commands::download_update,
            commands::open_download_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod adapters;
pub mod commands;
pub mod models;
pub mod sanitize;
pub mod state;

fn resolve_data_dir() -> std::path::PathBuf {
    // On Linux AppImage: current_exe() returns a path inside the ephemeral
    // FUSE mount (e.g. /tmp/.mount_xxxx/usr/bin/easy-read), which is destroyed
    // on exit. The $APPIMAGE env var always points to the actual .AppImage
    // file on disk — use its parent directory instead.
    #[cfg(target_os = "linux")]
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        if let Some(parent) = std::path::Path::new(&appimage).parent() {
            return parent.join("data");
        }
    }

    std::env::current_exe()
        .expect("cannot resolve exe path")
        .parent()
        .expect("exe has no parent dir")
        .join("data")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;
            let data_dir = resolve_data_dir();
            let app_state = state::load_state(&data_dir);
            app.manage(std::sync::Mutex::new(app_state));
            app.manage(std::sync::Mutex::new(state::BookIndex::default()));
            app.manage(data_dir);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::book::pick_file,
            commands::book::open_book,
            commands::preferences::get_state,
            commands::progress::update_progress,
            commands::progress::save_session_state,
            commands::annotations::toggle_bookmark,
            commands::annotations::get_bookmarks,
            commands::preferences::save_preference,
            commands::preferences::clear_last_opened,
            commands::search::search_book,
            commands::annotations::add_highlight,
            commands::annotations::remove_highlight,
            commands::annotations::get_highlights,
            commands::annotations::add_quote,
            commands::annotations::remove_quote,
            commands::annotations::get_quotes,
            commands::history::get_book_history,
            commands::history::delete_book_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

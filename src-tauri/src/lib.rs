pub mod adapters;
pub mod commands;
pub mod models;
pub mod sanitize;
pub mod state;

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tauri::Manager;

fn probe_writable_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|err| format!("failed to create data directory {}: {err}", path.display()))?;

    let probe_path = path.join(".easy-read-write-probe");
    {
        let mut probe_file = fs::File::create(&probe_path).map_err(|err| {
            format!(
                "failed to create data directory probe {}: {err}",
                probe_path.display()
            )
        })?;
        probe_file
            .write_all(b"probe")
            .map_err(|err| format!("failed to write data directory probe: {err}"))?;
        probe_file
            .sync_all()
            .map_err(|err| format!("failed to sync data directory probe: {err}"))?;
    }
    fs::remove_file(&probe_path).map_err(|err| {
        format!(
            "failed to remove data directory probe {}: {err}",
            probe_path.display()
        )
    })?;

    Ok(())
}

fn app_data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|err| {
        eprintln!("Failed to resolve Tauri app data directory: {err}");
        std::env::current_dir()
            .unwrap_or_else(|cwd_err| {
                eprintln!("Failed to resolve current directory: {cwd_err}");
                PathBuf::from(".")
            })
            .join("data")
    })
}

fn exe_adjacent_data_dir() -> Option<PathBuf> {
    // On Linux AppImage: current_exe() returns a path inside the ephemeral
    // FUSE mount (e.g. /tmp/.mount_xxxx/usr/bin/easy-read), which is destroyed
    // on exit. The $APPIMAGE env var points to the actual .AppImage file.
    #[cfg(target_os = "linux")]
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        if let Some(parent) = std::path::Path::new(&appimage).parent() {
            return Some(parent.join("data"));
        }
    }

    std::env::current_exe()
        .map_err(|err| eprintln!("Failed to resolve executable path: {err}"))
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join("data")))
}

fn resolve_data_dir(app: &tauri::AppHandle) -> PathBuf {
    if let Some(data_dir) = exe_adjacent_data_dir() {
        match probe_writable_dir(&data_dir) {
            Ok(()) => {
                eprintln!("Using data directory: {}", data_dir.display());
                return data_dir;
            }
            Err(err) => {
                eprintln!(
                    "Cannot write to executable-adjacent data directory {}; falling back: {err}",
                    data_dir.display()
                );
            }
        }
    }

    let fallback = app_data_dir(app);
    if let Err(err) = probe_writable_dir(&fallback) {
        eprintln!(
            "Using fallback data directory despite failed write probe {}: {err}",
            fallback.display()
        );
    } else {
        eprintln!("Using data directory: {}", fallback.display());
    }
    fallback
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = resolve_data_dir(app.handle());
            let loaded_state = state::load_state(&data_dir);
            app.manage(std::sync::Mutex::new(loaded_state.state));
            app.manage(loaded_state.state_recovered);
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

mod commands;
mod db;
mod download;
mod models;
mod sources;
mod state;
mod sushi;
mod tray;

use download::resolve_ffmpeg_path;
use state::AppState;
use tauri::Manager;

fn persist_resolved_ffmpeg(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let Ok(mut settings) = state.settings.lock() else {
        return;
    };

    let configured = settings.ffmpeg_path.clone();
    let resolved = resolve_ffmpeg_path(&configured);
    if resolved.source == download::FfmpegSource::Missing {
        return;
    }

    if configured.trim().is_empty() || configured == "ffmpeg" {
        settings.ffmpeg_path = resolved.path.clone();
        if let Ok(db) = state.db.lock() {
            let _ = db.save_settings(&settings);
        }
    }
}

async fn prepare_ffmpeg_in_background(app: tauri::AppHandle) {
    // Só registra FFmpeg já instalado no PATH; download do bundle é lazy no primeiro HLS.
    persist_resolved_ffmpeg(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .setup(|app| {
            tray::setup_tray(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                tray::attach_window_handlers(&window);
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                let items = state
                    .db
                    .lock()
                    .ok()
                    .and_then(|db| db.load_downloads().ok())
                    .unwrap_or_default();
                state.downloads.restore(items).await;
            });

            let ffmpeg_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                prepare_ffmpeg_in_background(ffmpeg_handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::parse_anime_url,
            commands::search_catalog,
            commands::browse_catalog,
            commands::get_categories,
            commands::get_search_history,
            commands::start_downloads,
            commands::cancel_download,
            commands::pause_download,
            commands::resume_download,
            commands::pause_anime,
            commands::resume_anime,
            commands::cancel_anime,
            commands::delete_download,
            commands::retry_download,
            commands::get_downloads,
            commands::get_settings,
            commands::save_settings,
            commands::check_ffmpeg,
            commands::get_ffmpeg_info,
            commands::pick_download_folder,
            commands::fetch_poster,
            commands::hide_window_to_tray,
            commands::exit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

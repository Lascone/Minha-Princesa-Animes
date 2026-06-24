use crate::models::{
    AnimeInfo, AnimeSourceId, AppSettings, BrowseRequest, CatalogPage, CategoryInfo, DownloadItem,
    DownloadRequest, SearchRequest,
};
use crate::sources::{self, source_for_url, SourceError};
use crate::download::{resolve_ffmpeg_path, FfmpegSource};
use base64::Engine;
use serde::Serialize;
use tauri::Manager;
use tauri::State;
use crate::state::AppState;

const CATALOG_CACHE_VERSION: &str = "v2";

#[tauri::command]
pub async fn parse_anime_url(url: String) -> Result<AnimeInfo, String> {
    let source = source_for_url(&url).map_err(|e| e.to_string())?;
    sources::parse_anime(source, &url)
        .await
        .map_err(|e| match e {
            SourceError::UnsupportedUrl => {
                "URL inválida. Cole um link de anime ou episódio de uma das fontes suportadas."
                    .to_string()
            }
            other => other.to_string(),
        })
}

#[tauri::command]
pub async fn search_catalog(
    req: SearchRequest,
    state: State<'_, AppState>,
) -> Result<CatalogPage, String> {
    let source = req.source;
    let cache_key = format!(
        "search:{CATALOG_CACHE_VERSION}:{source:?}:{}:{}:{:?}:{:?}",
        req.query, req.page, req.filters.media_filter, req.filters.sort
    );
    if let Ok(db) = state.db.lock() {
        if let Ok(Some(_cached)) = db.get_catalog_cache(&cache_key) {
            // skip cache for search to keep results fresh
        }
    }

    let result = sources::search(source, &req)
        .await
        .map_err(|e| e.to_string())?;

    if req.page == 1 && !req.query.is_empty() && !result.items.is_empty() {
        if let Ok(db) = state.db.lock() {
            let _ = db.add_search(&req.query);
            let _ = db.cache_catalog(&cache_key, &result.items);
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn browse_catalog(
    req: BrowseRequest,
    state: State<'_, AppState>,
) -> Result<CatalogPage, String> {
    let source = req.source;
    let cache_key = format!(
        "browse:{CATALOG_CACHE_VERSION}:{source:?}:{:?}:{}:{}:{:?}:{:?}",
        req.catalog_type,
        req.page,
        req.category_slug.as_deref().unwrap_or(""),
        req.filters.media_filter,
        req.filters.sort
    );
    if let Ok(db) = state.db.lock() {
        if let Ok(Some(items)) = db.get_catalog_cache(&cache_key) {
            if !items.is_empty() {
                return Ok(CatalogPage {
                    items,
                    page: req.page,
                    has_next: true,
                });
            }
        }
    }

    let result = sources::browse(source, &req)
        .await
        .map_err(|e| e.to_string())?;

    if !result.items.is_empty() {
        if let Ok(db) = state.db.lock() {
            let _ = db.cache_catalog(&cache_key, &result.items);
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_categories(source: Option<AnimeSourceId>) -> Result<Vec<CategoryInfo>, String> {
    let source = source.unwrap_or_default();
    sources::categories(source)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_search_history(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_search_history().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_downloads(
    app: tauri::AppHandle,
    request: DownloadRequest,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    state
        .downloads
        .start(app, request, settings)
        .await
}

#[tauri::command]
pub async fn pause_download(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.downloads.pause(&id).await
}

#[tauri::command]
pub async fn resume_download(
    app: tauri::AppHandle,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.downloads.resume(app, &id).await
}

#[tauri::command]
pub async fn pause_anime(title: String, state: State<'_, AppState>) -> Result<u32, String> {
    state.downloads.pause_anime(&title).await
}

#[tauri::command]
pub async fn resume_anime(
    app: tauri::AppHandle,
    title: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    state.downloads.resume_anime(app, &title).await
}

#[tauri::command]
pub async fn cancel_anime(title: String, state: State<'_, AppState>) -> Result<u32, String> {
    state.downloads.cancel_anime(&title).await
}

#[tauri::command]
pub async fn delete_download(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.downloads.delete(&id).await
}

#[tauri::command]
pub async fn cancel_download(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.downloads.cancel(&id).await
}

#[tauri::command]
pub async fn retry_download(
    app: tauri::AppHandle,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    state.downloads.retry(app, &id, settings).await
}

#[tauri::command]
pub async fn get_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadItem>, String> {
    Ok(state.downloads.list().await)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state
        .settings
        .lock()
        .map(|s| s.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    let max = settings.max_concurrent.clamp(1, 10);
    let settings = AppSettings {
        max_concurrent: max,
        ..settings
    };
    state.downloads.set_max_concurrent(max);
    {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        *s = settings.clone();
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_settings(&settings)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegInfo {
    pub path: String,
    pub available: bool,
    pub source: String,
    pub auto_managed: bool,
}

#[tauri::command]
pub fn get_ffmpeg_info(state: State<'_, AppState>) -> FfmpegInfo {
    let configured = state
        .settings
        .lock()
        .map(|s| s.ffmpeg_path.clone())
        .unwrap_or_default();
    let resolved = resolve_ffmpeg_path(&configured);
    FfmpegInfo {
        available: resolved.source != FfmpegSource::Missing,
        path: resolved.path,
        source: resolved.source.label().to_string(),
        auto_managed: configured.trim().is_empty(),
    }
}

#[tauri::command]
pub fn check_ffmpeg(path: String) -> bool {
    resolve_ffmpeg_path(&path).source != FfmpegSource::Missing
}

#[tauri::command]
pub async fn pick_download_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app
        .dialog()
        .file()
        .set_title("Selecionar pasta de downloads")
        .blocking_pick_folder();

    Ok(folder.map(|p| p.to_string()))
}

fn guess_image_mime(url: &str, bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return "image/png";
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg";
    }
    if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        return "image/webp";
    }
    let lower = url.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    }
}

#[tauri::command]
pub async fn fetch_poster(url: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    {
        let cache = state.poster_cache.lock().await;
        if let Some(cached) = cache.get(trimmed) {
            return Ok(Some(cached.clone()));
        }
    }

    let source = source_for_url(trimmed).unwrap_or(AnimeSourceId::Sushianimes);
    let bytes = sources::fetch_image(source, trimmed)
        .await
        .map_err(|e| e.to_string())?;

    if bytes.len() < 128 {
        return Ok(None);
    }

    let mime = guess_image_mime(trimmed, &bytes);
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:{mime};base64,{encoded}");

    let mut cache = state.poster_cache.lock().await;
    if cache.len() > 300 {
        cache.clear();
    }
    cache.insert(trimmed.to_string(), data_url.clone());

    Ok(Some(data_url))
}

#[tauri::command]
pub fn hide_window_to_tray(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Janela principal não encontrada".to_string())?;
    crate::tray::hide_to_tray(&window);
    Ok(())
}

#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

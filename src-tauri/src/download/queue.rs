use crate::db::CacheDb;
use crate::download::validate::{is_valid_episode_file, should_redownload};
use crate::download::{build_output_path, ensure_ffmpeg_path, hls};
use crate::models::{AppSettings, DownloadItem, DownloadRequest, DownloadStatus, EpisodeInfo};
use crate::sources::{self, source_for_url, StreamKind};
use crate::sources::shared::stream::{effective_stream_kind, needs_ffmpeg};
use crate::sushi::client::USER_AGENT_STR;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc as StdArc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::process::Child;
use tokio::sync::{Mutex as AsyncMutex, Notify, Semaphore};
use uuid::Uuid;

const PROGRESS_EMIT_MIN_MS: u64 = 500;
const PROGRESS_PERSIST_MIN_MS: u64 = 1000;
const MP4_PROGRESS_CHUNK_BYTES: u64 = 512 * 1024;
const MP4_STALL_TIMEOUT: Duration = Duration::from_secs(60);

async fn fetch_stream_with_headers(
    url: &str,
    referer: &str,
    origin: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STR));
    headers.insert(REFERER, HeaderValue::from_str(referer).unwrap_or_else(|_| HeaderValue::from_static("")));
    headers.insert(ORIGIN, HeaderValue::from_str(origin).unwrap_or_else(|_| HeaderValue::from_static("")));

    reqwest::Client::builder()
        .default_headers(headers)
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

struct ProgressThrottle {
    last_emit: Instant,
    last_persist: Instant,
    last_progress: f64,
}

pub struct DownloadManager {
    items: StdArc<AsyncMutex<HashMap<String, DownloadItem>>>,
    cancel_flags: StdArc<AsyncMutex<HashMap<String, bool>>>,
    pause_flags: StdArc<AsyncMutex<HashMap<String, bool>>>,
    stop_flags: StdArc<AsyncMutex<HashMap<String, StdArc<AtomicBool>>>>,
    child_slots: StdArc<AsyncMutex<HashMap<String, StdArc<AsyncMutex<Option<Child>>>>>>,
    semaphore: StdArc<Semaphore>,
    max_concurrent: StdArc<AtomicU32>,
    queue_notify: StdArc<Notify>,
    worker_running: StdArc<AtomicBool>,
    progress_throttle: StdArc<AsyncMutex<HashMap<String, ProgressThrottle>>>,
    cached_ffmpeg: StdArc<AsyncMutex<Option<String>>>,
    db: StdArc<Mutex<CacheDb>>,
}

impl DownloadManager {
    pub fn new(db: StdArc<Mutex<CacheDb>>) -> Self {
        let max = 3usize;
        Self {
            items: StdArc::new(AsyncMutex::new(HashMap::new())),
            cancel_flags: StdArc::new(AsyncMutex::new(HashMap::new())),
            pause_flags: StdArc::new(AsyncMutex::new(HashMap::new())),
            stop_flags: StdArc::new(AsyncMutex::new(HashMap::new())),
            child_slots: StdArc::new(AsyncMutex::new(HashMap::new())),
            semaphore: StdArc::new(Semaphore::new(max)),
            max_concurrent: StdArc::new(AtomicU32::new(max as u32)),
            queue_notify: StdArc::new(Notify::new()),
            worker_running: StdArc::new(AtomicBool::new(false)),
            progress_throttle: StdArc::new(AsyncMutex::new(HashMap::new())),
            cached_ffmpeg: StdArc::new(AsyncMutex::new(None)),
            db,
        }
    }

    pub fn set_max_concurrent(&self, max: u32) {
        let max = max.clamp(1, 10) as usize;
        let old = self.max_concurrent.swap(max as u32, Ordering::SeqCst) as usize;
        if max > old {
            self.semaphore.add_permits(max - old);
        }
    }

    pub async fn restore(&self, mut items: Vec<DownloadItem>) {
        for item in &mut items {
            if item.status == DownloadStatus::Paused {
                continue;
            }
            if matches!(
                item.status,
                DownloadStatus::Downloading | DownloadStatus::Queued
            ) {
                item.status = DownloadStatus::Failed;
                item.error = Some("Download interrompido ao fechar o app".to_string());
                item.progress = 0.0;
                item.speed = String::new();
            }
            if item.status == DownloadStatus::Completed {
                if let Some(ref path) = item.output_path {
                    if !std::path::Path::new(path).exists() {
                        item.status = DownloadStatus::Failed;
                        item.error = Some("Arquivo não encontrado no disco".to_string());
                        item.output_path = None;
                    }
                }
            }
        }

        let mut map = self.items.lock().await;
        for item in items {
            let _ = self.persist_item_sync(&item);
            map.insert(item.id.clone(), item);
        }
    }

    fn persist_item_sync(&self, item: &DownloadItem) -> Result<(), String> {
        self.db
            .lock()
            .map_err(|e| e.to_string())?
            .upsert_download(item)
            .map_err(|e| e.to_string())
    }

    async fn persist_item(&self, item: &DownloadItem) {
        let _ = self.persist_item_sync(item);
    }

    pub async fn list(&self) -> Vec<DownloadItem> {
        let items = self.items.lock().await;
        let mut list: Vec<_> = items.values().cloned().collect();
        list.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| a.anime_title.cmp(&b.anime_title))
                .then_with(|| a.episode.season.cmp(&b.episode.season))
                .then_with(|| a.episode.number.cmp(&b.episode.number))
        });
        list
    }

    pub async fn start(
        &self,
        app: AppHandle,
        request: DownloadRequest,
        settings: AppSettings,
    ) -> Result<Vec<String>, String> {
        self.set_max_concurrent(settings.max_concurrent);

        let mut ids = Vec::new();
        for episode in request.episodes {
            let id = Uuid::new_v4().to_string();
            let label = format!(
                "S{:02}E{:02} - {}",
                episode.season, episode.number, episode.title
            );
            let item = DownloadItem {
                id: id.clone(),
                anime_title: request.anime_title.clone(),
                episode_label: label,
                episode: episode.clone(),
                status: DownloadStatus::Queued,
                progress: 0.0,
                speed: String::new(),
                output_path: None,
                error: None,
                updated_at: unix_now(),
            };
            self.items.lock().await.insert(id.clone(), item.clone());
            self.persist_item(&item).await;
            self.cancel_flags.lock().await.insert(id.clone(), false);
            self.pause_flags.lock().await.insert(id.clone(), false);
            ids.push(id);
        }

        self.ensure_worker(app);
        Ok(ids)
    }

    fn ensure_worker(&self, app: AppHandle) {
        if self
            .worker_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            self.queue_notify.notify_one();
            return;
        }

        let mgr = self.clone_inner();
        tokio::spawn(async move {
            mgr.worker_loop(app).await;
        });
    }

    async fn worker_loop(&self, app: AppHandle) {
        loop {
            let mut spawned = false;
            while let Some((id, anime_title, episode, settings)) = self.next_runnable_job().await
            {
                let permit = match self.semaphore.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => break,
                };

                spawned = true;
                let mgr = self.clone_inner();
                let app_clone = app.clone();
                tokio::spawn(async move {
                    mgr.run_download(app_clone, id, anime_title, episode, settings)
                        .await;
                    drop(permit);
                    mgr.queue_notify.notify_one();
                });
            }

            if !spawned && !self.has_runnable_jobs().await {
                self.worker_running.store(false, Ordering::SeqCst);
                self.queue_notify.notified().await;
                if !self.has_runnable_jobs().await {
                    self.worker_running.store(false, Ordering::SeqCst);
                    break;
                }
                if self
                    .worker_running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    continue;
                }
            } else if spawned {
                tokio::time::sleep(Duration::from_millis(100)).await;
            } else {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }

    async fn has_runnable_jobs(&self) -> bool {
        let items = self.items.lock().await;
        let cancels = self.cancel_flags.lock().await;
        let pauses = self.pause_flags.lock().await;
        items.values().any(|i| {
            i.status == DownloadStatus::Queued
                && !cancels.get(&i.id).copied().unwrap_or(false)
                && !pauses.get(&i.id).copied().unwrap_or(false)
        })
    }

    async fn next_runnable_job(&self) -> Option<(String, String, EpisodeInfo, AppSettings)> {
        let settings = self.load_settings()?;
        let mut candidates: Vec<_> = {
            let items = self.items.lock().await;
            let cancels = self.cancel_flags.lock().await;
            let pauses = self.pause_flags.lock().await;
            items
                .values()
                .filter(|i| {
                    i.status == DownloadStatus::Queued
                        && !cancels.get(&i.id).copied().unwrap_or(false)
                        && !pauses.get(&i.id).copied().unwrap_or(false)
                })
                .cloned()
                .collect()
        };
        candidates.sort_by_key(|i| i.updated_at);
        let item = candidates.first()?;
        Some((
            item.id.clone(),
            item.anime_title.clone(),
            item.episode.clone(),
            settings,
        ))
    }

    fn load_settings(&self) -> Option<AppSettings> {
        self.db
            .lock()
            .ok()
            .map(|db| db.load_settings())
    }

    async fn kill_active(&self, id: &str) {
        if let Some(flag) = self.stop_flags.lock().await.get(id) {
            flag.store(true, Ordering::Relaxed);
        }
        if let Some(slot) = self.child_slots.lock().await.remove(id) {
            if let Some(mut child) = slot.lock().await.take() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
        }
    }

    pub async fn cancel(&self, id: &str) -> Result<(), String> {
        self.kill_active(id).await;
        let item = {
            let mut items = self.items.lock().await;
            let Some(item) = items.get_mut(id) else {
                return Ok(());
            };
            if item.status == DownloadStatus::Downloading
                || item.status == DownloadStatus::Queued
                || item.status == DownloadStatus::Paused
            {
                self.cancel_flags.lock().await.insert(id.to_string(), true);
                self.pause_flags.lock().await.insert(id.to_string(), false);
                item.status = DownloadStatus::Cancelled;
                item.updated_at = unix_now();
                item.clone()
            } else {
                return Ok(());
            }
        };
        self.persist_item(&item).await;
        self.queue_notify.notify_one();
        Ok(())
    }

    pub async fn pause(&self, id: &str) -> Result<(), String> {
        self.kill_active(id).await;
        let item = {
            let mut items = self.items.lock().await;
            let Some(item) = items.get_mut(id) else {
                return Ok(());
            };
            if item.status == DownloadStatus::Downloading || item.status == DownloadStatus::Queued
            {
                self.pause_flags.lock().await.insert(id.to_string(), true);
                item.status = DownloadStatus::Paused;
                item.speed = String::new();
                item.updated_at = unix_now();
                item.clone()
            } else {
                return Ok(());
            }
        };
        self.persist_item(&item).await;
        self.queue_notify.notify_one();
        Ok(())
    }

    pub async fn resume(&self, app: AppHandle, id: &str) -> Result<(), String> {
        let item = {
            let mut items = self.items.lock().await;
            let Some(item) = items.get_mut(id) else {
                return Err("Download não encontrado".to_string());
            };
            if item.status != DownloadStatus::Paused {
                return Err("Só é possível retomar downloads pausados".to_string());
            }
            self.pause_flags.lock().await.insert(id.to_string(), false);
            self.cancel_flags.lock().await.insert(id.to_string(), false);
            item.status = DownloadStatus::Queued;
            item.progress = 0.0;
            item.error = None;
            item.output_path = None;
            item.speed = String::new();
            item.updated_at = unix_now();
            item.clone()
        };
        self.persist_item(&item).await;
        self.ensure_worker(app);
        Ok(())
    }

    pub async fn pause_anime(&self, title: &str) -> Result<u32, String> {
        let ids: Vec<String> = {
            let items = self.items.lock().await;
            items
                .values()
                .filter(|i| {
                    i.anime_title == title
                        && matches!(
                            i.status,
                            DownloadStatus::Downloading | DownloadStatus::Queued
                        )
                })
                .map(|i| i.id.clone())
                .collect()
        };
        let mut count = 0u32;
        for id in ids {
            self.pause(&id).await?;
            count += 1;
        }
        Ok(count)
    }

    pub async fn resume_anime(&self, app: AppHandle, title: &str) -> Result<u32, String> {
        let ids: Vec<String> = {
            let items = self.items.lock().await;
            items
                .values()
                .filter(|i| i.anime_title == title && i.status == DownloadStatus::Paused)
                .map(|i| i.id.clone())
                .collect()
        };
        let mut count = 0u32;
        for id in ids {
            self.resume(app.clone(), &id).await?;
            count += 1;
        }
        Ok(count)
    }

    pub async fn cancel_anime(&self, title: &str) -> Result<u32, String> {
        let ids: Vec<String> = {
            let items = self.items.lock().await;
            items
                .values()
                .filter(|i| {
                    i.anime_title == title
                        && matches!(
                            i.status,
                            DownloadStatus::Downloading
                                | DownloadStatus::Queued
                                | DownloadStatus::Paused
                        )
                })
                .map(|i| i.id.clone())
                .collect()
        };
        let mut count = 0u32;
        for id in ids {
            self.cancel(&id).await?;
            count += 1;
        }
        Ok(count)
    }

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        self.kill_active(id).await;
        {
            let mut items = self.items.lock().await;
            items.remove(id);
        }
        self.cancel_flags.lock().await.remove(id);
        self.pause_flags.lock().await.remove(id);
        self.stop_flags.lock().await.remove(id);
        self.db
            .lock()
            .map_err(|e| e.to_string())?
            .delete_download(id)
            .map_err(|e| e.to_string())
    }

    pub async fn retry(
        &self,
        app: AppHandle,
        id: &str,
        settings: AppSettings,
    ) -> Result<(), String> {
        self.set_max_concurrent(settings.max_concurrent);

        let saved = {
            let mut items = self.items.lock().await;
            let item = items
                .get_mut(id)
                .ok_or_else(|| "Download não encontrado".to_string())?;
            if item.status != DownloadStatus::Failed && item.status != DownloadStatus::Cancelled {
                return Err(
                    "Só é possível tentar novamente downloads com falha ou cancelados"
                        .to_string(),
                );
            }
            item.status = DownloadStatus::Queued;
            item.progress = 0.0;
            item.error = None;
            item.output_path = None;
            item.speed = String::new();
            item.updated_at = unix_now();
            item.clone()
        };

        self.persist_item(&saved).await;
        self.cancel_flags.lock().await.insert(id.to_string(), false);
        self.pause_flags.lock().await.insert(id.to_string(), false);
        self.ensure_worker(app);
        Ok(())
    }

    fn clone_inner(&self) -> Self {
        Self {
            items: StdArc::clone(&self.items),
            cancel_flags: StdArc::clone(&self.cancel_flags),
            pause_flags: StdArc::clone(&self.pause_flags),
            stop_flags: StdArc::clone(&self.stop_flags),
            child_slots: StdArc::clone(&self.child_slots),
            semaphore: StdArc::clone(&self.semaphore),
            max_concurrent: StdArc::clone(&self.max_concurrent),
            queue_notify: StdArc::clone(&self.queue_notify),
            worker_running: StdArc::clone(&self.worker_running),
            progress_throttle: StdArc::clone(&self.progress_throttle),
            cached_ffmpeg: StdArc::clone(&self.cached_ffmpeg),
            db: StdArc::clone(&self.db),
        }
    }

    async fn is_stopped(&self, id: &str) -> bool {
        if self
            .cancel_flags
            .lock()
            .await
            .get(id)
            .copied()
            .unwrap_or(false)
        {
            return true;
        }
        if self.pause_flags.lock().await.get(id).copied().unwrap_or(false) {
            return true;
        }
        if let Some(flag) = self.stop_flags.lock().await.get(id) {
            if flag.load(Ordering::Relaxed) {
                return true;
            }
        }
        false
    }

    async fn get_stop_flag(&self, id: &str) -> StdArc<AtomicBool> {
        let mut flags = self.stop_flags.lock().await;
        if let Some(f) = flags.get(id) {
            return StdArc::clone(f);
        }
        let flag = StdArc::new(AtomicBool::new(false));
        flags.insert(id.to_string(), StdArc::clone(&flag));
        flag
    }

    async fn update_progress_throttled(
        &self,
        app: &AppHandle,
        id: &str,
        progress: f64,
        speed: &str,
        force_persist: bool,
    ) {
        let now = Instant::now();
        let (should_emit, should_persist) = {
            let mut throttle = self.progress_throttle.lock().await;
            let entry = throttle.entry(id.to_string()).or_insert(ProgressThrottle {
                last_emit: Instant::now() - Duration::from_secs(10),
                last_persist: Instant::now() - Duration::from_secs(10),
                last_progress: 0.0,
            });

            let emit = now.duration_since(entry.last_emit).as_millis() as u64
                >= PROGRESS_EMIT_MIN_MS
                || (progress - entry.last_progress).abs() >= 5.0;
            let persist = force_persist
                || now.duration_since(entry.last_persist).as_millis() as u64
                    >= PROGRESS_PERSIST_MIN_MS;

            if emit {
                entry.last_emit = now;
                entry.last_progress = progress;
            }
            if persist {
                entry.last_persist = now;
            }

            (emit, persist)
        };

        if !should_emit && !should_persist {
            return;
        }

        let item = {
            let mut items = self.items.lock().await;
            let Some(item) = items.get_mut(id) else {
                return;
            };
            item.progress = progress;
            item.speed = speed.to_string();
            item.updated_at = unix_now();
            item.clone()
        };

        if should_persist {
            self.persist_item(&item).await;
        }
        if should_emit {
            let _ = app.emit("download-progress", item);
        }
    }

    async fn update_item(&self, app: &AppHandle, id: &str, update: impl FnOnce(&mut DownloadItem)) {
        let item = {
            let mut items = self.items.lock().await;
            let Some(item) = items.get_mut(id) else {
                return;
            };
            update(item);
            item.updated_at = unix_now();
            item.clone()
        };
        self.persist_item(&item).await;
        let _ = app.emit("download-progress", item);
    }

    async fn run_download(
        &self,
        app: AppHandle,
        id: String,
        anime_title: String,
        episode: EpisodeInfo,
        settings: AppSettings,
    ) {
        if self.is_stopped(&id).await {
            return;
        }

        let stop_flag = self.get_stop_flag(&id).await;
        stop_flag.store(false, Ordering::Relaxed);

        self.update_item(&app, &id, |item| {
            item.status = DownloadStatus::Downloading;
            item.progress = 0.0;
            item.speed = "Resolvendo link do vídeo...".to_string();
        })
        .await;

        let output_path = build_output_path(&settings, &anime_title, &episode);

        if output_path.exists()
            && !settings.overwrite
            && !should_redownload(&output_path)
            && is_valid_episode_file(&output_path).is_ok()
        {
            self.update_item(&app, &id, |item| {
                item.status = DownloadStatus::Completed;
                item.progress = 100.0;
                item.output_path = Some(output_path.to_string_lossy().to_string());
            })
            .await;
            return;
        }

        if output_path.exists() && should_redownload(&output_path) {
            let _ = tokio::fs::remove_file(&output_path).await;
        }

        let source = match source_for_url(&episode.url) {
            Ok(s) => s,
            Err(e) => {
                self.fail(&app, &id, e.to_string()).await;
                return;
            }
        };

        let stream = match sources::resolve_stream(source, &episode.url).await {
            Ok(s) => s,
            Err(e) => {
                self.fail(&app, &id, e.to_string()).await;
                return;
            }
        };

        if self.is_stopped(&id).await {
            self.handle_stop(&app, &id, &output_path).await;
            return;
        }

        let result = self
            .download_with_fallback(
                &app,
                &id,
                &settings,
                &stream.url,
                stream.kind,
                &stream.referer,
                &stream.origin,
                &output_path,
                stop_flag,
            )
            .await;

        match result {
            Ok(()) => match is_valid_episode_file(&output_path) {
                Ok(()) => {
                    self.update_item(&app, &id, |item| {
                        item.status = DownloadStatus::Completed;
                        item.progress = 100.0;
                        item.output_path = Some(output_path.to_string_lossy().to_string());
                    })
                    .await;
                }
                Err(e) => {
                    let _ = tokio::fs::remove_file(&output_path).await;
                    self.fail(&app, &id, e).await;
                }
            },
            Err(e) if e == "Cancelado" || e == "Pausado" => {
                self.handle_stop(&app, &id, &output_path).await;
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&output_path).await;
                self.fail(&app, &id, e).await;
            }
        }
    }

    async fn handle_stop(&self, app: &AppHandle, id: &str, output_path: &PathBuf) {
        let _ = tokio::fs::remove_file(output_path).await;
        let paused = self.pause_flags.lock().await.get(id).copied().unwrap_or(false);
        let cancelled = self
            .cancel_flags
            .lock()
            .await
            .get(id)
            .copied()
            .unwrap_or(false);

        if paused && !cancelled {
            self.update_item(app, id, |item| {
                item.status = DownloadStatus::Paused;
                item.progress = 0.0;
                item.speed = String::new();
            })
            .await;
        } else if cancelled {
            self.update_item(app, id, |item| {
                item.status = DownloadStatus::Cancelled;
                item.speed = String::new();
            })
            .await;
        }
    }

    async fn download_with_fallback(
        &self,
        app: &AppHandle,
        id: &str,
        settings: &AppSettings,
        url: &str,
        kind: StreamKind,
        referer: &str,
        origin: &str,
        output: &PathBuf,
        stop_flag: StdArc<AtomicBool>,
    ) -> Result<(), String> {
        let effective = effective_stream_kind(url, kind);

        let primary = match effective {
            StreamKind::Hls => {
                self.download_hls(app, id, settings, url, referer, origin, output, stop_flag.clone())
                    .await
            }
            StreamKind::Mp4 => {
                self.download_mp4(app, id, url, referer, origin, output, stop_flag.clone())
                    .await
            }
        };

        if primary.is_ok() {
            return primary;
        }

        let primary_err = primary.err().unwrap_or_default();
        if primary_err == "Cancelado" || primary_err == "Pausado" {
            return Err(primary_err);
        }

        let try_hls_fallback = effective == StreamKind::Mp4
            && (primary_err.contains("playlist HLS")
                || primary_err.contains("mpegurl")
                || needs_ffmpeg(url, kind));

        if try_hls_fallback {
            let ffmpeg_path = self.resolved_ffmpeg(settings).await?;
            if hls::ffmpeg_available(&ffmpeg_path) {
                return self
                    .download_hls(app, id, settings, url, referer, origin, output, stop_flag)
                    .await
                    .map_err(|e| format!("{primary_err}; fallback HLS: {e}"));
            }
        }

        Err(primary_err)
    }

    async fn resolved_ffmpeg(&self, settings: &AppSettings) -> Result<String, String> {
        {
            let cached = self.cached_ffmpeg.lock().await;
            if let Some(ref p) = *cached {
                return Ok(p.clone());
            }
        }
        let path = ensure_ffmpeg_path(&settings.ffmpeg_path).await?;
        *self.cached_ffmpeg.lock().await = Some(path.clone());
        Ok(path)
    }

    async fn fail(&self, app: &AppHandle, id: &str, error: String) {
        self.update_item(app, id, |item| {
            item.status = DownloadStatus::Failed;
            item.error = Some(error);
        })
        .await;
    }

    async fn download_hls(
        &self,
        app: &AppHandle,
        id: &str,
        settings: &AppSettings,
        url: &str,
        referer: &str,
        origin: &str,
        output: &PathBuf,
        stop_flag: StdArc<AtomicBool>,
    ) -> Result<(), String> {
        self.update_item(app, id, |item| {
            item.speed = "Localizando FFmpeg...".to_string();
        })
        .await;

        let ffmpeg_path = self.resolved_ffmpeg(settings).await?;

        self.update_item(app, id, |item| {
            item.speed = "Conectando ao stream...".to_string();
            item.progress = 0.0;
        })
        .await;

        let child_slot = StdArc::new(AsyncMutex::new(None::<Child>));
        self.child_slots
            .lock()
            .await
            .insert(id.to_string(), StdArc::clone(&child_slot));
        let mgr = self.clone_inner();
        let app = app.clone();
        let id_owned = id.to_string();

        let result = hls::download_hls(
            &ffmpeg_path,
            url,
            output,
            referer,
            origin,
            stop_flag.clone(),
            child_slot,
            {
                let mgr = mgr.clone_inner();
                let app = app.clone();
                let id_owned = id_owned.clone();
                let gate = StdArc::new(std::sync::Mutex::new((Instant::now(), 0.0f64, 0u64)));
                move |event| match event {
                    hls::HlsEvent::Connecting(secs) => {
                        let mgr = mgr.clone_inner();
                        let app = app.clone();
                        let id = id_owned.clone();
                        tauri::async_runtime::spawn(async move {
                            mgr.update_item(&app, &id, |item| {
                                item.speed =
                                    format!("Conectando ao stream... ({secs}s)");
                            })
                            .await;
                        });
                    }
                    hls::HlsEvent::StallWarning(secs) => {
                        let mgr = mgr.clone_inner();
                        let app = app.clone();
                        let id = id_owned.clone();
                        tauri::async_runtime::spawn(async move {
                            mgr.update_item(&app, &id, |item| {
                                item.speed = format!("Sem progresso há {secs}s…");
                            })
                            .await;
                        });
                    }
                    hls::HlsEvent::BytesTransferred(bytes) => {
                        let mut g = gate.lock().unwrap_or_else(|e| e.into_inner());
                        let now = Instant::now();
                        let mb = bytes as f64 / 1_048_576.0;
                        let speed = if g.2 > 0 {
                            let delta = bytes.saturating_sub(g.2) as f64 / 1_048_576.0;
                            let secs = now.duration_since(g.0).as_secs_f64().max(0.1);
                            format!("{:.1} MB/s ({mb:.0} MB)", delta / secs)
                        } else {
                            format!("{mb:.0} MB baixados")
                        };
                        g.0 = now;
                        g.2 = bytes;
                        drop(g);
                        let mgr = mgr.clone_inner();
                        let app = app.clone();
                        let id = id_owned.clone();
                        tauri::async_runtime::spawn(async move {
                            mgr.update_item(&app, &id, |item| {
                                item.speed = speed;
                            })
                            .await;
                        });
                    }
                    hls::HlsEvent::Progress(progress) => {
                        let mut g = gate.lock().unwrap_or_else(|e| e.into_inner());
                        let now = Instant::now();
                        if now.duration_since(g.0).as_millis() < PROGRESS_EMIT_MIN_MS as u128
                            && (progress - g.1).abs() < 1.0
                        {
                            return;
                        }
                        g.0 = now;
                        g.1 = progress;
                        drop(g);
                        let mgr = mgr.clone_inner();
                        let app = app.clone();
                        let id = id_owned.clone();
                        tauri::async_runtime::spawn(async move {
                            let label = if progress < 3.0 {
                                "Conectando ao stream..."
                            } else {
                                "Convertendo vídeo…"
                            };
                            mgr.update_progress_throttled(&app, &id, progress, label, false)
                                .await;
                        });
                    }
                }
            },
        )
        .await;

        self.child_slots.lock().await.remove(id);

        match result {
            Ok(()) => Ok(()),
            Err(hls::HlsError::Cancelled) => {
                if self.pause_flags.lock().await.get(id).copied().unwrap_or(false) {
                    Err("Pausado".to_string())
                } else {
                    Err("Cancelado".to_string())
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }

    async fn download_mp4(
        &self,
        app: &AppHandle,
        id: &str,
        url: &str,
        referer: &str,
        origin: &str,
        output: &PathBuf,
        stop_flag: StdArc<AtomicBool>,
    ) -> Result<(), String> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let response = fetch_stream_with_headers(url, referer, origin)
            .await
            .map_err(|e| e.to_string())?;

        if is_playlist_content_type(response.headers().get("content-type")) {
            return Err("Resposta é playlist HLS, não MP4 direto".to_string());
        }

        let total = response.content_length().unwrap_or(0);
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut file = tokio::fs::File::create(output)
            .await
            .map_err(|e| e.to_string())?;

        use tokio::io::AsyncWriteExt;

        let started = Instant::now();
        let mut last_reported_bytes: u64 = 0;
        let mut last_emit = Instant::now();

        use tokio::time::timeout;

        loop {
            if stop_flag.load(Ordering::Relaxed) || self.is_stopped(id).await {
                let _ = tokio::fs::remove_file(output).await;
                if self.pause_flags.lock().await.get(id).copied().unwrap_or(false) {
                    return Err("Pausado".to_string());
                }
                return Err("Cancelado".to_string());
            }

            let chunk_result = timeout(MP4_STALL_TIMEOUT, stream.next()).await;
            let chunk = match chunk_result {
                Ok(Some(Ok(bytes))) => bytes,
                Ok(Some(Err(e))) => return Err(e.to_string()),
                Ok(None) => break,
                Err(_) => {
                    let _ = tokio::fs::remove_file(output).await;
                    return Err(format!(
                        "Download travado (sem progresso há {}s)",
                        MP4_STALL_TIMEOUT.as_secs()
                    ));
                }
            };

            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;

            let should_update = downloaded.saturating_sub(last_reported_bytes) >= MP4_PROGRESS_CHUNK_BYTES
                || last_emit.elapsed().as_millis() >= PROGRESS_EMIT_MIN_MS as u128;

            if should_update {
                last_reported_bytes = downloaded;
                last_emit = Instant::now();

                let progress = if total > 0 {
                    (downloaded as f64 / total as f64) * 100.0
                } else {
                    50.0
                };

                let elapsed = started.elapsed().as_secs_f64().max(0.1);
                let speed_label = if total > 0 {
                    let mbps = (downloaded as f64 / 1_048_576.0) / elapsed;
                    format!("{mbps:.1} MB/s")
                } else {
                    "Baixando…".to_string()
                };

                self.update_progress_throttled(app, id, progress, &speed_label, false)
                    .await;
            }
        }

        Ok(())
    }
}

fn is_playlist_content_type(value: Option<&reqwest::header::HeaderValue>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let ct = value.to_str().unwrap_or("").to_lowercase();
    ct.contains("mpegurl")
        || ct.contains("application/x-mpegurl")
        || ct.starts_with("text/plain")
        || ct.starts_with("text/html")
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new(StdArc::new(Mutex::new(
            CacheDb::open().expect("failed to open database"),
        )))
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::download::hls;
    use crate::models::AnimeSourceId;
    use crate::sources::shared::stream::effective_stream_kind;
    use crate::sources::{self, StreamKind};

    async fn download_test_episode(episode_url: &str, output_name: &str, min_bytes: u64) {
        use crate::download::validate::is_valid_episode_file;
        use crate::download::{resolve_ffmpeg_path, FfmpegSource};
        use futures_util::StreamExt;
        use std::sync::atomic::AtomicBool;
        use std::time::Instant;

        let source = AnimeSourceId::detect_from_url(episode_url).expect("source");
        let started = Instant::now();
        let stream = sources::resolve_stream(source, episode_url)
            .await
            .expect("stream");
        eprintln!("{episode_url} {:?} {} (resolve in {:?})", stream.kind, stream.url, started.elapsed());

        let output = std::env::temp_dir().join(output_name);
        let _ = std::fs::remove_file(&output);

        let dl_start = Instant::now();
        let effective = effective_stream_kind(&stream.url, stream.kind);
        match effective {
            StreamKind::Hls => {
                let resolved = resolve_ffmpeg_path("ffmpeg");
                assert_ne!(
                    resolved.source,
                    FfmpegSource::Missing,
                    "ffmpeg required for HLS test"
                );
                let ffmpeg = resolved.path;
                let stop = StdArc::new(AtomicBool::new(false));
                let slot = StdArc::new(AsyncMutex::new(None));
                hls::download_hls(
                    &ffmpeg,
                    &stream.url,
                    &output,
                    &stream.referer,
                    &stream.origin,
                    stop,
                    slot,
                    |event| match event {
                        hls::HlsEvent::Connecting(secs) => eprintln!("connecting {secs}s"),
                        hls::HlsEvent::StallWarning(secs) => eprintln!("stall warning {secs}s"),
                        hls::HlsEvent::BytesTransferred(b) => eprintln!("bytes {b}"),
                        hls::HlsEvent::Progress(p) => eprintln!("progress {:.0}%", p),
                    },
                )
                .await
                .expect("ffmpeg download");
            }
            StreamKind::Mp4 => {
                let response = fetch_stream_with_headers(&stream.url, &stream.referer, &stream.origin)
                    .await
                    .expect("fetch mp4");
                let mut file = tokio::fs::File::create(&output).await.unwrap();
                let mut stream_body = response.bytes_stream();
                use tokio::io::AsyncWriteExt;
                while let Some(chunk) = stream_body.next().await {
                    file.write_all(&chunk.unwrap()).await.unwrap();
                }
            }
        }

        let size = std::fs::metadata(&output).unwrap().len();
        eprintln!("ok: {} bytes in {:?} total {:?}", size, dl_start.elapsed(), started.elapsed());
        is_valid_episode_file(&output).expect("valid episode");
        assert!(size > min_bytes);
        let _ = std::fs::remove_file(&output);
    }

    #[tokio::test]
    #[ignore = "rede + ffmpeg: cargo test download_rezero_ep1 -- --ignored --nocapture"]
    async fn download_rezero_ep1() {
        download_test_episode(
            "https://sushianimes.com.br/anime/re-zero-kara-hajimeru-isekai-seikatsu-dublado-989-1-season-1-episode",
            "shishi_rezero_s01e01_test.mp4",
            1_000_000,
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "rede + ffmpeg: cargo test download_overlord_ep1 -- --ignored --nocapture"]
    async fn download_overlord_ep1() {
        download_test_episode(
            "https://sushianimes.com.br/anime/overlord-175-1-season-1-episode",
            "minha_princesa_overlord_s01e01_test.mp4",
            5_000_000,
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "rede + ffmpeg: cargo test resolve_goyabu_episode_71102_stream -- --ignored --nocapture"]
    async fn resolve_goyabu_episode_71102_stream() {
        download_test_episode(
            "https://goyabu.io/71102",
            "minha_princesa_goyabu_71102_test.mp4",
            1_000_000,
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "rede + ffmpeg: cargo test download_meusanimes_eden_ep1 -- --ignored --nocapture"]
    async fn download_meusanimes_eden_ep1() {
        download_test_episode(
            "https://meusanimes.blog/e/eden-1-episodio-1/",
            "meusanimes_eden_e1_test.mp4",
            1_000_000,
        )
        .await;
    }

    #[tokio::test]
    #[ignore = "rede + ffmpeg: cargo test download_animesdigital_megami_ep12 -- --ignored --nocapture"]
    async fn download_animesdigital_megami_ep12() {
        download_test_episode(
            "https://animesdigital.org/video/a/136716/",
            "animesdigital_megami_e12_test.mp4",
            1_000_000,
        )
        .await;
    }

    struct SmokeCase {
        label: &'static str,
        source: AnimeSourceId,
        anime_url: &'static str,
        episode_url: &'static str,
        download_file: &'static str,
        min_bytes: u64,
    }

    async fn smoke_catalog(source: AnimeSourceId, label: &str) {
        use crate::models::{BrowseRequest, CatalogFilters, CatalogSort, CatalogType, MediaFilter};

        let req = BrowseRequest {
            catalog_type: CatalogType::Animes,
            page: 1,
            category_slug: None,
            filters: CatalogFilters {
                media_filter: MediaFilter::Anime,
                sort: CatalogSort::Default,
                category: None,
                title_filter: None,
            },
            source,
        };
        let page = sources::browse(source, &req)
            .await
            .unwrap_or_else(|e| panic!("{label} catalog: {e}"));
        assert!(
            !page.items.is_empty(),
            "{label} catalog returned no items"
        );
        eprintln!("{label} catalog OK ({} items)", page.items.len());
    }

    #[tokio::test]
    #[ignore = "rede + ffmpeg: cargo test smoke_all_sources -- --ignored --nocapture"]
    async fn smoke_all_sources() {
        let cases = [
            SmokeCase {
                label: "Sushi Animes",
                source: AnimeSourceId::Sushianimes,
                anime_url: "https://sushianimes.com.br/anime/overlord-175",
                episode_url: "https://sushianimes.com.br/anime/overlord-175-1-season-1-episode",
                download_file: "smoke_sushi_overlord_e1.mp4",
                min_bytes: 2_000_000,
            },
            SmokeCase {
                label: "Goyabu",
                source: AnimeSourceId::Goyabu,
                anime_url: "https://goyabu.io/anime/ichijouma-mankitsugurashi",
                episode_url: "https://goyabu.io/71102",
                download_file: "smoke_goyabu_71102.mp4",
                min_bytes: 1_000_000,
            },
            SmokeCase {
                label: "Meus Animes",
                source: AnimeSourceId::Meusanimes,
                anime_url: "https://meusanimes.blog/a/one-piece-1/",
                episode_url: "https://meusanimes.blog/e/one-piece-1-episodio-2/",
                download_file: "smoke_meusanimes_op_e2.mp4",
                min_bytes: 1_000_000,
            },
            SmokeCase {
                label: "Animes Online CC",
                source: AnimeSourceId::Animesonlinecc,
                anime_url: "https://animesonlinecc.to/anime/one-piece/",
                episode_url: "https://animesonlinecc.to/episodio/one-piece-episodio-835/",
                download_file: "smoke_aocc_op_e835.mp4",
                min_bytes: 1_000_000,
            },
            SmokeCase {
                label: "Animes Digital",
                source: AnimeSourceId::Animesdigital,
                anime_url: "https://animesdigital.org/anime/b/one-piece-todos-episodios-5/",
                episode_url: "https://animesdigital.org/video/a/136491/",
                download_file: "smoke_animesdigital_op_e136491.mp4",
                min_bytes: 1_000_000,
            },
        ];

        for case in &cases {
            eprintln!("\n========== {} ==========", case.label);
            smoke_catalog(case.source, case.label).await;

            let anime = sources::parse_anime(case.source, case.anime_url)
                .await
                .unwrap_or_else(|e| panic!("{} parse anime: {e}", case.label));
            assert!(
                !anime.seasons.is_empty() && !anime.seasons[0].episodes.is_empty(),
                "{} parse returned no episodes",
                case.label
            );
            eprintln!(
                "{} parse OK: {} ({} eps)",
                case.label,
                anime.title,
                anime.seasons[0].episodes.len()
            );

            download_test_episode(case.episode_url, case.download_file, case.min_bytes).await;
            eprintln!("{} download OK", case.label);
        }

        eprintln!("\nAll 5 sources passed smoke test.");
    }
}

use crate::db::CacheDb;
use crate::download::DownloadManager;
use crate::models::AppSettings;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub downloads: DownloadManager,
    pub db: Arc<Mutex<CacheDb>>,
    pub poster_cache: AsyncMutex<HashMap<String, String>>,
}

impl AppState {
    pub fn new() -> Self {
        let db = Arc::new(Mutex::new(CacheDb::open().expect("failed to open database")));
        let settings = {
            let guard = db.lock().expect("db lock");
            let settings = guard.load_settings();
            let _ = guard.save_settings(&settings);
            settings
        };
        let downloads = DownloadManager::new(Arc::clone(&db));
        Self {
            settings: Mutex::new(settings),
            downloads,
            db,
            poster_cache: AsyncMutex::new(HashMap::new()),
        }
    }
}

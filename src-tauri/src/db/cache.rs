use crate::models::{AppSettings, CatalogItem, DownloadItem, APP_DATA_DIR};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct CacheDb {
    conn: Connection,
}

impl CacheDb {
    pub fn open() -> Result<Self, DbError> {
        let path = db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), DbError> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS catalog_cache (
                cache_key TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS downloads (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_downloads_updated ON downloads(updated_at DESC);
            ",
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn load_settings(&self) -> AppSettings {
        let mut settings = AppSettings::default();
        if let Ok(Some(json)) = self.get_setting("app_settings") {
            if let Ok(parsed) = serde_json::from_str(&json) {
                settings = parsed;
            }
        }
        if settings.download_folder.contains("ShishiAnimes") {
            settings.download_folder = settings
                .download_folder
                .replace("ShishiAnimes", APP_DATA_DIR);
        }
        settings
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), DbError> {
        let json = serde_json::to_string(settings).unwrap_or_default();
        self.set_setting("app_settings", &json)
    }

    pub fn add_search(&self, query: &str) -> Result<(), DbError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT INTO search_history (query, created_at) VALUES (?1, ?2)",
            params![query, now],
        )?;
        self.conn.execute(
            "DELETE FROM search_history WHERE id NOT IN (
                SELECT id FROM search_history ORDER BY created_at DESC LIMIT 20
            )",
            [],
        )?;
        Ok(())
    }

    pub fn get_search_history(&self) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT query FROM search_history ORDER BY created_at DESC LIMIT 10")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn cache_catalog(&self, key: &str, items: &[CatalogItem]) -> Result<(), DbError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let data = serde_json::to_string(items).unwrap_or_default();
        self.conn.execute(
            "INSERT OR REPLACE INTO catalog_cache (cache_key, data, created_at) VALUES (?1, ?2, ?3)",
            params![key, data, now],
        )?;
        Ok(())
    }

    pub fn get_catalog_cache(&self, key: &str) -> Result<Option<Vec<CatalogItem>>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT data, created_at FROM catalog_cache WHERE cache_key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let data: String = row.get(0)?;
            let created_at: i64 = row.get(1)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            // Cache valid for 1 hour
            if now - created_at < 3600 {
                if let Ok(items) = serde_json::from_str(&data) {
                    return Ok(Some(items));
                }
            }
        }
        Ok(None)
    }

    pub fn upsert_download(&self, item: &DownloadItem) -> Result<(), DbError> {
        let data = serde_json::to_string(item).unwrap_or_default();
        self.conn.execute(
            "INSERT OR REPLACE INTO downloads (id, data, updated_at) VALUES (?1, ?2, ?3)",
            params![item.id, data, item.updated_at as i64],
        )?;
        Ok(())
    }

    pub fn load_downloads(&self) -> Result<Vec<DownloadItem>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM downloads ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| {
            let data: String = row.get(0)?;
            Ok(data)
        })?;
        let mut items = Vec::new();
        for row in rows {
            let data: String = row?;
            if let Ok(item) = serde_json::from_str::<DownloadItem>(&data) {
                items.push(item);
            }
        }
        Ok(items)
    }

    pub fn delete_download(&self, id: &str) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM downloads WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn db_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let new_dir = base.join(APP_DATA_DIR);
    let new_path = new_dir.join("cache.db");
    let old_path = base.join("ShishiAnimes").join("cache.db");
    if !new_path.exists() && old_path.exists() {
        let _ = std::fs::create_dir_all(&new_dir);
        let _ = std::fs::copy(&old_path, &new_path);
    }
    new_path
}

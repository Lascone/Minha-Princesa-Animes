use crate::download::naming::sanitize_filename;
use crate::models::AppSettings;
use crate::sources::{self, source_for_url};
use std::path::{Path, PathBuf};

const POSTER_EXTENSIONS: [&str; 4] = ["jpg", "png", "webp", "jpeg"];

pub fn poster_dir(settings: &AppSettings, anime_title: &str) -> PathBuf {
    Path::new(&settings.download_folder).join(sanitize_filename(anime_title))
}

pub fn find_poster_on_disk(settings: &AppSettings, anime_title: &str) -> Option<PathBuf> {
    let dir = poster_dir(settings, anime_title);
    for ext in POSTER_EXTENSIONS {
        let path = dir.join(format!("poster.{ext}"));
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn image_extension(url: &str, bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG") {
        "png"
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        "webp"
    } else {
        let lower = url.to_lowercase();
        if lower.ends_with(".png") {
            "png"
        } else if lower.ends_with(".webp") {
            "webp"
        } else {
            "jpg"
        }
    }
}

pub async fn ensure_poster(
    settings: &AppSettings,
    anime_title: &str,
    poster_url: &str,
) -> Result<PathBuf, String> {
    if let Some(existing) = find_poster_on_disk(settings, anime_title) {
        return Ok(existing);
    }

    let trimmed = poster_url.trim();
    if trimmed.is_empty() {
        return Err("URL da capa vazia".to_string());
    }

    let source = source_for_url(trimmed).map_err(|e| e.to_string())?;
    let bytes = sources::fetch_image(source, trimmed)
        .await
        .map_err(|e| e.to_string())?;

    if bytes.len() < 128 {
        return Err("Capa muito pequena ou inválida".to_string());
    }

    let ext = image_extension(trimmed, &bytes);
    let dir = poster_dir(settings, anime_title);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Não foi possível criar pasta do anime: {e}"))?;

    let path = dir.join(format!("poster.{ext}"));
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| format!("Não foi possível salvar a capa: {e}"))?;

    Ok(path)
}

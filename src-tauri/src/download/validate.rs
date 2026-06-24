use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const MIN_EPISODE_BYTES: u64 = 5 * 1024 * 1024;

pub fn should_redownload(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => meta.len() < MIN_EPISODE_BYTES,
        Err(_) => false,
    }
}

pub fn is_valid_episode_file(path: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Arquivo não encontrado após download: {e}"))?;

    let size = meta.len();
    if size < MIN_EPISODE_BYTES {
        return Err(format!(
            "Download incompleto ({:.1} KB) — stream inválido ou CDN bloqueou",
            size as f64 / 1024.0
        ));
    }

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 16];
    let read = file.read(&mut header).map_err(|e| e.to_string())?;
    if read == 0 {
        return Err("Arquivo vazio".to_string());
    }

    let prefix = String::from_utf8_lossy(&header[..read.min(12)]).to_lowercase();
    if prefix.contains("<html") || prefix.contains("<!doctype") {
        return Err("Download retornou HTML em vez de vídeo (acesso bloqueado?)".to_string());
    }

    if read >= 8 {
        let ftyp = &header[4..8];
        if ftyp == b"ftyp" {
            return Ok(());
        }
    }

    // HLS muxed via ffmpeg may still be valid without ftyp at start in edge cases;
    // size check above is the primary guard.
    if size >= MIN_EPISODE_BYTES {
        return Ok(());
    }

    Err("Arquivo não parece ser um vídeo MP4 válido".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(name)
    }

    #[test]
    fn rejects_small_file() {
        let path = temp_path("shishi_validate_small.mp4");
        std::fs::write(&path, b"tiny").unwrap();
        assert!(is_valid_episode_file(&path).is_err());
        assert!(should_redownload(&path));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_html() {
        let path = temp_path("shishi_validate_html.mp4");
        let mut data = vec![0u8; MIN_EPISODE_BYTES as usize + 1];
        data[..15].copy_from_slice(b"<!DOCTYPE html>");
        std::fs::write(&path, &data).unwrap();
        assert!(is_valid_episode_file(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }
}

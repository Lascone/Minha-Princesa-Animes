use crate::download::hls;
use std::path::{Path, PathBuf};

const FFMPEG_ZIP_URL: &str =
    "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfmpegSource {
    Configured,
    Bundled,
    AppData,
    System,
    Missing,
}

impl FfmpegSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Configured => "configurado",
            Self::Bundled => "incluído no app",
            Self::AppData => "baixado automaticamente",
            Self::System => "instalado no sistema",
            Self::Missing => "não encontrado",
        }
    }
}

pub struct FfmpegResolution {
    pub path: String,
    pub source: FfmpegSource,
}

pub fn resolve_ffmpeg_path(configured: &str) -> FfmpegResolution {
    if !configured.trim().is_empty() && hls::ffmpeg_available(configured) {
        return FfmpegResolution {
            path: configured.to_string(),
            source: FfmpegSource::Configured,
        };
    }

    for (path, source) in candidate_paths() {
        if hls::ffmpeg_available(&path) {
            return FfmpegResolution { path, source };
        }
    }

    FfmpegResolution {
        path: configured.to_string(),
        source: FfmpegSource::Missing,
    }
}

pub async fn ensure_ffmpeg_path(configured: &str) -> Result<String, String> {
    let resolved = resolve_ffmpeg_path(configured);
    if resolved.source != FfmpegSource::Missing {
        return Ok(resolved.path);
    }

    #[cfg(target_os = "windows")]
    {
        download_ffmpeg_to_app_data().await
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(
            "FFmpeg não encontrado. Instale FFmpeg e adicione ao PATH do sistema.".to_string(),
        )
    }
}

fn candidate_paths() -> Vec<(String, FfmpegSource)> {
    let mut paths = Vec::new();

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let binaries = std::path::Path::new(&manifest).join("binaries");
        for name in ["ffmpeg-x86_64-pc-windows-msvc.exe", "ffmpeg.exe"] {
            let candidate = binaries.join(name);
            if candidate.is_file() {
                paths.push((
                    candidate.to_string_lossy().into_owned(),
                    FfmpegSource::Bundled,
                ));
            }
        }
    }

    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
    {
        for name in ["ffmpeg.exe", "ffmpeg-x86_64-pc-windows-msvc.exe"] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                paths.push((
                    candidate.to_string_lossy().into_owned(),
                    FfmpegSource::Bundled,
                ));
            }
        }
    }

    if let Some(app_bin) = app_data_ffmpeg_path() {
        paths.push((
            app_bin.to_string_lossy().into_owned(),
            FfmpegSource::AppData,
        ));
    }

    paths.push(("ffmpeg".to_string(), FfmpegSource::System));

    for path in windows_system_candidates() {
        paths.push((path, FfmpegSource::System));
    }

    paths
}

fn app_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join(crate::models::APP_DATA_DIR))
}

fn app_data_ffmpeg_path() -> Option<PathBuf> {
    app_data_dir().map(|dir| dir.join("bin").join("ffmpeg.exe"))
}

fn windows_system_candidates() -> Vec<String> {
    #[cfg(not(target_os = "windows"))]
    return Vec::new();

    #[cfg(target_os = "windows")]
    {
        let mut paths = Vec::new();

        if let Some(local) = dirs::data_local_dir() {
            paths.push(
                local
                    .join("Microsoft")
                    .join("WinGet")
                    .join("Links")
                    .join("ffmpeg.exe")
                    .to_string_lossy()
                    .into_owned(),
            );

            let winget_packages = local
                .join("Microsoft")
                .join("WinGet")
                .join("Packages");
            if let Ok(entries) = std::fs::read_dir(winget_packages) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if !name.contains("ffmpeg") {
                        continue;
                    }
                    let candidate = entry.path().join("ffmpeg.exe");
                    if candidate.is_file() {
                        paths.push(candidate.to_string_lossy().into_owned());
                    }
                    if let Ok(sub) = std::fs::read_dir(entry.path()) {
                        for sub_entry in sub.flatten() {
                            let bin = sub_entry.path().join("bin").join("ffmpeg.exe");
                            if bin.is_file() {
                                paths.push(bin.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }

        if let Ok(program_files) = std::env::var("ProgramFiles") {
            paths.push(
                Path::new(&program_files)
                    .join("ffmpeg")
                    .join("bin")
                    .join("ffmpeg.exe")
                    .to_string_lossy()
                    .into_owned(),
            );
        }

        paths
    }
}

#[cfg(target_os = "windows")]
async fn download_ffmpeg_to_app_data() -> Result<String, String> {
    let dest = app_data_ffmpeg_path().ok_or("Pasta de dados do app indisponível")?;
    if dest.is_file() && hls::ffmpeg_available(dest.to_str().unwrap_or("")) {
        return Ok(dest.to_string_lossy().into_owned());
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let tmp_zip = std::env::temp_dir().join("minha_princesa_ffmpeg.zip");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(FFMPEG_ZIP_URL)
        .send()
        .await
        .map_err(|e| format!("Falha ao baixar FFmpeg: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Download do FFmpeg rejeitado: {e}"))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Falha ao ler FFmpeg: {e}"))?;

    std::fs::write(&tmp_zip, &bytes).map_err(|e| e.to_string())?;

    let file = std::fs::File::open(&tmp_zip).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("ZIP inválido: {e}"))?;

    let mut extracted = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        if name.ends_with("/bin/ffmpeg.exe") {
            let mut out = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
            extracted = true;
            break;
        }
    }

    let _ = std::fs::remove_file(&tmp_zip);

    if !extracted {
        return Err("ffmpeg.exe não encontrado no pacote baixado".to_string());
    }

    let path = dest.to_string_lossy().into_owned();
    if !hls::ffmpeg_available(&path) {
        return Err("FFmpeg baixado mas não executável".to_string());
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_does_not_panic_on_empty() {
        let _ = resolve_ffmpeg_path("");
    }
}

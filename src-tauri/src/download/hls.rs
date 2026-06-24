use crate::download::process_util;
use crate::sushi::client::USER_AGENT_STR;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::timeout;

#[derive(Error, Debug)]
pub enum HlsError {
    #[error("FFmpeg failed: {0}")]
    Failed(String),
    #[error("Download cancelado")]
    Cancelled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn download_hls(
    ffmpeg_path: &str,
    stream_url: &str,
    output_path: &Path,
    referer: &str,
    origin: &str,
    stop_flag: Arc<AtomicBool>,
    child_slot: Arc<AsyncMutex<Option<Child>>>,
    on_progress: impl Fn(f64) + Send + Sync,
) -> Result<(), HlsError> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let headers = format!(
        "Referer: {referer}\r\nOrigin: {origin}\r\nUser-Agent: {USER_AGENT_STR}\r\n"
    );

    let mut cmd = Command::new(ffmpeg_path);
    cmd.args([
        "-y",
        "-loglevel",
        "warning",
        "-extension_picky",
        "0",
        "-headers",
        &headers,
        "-i",
        stream_url,
        "-c",
        "copy",
        "-bsf:a",
        "aac_adtstoasc",
        output_path.to_str().unwrap_or("output.mp4"),
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(process_util::hide_console());
    let mut child = cmd.spawn()?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| HlsError::Failed("sem stderr do FFmpeg".to_string()))?;

    *child_slot.lock().await = Some(child);

    let mut reader = BufReader::new(stderr).lines();
    let mut exit_status = None;

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            kill_child(&child_slot).await;
            return Err(HlsError::Cancelled);
        }

        match timeout(Duration::from_millis(250), reader.next_line()).await {
            Ok(Ok(Some(line))) => {
                if line.contains("time=") {
                    on_progress(parse_ffmpeg_time(&line));
                }
            }
            Ok(Ok(None)) => break,
            Ok(Err(e)) => {
                kill_child(&child_slot).await;
                return Err(HlsError::Io(e));
            }
            Err(_) => {
                if let Some(mut c) = child_slot.lock().await.take() {
                    match c.try_wait() {
                        Ok(Some(status)) => {
                            exit_status = Some(status);
                            break;
                        }
                        Ok(None) => {
                            *child_slot.lock().await = Some(c);
                        }
                        Err(e) => return Err(HlsError::Io(e)),
                    }
                } else {
                    break;
                }
            }
        }
    }

    if exit_status.is_none() {
        if let Some(mut c) = child_slot.lock().await.take() {
            exit_status = Some(c.wait().await?);
        }
    }

    *child_slot.lock().await = None;

    if stop_flag.load(Ordering::Relaxed) {
        return Err(HlsError::Cancelled);
    }

    match exit_status {
        Some(status) if status.success() => {
            on_progress(100.0);
            Ok(())
        }
        Some(status) => Err(HlsError::Failed(format!("exit code {:?}", status.code()))),
        None => Err(HlsError::Failed("FFmpeg terminou sem status".to_string())),
    }
}

async fn kill_child(child_slot: &Arc<AsyncMutex<Option<Child>>>) {
    if let Some(mut child) = child_slot.lock().await.take() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

fn parse_ffmpeg_time(line: &str) -> f64 {
    if let Some(idx) = line.find("time=") {
        let time_str = &line[idx + 5..];
        let end = time_str.find(' ').unwrap_or(time_str.len());
        let parts: Vec<&str> = time_str[..end].split(':').collect();
        if parts.len() == 3 {
            let h: f64 = parts[0].parse().unwrap_or(0.0);
            let m: f64 = parts[1].parse().unwrap_or(0.0);
            let s: f64 = parts[2].parse().unwrap_or(0.0);
            let total_secs = h * 3600.0 + m * 60.0 + s;
            return (total_secs / 1440.0 * 100.0).min(95.0);
        }
    }
    50.0
}

pub fn ffmpeg_available(path: &str) -> bool {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;

    let mut cmd = std::process::Command::new(path);
    cmd.arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(process_util::hide_console());
    cmd.status()
        .map(|s| s.success())
        .unwrap_or(false)
}

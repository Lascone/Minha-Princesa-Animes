use crate::download::process_util;
use crate::sushi::client::USER_AGENT_STR;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{interval, timeout, MissedTickBehavior};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(90);
const LINE_READ_TIMEOUT: Duration = Duration::from_millis(500);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);
const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(30);
const STALL_TIMEOUT: Duration = Duration::from_secs(60);
const STALL_WARN_AFTER: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy)]
pub enum HlsEvent {
    /// Atualiza só o texto de status (progresso permanece baixo).
    Connecting(u64),
    /// Progresso real do FFmpeg (3–95%).
    Progress(f64),
    /// Aviso antes de abortar: sem bytes nem time= há N segundos.
    StallWarning(u64),
    /// Arquivo crescendo mas FFmpeg ainda não reportou time=.
    BytesTransferred(u64),
}

#[derive(Error, Debug)]
pub enum HlsError {
    #[error("FFmpeg failed: {0}")]
    Failed(String),
    #[error("Download cancelado")]
    Cancelled,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn verify_stream_reachable(
    stream_url: &str,
    referer: &str,
    origin: &str,
) -> Result<(), HlsError> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STR));
    headers.insert(
        REFERER,
        HeaderValue::from_str(referer).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    headers.insert(
        ORIGIN,
        HeaderValue::from_str(origin).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    let response = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(PREFLIGHT_TIMEOUT)
        .build()
        .map_err(|e| HlsError::Failed(e.to_string()))?
        .get(stream_url)
        .send()
        .await
        .map_err(|e| HlsError::Failed(format!("Stream inacessível: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        return Err(HlsError::Failed(format!(
            "Stream retornou HTTP {} (referer: {referer})",
            status.as_u16()
        )));
    }

    if stream_url.contains(".m3u8") || stream_url.contains(".txt") {
        let body = response
            .text()
            .await
            .map_err(|e| HlsError::Failed(e.to_string()))?;
        if !body.contains("#EXTM3U") && !body.contains("#EXT-X-") {
            return Err(HlsError::Failed(
                "Resposta do stream não é um manifesto HLS válido".to_string(),
            ));
        }
    }

    Ok(())
}

pub async fn download_hls(
    ffmpeg_path: &str,
    stream_url: &str,
    output_path: &Path,
    referer: &str,
    origin: &str,
    stop_flag: Arc<AtomicBool>,
    child_slot: Arc<AsyncMutex<Option<Child>>>,
    on_event: impl Fn(HlsEvent) + Send + Sync,
) -> Result<(), HlsError> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    verify_stream_reachable(stream_url, referer, origin).await?;

    let headers = format!(
        "Referer: {referer}\r\nOrigin: {origin}\r\nUser-Agent: {USER_AGENT_STR}\r\n"
    );

    let mut cmd = Command::new(ffmpeg_path);
    cmd.args([
        "-y",
        "-hide_banner",
        "-nostdin",
        "-stats_period",
        "0.5",
        "-rw_timeout",
        "30000000",
        "-timeout",
        "30000000",
        "-reconnect",
        "1",
        "-reconnect_streamed",
        "1",
        "-reconnect_delay_max",
        "5",
        "-extension_picky",
        "0",
        "-user_agent",
        USER_AGENT_STR,
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
    let started = Instant::now();
    let mut last_activity = Instant::now();
    let mut stderr_tail = String::new();
    let mut duration_secs: Option<f64> = None;
    let mut stream_started = false;
    let mut last_file_bytes: u64 = 0;
    let mut last_progress_pct: f64 = 0.0;
    let mut heartbeat = interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            kill_child(&child_slot).await;
            return Err(HlsError::Cancelled);
        }

        if !stream_started && started.elapsed() > CONNECT_TIMEOUT {
            kill_child(&child_slot).await;
            return Err(HlsError::Failed(format!(
                "Timeout ao conectar ao stream (90s). Última saída: {}",
                tail_lines(&stderr_tail, 3)
            )));
        }

        tokio::select! {
            _ = heartbeat.tick() => {
                let file_bytes = output_file_bytes(output_path);
                if file_bytes > last_file_bytes {
                    last_file_bytes = file_bytes;
                    last_activity = Instant::now();
                    if stream_started {
                        on_event(HlsEvent::BytesTransferred(file_bytes));
                    }
                }

                if !stream_started {
                    on_event(HlsEvent::Connecting(started.elapsed().as_secs()));
                } else {
                    let stalled = last_activity.elapsed();
                    if stalled >= STALL_WARN_AFTER && stalled < STALL_TIMEOUT {
                        on_event(HlsEvent::StallWarning(stalled.as_secs()));
                    }
                    if stalled >= STALL_TIMEOUT {
                        kill_child(&child_slot).await;
                        return Err(HlsError::Failed(format!(
                            "Download travado (sem progresso há {}s). Última saída: {}",
                            STALL_TIMEOUT.as_secs(),
                            tail_lines(&stderr_tail, 3)
                        )));
                    }
                }
            }
            line_result = timeout(LINE_READ_TIMEOUT, reader.next_line()) => {
                match line_result {
                    Ok(Ok(Some(line))) => {
                        append_stderr_tail(&mut stderr_tail, &line);
                        last_activity = Instant::now();
                        if is_ffmpeg_fatal(&line) {
                            kill_child(&child_slot).await;
                            return Err(HlsError::Failed(line));
                        }
                        if let Some(d) = parse_ffmpeg_duration(&line) {
                            duration_secs = Some(d);
                            stream_started = true;
                        }
                        if line.contains("time=") {
                            stream_started = true;
                            if let Some(progress) = parse_ffmpeg_time(&line, duration_secs) {
                                last_progress_pct = progress;
                                on_event(HlsEvent::Progress(progress));
                            }
                        }
                    }
                    Ok(Ok(None)) => break,
                    Ok(Err(e)) => {
                        kill_child(&child_slot).await;
                        return Err(HlsError::Io(e));
                    }
                    Err(_) => {
                        if last_activity.elapsed() > CONNECT_TIMEOUT && !stream_started {
                            kill_child(&child_slot).await;
                            return Err(HlsError::Failed(format!(
                                "Sem resposta do FFmpeg (90s). Última saída: {}",
                                tail_lines(&stderr_tail, 3)
                            )));
                        }
                        if stream_started && last_activity.elapsed() >= STALL_TIMEOUT {
                            kill_child(&child_slot).await;
                            return Err(HlsError::Failed(format!(
                                "Download travado (sem progresso há {}s, {:.0}%). Última saída: {}",
                                STALL_TIMEOUT.as_secs(),
                                last_progress_pct,
                                tail_lines(&stderr_tail, 3)
                            )));
                        }
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
            on_event(HlsEvent::Progress(100.0));
            Ok(())
        }
        Some(status) => Err(HlsError::Failed(format!(
            "FFmpeg exit {:?}. {}",
            status.code(),
            tail_lines(&stderr_tail, 3)
        ))),
        None => Err(HlsError::Failed(format!(
            "FFmpeg terminou sem status. {}",
            tail_lines(&stderr_tail, 3)
        ))),
    }
}

fn output_file_bytes(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn is_ffmpeg_fatal(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("http error")
        || lower.contains("403 forbidden")
        || lower.contains("404 not found")
        || lower.contains("invalid data found")
        || lower.contains("error opening input")
        || lower.contains("no such file or directory")
        || lower.contains("server returned")
}

fn append_stderr_tail(buf: &mut String, line: &str) {
    buf.push_str(line);
    buf.push('\n');
    if buf.len() > 4_096 {
        let drain = buf.len().saturating_sub(4_096);
        buf.drain(..drain);
    }
}

fn tail_lines(text: &str, count: usize) -> String {
    text.lines()
        .rev()
        .take(count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ")
}

async fn kill_child(child_slot: &Arc<AsyncMutex<Option<Child>>>) {
    if let Some(mut child) = child_slot.lock().await.take() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

fn parse_hms_timestamp(value: &str) -> Option<f64> {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let s: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

fn parse_ffmpeg_duration(line: &str) -> Option<f64> {
    let idx = line.find("Duration:")?;
    let rest = line[idx + 9..].trim();
    let end = rest.find(',').unwrap_or(rest.len());
    parse_hms_timestamp(rest[..end].trim())
}

fn parse_ffmpeg_time(line: &str, duration_secs: Option<f64>) -> Option<f64> {
    let idx = line.find("time=")?;
    let time_str = &line[idx + 5..];
    let end = time_str.find(' ').unwrap_or(time_str.len());
    let total_secs = parse_hms_timestamp(time_str[..end].trim())?;
    let episode_len = duration_secs.unwrap_or(1440.0).max(1.0);
    Some((total_secs / episode_len * 100.0).min(95.0))
}

pub fn ffmpeg_available(path: &str) -> bool {
    if path.trim().is_empty() {
        return false;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_time_returns_none_without_timestamp() {
        assert!(parse_ffmpeg_time("frame=  100 fps= 25", None).is_none());
    }

    #[test]
    fn parse_time_computes_percent() {
        let line = "size=    1024kB time=00:06:00.00 bitrate= 500.0kbits/s";
        let pct = parse_ffmpeg_time(line, Some(1200.0)).unwrap();
        assert!((pct - 30.0).abs() < 0.5);
    }
}

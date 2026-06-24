use crate::sources::goyabu::client::GoyabuClient;
use crate::sources::{SourceError, StreamKind, StreamResolution};
use regex::Regex;
use serde::Deserialize;

pub async fn resolve_stream(episode_url: &str) -> Result<StreamResolution, SourceError> {
    let client = GoyabuClient::new()?;
    let normalized = GoyabuClient::normalize_url(episode_url);
    let html = client.get(&normalized).await?;

    let token = extract_blogger_token(&html)?;
    let response = client
        .post_form(&[("action", "decode_blogger_video"), ("token", &token)])
        .await?;

    let decoded: AjaxResponse = serde_json::from_str(&response)
        .map_err(|e| SourceError::Parse(format!("Resposta do player inválida: {e}")))?;

    if !decoded.success {
        let msg = decoded
            .data
            .and_then(|d| d.message)
            .unwrap_or_else(|| "Erro ao decodificar vídeo".to_string());
        return Err(SourceError::Parse(msg));
    }

    let data = decoded
        .data
        .ok_or_else(|| SourceError::Parse("Resposta vazia do player".to_string()))?;

    let play = data.play.unwrap_or_default();
    if play.is_empty() {
        return Err(SourceError::NoStream);
    }

    let best = play
        .iter()
        .max_by_key(|item| item.size.unwrap_or(0))
        .ok_or(SourceError::NoStream)?;

    let url = best
        .src
        .clone()
        .filter(|s| s.starts_with("http"))
        .ok_or(SourceError::NoStream)?;

    let kind = if url.contains(".m3u8") || url.contains(".txt") {
        StreamKind::Hls
    } else {
        StreamKind::Mp4
    };

    Ok(StreamResolution {
        url,
        kind,
        referer: normalized,
        origin: GoyabuClient::base_url().to_string(),
    })
}

fn extract_blogger_token(html: &str) -> Result<String, SourceError> {
    if let Some(re) = Regex::new(r#""blogger_token"\s*:\s*"([^"]+)""#).ok() {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                return Ok(m.as_str().to_string());
            }
        }
    }
    Err(SourceError::Parse(
        "Token do player não encontrado na página".to_string(),
    ))
}

#[derive(Deserialize)]
struct AjaxResponse {
    success: bool,
    data: Option<AjaxData>,
}

#[derive(Deserialize)]
struct AjaxData {
    message: Option<String>,
    play: Option<Vec<PlayItem>>,
}

#[derive(Deserialize)]
struct PlayItem {
    size: Option<u32>,
    src: Option<String>,
}

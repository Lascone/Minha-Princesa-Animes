use crate::sushi::client::{SushiClient, SushiError};
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Hls,
    Mp4,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub url: String,
    pub kind: StreamKind,
}

impl StreamInfo {
    #[allow(dead_code)]
    pub fn is_hls(&self) -> bool {
        self.kind == StreamKind::Hls
    }
}

pub async fn resolve_stream_url(client: &SushiClient, embed_id: &str) -> Result<StreamInfo, SushiError> {
    let html = client
        .post_form("/ajax/embed", &[("id", embed_id)])
        .await?;

    parse_stream_from_embed(&html)
}

pub fn parse_stream_from_embed(html: &str) -> Result<StreamInfo, SushiError> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let document = Html::parse_fragment(html);
    let source_sel = Selector::parse("source").unwrap();
    for source in document.select(&source_sel) {
        if let Some(src) = source.value().attr("src") {
            push_candidate(&mut candidates, &mut seen, src);
        }
    }

    let patterns = [
        r#"file:\s*["']([^"']+)["']"#,
        r#"src:\s*["']([^"']+)["']"#,
        r#""file"\s*:\s*"([^"]+)""#,
        r#""src"\s*:\s*"([^"]+)""#,
        r#""sources"\s*:\s*\[\s*\{\s*"file"\s*:\s*"([^"]+)""#,
        r#""playlist"\s*:\s*"([^"]+)""#,
        r#"(https?://[^\s"'<>]+\.m3u8[^\s"'<>]*)"#,
        r#"(https?://[^\s"'<>]+\.txt[^\s"'<>]*)"#,
        r#"(https?://[^\s"'<>]+\.mp4[^\s"'<>]*)"#,
        r#"(https?://[^\s"'<>]+\.ts(?:\?[^\s"'<>]*)?)"#,
    ];

    for pattern in patterns {
        let Ok(re) = Regex::new(pattern) else {
            continue;
        };
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1).or_else(|| caps.get(0)) {
                push_candidate(&mut candidates, &mut seen, m.as_str());
            }
        }
    }

    pick_best_stream(&candidates).ok_or(SushiError::NoStream)
}

fn push_candidate(candidates: &mut Vec<String>, seen: &mut HashSet<String>, url: &str) {
    let url = url.trim();
    if url.is_empty() || !url.starts_with("http") {
        return;
    }
    let lower = url.to_lowercase();
    if is_junk_media_url(&lower) {
        return;
    }
    if seen.insert(url.to_string()) {
        candidates.push(url.to_string());
    }
}

fn is_junk_media_url(lower: &str) -> bool {
    lower.contains(".js")
        || lower.contains(".css")
        || lower.contains(".png")
        || lower.contains(".jpg")
        || lower.contains(".webp")
        || lower.contains(".svg")
        || lower.contains("jwplayer")
        || lower.contains("jwpcdn.com")
        || lower.contains("notice.txt")
        || lower.contains("/player/v/")
        || lower.contains("cloudflare-static")
        || lower.contains("googletagmanager")
        || lower.contains("google-analytics")
}

fn stream_priority(url: &str) -> u8 {
    let lower = url.to_lowercase();
    let is_hls = lower.contains(".m3u8")
        || lower.contains("videohls")
        || (lower.contains(".txt") && (lower.contains("playlist") || lower.contains("/hls")));
    let is_direct_mp4 = lower.contains(".mp4") && !is_hls && !lower.contains("/index.");

    if is_direct_mp4 {
        0
    } else if lower.contains(".m3u8") {
        1
    } else if lower.contains(".txt") {
        2
    } else if lower.contains(".mp4") {
        3
    } else if lower.contains(".ts") {
        4
    } else {
        99
    }
}

fn classify_url(url: &str) -> Option<StreamInfo> {
    let lower = url.to_lowercase();
    if lower.contains(".m3u8") || lower.contains(".txt") || lower.contains(".ts") {
        Some(StreamInfo {
            url: url.to_string(),
            kind: StreamKind::Hls,
        })
    } else if lower.contains(".mp4") {
        Some(StreamInfo {
            url: url.to_string(),
            kind: StreamKind::Mp4,
        })
    } else {
        None
    }
}

fn pick_best_stream(candidates: &[String]) -> Option<StreamInfo> {
    let mut sorted: Vec<&String> = candidates.iter().collect();
    sorted.sort_by_key(|url| stream_priority(url));

    for url in sorted {
        if let Some(info) = classify_url(url) {
            return Some(info);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_direct_mp4_over_m3u8() {
        let html = r#"
            file: "https://cdn.example.com/video.mp4"
            <source src="https://cdn.example.com/master.m3u8">
        "#;
        let info = parse_stream_from_embed(html).unwrap();
        assert!(info.url.contains(".mp4"));
        assert_eq!(info.kind, StreamKind::Mp4);
    }

    #[test]
    fn ignores_jwplayer_notice_txt() {
        let html = r#"
            https://ssl.p.jwpcdn.com/player/v/8.23.1/notice.txt
            file: "https://cdn.example.com/ep/master.m3u8"
        "#;
        let info = parse_stream_from_embed(html).unwrap();
        assert!(info.url.contains("master.m3u8"));
    }

    #[test]
    fn txt_playlist_is_hls() {
        let html = r#"<source src="https://cdn.example.com/playlist.txt">"#;
        let info = parse_stream_from_embed(html).unwrap();
        assert_eq!(info.kind, StreamKind::Hls);
    }
}

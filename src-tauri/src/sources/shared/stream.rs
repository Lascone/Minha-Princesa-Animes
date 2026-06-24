use crate::sources::{SourceError, StreamKind};
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ParsedStream {
    pub url: String,
    pub kind: StreamKind,
}

pub fn parse_stream_from_html(html: &str) -> Result<ParsedStream, SourceError> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let document = Html::parse_fragment(html);
    if let Ok(source_sel) = Selector::parse("source") {
        for source in document.select(&source_sel) {
            if let Some(src) = source.value().attr("src") {
                push_candidate(&mut candidates, &mut seen, src);
            }
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

    pick_best_stream(&candidates).ok_or(SourceError::NoStream)
}

pub fn extract_iframe_srcs(html: &str) -> Vec<String> {
    let mut urls = Vec::new();
    if let Ok(re) = Regex::new(r#"<iframe[^>]+src="([^"]+)""#) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                let url = m.as_str().trim();
                if url.starts_with("http") {
                    urls.push(url.to_string());
                }
            }
        }
    }
    urls
}

pub fn extract_m3u8_from_query(url: &str) -> Option<String> {
    if let Ok(re) = Regex::new(r"[?&]d=([^&]+)") {
        if let Some(caps) = re.captures(url) {
            if let Some(m) = caps.get(1) {
                let decoded = urlencoding::decode(m.as_str()).ok()?.into_owned();
                if decoded.contains(".m3u8") || decoded.contains(".mp4") {
                    return Some(decoded);
                }
            }
        }
    }
    None
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
    if lower.contains(".m3u8") {
        0
    } else if lower.contains(".txt") {
        1
    } else if lower.contains(".mp4") {
        2
    } else if lower.contains(".ts") {
        3
    } else {
        99
    }
}

fn classify_url(url: &str) -> Option<ParsedStream> {
    let lower = url.to_lowercase();
    if lower.contains(".m3u8") || lower.contains(".txt") || lower.contains(".ts") {
        Some(ParsedStream {
            url: url.to_string(),
            kind: StreamKind::Hls,
        })
    } else if lower.contains(".mp4") {
        Some(ParsedStream {
            url: url.to_string(),
            kind: StreamKind::Mp4,
        })
    } else {
        None
    }
}

pub fn parse_blogger_video_config(html: &str) -> Option<ParsedStream> {
    let json_start = html.find("var VIDEO_CONFIG = ")?;
    let json_text = &html[json_start + 19..];
    let json_text = json_text.trim_start();
    let end = json_text.find("</script>").unwrap_or(json_text.len());
    let json_text = json_text[..end].trim().trim_end_matches(';');
    let data: serde_json::Value = serde_json::from_str(json_text).ok()?;
    let streams = data.get("streams")?.as_array()?;
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for stream in streams {
        if let Some(play_url) = stream.get("play_url").and_then(|v| v.as_str()) {
            push_candidate(&mut candidates, &mut seen, play_url);
        }
    }
    pick_best_stream(&candidates)
}

pub fn stream_from_url(url: &str) -> Option<ParsedStream> {
    classify_url(url.trim())
}

fn pick_best_stream(candidates: &[String]) -> Option<ParsedStream> {
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
    fn prefers_m3u8_over_mp4() {
        let html = r#"
            file: "https://cdn.example.com/video.mp4"
            <source src="https://cdn.example.com/master.m3u8">
        "#;
        let info = parse_stream_from_html(html).unwrap();
        assert!(info.url.contains(".m3u8"));
        assert_eq!(info.kind, StreamKind::Hls);
    }

    #[test]
    fn extract_m3u8_param() {
        let url = "https://api.anivideo.net/videohls.php?d=https://cdn.example.com/stream/index.m3u8";
        let extracted = extract_m3u8_from_query(url).unwrap();
        assert!(extracted.contains(".m3u8"));
    }
}

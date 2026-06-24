use crate::sources::animesdigital::client::AnimesdigitalClient;
use crate::sources::shared::stream::{
    extract_iframe_srcs, extract_m3u8_from_query, parse_stream_from_html, stream_from_url,
};
use crate::sources::{SourceError, StreamKind, StreamResolution};
use regex::Regex;

pub async fn resolve_stream(episode_url: &str) -> Result<StreamResolution, SourceError> {
    let client = AnimesdigitalClient::new()?;
    let normalized = AnimesdigitalClient::normalize_url(episode_url);
    let html = client.get(&normalized).await?;

    let iframe_urls = extract_player_iframes(&html);
    let mut candidates = Vec::new();

    for iframe_url in iframe_urls {
        if let Ok(stream) = resolve_iframe(&client, &iframe_url, &normalized).await {
            candidates.push(stream);
        }
    }

    if candidates.is_empty() {
        if let Ok(parsed) = parse_stream_from_html(&html) {
            return Ok(StreamResolution {
                url: parsed.url,
                kind: parsed.kind,
                referer: normalized.clone(),
                origin: AnimesdigitalClient::base_url().to_string(),
            });
        }
        return Err(SourceError::NoStream);
    }

    candidates
        .into_iter()
        .min_by_key(|s| stream_priority(&s.url, &s.kind))
        .ok_or(SourceError::NoStream)
}

fn extract_player_iframes(html: &str) -> Vec<String> {
    let mut ordered = Vec::new();
    if let Ok(re) = Regex::new(
        r#"(?s)<div[^>]*class="[^"]*tab-video[^"]*"[^>]*data-video="(\d+)"[^>]*>.*?<iframe[^>]+src="([^"]+)""#,
    ) {
        let mut tabs: Vec<(u32, String)> = re
            .captures_iter(html)
            .filter_map(|caps| {
                let idx = caps.get(1)?.as_str().parse().ok()?;
                let url = caps.get(2)?.as_str().trim().to_string();
                url.starts_with("http").then_some((idx, url))
            })
            .collect();
        tabs.sort_by_key(|(idx, _)| *idx);
        ordered.extend(tabs.into_iter().map(|(_, url)| url));
    }

    if ordered.is_empty() {
        ordered = extract_iframe_srcs(html);
    } else {
        for url in extract_iframe_srcs(html) {
            if !ordered.iter().any(|u| u == &url) {
                ordered.push(url);
            }
        }
    }

    ordered
}

async fn resolve_iframe(
    client: &AnimesdigitalClient,
    iframe_url: &str,
    episode_url: &str,
) -> Result<StreamResolution, SourceError> {
    if let Some(direct) = extract_m3u8_from_query(iframe_url) {
        let (referer, origin) = stream_headers_for(iframe_url, episode_url);
        let kind = if direct.contains(".m3u8") || direct.contains(".txt") {
            StreamKind::Hls
        } else {
            StreamKind::Mp4
        };
        return Ok(StreamResolution {
            url: direct,
            kind,
            referer,
            origin,
        });
    }

    if iframe_url.contains("anivideo.net") || iframe_url.contains("videohls") {
        let page = client.fetch_url(iframe_url, Some(episode_url)).await?;
        let parsed = parse_stream_from_html(&page)?;
        let (referer, origin) = stream_headers_for(iframe_url, episode_url);
        return Ok(StreamResolution {
            url: parsed.url,
            kind: parsed.kind,
            referer,
            origin,
        });
    }

    if iframe_url.contains("animesdigital.org") || iframe_url.contains("animesdigital.") {
        let page = client.fetch_url(iframe_url, Some(episode_url)).await?;
        if let Ok(parsed) = parse_stream_from_html(&page) {
            let (referer, origin) = stream_headers_for(iframe_url, episode_url);
            return Ok(StreamResolution {
                url: parsed.url,
                kind: parsed.kind,
                referer,
                origin,
            });
        }
        if page.contains("blogger.com/video.g") {
            if let Some(blogger_url) = extract_blogger_url(&page) {
                return resolve_blogger_fallback(&blogger_url, episode_url).await;
            }
        }
    }

    if iframe_url.contains("blogger.com/video.g") {
        return resolve_blogger_fallback(iframe_url, episode_url).await;
    }

    if let Some(parsed) = stream_from_url(iframe_url) {
        let (referer, origin) = stream_headers_for(iframe_url, episode_url);
        return Ok(StreamResolution {
            url: parsed.url,
            kind: parsed.kind,
            referer,
            origin,
        });
    }

    Err(SourceError::NoStream)
}

async fn resolve_blogger_fallback(
    blogger_url: &str,
    episode_url: &str,
) -> Result<StreamResolution, SourceError> {
    use crate::sources::dooplay::DooplayClient;
    use crate::sources::dooplay::resolve_blogger_stream;

    let proxy = DooplayClient::new(crate::sources::dooplay::MEUSANIMES_CONFIG)?;
    let parsed = resolve_blogger_stream(&proxy, blogger_url, episode_url).await?;
    let (referer, origin) = stream_headers_for(blogger_url, episode_url);
    Ok(StreamResolution {
        url: parsed.url,
        kind: parsed.kind,
        referer,
        origin,
    })
}

fn extract_blogger_url(html: &str) -> Option<String> {
    if let Ok(re) = Regex::new(r#"https?://(?:www\.)?blogger\.com/video\.g\?token=[^"'\s<>]+"#) {
        return re
            .find(html)
            .map(|m| m.as_str().to_string());
    }
    None
}

fn stream_priority(url: &str, kind: &StreamKind) -> u8 {
    let lower = url.to_lowercase();
    if lower.contains(".m3u8") || matches!(kind, StreamKind::Hls) {
        0
    } else if lower.contains(".mp4") {
        1
    } else if lower.contains("googlevideo") {
        2
    } else {
        99
    }
}

fn stream_headers_for(iframe_url: &str, episode_url: &str) -> (String, String) {
    let lower = iframe_url.to_lowercase();
    if lower.contains("anivideo.net") || lower.contains("videohls") {
        let referer = iframe_url.split('#').next().unwrap_or(iframe_url).to_string();
        return (referer, "https://anivideo.net".to_string());
    }
    (
        episode_url.to_string(),
        AnimesdigitalClient::base_url().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_tab_video_iframes_by_data_video() {
        let html = r#"
            <div id="player2" class="tab-video" data-video="2">
              <iframe src="https://cdn.example.com/player2.m3u8"></iframe>
            </div>
            <div id="player1" class="tab-video" data-video="1">
              <iframe src="https://api.anivideo.net/videohls.php?d=https://cdn.example.com/ep.m3u8"></iframe>
            </div>
        "#;
        let urls = extract_player_iframes(html);
        assert!(urls[0].contains("anivideo"));
        assert!(urls[1].contains("player2"));
    }

    #[tokio::test]
    #[ignore = "rede: cargo test resolve_animesdigital_stream -- --ignored --nocapture"]
    async fn resolve_animesdigital_stream() {
        let stream = resolve_stream("https://animesdigital.org/video/a/136491/")
            .await
            .expect("stream");
        assert!(
            stream.url.contains(".m3u8") || stream.url.contains(".mp4"),
            "unexpected url: {}",
            stream.url
        );
    }

    #[tokio::test]
    #[ignore = "rede: cargo test resolve_animesdigital_megami_ep12 -- --ignored --nocapture"]
    async fn resolve_animesdigital_megami_ep12() {
        let stream = resolve_stream("https://animesdigital.org/video/a/136716/")
            .await
            .expect("stream");
        eprintln!("megami ep12: {} {:?}", stream.url, stream.kind);
        assert!(
            stream.url.contains(".m3u8")
                || stream.url.contains(".mp4")
                || stream.url.contains("googlevideo.com"),
            "unexpected url: {}",
            stream.url
        );
    }
}

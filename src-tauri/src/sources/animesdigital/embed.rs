use crate::sources::animesdigital::client::AnimesdigitalClient;
use crate::sources::shared::stream::{extract_iframe_srcs, extract_m3u8_from_query, parse_stream_from_html};
use crate::sources::{SourceError, StreamKind, StreamResolution};

pub async fn resolve_stream(episode_url: &str) -> Result<StreamResolution, SourceError> {
    let client = AnimesdigitalClient::new()?;
    let normalized = AnimesdigitalClient::normalize_url(episode_url);
    let html = client.get(&normalized).await?;

    for iframe_url in extract_iframe_srcs(&html) {
        if let Some(direct) = extract_m3u8_from_query(&iframe_url) {
            let (referer, origin) = stream_headers_for(&iframe_url, &normalized);
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
            let page = client
                .fetch_url(&iframe_url, Some(&normalized))
                .await?;
            if let Ok(parsed) = parse_stream_from_html(&page) {
                let (referer, origin) = stream_headers_for(&iframe_url, &normalized);
                return Ok(StreamResolution {
                    url: parsed.url,
                    kind: parsed.kind,
                    referer,
                    origin,
                });
            }
        }
    }

    if let Ok(parsed) = parse_stream_from_html(&html) {
        return Ok(StreamResolution {
            url: parsed.url,
            kind: parsed.kind,
            referer: normalized.clone(),
            origin: AnimesdigitalClient::base_url().to_string(),
        });
    }

    Err(SourceError::NoStream)
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
}

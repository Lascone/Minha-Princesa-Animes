use crate::sources::dooplay::client::DooplayClient;
use crate::sources::dooplay::config::DooplaySiteConfig;
use crate::sources::dooplay::resolve_blogger_stream;
use crate::sources::shared::stream::{
    extract_iframe_srcs, extract_m3u8_from_query, parse_stream_from_html, stream_from_url,
};
use crate::sources::{SourceError, StreamKind, StreamResolution};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

pub async fn resolve_stream(
    config: DooplaySiteConfig,
    episode_url: &str,
) -> Result<StreamResolution, SourceError> {
    let client = DooplayClient::new(config)?;
    let normalized = DooplayClient::normalize_url(config, episode_url);
    let html = client.get(&normalized).await?;

    let iframes = extract_iframe_srcs(&html);
    for iframe_url in &iframes {
        if let Some(direct) = extract_m3u8_from_query(iframe_url) {
            let (referer, origin) = iframe_stream_headers(iframe_url, &normalized, config.base_url);
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

        if iframe_url.contains("blogger.com/video.g") {
            if let Ok(stream) = resolve_blogger_iframe(&client, iframe_url, &normalized, config).await
            {
                return Ok(stream);
            }
        }

        if iframe_url.contains("meusdoramas.club") || iframe_url.contains("doramas") {
            if let Ok(stream) =
                resolve_meusdoramas_iframe(&client, iframe_url, &normalized, config).await
            {
                return Ok(stream);
            }
        }
    }

    if let Ok(parsed) = parse_stream_from_html(&html) {
        return Ok(StreamResolution {
            url: parsed.url,
            kind: parsed.kind,
            referer: normalized,
            origin: config.base_url.to_string(),
        });
    }

    Err(SourceError::NoStream)
}

async fn resolve_blogger_iframe(
    client: &DooplayClient,
    iframe_url: &str,
    referer: &str,
    config: DooplaySiteConfig,
) -> Result<StreamResolution, SourceError> {
    let parsed = resolve_blogger_stream(client, iframe_url, referer).await?;
    let (stream_referer, origin) = iframe_stream_headers(iframe_url, referer, config.base_url);
    Ok(StreamResolution {
        url: parsed.url,
        kind: parsed.kind,
        referer: stream_referer,
        origin,
    })
}

#[derive(Debug, Deserialize)]
struct MeusdoramasVideoResponse {
    success: bool,
    #[serde(rename = "videoUrl")]
    video_url: Value,
}

#[derive(Debug, Deserialize)]
struct MeusdoramasSource {
    file: String,
}

fn parse_meusdoramas_route(iframe_url: &str) -> Option<(String, String, String, String)> {
    let hash = iframe_url.split('#').nth(1)?;
    let re = Regex::new(r"/video/(\d+)/(\d+)/(\d+)").ok()?;
    let caps = re.captures(hash)?;
    let api_base = iframe_url.split('#').next()?.trim_end_matches('/').to_string();
    Some((
        api_base,
        caps.get(1)?.as_str().to_string(),
        caps.get(2)?.as_str().to_string(),
        caps.get(3)?.as_str().to_string(),
    ))
}

async fn resolve_meusdoramas_iframe(
    client: &DooplayClient,
    iframe_url: &str,
    referer: &str,
    config: DooplaySiteConfig,
) -> Result<StreamResolution, SourceError> {
    let (api_base, tmdb, season, episode) =
        parse_meusdoramas_route(iframe_url).ok_or(SourceError::NoStream)?;
    let api_url = format!(
        "{api_base}/posts/get-video.php?episode_number={episode}&season_number={season}&tmdb={tmdb}"
    );
    let body = client.fetch_url(&api_url, Some(referer)).await?;
    let data: MeusdoramasVideoResponse =
        serde_json::from_str(&body).map_err(|e| SourceError::Parse(e.to_string()))?;
    if !data.success {
        return Err(SourceError::NoStream);
    }

    let (stream_referer, origin) = iframe_stream_headers(iframe_url, referer, config.base_url);
    match data.video_url {
        Value::String(url) => {
            resolve_video_target(client, &url, referer, &stream_referer, &origin, config).await
        }
        Value::Array(items) => {
            let sources: Vec<MeusdoramasSource> = serde_json::from_value(Value::Array(items))
                .map_err(|e| SourceError::Parse(e.to_string()))?;
            let mut candidates: Vec<String> = sources.into_iter().map(|s| s.file).collect();
            candidates.sort_by_key(|url| stream_priority(url));
            let best = candidates
                .iter()
                .find_map(|url| stream_from_url(url))
                .ok_or(SourceError::NoStream)?;
            Ok(StreamResolution {
                url: best.url,
                kind: best.kind,
                referer: stream_referer,
                origin,
            })
        }
        _ => Err(SourceError::NoStream),
    }
}

async fn resolve_video_target(
    client: &DooplayClient,
    url: &str,
    episode_referer: &str,
    iframe_referer: &str,
    origin: &str,
    config: DooplaySiteConfig,
) -> Result<StreamResolution, SourceError> {
    if url.contains("blogger.com/video.g") {
        return resolve_blogger_iframe(client, url, episode_referer, config).await;
    }

    if let Some(parsed) = stream_from_url(url) {
        return Ok(StreamResolution {
            url: parsed.url,
            kind: parsed.kind,
            referer: iframe_referer.to_string(),
            origin: origin.to_string(),
        });
    }

    Err(SourceError::NoStream)
}

fn stream_priority(url: &str) -> u8 {
    let lower = url.to_lowercase();
    if lower.contains(".m3u8") {
        0
    } else if lower.contains(".txt") {
        1
    } else if lower.contains(".mp4") {
        2
    } else {
        99
    }
}

fn iframe_stream_headers(
    iframe_url: &str,
    episode_referer: &str,
    site_base_url: &str,
) -> (String, String) {
    let base = iframe_url.split('#').next().unwrap_or(iframe_url);
    if let Some(scheme_end) = base.find("://") {
        let rest = &base[scheme_end + 3..];
        if let Some(host_end) = rest.find('/') {
            let origin = &base[..scheme_end + 3 + host_end];
            return (base.to_string(), origin.to_string());
        }
    }
    (
        episode_referer.to_string(),
        site_base_url.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::dooplay::config::{AOCC_CONFIG, MEUSANIMES_CONFIG};

    #[test]
    fn parse_meusdoramas_hash_route() {
        let route = parse_meusdoramas_route(
            "https://serv01.meusdoramas.club/#/video/92584/1/1/",
        )
        .expect("route");
        assert_eq!(route.0, "https://serv01.meusdoramas.club");
        assert_eq!(route.1, "92584");
        assert_eq!(route.2, "1");
        assert_eq!(route.3, "1");
    }

    #[tokio::test]
    #[ignore = "rede: cargo test resolve_aocc_stream -- --ignored --nocapture"]
    async fn resolve_aocc_stream() {
        let stream = resolve_stream(
            AOCC_CONFIG,
            "https://animesonlinecc.to/episodio/one-piece-episodio-835/",
        )
        .await
        .expect("stream");
        assert!(
            stream.url.contains(".m3u8")
                || stream.url.contains(".mp4")
                || stream.url.contains("googlevideo.com"),
            "unexpected url: {}",
            stream.url
        );
    }

    #[tokio::test]
    #[ignore = "rede: cargo test meusanimes_get_video_api -- --ignored --nocapture"]
    async fn meusanimes_get_video_api() {
        use crate::sources::dooplay::client::DooplayClient;
        let client = DooplayClient::new(MEUSANIMES_CONFIG).unwrap();
        for (label, iframe) in [
            (
                "eden",
                "https://serv01.meusdoramas.club/#/video/92584/1/1/",
            ),
            (
                "one-piece-e2",
                "https://serv01.meusdoramas.club/#/video/37854/1/2/",
            ),
        ] {
            let route = parse_meusdoramas_route(iframe).expect("route");
            let api_url = format!(
                "{}/posts/get-video.php?episode_number={}&season_number={}&tmdb={}",
                route.0, route.3, route.2, route.1
            );
            let body = client
                .fetch_url(&api_url, Some("https://meusanimes.blog/"))
                .await
                .expect("api");
            eprintln!("{label}: {body}");
        }
    }

    #[tokio::test]
    #[ignore = "rede: cargo test resolve_meusanimes_stream -- --ignored --nocapture"]
    async fn resolve_meusanimes_stream() {
        let stream = resolve_stream(
            MEUSANIMES_CONFIG,
            "https://meusanimes.blog/e/one-piece-1-episodio-2/",
        )
        .await
        .expect("stream");
        eprintln!("op ep2 stream: {} {:?}", stream.url, stream.kind);
        assert!(
            stream.url.contains(".m3u8")
                || stream.url.contains(".mp4")
                || stream.url.contains("googlevideo.com"),
            "unexpected url: {}",
            stream.url
        );
    }

    #[tokio::test]
    #[ignore = "rede: cargo test resolve_meusanimes_eden_stream -- --ignored --nocapture"]
    async fn resolve_meusanimes_eden_stream() {
        let stream = resolve_stream(
            MEUSANIMES_CONFIG,
            "https://meusanimes.blog/e/eden-1-episodio-1/",
        )
        .await
        .expect("stream");
        eprintln!("eden stream: {} {:?}", stream.url, stream.kind);
        assert!(
            stream.url.contains(".m3u8")
                || stream.url.contains(".mp4")
                || stream.url.contains("googlevideo.com"),
            "unexpected url: {}",
            stream.url
        );
    }
}

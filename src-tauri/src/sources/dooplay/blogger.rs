use crate::sources::dooplay::DooplayClient;
use crate::sources::shared::stream::{parse_blogger_video_config, stream_from_url, ParsedStream};
use crate::sources::SourceError;
use regex::Regex;
use std::time::{SystemTime, UNIX_EPOCH};

const BLOGGER_BASE: &str = "https://www.blogger.com/";

pub async fn resolve_blogger_stream(
    client: &DooplayClient,
    url: &str,
    referer: &str,
) -> Result<ParsedStream, SourceError> {
    let page = client.fetch_url(url, Some(referer)).await?;
    if let Some(parsed) = parse_blogger_video_config(&page) {
        return Ok(parsed);
    }
    resolve_blogger_rpc(client, url, &page).await
}

async fn resolve_blogger_rpc(
    client: &DooplayClient,
    url: &str,
    page: &str,
) -> Result<ParsedStream, SourceError> {
    let token = extract_blogger_token(url).ok_or(SourceError::NoStream)?;
    let form_session_id = extract_json_field(page, "FdrFJe").ok_or(SourceError::NoStream)?;
    let blog_id = extract_json_field(page, "cfb2h").ok_or(SourceError::NoStream)?;
    let request_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        % 86_400;

    let rpc_url = format!(
        "{BLOGGER_BASE}_/BloggerVideoPlayerUi/data/batchexecute?rpcids=WcwnYd&source-path=%2Fvideo.g&f.sid={form_session_id}&bl={blog_id}&hl=en-US&_reqid={request_id}&rt=c"
    );
    let f_req = format!(r#"[[["WcwnYd","[\"{token}\",\"\",0]",null,"generic"]]]"#);
    let body = format!("f.req={}&", urlencoding::encode(&f_req));

    let response = client
        .post_form(&rpc_url, &body, Some(BLOGGER_BASE))
        .await?;

    if !response.contains("https://") {
        return Err(SourceError::NoStream);
    }

    let mut candidates = parse_rpc_video_urls(&response);
    if candidates.is_empty() {
        if let Ok(re) = Regex::new(r#"https?://[^\s"']+"#) {
            for m in re.find_iter(&response) {
                let url = decode_double_escaped_url(m.as_str())
                    .unwrap_or_else(|| m.as_str().to_string());
                if url.contains("googlevideo") || stream_from_url(&url).is_some() {
                    candidates.push(url);
                }
            }
        }
    }

    candidates
        .iter()
        .find_map(|url| stream_from_url(url))
        .or_else(|| {
            candidates
                .iter()
                .find(|url| url.contains("googlevideo") || url.contains(".mp4"))
                .map(|url| ParsedStream {
                    url: url.clone(),
                    kind: crate::sources::StreamKind::Mp4,
                })
        })
        .ok_or(SourceError::NoStream)
}

fn extract_blogger_token(url: &str) -> Option<String> {
    let re = Regex::new(r"[?&]token=([^&]+)").ok()?;
    re.captures(url).and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn extract_json_field(html: &str, key: &str) -> Option<String> {
    let needle = format!("{key}\":\"");
    let start = html.find(&needle)? + needle.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_rpc_video_urls(response: &str) -> Vec<String> {
    let Some(chunk) = response.split("[[\\\"").nth(1) else {
        return Vec::new();
    };
    let chunk = chunk.split("]]]").next().unwrap_or(chunk);
    chunk
        .split("],[")
        .filter_map(|part| {
            let raw = part.split("\\\"").nth(1)?;
            let url = decode_rpc_url(raw)?;
            url.starts_with("http").then_some(url)
        })
        .collect()
}

fn decode_rpc_url(raw: &str) -> Option<String> {
    let once: String = serde_json::from_str(&format!("\"{raw}\"")).ok()?;
    serde_json::from_str(&format!("\"{once}\"")).ok()
}

fn decode_double_escaped_url(value: &str) -> Option<String> {
    decode_rpc_url(value).or_else(|| {
        let unescaped = value.replace("\\/", "/").replace("\\u0026", "&");
        Some(unescaped)
    })
}

use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use reqwest::Response;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

const BASE_URL: &str = "https://sushianimes.com.br";
pub const USER_AGENT_STR: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Error, Debug)]
pub enum SushiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("No stream URL found")]
    NoStream,
}

pub struct SushiClient {
    client: reqwest::Client,
    csrf_token: Arc<Mutex<Option<String>>>,
}

impl SushiClient {
    pub fn new() -> Result<Self, SushiError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STR));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .build()?;

        Ok(Self {
            client,
            csrf_token: Arc::new(Mutex::new(None)),
        })
    }

    #[allow(dead_code)]
    pub fn base_url() -> &'static str {
        BASE_URL
    }

    #[allow(dead_code)]
    pub fn user_agent() -> &'static str {
        USER_AGENT_STR
    }

    pub fn stream_headers(referer: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STR));
        headers.insert(REFERER, HeaderValue::from_static(BASE_URL));
        headers.insert(ORIGIN, HeaderValue::from_static(BASE_URL));
        if let Some(ref_url) = referer {
            if let Ok(value) = HeaderValue::from_str(ref_url) {
                headers.insert(REFERER, value);
            }
        }
        headers
    }

    pub fn ffmpeg_headers_arg(referer: Option<&str>) -> String {
        let referer = referer.unwrap_or(BASE_URL);
        format!(
            "Referer: {referer}\r\nOrigin: {BASE_URL}\r\nUser-Agent: {USER_AGENT_STR}\r\n"
        )
    }

    pub async fn fetch_stream(
        &self,
        url: &str,
        referer: Option<&str>,
    ) -> Result<Response, SushiError> {
        self.client
            .get(url)
            .headers(Self::stream_headers(referer))
            .send()
            .await?
            .error_for_status()
            .map_err(SushiError::from)
    }

    pub async fn get(&self, path_or_url: &str) -> Result<String, SushiError> {
        let url = Self::normalize_url(path_or_url);
        let html = self.client.get(&url).send().await?.error_for_status()?.text().await?;

        if let Some(token) = Self::extract_csrf(&html) {
            *self.csrf_token.lock().await = Some(token);
        }

        Ok(html)
    }

    pub async fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<String, SushiError> {
        let url = format!("{BASE_URL}{path}");
        let token = self.csrf_token.lock().await.clone();

        let mut request = self
            .client
            .post(&url)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(form);

        if let Some(token) = token {
            request = request.header("X-CSRF-Token", token);
        }

        Ok(request.send().await?.error_for_status()?.text().await?)
    }

    pub async fn post_search(&self, query: &str) -> Result<String, SushiError> {
        let _ = self.get("/").await?;
        let url = format!("{BASE_URL}/search");
        let token = self.csrf_token.lock().await.clone();

        let mut request = self
            .client
            .post(&url)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&[("_ACTION", "search"), ("q", query)]);

        if let Some(token) = token {
            request = request.header("X-CSRF-Token", token);
        }

        Ok(request.send().await?.error_for_status()?.text().await?)
    }

    pub async fn fetch_image(&self, url: &str) -> Result<Vec<u8>, SushiError> {
        let normalized = Self::normalize_url(url);
        let referer = if normalized.contains("sushianimes.com.br") {
            Some(BASE_URL)
        } else {
            Some(BASE_URL)
        };
        let bytes = self
            .client
            .get(&normalized)
            .headers(Self::stream_headers(referer))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }

    pub fn normalize_url(input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else if trimmed.starts_with('/') {
            format!("{BASE_URL}{trimmed}")
        } else {
            format!("{BASE_URL}/{trimmed}")
        }
    }

    pub fn extract_csrf(html: &str) -> Option<String> {
        let re = Regex::new(r#"<meta name="csrf-token" content="([^"]+)""#).ok()?;
        re.captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    pub fn is_anime_url(url: &str) -> bool {
        url.contains("/anime/") && !url.contains("-season-") && !url.contains("-episode")
    }

    pub fn is_movie_url(url: &str) -> bool {
        url.contains("/assistir/") || url.contains("/filme/")
    }

    pub fn resolve_movie_watch_url(url: &str) -> String {
        url.replace("/assistir/", "/filme/")
    }

    pub fn is_episode_url(url: &str) -> bool {
        url.contains("-season-") && url.contains("-episode")
    }

    pub fn is_supported_watch_url(url: &str) -> bool {
        Self::is_anime_url(url) || Self::is_episode_url(url) || Self::is_movie_url(url)
    }
}

impl Default for SushiClient {
    fn default() -> Self {
        Self::new().expect("failed to create HTTP client")
    }
}

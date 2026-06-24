use crate::sources::SourceError;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use reqwest::Response;

const BASE_URL: &str = "https://goyabu.io";

pub struct GoyabuClient {
    client: reqwest::Client,
}

impl GoyabuClient {
    pub fn new() -> Result<Self, SourceError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::sushi::client::USER_AGENT_STR),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .build()?;

        Ok(Self { client })
    }

    pub fn base_url() -> &'static str {
        BASE_URL
    }

    pub fn stream_headers(referer: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::sushi::client::USER_AGENT_STR),
        );
        headers.insert(REFERER, HeaderValue::from_static(BASE_URL));
        headers.insert(ORIGIN, HeaderValue::from_static(BASE_URL));
        if let Some(ref_url) = referer {
            if let Ok(value) = HeaderValue::from_str(ref_url) {
                headers.insert(REFERER, value);
            }
        }
        headers
    }

    pub async fn get(&self, path_or_url: &str) -> Result<String, SourceError> {
        let url = Self::normalize_url(path_or_url);
        let response = self
            .client
            .get(&url)
            .headers(Self::stream_headers(Some(&url)))
            .send()
            .await?
            .error_for_status()?;
        Ok(response.text().await?)
    }

    pub async fn post_form(&self, fields: &[(&str, &str)]) -> Result<String, SourceError> {
        let url = format!("{BASE_URL}/wp-admin/admin-ajax.php");
        let response = self
            .client
            .post(&url)
            .headers(Self::stream_headers(Some(BASE_URL)))
            .form(fields)
            .send()
            .await?
            .error_for_status()?;
        Ok(response.text().await?)
    }

    pub async fn fetch_stream(
        &self,
        url: &str,
        referer: Option<&str>,
    ) -> Result<Response, SourceError> {
        Ok(self
            .client
            .get(url)
            .headers(Self::stream_headers(referer))
            .send()
            .await?
            .error_for_status()?)
    }

    pub async fn fetch_image(&self, url: &str) -> Result<Vec<u8>, SourceError> {
        let normalized = Self::normalize_url(url);
        let bytes = self
            .client
            .get(&normalized)
            .headers(Self::stream_headers(Some(BASE_URL)))
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

    pub fn is_episode_url(url: &str) -> bool {
        let normalized = Self::normalize_url(url);
        if !normalized.contains("goyabu") {
            return false;
        }
        let path = normalized
            .split("goyabu.io")
            .nth(1)
            .unwrap_or("")
            .split('?')
            .next()
            .unwrap_or("")
            .trim_matches('/');
        !path.is_empty() && path.chars().all(|c| c.is_ascii_digit())
    }

    pub fn is_anime_url(url: &str) -> bool {
        let normalized = Self::normalize_url(url);
        normalized.contains("/anime/") && !Self::is_episode_url(&normalized)
    }

    pub fn is_supported_watch_url(url: &str) -> bool {
        Self::is_anime_url(url) || Self::is_episode_url(url)
    }
}

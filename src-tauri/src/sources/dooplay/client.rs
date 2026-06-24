use crate::sources::dooplay::config::DooplaySiteConfig;
use crate::sources::SourceError;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use reqwest::Response;

pub struct DooplayClient {
    config: DooplaySiteConfig,
    client: reqwest::Client,
}

impl DooplayClient {
    pub fn new(config: DooplaySiteConfig) -> Result<Self, SourceError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::sushi::client::USER_AGENT_STR),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .build()?;

        Ok(Self { config, client })
    }

    pub fn config(&self) -> DooplaySiteConfig {
        self.config
    }

    pub fn base_url(&self) -> &'static str {
        self.config.base_url
    }

    pub fn stream_headers(&self, referer: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::sushi::client::USER_AGENT_STR),
        );
        headers.insert(REFERER, HeaderValue::from_static(self.config.base_url));
        headers.insert(ORIGIN, HeaderValue::from_static(self.config.base_url));
        if let Some(ref_url) = referer {
            if let Ok(value) = HeaderValue::from_str(ref_url) {
                headers.insert(REFERER, value);
            }
        }
        headers
    }

    pub async fn get(&self, path_or_url: &str) -> Result<String, SourceError> {
        let url = Self::normalize_url(self.config, path_or_url);
        let response = self
            .client
            .get(&url)
            .headers(self.stream_headers(Some(&url)))
            .send()
            .await?
            .error_for_status()?;
        Ok(response.text().await?)
    }

    pub async fn fetch_url(&self, url: &str, referer: Option<&str>) -> Result<String, SourceError> {
        let response = self
            .client
            .get(url)
            .headers(self.stream_headers(referer))
            .send()
            .await?
            .error_for_status()?;
        Ok(response.text().await?)
    }

    pub async fn fetch_bytes(&self, url: &str, referer: Option<&str>) -> Result<Vec<u8>, SourceError> {
        let bytes = self
            .client
            .get(url)
            .headers(self.stream_headers(referer))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }

    #[allow(dead_code)]
    pub async fn fetch_stream(
        &self,
        url: &str,
        referer: Option<&str>,
    ) -> Result<Response, SourceError> {
        Ok(self
            .client
            .get(url)
            .headers(self.stream_headers(referer))
            .send()
            .await?
            .error_for_status()?)
    }

    pub fn normalize_url(config: DooplaySiteConfig, input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else if trimmed.starts_with('/') {
            format!("{}{trimmed}", config.base_url)
        } else {
            format!("{}/{trimmed}", config.base_url)
        }
    }

    pub fn is_episode_url(config: DooplaySiteConfig, url: &str) -> bool {
        let normalized = Self::normalize_url(config, url);
        normalized.contains(config.episode_prefix)
    }

    pub fn is_anime_url(config: DooplaySiteConfig, url: &str) -> bool {
        let normalized = Self::normalize_url(config, url);
        normalized.contains(config.anime_prefix) && !Self::is_episode_url(config, &normalized)
    }

    pub fn is_supported_watch_url(config: DooplaySiteConfig, url: &str) -> bool {
        Self::is_anime_url(config, url) || Self::is_episode_url(config, url)
    }
}

mod blogger;
mod client;
pub mod config;
mod embed;
pub use blogger::resolve_blogger_stream;
pub mod parser;

pub use config::{AOCC_CONFIG, DooplaySiteConfig, MEUSANIMES_CONFIG};
pub use client::DooplayClient;

use crate::models::{AnimeInfo, CatalogType};
use crate::sources::dooplay::parser::{parse_anime_page, parse_episode_anime_url};
use crate::sources::{AnimeSource, SourceError, SourceId, StreamResolution};

pub struct DooplaySource {
    config: DooplaySiteConfig,
}

impl DooplaySource {
    pub fn new(config: DooplaySiteConfig) -> Self {
        Self { config }
    }
}

impl AnimeSource for DooplaySource {
    fn id(&self) -> SourceId {
        self.config.source_id
    }

    fn base_url(&self) -> &'static str {
        self.config.base_url
    }

    fn normalize_url(&self, input: &str) -> String {
        DooplayClient::normalize_url(self.config, input)
    }

    fn is_episode_url(&self, url: &str) -> bool {
        DooplayClient::is_episode_url(self.config, url)
    }

    fn is_supported_watch_url(&self, url: &str) -> bool {
        DooplayClient::is_supported_watch_url(self.config, url)
    }
}

pub async fn parse_anime(config: DooplaySiteConfig, url: &str) -> Result<AnimeInfo, SourceError> {
    let client = DooplayClient::new(config)?;
    let normalized = DooplayClient::normalize_url(config, url);

    if !DooplayClient::is_supported_watch_url(config, &normalized) {
        return Err(SourceError::UnsupportedUrl);
    }

    if DooplayClient::is_episode_url(config, &normalized) {
        let anime_url = parse_episode_anime_url(&client, &normalized).await?;
        return parse_anime_page(&client, &anime_url).await;
    }

    parse_anime_page(&client, &normalized).await
}

pub async fn resolve_stream(
    config: DooplaySiteConfig,
    episode_url: &str,
) -> Result<StreamResolution, SourceError> {
    embed::resolve_stream(config, episode_url).await
}

pub async fn fetch_image(config: DooplaySiteConfig, url: &str) -> Result<Vec<u8>, SourceError> {
    let client = DooplayClient::new(config)?;
    client.fetch_bytes(url, Some(config.base_url)).await
}

pub fn supports_catalog_type(catalog_type: CatalogType) -> bool {
    matches!(catalog_type, CatalogType::Animes | CatalogType::Category)
}

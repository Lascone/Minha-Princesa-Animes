mod client;
mod embed;
mod parser;

pub use client::GoyabuClient;

use crate::models::{AnimeInfo, BrowseRequest, CatalogPage, CatalogType, CategoryInfo, SearchRequest};
use crate::sources::goyabu::parser::{categories_list, parse_anime_page, parse_episode_anime_url};
use crate::sources::{AnimeSource, SourceError, SourceId, StreamResolution};
use crate::sources::goyabu::embed::resolve_stream as resolve_goyabu_stream;

pub struct GoyabuSource;

impl AnimeSource for GoyabuSource {
    fn id(&self) -> SourceId {
        SourceId::Goyabu
    }

    fn base_url(&self) -> &'static str {
        GoyabuClient::base_url()
    }

    fn normalize_url(&self, input: &str) -> String {
        GoyabuClient::normalize_url(input)
    }

    fn is_episode_url(&self, url: &str) -> bool {
        GoyabuClient::is_episode_url(url)
    }

    fn is_supported_watch_url(&self, url: &str) -> bool {
        GoyabuClient::is_supported_watch_url(url)
    }
}

pub async fn parse_anime(url: &str) -> Result<AnimeInfo, SourceError> {
    let client = GoyabuClient::new()?;
    let normalized = GoyabuClient::normalize_url(url);

    if !GoyabuClient::is_supported_watch_url(&normalized) {
        return Err(SourceError::UnsupportedUrl);
    }

    if GoyabuClient::is_episode_url(&normalized) {
        let anime_url = parse_episode_anime_url(&client, &normalized).await?;
        return parse_anime_page(&client, &anime_url).await;
    }

    parse_anime_page(&client, &normalized).await
}

pub async fn browse(req: &BrowseRequest) -> Result<CatalogPage, SourceError> {
    parser::browse(req).await
}

pub async fn search(req: &SearchRequest) -> Result<CatalogPage, SourceError> {
    parser::search(req).await
}

pub async fn categories() -> Result<Vec<CategoryInfo>, SourceError> {
    categories_list().await
}

pub async fn resolve_stream(episode_url: &str) -> Result<StreamResolution, SourceError> {
    resolve_goyabu_stream(episode_url).await
}

pub async fn fetch_image(url: &str) -> Result<Vec<u8>, SourceError> {
    let client = GoyabuClient::new()?;
    client.fetch_image(url).await
}

pub fn supports_catalog_type(catalog_type: CatalogType) -> bool {
    matches!(catalog_type, CatalogType::Animes | CatalogType::Category)
}

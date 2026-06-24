pub mod animesdigital;
pub mod animesonlinecc;
pub mod dooplay;
pub mod goyabu;
pub mod meusanimes;
pub mod registry;
pub mod shared;
pub mod sushianimes;

use crate::models::{
    AnimeInfo, AnimeSourceId, BrowseRequest, CatalogPage, CategoryInfo, SearchRequest,
};
use thiserror::Error;

pub type SourceId = AnimeSourceId;

#[derive(Error, Debug)]
pub enum SourceError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("No stream URL found")]
    NoStream,
    #[error("URL não suportada para esta fonte")]
    UnsupportedUrl,
    #[error("Fonte desconhecida")]
    UnknownSource,
    #[error("{0}")]
    Other(String),
}

impl From<crate::sushi::client::SushiError> for SourceError {
    fn from(e: crate::sushi::client::SushiError) -> Self {
        match e {
            crate::sushi::client::SushiError::Http(e) => SourceError::Http(e),
            crate::sushi::client::SushiError::Parse(s) => SourceError::Parse(s),
            crate::sushi::client::SushiError::NoStream => SourceError::NoStream,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Hls,
    Mp4,
}

#[derive(Debug, Clone)]
pub struct StreamResolution {
    pub url: String,
    pub kind: StreamKind,
    pub referer: String,
    pub origin: String,
}

impl StreamResolution {
    pub fn ffmpeg_headers_arg(&self) -> String {
        format!(
            "Referer: {}\r\nOrigin: {}\r\nUser-Agent: {}\r\n",
            self.referer,
            self.origin,
            crate::sushi::client::USER_AGENT_STR
        )
    }
}

/// Capabilities shared by every anime source.
pub trait AnimeSource {
    fn id(&self) -> SourceId;
    fn base_url(&self) -> &'static str;
    fn normalize_url(&self, input: &str) -> String;
    fn is_episode_url(&self, url: &str) -> bool;
    fn is_supported_watch_url(&self, url: &str) -> bool;

    fn image_referer(&self, image_url: &str) -> Option<&'static str> {
        let _ = image_url;
        Some(self.base_url())
    }
}

pub async fn parse_anime(source: SourceId, url: &str) -> Result<AnimeInfo, SourceError> {
    match source {
        SourceId::Sushianimes => sushianimes::parse_anime(url).await,
        SourceId::Goyabu => goyabu::parse_anime(url).await,
        SourceId::Meusanimes => meusanimes::parse_anime(url).await,
        SourceId::Animesonlinecc => animesonlinecc::parse_anime(url).await,
        SourceId::Animesdigital => animesdigital::parse_anime(url).await,
    }
}

pub async fn browse(source: SourceId, req: &BrowseRequest) -> Result<CatalogPage, SourceError> {
    match source {
        SourceId::Sushianimes => sushianimes::browse(req).await,
        SourceId::Goyabu => goyabu::browse(req).await,
        SourceId::Meusanimes => meusanimes::browse(req).await,
        SourceId::Animesonlinecc => animesonlinecc::browse(req).await,
        SourceId::Animesdigital => animesdigital::browse(req).await,
    }
}

pub async fn search(source: SourceId, req: &SearchRequest) -> Result<CatalogPage, SourceError> {
    match source {
        SourceId::Sushianimes => sushianimes::search(req).await,
        SourceId::Goyabu => goyabu::search(req).await,
        SourceId::Meusanimes => meusanimes::search(req).await,
        SourceId::Animesonlinecc => animesonlinecc::search(req).await,
        SourceId::Animesdigital => animesdigital::search(req).await,
    }
}

pub async fn categories(source: SourceId) -> Result<Vec<CategoryInfo>, SourceError> {
    match source {
        SourceId::Sushianimes => sushianimes::categories().await,
        SourceId::Goyabu => goyabu::categories().await,
        SourceId::Meusanimes => meusanimes::categories().await,
        SourceId::Animesonlinecc => animesonlinecc::categories().await,
        SourceId::Animesdigital => animesdigital::categories().await,
    }
}

pub async fn resolve_stream(source: SourceId, episode_url: &str) -> Result<StreamResolution, SourceError> {
    match source {
        SourceId::Sushianimes => sushianimes::resolve_stream(episode_url).await,
        SourceId::Goyabu => goyabu::resolve_stream(episode_url).await,
        SourceId::Meusanimes => meusanimes::resolve_stream(episode_url).await,
        SourceId::Animesonlinecc => animesonlinecc::resolve_stream(episode_url).await,
        SourceId::Animesdigital => animesdigital::resolve_stream(episode_url).await,
    }
}

pub async fn fetch_image(source: SourceId, url: &str) -> Result<Vec<u8>, SourceError> {
    match source {
        SourceId::Sushianimes => sushianimes::fetch_image(url).await,
        SourceId::Goyabu => goyabu::fetch_image(url).await,
        SourceId::Meusanimes => meusanimes::fetch_image(url).await,
        SourceId::Animesonlinecc => animesonlinecc::fetch_image(url).await,
        SourceId::Animesdigital => animesdigital::fetch_image(url).await,
    }
}

pub fn source_for_url(url: &str) -> Result<SourceId, SourceError> {
    AnimeSourceId::detect_from_url(url).ok_or(SourceError::UnknownSource)
}

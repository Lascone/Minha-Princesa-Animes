use crate::models::{AnimeInfo, BrowseRequest, CatalogPage, CategoryInfo, SearchRequest};
use crate::sources::dooplay::{self, MEUSANIMES_CONFIG};
use crate::sources::{AnimeSource, SourceError, SourceId, StreamResolution};

pub struct MeusanimesSource;

impl AnimeSource for MeusanimesSource {
    fn id(&self) -> SourceId {
        SourceId::Meusanimes
    }

    fn base_url(&self) -> &'static str {
        MEUSANIMES_CONFIG.base_url
    }

    fn normalize_url(&self, input: &str) -> String {
        dooplay::DooplayClient::normalize_url(MEUSANIMES_CONFIG, input)
    }

    fn is_episode_url(&self, url: &str) -> bool {
        dooplay::DooplayClient::is_episode_url(MEUSANIMES_CONFIG, url)
    }

    fn is_supported_watch_url(&self, url: &str) -> bool {
        dooplay::DooplayClient::is_supported_watch_url(MEUSANIMES_CONFIG, url)
    }
}

pub async fn parse_anime(url: &str) -> Result<AnimeInfo, SourceError> {
    dooplay::parse_anime(MEUSANIMES_CONFIG, url).await
}

pub async fn browse(req: &BrowseRequest) -> Result<CatalogPage, SourceError> {
    dooplay::parser::browse(req, MEUSANIMES_CONFIG).await
}

pub async fn search(req: &SearchRequest) -> Result<CatalogPage, SourceError> {
    dooplay::parser::search(req, MEUSANIMES_CONFIG).await
}

pub async fn categories() -> Result<Vec<CategoryInfo>, SourceError> {
    dooplay::parser::categories_list(MEUSANIMES_CONFIG).await
}

pub async fn resolve_stream(episode_url: &str) -> Result<StreamResolution, SourceError> {
    dooplay::resolve_stream(MEUSANIMES_CONFIG, episode_url).await
}

pub async fn fetch_image(url: &str) -> Result<Vec<u8>, SourceError> {
    dooplay::fetch_image(MEUSANIMES_CONFIG, url).await
}

use crate::models::{AnimeInfo, BrowseRequest, CatalogPage, CategoryInfo, SearchRequest};
use crate::sources::dooplay::{self, AOCC_CONFIG};
use crate::sources::{AnimeSource, SourceError, SourceId, StreamResolution};

pub struct AnimesonlineccSource;

impl AnimeSource for AnimesonlineccSource {
    fn id(&self) -> SourceId {
        SourceId::Animesonlinecc
    }

    fn base_url(&self) -> &'static str {
        AOCC_CONFIG.base_url
    }

    fn normalize_url(&self, input: &str) -> String {
        dooplay::DooplayClient::normalize_url(AOCC_CONFIG, input)
    }

    fn is_episode_url(&self, url: &str) -> bool {
        dooplay::DooplayClient::is_episode_url(AOCC_CONFIG, url)
    }

    fn is_supported_watch_url(&self, url: &str) -> bool {
        dooplay::DooplayClient::is_supported_watch_url(AOCC_CONFIG, url)
    }
}

pub async fn parse_anime(url: &str) -> Result<AnimeInfo, SourceError> {
    dooplay::parse_anime(AOCC_CONFIG, url).await
}

pub async fn browse(req: &BrowseRequest) -> Result<CatalogPage, SourceError> {
    dooplay::parser::browse(req, AOCC_CONFIG).await
}

pub async fn search(req: &SearchRequest) -> Result<CatalogPage, SourceError> {
    dooplay::parser::search(req, AOCC_CONFIG).await
}

pub async fn categories() -> Result<Vec<CategoryInfo>, SourceError> {
    dooplay::parser::categories_list(AOCC_CONFIG).await
}

pub async fn resolve_stream(episode_url: &str) -> Result<StreamResolution, SourceError> {
    dooplay::resolve_stream(AOCC_CONFIG, episode_url).await
}

pub async fn fetch_image(url: &str) -> Result<Vec<u8>, SourceError> {
    dooplay::fetch_image(AOCC_CONFIG, url).await
}

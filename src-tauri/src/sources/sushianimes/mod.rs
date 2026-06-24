use crate::models::{
    AnimeInfo, BrowseRequest, CatalogPage, CatalogType, CategoryInfo, SearchRequest,
};
use crate::sources::{AnimeSource, SourceError, SourceId, StreamKind, StreamResolution};
use crate::sushi::client::SushiClient;
use crate::sushi::{
    apply_catalog_filters, browse_catalog, parse_anime_page, parse_categories, search_catalog,
    parse_episode_embed_id, resolve_stream_url,
};

pub struct SushiSource;

impl AnimeSource for SushiSource {
    fn id(&self) -> SourceId {
        SourceId::Sushianimes
    }

    fn base_url(&self) -> &'static str {
        SushiClient::base_url()
    }

    fn normalize_url(&self, input: &str) -> String {
        SushiClient::normalize_url(input)
    }

    fn is_episode_url(&self, url: &str) -> bool {
        SushiClient::is_episode_url(url)
    }

    fn is_supported_watch_url(&self, url: &str) -> bool {
        SushiClient::is_supported_watch_url(url)
    }
}

pub async fn parse_anime(url: &str) -> Result<AnimeInfo, SourceError> {
    let client = SushiClient::new()?;
    let normalized = SushiClient::normalize_url(url);

    if !SushiClient::is_supported_watch_url(&normalized) {
        return Err(SourceError::UnsupportedUrl);
    }

    if SushiClient::is_episode_url(&normalized) {
        let re = regex::Regex::new(r"-\d+-season-\d+-episode").unwrap();
        let anime_url = re
            .split(&normalized)
            .next()
            .unwrap_or(&normalized)
            .to_string();
        return parse_anime_page(&client, &anime_url).await.map_err(Into::into);
    }

    parse_anime_page(&client, &normalized)
        .await
        .map_err(Into::into)
}

pub async fn browse(req: &BrowseRequest) -> Result<CatalogPage, SourceError> {
    let client = SushiClient::new()?;
    let result = browse_catalog(
        &client,
        req.catalog_type.clone(),
        req.page,
        req.category_slug.as_deref(),
    )
    .await?;
    Ok(apply_catalog_filters(result, &req.filters))
}

pub async fn search(req: &SearchRequest) -> Result<CatalogPage, SourceError> {
    let client = SushiClient::new()?;
    let result = search_catalog(&client, &req.query, req.page).await?;
    Ok(apply_catalog_filters(result, &req.filters))
}

pub async fn categories() -> Result<Vec<CategoryInfo>, SourceError> {
    let client = SushiClient::new()?;
    parse_categories(&client).await.map_err(Into::into)
}

pub async fn resolve_stream(episode_url: &str) -> Result<StreamResolution, SourceError> {
    let client = SushiClient::new()?;
    let embed_id = parse_episode_embed_id(&client, episode_url).await?;
    let stream = resolve_stream_url(&client, &embed_id).await?;
    let base = SushiClient::base_url();
    Ok(StreamResolution {
        url: stream.url,
        kind: match stream.kind {
            crate::sushi::StreamKind::Hls => StreamKind::Hls,
            crate::sushi::StreamKind::Mp4 => StreamKind::Mp4,
        },
        referer: episode_url.to_string(),
        origin: base.to_string(),
    })
}

pub async fn fetch_image(url: &str) -> Result<Vec<u8>, SourceError> {
    let client = SushiClient::new()?;
    client.fetch_image(url).await.map_err(Into::into)
}

/// Sushi supports movies tab; Goyabu does not.
pub fn supports_catalog_type(catalog_type: CatalogType) -> bool {
    matches!(
        catalog_type,
        CatalogType::Animes | CatalogType::Filmes | CatalogType::Category
    )
}

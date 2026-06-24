use crate::models::{
    AnimeInfo, BrowseRequest, CatalogItem, CatalogPage, CatalogType, CategoryInfo, EpisodeInfo,
    SeasonInfo,
};
use crate::sources::goyabu::client::GoyabuClient;
use crate::sources::SourceError;
use crate::sushi::apply_catalog_filters;
use regex::Regex;
use scraper::{Html, Selector};
use urlencoding::encode;

pub async fn parse_anime_page(client: &GoyabuClient, url: &str) -> Result<AnimeInfo, SourceError> {
    let normalized = GoyabuClient::normalize_url(url);
    let html = client.get(&normalized).await?;
    parse_anime_html(&html, &normalized)
}

pub fn parse_anime_html(html: &str, url: &str) -> Result<AnimeInfo, SourceError> {
    let document = Html::parse_document(html);

    let title = extract_text(&document, "h1.text-hidden")
        .or_else(|| extract_text(&document, "h1"))
        .unwrap_or_else(|| "Anime".to_string());

    let poster = extract_poster(&document, html);
    let synopsis = extract_synopsis(&document);

    let episodes = parse_episodes_json(html)?;
    let season = SeasonInfo {
        number: 1,
        episodes,
    };

    Ok(AnimeInfo {
        title,
        url: url.to_string(),
        poster,
        synopsis,
        seasons: vec![season],
    })
}

pub async fn parse_episode_anime_url(client: &GoyabuClient, episode_url: &str) -> Result<String, SourceError> {
    let html = client.get(episode_url).await?;
    let re = Regex::new(r#"href="(https://goyabu\.io/anime/[^"]+)""#).map_err(|e| SourceError::Parse(e.to_string()))?;
    if let Some(caps) = re.captures(&html) {
        if let Some(m) = caps.get(1) {
            return Ok(m.as_str().to_string());
        }
    }
    Err(SourceError::Parse("Link do anime não encontrado na página do episódio".to_string()))
}

fn parse_episodes_json(html: &str) -> Result<Vec<EpisodeInfo>, SourceError> {
    let marker = "const allEpisodes = ";
    let start = html
        .find(marker)
        .ok_or_else(|| SourceError::Parse("Lista de episódios não encontrada".to_string()))?;
    let json_start = start + marker.len();
    let rest = &html[json_start..];
    let end = rest
        .find("];")
        .ok_or_else(|| SourceError::Parse("JSON de episódios inválido".to_string()))?;
    let json_str = format!("{}]", &rest[..end]);

    let items: Vec<GoyabuEpisodeJson> =
        serde_json::from_str(&json_str).map_err(|e| SourceError::Parse(e.to_string()))?;

    let mut episodes: Vec<EpisodeInfo> = items
        .into_iter()
        .filter_map(|ep| {
            let number = ep.episodio.parse::<u32>().ok()?;
            let url = GoyabuClient::normalize_url(&ep.link);
            Some(EpisodeInfo {
                number,
                season: 1,
                title: if ep.episode_name.is_empty() {
                    format!("Episódio {number}")
                } else {
                    ep.episode_name
                },
                description: None,
                url,
            })
        })
        .collect();

    episodes.sort_by_key(|e| e.number);
    if episodes.is_empty() {
        return Err(SourceError::Parse("Nenhum episódio encontrado".to_string()));
    }
    Ok(episodes)
}

#[derive(serde::Deserialize)]
struct GoyabuEpisodeJson {
    link: String,
    episodio: String,
    #[serde(default)]
    episode_name: String,
}

fn extract_text(document: &Html, selector_str: &str) -> Option<String> {
    let sel = Selector::parse(selector_str).ok()?;
    document
        .select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_poster(document: &Html, html: &str) -> Option<String> {
    if let Ok(sel) = Selector::parse(".streamer-poster img.cover, figure.thumb img.cover") {
        if let Some(img) = document.select(&sel).next() {
            if let Some(src) = img.value().attr("src") {
                return Some(GoyabuClient::normalize_url(src));
            }
        }
    }
    let re = Regex::new(r#"<meta property="og:image" content="([^"]+)""#).ok()?;
    re.captures(html)
        .and_then(|c| c.get(1))
        .map(|m| GoyabuClient::normalize_url(m.as_str()))
}

fn extract_synopsis(document: &Html) -> Option<String> {
    if let Ok(sel) = Selector::parse(".streamer-info .sinopse, .sinopse, .descricao") {
        if let Some(el) = document.select(&sel).next() {
            let text = el.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

pub async fn browse_catalog(
    client: &GoyabuClient,
    catalog_type: CatalogType,
    page: u32,
    category_slug: Option<&str>,
) -> Result<CatalogPage, SourceError> {
    let path = match catalog_type {
        CatalogType::Animes => {
            if page <= 1 {
                "/lista-de-animes?l=todos".to_string()
            } else {
                format!("/lista-de-animes?l=todos&paged={page}")
            }
        }
        CatalogType::Filmes => {
            return Ok(CatalogPage {
                items: vec![],
                page,
                has_next: false,
            });
        }
        CatalogType::Category => {
            let slug = category_slug.unwrap_or("acao");
            if page <= 1 {
                format!("/generos/{slug}")
            } else {
                format!("/generos/{slug}?paged={page}")
            }
        }
    };

    let html = client.get(&path).await?;
    parse_catalog_html(&html, page)
}

pub fn parse_catalog_html(html: &str, page: u32) -> Result<CatalogPage, SourceError> {
    let document = Html::parse_document(html);
    let article_sel = Selector::parse("article.boxAN").unwrap();
    let link_sel = Selector::parse("a[href*='/anime/']").unwrap();
    let title_sel = Selector::parse(".title, div.title").unwrap();
    let img_sel = Selector::parse("img.cover").unwrap();

    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for article in document.select(&article_sel) {
        let link = article
            .select(&link_sel)
            .next()
            .and_then(|a| a.value().attr("href"));
        let Some(href) = link else { continue };
        let url = GoyabuClient::normalize_url(href);
        if !seen.insert(url.clone()) {
            continue;
        }

        let title = article
            .select(&title_sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Anime".to_string());

        let poster = article
            .select(&img_sel)
            .next()
            .and_then(|img| img.value().attr("src"))
            .map(|s| GoyabuClient::normalize_url(s));

        items.push(CatalogItem {
            title,
            url,
            poster,
            category: None,
        });
    }

    let has_next = parse_has_next(html, page);

    Ok(CatalogPage {
        items,
        page,
        has_next,
    })
}

fn parse_has_next(html: &str, page: u32) -> bool {
    if let Some(re) = Regex::new(r#"data-total-pages="(\d+)""#).ok() {
        if let Some(caps) = re.captures(html) {
            if let Some(total) = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()) {
                return page < total;
            }
        }
    }
    html.contains(&format!("paged={}", page + 1))
}

pub async fn search_catalog(
    client: &GoyabuClient,
    query: &str,
    page: u32,
) -> Result<CatalogPage, SourceError> {
    let encoded = encode(query);
    let path = if page <= 1 {
        format!("/?s={encoded}")
    } else {
        format!("/?s={encoded}&paged={page}")
    };
    let html = client.get(&path).await?;
    parse_catalog_html(&html, page)
}

pub async fn parse_categories(client: &GoyabuClient) -> Result<Vec<CategoryInfo>, SourceError> {
    let html = client.get("/generos").await?;
    let document = Html::parse_document(&html);
    let link_sel = Selector::parse("a[href*='/generos/']").unwrap();

    let mut categories = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for link in document.select(&link_sel) {
        let Some(href) = link.value().attr("href") else { continue };
        if href.ends_with("/generos/") || href.ends_with("/generos") {
            continue;
        }
        let url = GoyabuClient::normalize_url(href);
        let slug = href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();
        if slug.is_empty() || !seen.insert(slug.clone()) {
            continue;
        }
        let name = link.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            continue;
        }
        categories.push(CategoryInfo {
            name,
            slug,
            url,
        });
    }

    categories.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(categories)
}

pub async fn browse(req: &BrowseRequest) -> Result<CatalogPage, SourceError> {
    let client = GoyabuClient::new()?;
    let result = browse_catalog(
        &client,
        req.catalog_type.clone(),
        req.page,
        req.category_slug.as_deref(),
    )
    .await?;
    Ok(apply_catalog_filters(result, &req.filters))
}

pub async fn search(req: &crate::models::SearchRequest) -> Result<CatalogPage, SourceError> {
    let client = GoyabuClient::new()?;
    let result = search_catalog(&client, &req.query, req.page).await?;
    Ok(apply_catalog_filters(result, &req.filters))
}

pub async fn categories_list() -> Result<Vec<CategoryInfo>, SourceError> {
    let client = GoyabuClient::new()?;
    parse_categories(&client).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_catalog_html_extracts_items() {
        let html = r#"
        <article class="boxAN">
          <a href="/anime/test-anime"><img class="cover" src="/poster.webp"/><div class="title">Test Anime</div></a>
        </article>
        <div class="pagination" data-current-page="1" data-total-pages="3"></div>
        "#;
        let page = parse_catalog_html(html, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].title, "Test Anime");
        assert!(page.items[0].url.contains("/anime/test-anime"));
        assert!(page.has_next);
    }

    #[tokio::test]
    #[ignore = "rede: cargo test parse_goyabu_anime_ichijouma -- --ignored --nocapture"]
    async fn parse_goyabu_anime_ichijouma() {
        let client = GoyabuClient::new().unwrap();
        let info = parse_anime_page(
            &client,
            "https://goyabu.io/anime/ichijouma-mankitsugurashi",
        )
        .await
        .expect("anime");
        assert!(info.title.to_lowercase().contains("ichijouma"));
        assert!(!info.seasons.is_empty());
        assert!(!info.seasons[0].episodes.is_empty());
    }
}

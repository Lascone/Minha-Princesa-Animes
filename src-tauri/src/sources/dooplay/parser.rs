use crate::models::{
    AnimeInfo, BrowseRequest, CatalogItem, CatalogPage, CatalogType, CategoryInfo, EpisodeInfo,
    SeasonInfo,
};
use crate::sources::dooplay::client::DooplayClient;
use crate::sources::dooplay::config::DooplaySiteConfig;
use crate::sources::SourceError;
use crate::sushi::apply_catalog_filters;
use regex::Regex;
use scraper::{Html, Selector};
use urlencoding::encode;

pub async fn parse_anime_page(
    client: &DooplayClient,
    url: &str,
) -> Result<AnimeInfo, SourceError> {
    let config = client.config();
    let normalized = DooplayClient::normalize_url(config, url);
    let html = client.get(&normalized).await?;
    parse_anime_html(config, &html, &normalized)
}

pub fn parse_anime_html(
    config: DooplaySiteConfig,
    html: &str,
    url: &str,
) -> Result<AnimeInfo, SourceError> {
    let document = Html::parse_document(html);

    let title = extract_meta(&document, "og:title")
        .or_else(|| extract_text(&document, "h1"))
        .unwrap_or_else(|| "Anime".to_string());
    let title = clean_title(&title);

    let poster = extract_poster(config, &document, html);
    let synopsis = extract_meta(&document, "og:description")
        .or_else(|| extract_meta(&document, "description"));

    let episodes = parse_episode_list(config, html)?;
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

pub async fn parse_episode_anime_url(
    client: &DooplayClient,
    episode_url: &str,
) -> Result<String, SourceError> {
    let config = client.config();
    let html = client.get(episode_url).await?;

    let anime_re = Regex::new(&format!(
        r#"href="(https?://[^"]+{}[^"]+)""#,
        regex::escape(config.anime_prefix)
    ))
    .map_err(|e| SourceError::Parse(e.to_string()))?;

    for caps in anime_re.captures_iter(&html) {
        if let Some(m) = caps.get(1) {
            let url = m.as_str().to_string();
            if !DooplayClient::is_episode_url(config, &url) {
                return Ok(url);
            }
        }
    }

    Err(SourceError::Parse(
        "Link do anime não encontrado na página do episódio".to_string(),
    ))
}

fn parse_episode_list(
    config: DooplaySiteConfig,
    html: &str,
) -> Result<Vec<EpisodeInfo>, SourceError> {
    let document = Html::parse_document(html);
    let item_sel = Selector::parse("li .episodiotitle a, .episodios li a").unwrap();
    let num_sel = Selector::parse(".numerando").unwrap();

    let mut episodes = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for link in document.select(&item_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let url = DooplayClient::normalize_url(config, href);
        if !url.contains(config.episode_prefix) || !seen.insert(url.clone()) {
            continue;
        }

        let title_text = link.text().collect::<String>().trim().to_string();
        let numerando = link
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .find_map(|el| {
                el.select(&num_sel)
                    .next()
                    .map(|n| n.text().collect::<String>())
            })
            .unwrap_or_default();

        let number = parse_episode_number(&numerando, &url, &title_text);
        episodes.push(EpisodeInfo {
            number,
            season: 1,
            title: if title_text.is_empty() {
                format!("Episódio {number}")
            } else {
                title_text
            },
            description: None,
            url,
        });
    }

    if episodes.is_empty() {
        if let Ok(re) = Regex::new(&format!(
            r#"href=['"]([^'"]*{}[^'"]+)['"]"#,
            regex::escape(config.episode_prefix)
        )) {
            for caps in re.captures_iter(html) {
                if let Some(m) = caps.get(1) {
                    let url = DooplayClient::normalize_url(config, m.as_str());
                    if seen.insert(url.clone()) {
                        let number = parse_episode_number("", &url, "");
                        episodes.push(EpisodeInfo {
                            number,
                            season: 1,
                            title: format!("Episódio {number}"),
                            description: None,
                            url,
                        });
                    }
                }
            }
        }
    }

    episodes.sort_by_key(|e| e.number);
    if episodes.is_empty() {
        return Err(SourceError::Parse("Nenhum episódio encontrado".to_string()));
    }
    Ok(episodes)
}

fn parse_episode_number(numerando: &str, url: &str, title: &str) -> u32 {
    if let Ok(re) = Regex::new(r"(\d+)\s*-\s*(\d+)") {
        if let Some(caps) = re.captures(numerando) {
            if let Some(m) = caps.get(2) {
                if let Ok(n) = m.as_str().parse() {
                    return n;
                }
            }
        }
    }
    for text in [url, title] {
        if let Ok(re) = Regex::new(r"episodio-(\d+)") {
            if let Some(caps) = re.captures(&text.to_lowercase()) {
                if let Some(m) = caps.get(1) {
                    if let Ok(n) = m.as_str().parse() {
                        return n;
                    }
                }
            }
        }
        if let Ok(re) = Regex::new(r"episodios-(\d+)") {
            if let Some(caps) = re.captures(&text.to_lowercase()) {
                if let Some(m) = caps.get(1) {
                    if let Ok(n) = m.as_str().parse() {
                        return n;
                    }
                }
            }
        }
    }
    1
}

fn clean_title(title: &str) -> String {
    title
        .split(" - ")
        .next()
        .unwrap_or(title)
        .split(" Todos os")
        .next()
        .unwrap_or(title)
        .trim()
        .to_string()
}

fn extract_meta(document: &Html, property: &str) -> Option<String> {
    let sel = format!(r#"meta[property="{property}"], meta[name="{property}"]"#);
    let sel = Selector::parse(&sel).ok()?;
    document
        .select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_text(document: &Html, selector_str: &str) -> Option<String> {
    let sel = Selector::parse(selector_str).ok()?;
    document
        .select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_poster(config: DooplaySiteConfig, document: &Html, html: &str) -> Option<String> {
    if let Ok(sel) = Selector::parse(".sheader .poster img, .poster img, img.poster") {
        if let Some(img) = document.select(&sel).next() {
            if let Some(src) = img.value().attr("src") {
                return Some(DooplayClient::normalize_url(config, src));
            }
        }
    }
    if let Ok(re) = Regex::new(r#"<meta property="og:image" content="([^"]+)""#) {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                return Some(DooplayClient::normalize_url(config, m.as_str()));
            }
        }
    }
    None
}

pub async fn browse_catalog(
    client: &DooplayClient,
    catalog_type: CatalogType,
    page: u32,
    category_slug: Option<&str>,
) -> Result<CatalogPage, SourceError> {
    let config = client.config();
    let path = match catalog_type {
        CatalogType::Animes => {
            if page <= 1 {
                config.catalog_list_path.to_string()
            } else {
                format!("{}{page}/", config.catalog_prefix)
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
                format!("{}{slug}/", config.genres_path)
            } else {
                format!("{}{slug}/page/{page}/", config.genres_path)
            }
        }
    };

    let html = client.get(&path).await?;
    parse_catalog_html(config, &html, page)
}

pub fn parse_catalog_html(
    config: DooplaySiteConfig,
    html: &str,
    page: u32,
) -> Result<CatalogPage, SourceError> {
    let document = Html::parse_document(html);
    let article_sel = Selector::parse("article.item, article.boxAN").unwrap();
    let link_sel = Selector::parse("a[href]").unwrap();
    let title_sel = Selector::parse("h3, .title, div.title").unwrap();
    let img_sel = Selector::parse("img").unwrap();

    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for article in document.select(&article_sel) {
        let link = article
            .select(&link_sel)
            .find(|a| {
                a.value()
                    .attr("href")
                    .map(|h| h.contains(config.anime_prefix))
                    .unwrap_or(false)
            })
            .and_then(|a| a.value().attr("href"));
        let Some(href) = link else { continue };
        let url = DooplayClient::normalize_url(config, href);
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
            .map(|s| DooplayClient::normalize_url(config, s));

        items.push(CatalogItem {
            title,
            url,
            poster,
            category: None,
        });
    }

    if items.is_empty() {
        collect_catalog_links(config, &document, &mut items, &mut seen);
    }

    let has_next = parse_has_next(html, page, config);

    Ok(CatalogPage {
        items,
        page,
        has_next,
    })
}

fn collect_catalog_links(
    config: DooplaySiteConfig,
    document: &Html,
    items: &mut Vec<CatalogItem>,
    seen: &mut std::collections::HashSet<String>,
) {
    let link_sel = Selector::parse("a[href]").unwrap();
    let img_sel = Selector::parse("img").unwrap();

    for link in document.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        if !href.contains(config.anime_prefix) || href.contains(config.episode_prefix) {
            continue;
        }
        let url = DooplayClient::normalize_url(config, href);
        if !seen.insert(url.clone()) {
            continue;
        }

        let title = link
            .select(&img_sel)
            .next()
            .and_then(|img| img.value().attr("alt"))
            .map(|s| s.to_string())
            .or_else(|| {
                let t = link.text().collect::<String>().trim().to_string();
                if t.is_empty() { None } else { Some(t) }
            })
            .unwrap_or_else(|| "Anime".to_string());

        let poster = link
            .select(&img_sel)
            .next()
            .and_then(|img| img.value().attr("src"))
            .map(|s| DooplayClient::normalize_url(config, s));

        items.push(CatalogItem {
            title,
            url,
            poster,
            category: None,
        });
    }
}

fn parse_has_next(html: &str, page: u32, config: DooplaySiteConfig) -> bool {
    let next_page = page + 1;
    if html.contains(&format!("{}/page/{next_page}/", config.catalog_prefix.trim_end_matches('/')))
        || html.contains(&format!("{}{next_page}/", config.catalog_prefix))
        || html.contains(&format!("page/{next_page}"))
    {
        return true;
    }
    if let Ok(re) = Regex::new(r#"Página\s+\d+\s+de\s+(\d+)"#) {
        if let Some(caps) = re.captures(html) {
            if let Some(total) = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok()) {
                return page < total;
            }
        }
    }
    false
}

pub async fn search_catalog(
    client: &DooplayClient,
    query: &str,
    page: u32,
) -> Result<CatalogPage, SourceError> {
    let config = client.config();
    let encoded = encode(query);
    let path = if page <= 1 {
        format!("/?s={encoded}")
    } else {
        format!("/?s={encoded}&paged={page}")
    };
    let html = client.get(&path).await?;
    parse_catalog_html(config, &html, page)
}

pub async fn parse_categories(
    client: &DooplayClient,
) -> Result<Vec<CategoryInfo>, SourceError> {
    let config = client.config();
    let html = client.get(config.genres_path).await?;
    let document = Html::parse_document(&html);
    let link_sel = Selector::parse("a[href]").unwrap();

    let mut categories = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for link in document.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        if !href.contains(config.genres_path) || href.ends_with(config.genres_path) {
            continue;
        }
        let url = DooplayClient::normalize_url(config, href);
        let slug = href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();
        if slug.is_empty() || slug.starts_with("letra-") || !seen.insert(slug.clone()) {
            continue;
        }
        let name = link.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            continue;
        }
        categories.push(CategoryInfo { name, slug, url });
    }

    categories.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(categories)
}

pub async fn browse(req: &BrowseRequest, config: DooplaySiteConfig) -> Result<CatalogPage, SourceError> {
    let client = DooplayClient::new(config)?;
    let result = browse_catalog(
        &client,
        req.catalog_type.clone(),
        req.page,
        req.category_slug.as_deref(),
    )
    .await?;
    Ok(apply_catalog_filters(result, &req.filters))
}

pub async fn search(req: &crate::models::SearchRequest, config: DooplaySiteConfig) -> Result<CatalogPage, SourceError> {
    let client = DooplayClient::new(config)?;
    let result = search_catalog(&client, &req.query, req.page).await?;
    Ok(apply_catalog_filters(result, &req.filters))
}

pub async fn categories_list(config: DooplaySiteConfig) -> Result<Vec<CategoryInfo>, SourceError> {
    let client = DooplayClient::new(config)?;
    parse_categories(&client).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::dooplay::config::AOCC_CONFIG;

    #[test]
    fn parse_catalog_extracts_items() {
        let html = r#"
        <article class="item">
          <a href="/anime/test-anime/"><img src="/poster.jpg"/><h3>Test Anime</h3></a>
        </article>
        "#;
        let page = parse_catalog_html(AOCC_CONFIG, html, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].url.contains("/anime/test-anime"));
    }

    #[tokio::test]
    #[ignore = "rede: cargo test parse_aocc_one_piece -- --ignored --nocapture"]
    async fn parse_aocc_one_piece() {
        let client = DooplayClient::new(AOCC_CONFIG).unwrap();
        let info = parse_anime_page(
            &client,
            "https://animesonlinecc.to/anime/one-piece/",
        )
        .await
        .expect("anime");
        assert!(info.title.to_lowercase().contains("one piece"));
        assert!(!info.seasons.is_empty());
        assert!(!info.seasons[0].episodes.is_empty());
    }

    #[tokio::test]
    #[ignore = "rede: cargo test parse_meusanimes_one_piece -- --ignored --nocapture"]
    async fn parse_meusanimes_one_piece() {
        use crate::sources::dooplay::config::MEUSANIMES_CONFIG;
        let client = DooplayClient::new(MEUSANIMES_CONFIG).unwrap();
        let info = parse_anime_page(
            &client,
            "https://meusanimes.blog/a/one-piece-1/",
        )
        .await
        .expect("anime");
        assert!(info.title.to_lowercase().contains("one piece"));
        assert!(!info.seasons.is_empty());
    }

    #[tokio::test]
    #[ignore = "rede: cargo test browse_meusanimes_catalog -- --ignored --nocapture"]
    async fn browse_meusanimes_catalog() {
        use crate::models::{
            AnimeSourceId, BrowseRequest, CatalogFilters, CatalogSort, CatalogType, MediaFilter,
        };
        use crate::sources::meusanimes;

        let req = BrowseRequest {
            catalog_type: CatalogType::Animes,
            page: 1,
            category_slug: None,
            filters: CatalogFilters {
                media_filter: MediaFilter::Anime,
                sort: CatalogSort::Default,
                category: None,
                title_filter: None,
            },
            source: AnimeSourceId::Meusanimes,
        };
        let page = meusanimes::browse(&req).await.expect("catalog");
        assert!(!page.items.is_empty(), "catalog should have items after filters");
        assert!(page.items[0].url.contains("/a/"));
    }
}

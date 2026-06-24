use crate::models::{
    AnimeInfo, BrowseRequest, CatalogItem, CatalogPage, CatalogType, CategoryInfo, EpisodeInfo,
    SeasonInfo,
};
use crate::sources::animesdigital::client::AnimesdigitalClient;
use crate::sources::SourceError;
use crate::sushi::apply_catalog_filters;
use regex::Regex;
use scraper::{Html, Selector};
use urlencoding::encode;

pub async fn parse_anime_page(
    client: &AnimesdigitalClient,
    url: &str,
) -> Result<AnimeInfo, SourceError> {
    let normalized = AnimesdigitalClient::normalize_url(url);
    let html = client.get(&normalized).await?;
    parse_anime_html(&html, &normalized)
}

pub fn parse_anime_html(html: &str, url: &str) -> Result<AnimeInfo, SourceError> {
    let document = Html::parse_document(html);

    let title = extract_meta(&document, "og:title")
        .or_else(|| extract_text(&document, "h1"))
        .unwrap_or_else(|| "Anime".to_string());
    let title = clean_title(&title);

    let poster = extract_poster(&document, html);
    let synopsis = extract_synopsis(&document);

    let episodes = parse_episodes(html)?;
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
    client: &AnimesdigitalClient,
    episode_url: &str,
) -> Result<String, SourceError> {
    let html = client.get(episode_url).await?;
    let re = Regex::new(r#"href="(https://animesdigital\.org/anime/[^"]+)""#)
        .map_err(|e| SourceError::Parse(e.to_string()))?;
    for caps in re.captures_iter(&html) {
        if let Some(m) = caps.get(1) {
            let url = m.as_str().to_string();
            if AnimesdigitalClient::is_anime_url(&url) {
                return Ok(url);
            }
        }
    }
    Err(SourceError::Parse(
        "Link do anime não encontrado na página do episódio".to_string(),
    ))
}

fn parse_episodes(html: &str) -> Result<Vec<EpisodeInfo>, SourceError> {
    let document = Html::parse_document(html);
    let link_sel = Selector::parse("a[href*='/video/a/']").unwrap();

    let mut episodes = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for link in document.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let url = AnimesdigitalClient::normalize_url(href);
        if !seen.insert(url.clone()) {
            continue;
        }

        let alt = link
            .select(&Selector::parse("img").unwrap())
            .next()
            .and_then(|img| img.value().attr("alt"))
            .unwrap_or("");
        let text = link.text().collect::<String>();
        let label = if !alt.is_empty() { alt } else { &text };

        let number = parse_episode_number(label, &url);
        episodes.push(EpisodeInfo {
            number,
            season: 1,
            title: if label.trim().is_empty() {
                format!("Episódio {number}")
            } else {
                label.trim().to_string()
            },
            description: None,
            url,
        });
    }

    episodes.sort_by_key(|e| e.number);
    if episodes.is_empty() {
        return Err(SourceError::Parse("Nenhum episódio encontrado".to_string()));
    }
    Ok(episodes)
}

fn parse_episode_number(label: &str, url: &str) -> u32 {
    for text in [label, url] {
        if let Ok(re) = Regex::new(r"(?i)epis[oó]dio\s*(\d+)") {
            if let Some(caps) = re.captures(text) {
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

fn extract_poster(document: &Html, html: &str) -> Option<String> {
    if let Ok(sel) = Selector::parse(".poster img, .animeInfos img") {
        if let Some(img) = document.select(&sel).next() {
            if let Some(src) = img.value().attr("src") {
                return Some(AnimesdigitalClient::normalize_url(src));
            }
        }
    }
    if let Ok(re) = Regex::new(r#"<meta property="og:image" content="([^"]+)""#) {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                return Some(AnimesdigitalClient::normalize_url(m.as_str()));
            }
        }
    }
    None
}

fn extract_synopsis(document: &Html) -> Option<String> {
    if let Ok(sel) = Selector::parse(".sinopse, .descricao, .animeInfos .texto") {
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
    client: &AnimesdigitalClient,
    catalog_type: CatalogType,
    page: u32,
    category_slug: Option<&str>,
) -> Result<CatalogPage, SourceError> {
    match catalog_type {
        CatalogType::Category => {
            let slug = category_slug.unwrap_or("acao");
            let path = if page <= 1 {
                format!("/genero/{slug}/")
            } else {
                format!("/genero/{slug}/?pagina={page}")
            };
            let html = client.get(&path).await?;
            return parse_catalog_html(&html, page);
        }
        CatalogType::Filmes => {
            return fetch_listing_page(client, "/filmes/", "filmes", "0", page, 30).await;
        }
        CatalogType::Animes => {
            return fetch_listing_page(
                client,
                "/animes-legendados-online",
                "animes",
                "legendado",
                page,
                30,
            )
            .await;
        }
    }
}

#[derive(serde::Deserialize)]
struct ListAnimeResponse {
    #[serde(default)]
    page: u32,
    #[serde(default)]
    total_page: u32,
    #[serde(default)]
    results: Vec<String>,
    #[serde(default)]
    code: Option<String>,
}

fn extract_list_token(html: &str) -> Result<String, SourceError> {
    let re = Regex::new(r#"data-secury="([^"]+)""#)
        .map_err(|e| SourceError::Parse(e.to_string()))?;
    re.captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| SourceError::Parse("Token do catálogo não encontrado".to_string()))
}

fn build_list_filters(type_url: &str, filter_audio: &str) -> String {
    serde_json::json!({
        "filter_data": format!(
            "filter_letter=0&type_url={type_url}&filter_audio={filter_audio}&filter_order=name"
        ),
        "filter_genre_add": [],
        "filter_genre_del": [],
    })
    .to_string()
}

async fn fetch_listing_page(
    client: &AnimesdigitalClient,
    listing_path: &str,
    type_url: &str,
    filter_audio: &str,
    page: u32,
    limit: u32,
) -> Result<CatalogPage, SourceError> {
    let referer = AnimesdigitalClient::normalize_url(listing_path);
    let page_html = client.get(listing_path).await?;
    let token = extract_list_token(&page_html)?;
    let filters = build_list_filters(type_url, filter_audio);

    let page_s = page.to_string();
    let limit_s = limit.to_string();
    let body = client
        .post_form(
            "/func/listanime",
            &referer,
            &[
                ("token", &token),
                ("pagina", &page_s),
                ("search", "0"),
                ("limit", &limit_s),
                ("type", "lista"),
                ("filters", &filters),
            ],
        )
        .await?;

    let response: ListAnimeResponse = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("Resposta do catálogo inválida: {e}")))?;

    if response.code.as_deref() == Some("no_verify_nonce") {
        return Err(SourceError::Parse("Token do catálogo rejeitado".to_string()));
    }

    let combined = response.results.join("");
    let mut catalog = parse_catalog_html(&combined, page)?;
    catalog.has_next = page < response.total_page.max(1);
    Ok(catalog)
}

pub fn parse_catalog_html(html: &str, page: u32) -> Result<CatalogPage, SourceError> {
    let document = Html::parse_document(html);
    let link_sel = Selector::parse("a[href*='/anime/'], .itemA a").unwrap();
    let title_sel = Selector::parse(".title_anime, .title span, img").unwrap();

    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for link in document.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        if href.contains("/video/") {
            continue;
        }
        let url = AnimesdigitalClient::normalize_url(href);
        if !url.contains("/anime/") || !seen.insert(url.clone()) {
            continue;
        }

        let title = link
            .select(&title_sel)
            .find_map(|el| {
                if el.value().name() == "img" {
                    el.value().attr("alt").map(|s| s.to_string())
                } else {
                    let t = el.text().collect::<String>().trim().to_string();
                    if t.is_empty() { None } else { Some(t) }
                }
            })
            .or_else(|| {
                link.value().attr("title").map(|s| s.to_string())
            })
            .map(|t| clean_list_title(&t))
            .unwrap_or_else(|| "Anime".to_string());

        let poster = link
            .select(&Selector::parse("img").unwrap())
            .next()
            .and_then(|img| img.value().attr("src"))
            .map(|s| AnimesdigitalClient::normalize_url(s));

        let category = if url.contains("/filme") {
            Some("Filme".to_string())
        } else {
            None
        };

        items.push(CatalogItem {
            title,
            url,
            poster,
            category,
        });
    }

    let has_next = html.contains(&format!("pagina={}", page + 1))
        || html.contains(&format!("page/{}/", page + 1))
        || html.contains(&format!("paged={}", page + 1));

    Ok(CatalogPage {
        items,
        page,
        has_next,
    })
}

fn clean_list_title(title: &str) -> String {
    title
        .trim()
        .strip_prefix("Assistir ")
        .unwrap_or(title)
        .split(" Online")
        .next()
        .unwrap_or(title)
        .trim()
        .to_string()
}

pub async fn search_catalog(
    client: &AnimesdigitalClient,
    query: &str,
    page: u32,
) -> Result<CatalogPage, SourceError> {
    let listing_path = "/animes-legendados-online";
    let referer = AnimesdigitalClient::normalize_url(listing_path);
    let page_html = client.get(listing_path).await?;
    let token = extract_list_token(&page_html)?;
    let filters = build_list_filters("animes", "legendado");
    let encoded_query = encode(query).to_string();

    let page_s = page.to_string();
    let body = client
        .post_form(
            "/func/listanime",
            &referer,
            &[
                ("token", &token),
                ("pagina", &page_s),
                ("search", &encoded_query),
                ("limit", "30"),
                ("type", "lista"),
                ("filters", &filters),
            ],
        )
        .await?;

    let response: ListAnimeResponse = serde_json::from_str(&body)
        .map_err(|e| SourceError::Parse(format!("Resposta da busca inválida: {e}")))?;

    let combined = response.results.join("");
    let mut catalog = parse_catalog_html(&combined, page)?;
    catalog.has_next = page < response.total_page.max(1);
    Ok(catalog)
}

pub async fn parse_categories(client: &AnimesdigitalClient) -> Result<Vec<CategoryInfo>, SourceError> {
    let html = client.get("/generos/").await?;
    let document = Html::parse_document(&html);
    let link_sel = Selector::parse("a[href*='/genero/']").unwrap();

    let mut categories = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for link in document.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let url = AnimesdigitalClient::normalize_url(href);
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
        categories.push(CategoryInfo { name, slug, url });
    }

    categories.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(categories)
}

pub async fn browse(req: &BrowseRequest) -> Result<CatalogPage, SourceError> {
    let client = AnimesdigitalClient::new()?;
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
    let client = AnimesdigitalClient::new()?;
    let result = search_catalog(&client, &req.query, req.page).await?;
    Ok(apply_catalog_filters(result, &req.filters))
}

pub async fn categories_list() -> Result<Vec<CategoryInfo>, SourceError> {
    let client = AnimesdigitalClient::new()?;
    parse_categories(&client).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_catalog_extracts_anime_links() {
        let html = r#"
        <div class="itemA"><a href="https://animesdigital.org/anime/a/test-anime">
          <img src="/poster.jpg" alt="Assistir Test Anime Online em HD"/>
          <span class="title_anime">Test Anime</span>
        </a></div>
        "#;
        let page = parse_catalog_html(html, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].url.contains("test-anime"));
        assert_eq!(page.items[0].title, "Test Anime");
    }

    #[test]
    fn parse_catalog_extracts_legacy_anime_b_links() {
        let html = r#"
        <a href="https://animesdigital.org/anime/b/test-anime/">
          <img src="/poster.jpg" alt="Test Anime"/>
        </a>
        "#;
        let page = parse_catalog_html(html, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].url.contains("test-anime"));
    }

    #[tokio::test]
    #[ignore = "rede: cargo test browse_animesdigital_catalog -- --ignored --nocapture"]
    async fn browse_animesdigital_catalog() {
        let client = AnimesdigitalClient::new().unwrap();
        let page = browse_catalog(&client, CatalogType::Animes, 1, None)
            .await
            .expect("catalog");
        assert!(!page.items.is_empty(), "catalog should have items");
        assert!(page.items[0].url.contains("/anime/"));
    }

    #[tokio::test]
    #[ignore = "rede: cargo test parse_animesdigital_one_piece -- --ignored --nocapture"]
    async fn parse_animesdigital_one_piece() {
        let client = AnimesdigitalClient::new().unwrap();
        let info = parse_anime_page(
            &client,
            "https://animesdigital.org/anime/b/one-piece-todos-episodios-5/",
        )
        .await
        .expect("anime");
        assert!(info.title.to_lowercase().contains("one piece"));
        assert!(!info.seasons.is_empty());
        assert!(!info.seasons[0].episodes.is_empty());
    }
}

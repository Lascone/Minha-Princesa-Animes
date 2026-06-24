use crate::models::{
    AnimeInfo, CatalogFilters, CatalogItem, CatalogPage, CatalogSort, CatalogType, CategoryInfo,
    EpisodeInfo, MediaFilter, SeasonInfo,
};
use crate::sushi::client::{SushiClient, SushiError};
use regex::Regex;
use scraper::{Html, Selector};

pub async fn parse_anime_page(client: &SushiClient, url: &str) -> Result<AnimeInfo, SushiError> {
    let normalized = SushiClient::normalize_url(url);
    let fetch_url = if SushiClient::is_movie_url(&normalized) {
        SushiClient::resolve_movie_watch_url(&normalized)
    } else {
        normalized
    };
    let html = client.get(&fetch_url).await?;
    parse_anime_html(&html, &fetch_url)
}

pub fn parse_anime_html(html: &str, url: &str) -> Result<AnimeInfo, SushiError> {
    let document = Html::parse_document(html);

    let title = extract_meta(&document, "og:title")
        .or_else(|| extract_title_tag(&document))
        .unwrap_or_else(|| "Anime".to_string());
    let title = clean_anime_title(&title);

    let poster = extract_poster(&document, html);
    let synopsis = extract_meta(&document, "og:description")
        .or_else(|| extract_meta(&document, "description"));

    let seasons = match parse_seasons(&document) {
        Ok(seasons) => seasons,
        Err(_) if SushiClient::is_movie_url(url) => parse_movie_seasons(html, url, &title)?,
        Err(e) => return Err(e),
    };

    Ok(AnimeInfo {
        title,
        url: url.to_string(),
        poster,
        synopsis,
        seasons,
    })
}

fn parse_seasons(document: &Html) -> Result<Vec<SeasonInfo>, SushiError> {
    let pane_sel = Selector::parse(".epx-root .tab-pane, .tab-pane[id^='season-']").unwrap();
    let card_sel = Selector::parse("a.epx-card.js-epx-item, a.epx-card").unwrap();
    let title_sel = Selector::parse(".epx-title").unwrap();
    let desc_sel = Selector::parse(".epx-desc").unwrap();

    let mut seasons_map: std::collections::BTreeMap<u32, Vec<EpisodeInfo>> =
        std::collections::BTreeMap::new();
    let mut seen_urls = std::collections::HashSet::new();
    let mut seen_keys = std::collections::HashSet::new();

    let mut parsed_from_panes = false;

    for pane in document.select(&pane_sel) {
        let pane_season = pane
            .value()
            .attr("id")
            .and_then(extract_season_number)
            .unwrap_or(1);

        for card in pane.select(&card_sel) {
            if let Some(episode) = parse_episode_card(
                card,
                &title_sel,
                &desc_sel,
                pane_season,
                &mut seen_urls,
                &mut seen_keys,
            ) {
                seasons_map
                    .entry(episode.season)
                    .or_default()
                    .push(episode);
                parsed_from_panes = true;
            }
        }
    }

    if !parsed_from_panes {
        for card in document.select(&card_sel) {
            if let Some(episode) = parse_episode_card(
                card,
                &title_sel,
                &desc_sel,
                0,
                &mut seen_urls,
                &mut seen_keys,
            ) {
                seasons_map
                    .entry(episode.season)
                    .or_default()
                    .push(episode);
            }
        }
    }

    let mut seasons: Vec<SeasonInfo> = seasons_map
        .into_iter()
        .map(|(number, mut episodes)| {
            episodes.sort_by_key(|e| e.number);
            SeasonInfo { number, episodes }
        })
        .collect();

    seasons.sort_by_key(|s| s.number);

    if seasons.is_empty() {
        return Err(SushiError::Parse(
            "Nenhum episódio encontrado na página".to_string(),
        ));
    }

    Ok(seasons)
}

fn parse_episode_card(
    card: scraper::ElementRef<'_>,
    title_sel: &Selector,
    desc_sel: &Selector,
    pane_season: u32,
    seen_urls: &mut std::collections::HashSet<String>,
    seen_keys: &mut std::collections::HashSet<(u32, u32)>,
) -> Option<EpisodeInfo> {
    let href = card.value().attr("href")?;
    if href.is_empty() || !href.contains("-season-") {
        return None;
    }

    let ep_url = SushiClient::normalize_url(href);
    if !seen_urls.insert(ep_url.clone()) {
        return None;
    }

    let (url_batch, episode_number) = parse_episode_url(&ep_url)?;

    if pane_season > 0 && url_batch != pane_season {
        return None;
    }

    let season = if pane_season > 0 {
        pane_season
    } else {
        url_batch
    };

    let ep_title = card
        .select(title_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .unwrap_or_else(|| format!("Episódio {episode_number}"));

    let description = card
        .select(desc_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty());

    let episode_number = if episode_number > 0 {
        episode_number
    } else {
        extract_episode_number(&ep_title)?
    };

    let dedupe_key = (season, episode_number);
    if !seen_keys.insert(dedupe_key) {
        return None;
    }

    Some(EpisodeInfo {
        number: episode_number,
        season,
        title: description.clone().unwrap_or(ep_title),
        description,
        url: ep_url,
    })
}

fn extract_season_number(id: &str) -> Option<u32> {
    let re = Regex::new(r"season-(\d+)").ok()?;
    re.captures(id)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn parse_movie_seasons(html: &str, url: &str, title: &str) -> Result<Vec<SeasonInfo>, SushiError> {
    let _ = parse_embed_id_from_html(html)?;
    Ok(vec![SeasonInfo {
        number: 1,
        episodes: vec![EpisodeInfo {
            number: 1,
            season: 1,
            title: title.to_string(),
            description: None,
            url: url.to_string(),
        }],
    }])
}

pub async fn parse_episode_embed_id(client: &SushiClient, url: &str) -> Result<String, SushiError> {
    let html = client.get(url).await?;
    parse_embed_id_from_html(&html)
}

pub fn parse_embed_id_from_html(html: &str) -> Result<String, SushiError> {
    let re_selected =
        Regex::new(r#"class="[^"]*dropdown-source[^"]*selected[^"]*"[^>]*data-embed="(\d+)""#)
            .map_err(|e| SushiError::Parse(e.to_string()))?;
    if let Some(caps) = re_selected.captures(html) {
        return Ok(caps[1].to_string());
    }

    let re_embed = Regex::new(r#"data-embed="(\d+)""#)
        .map_err(|e| SushiError::Parse(e.to_string()))?;
    re_embed
        .captures(html)
        .map(|c| c[1].to_string())
        .ok_or_else(|| SushiError::Parse("Embed ID não encontrado".to_string()))
}

pub async fn browse_catalog(
    client: &SushiClient,
    catalog_type: CatalogType,
    page: u32,
    category_slug: Option<&str>,
) -> Result<CatalogPage, SushiError> {
    let path = match catalog_type {
        CatalogType::Animes => format!("/animes?page={page}"),
        CatalogType::Filmes => format!("/filmes?page={page}"),
        CatalogType::Category => {
            let slug = category_slug.unwrap_or("acao");
            format!("/category/{slug}?page={page}")
        }
    };

    let html = client.get(&path).await?;
    parse_catalog_html(&html, page)
}

pub async fn search_catalog(
    client: &SushiClient,
    query: &str,
    page: u32,
) -> Result<CatalogPage, SushiError> {
    let html = if page <= 1 {
        client.post_search(query).await?
    } else {
        let path = format!(
            "/search?q={}&page={}",
            urlencoding::encode(query),
            page
        );
        client.get(&path).await?
    };
    parse_catalog_html(&html, page)
}

pub async fn parse_categories(client: &SushiClient) -> Result<Vec<CategoryInfo>, SushiError> {
    let html = client.get("/categories").await?;
    let document = Html::parse_document(&html);
    let sel = Selector::parse("a[href*='/category/']").unwrap();

    let mut categories = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for el in document.select(&sel) {
        let href = el.value().attr("href").unwrap_or("");
        let url = SushiClient::normalize_url(href);
        let slug = href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();
        if slug.is_empty() || !seen.insert(slug.clone()) {
            continue;
        }
        let name = el.text().collect::<String>().trim().to_string();
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

pub fn apply_catalog_filters(mut page: CatalogPage, filters: &CatalogFilters) -> CatalogPage {
    if let Some(ref category) = filters.category {
        let category = category.to_lowercase();
        page.items.retain(|item| {
            item.category
                .as_ref()
                .map(|c| c.to_lowercase().contains(&category))
                .unwrap_or(false)
        });
    }

    if let Some(ref title_filter) = filters.title_filter {
        let needle = title_filter.to_lowercase();
        if !needle.is_empty() {
            page.items
                .retain(|item| item.title.to_lowercase().contains(&needle));
        }
    }

    match filters.media_filter {
        MediaFilter::All => {}
        MediaFilter::Anime => page.items.retain(|item| item.url.contains("/anime/")),
        MediaFilter::Filme => {
            page.items
                .retain(|item| item.url.contains("/filme/") || item.url.contains("/assistir/"))
        }
    }

    match filters.sort {
        CatalogSort::Default => {}
        CatalogSort::TitleAsc => {
            page.items
                .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        }
        CatalogSort::TitleDesc => {
            page.items
                .sort_by(|a, b| b.title.to_lowercase().cmp(&a.title.to_lowercase()))
        }
    }

    page
}

fn is_catalog_media_href(href: &str) -> bool {
    if href.contains("-season-") || href.contains("-episode") {
        return false;
    }
    href.contains("/anime/") || href.contains("/filme/") || href.contains("/assistir/")
}

fn parse_catalog_html(html: &str, page: u32) -> Result<CatalogPage, SushiError> {
    let document = Html::parse_document(html);

    let media_sel = Selector::parse("a.list-media, a.list-movie, a[href*='/anime/'], a[href*='/filme/'], a[href*='/assistir/']").unwrap();
    let title_sel = Selector::parse(".list-title").unwrap();
    let category_sel = Selector::parse(".list-category").unwrap();
    let poster_sel = Selector::parse(".media-cover, img").unwrap();

    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for media in document.select(&media_sel) {
        let href = media.value().attr("href").unwrap_or("");
        if !is_catalog_media_href(href) {
            continue;
        }
        let url = SushiClient::normalize_url(href);
        if !seen.insert(url.clone()) {
            continue;
        }

        let title = document
            .select(&title_sel)
            .find(|t| {
                t.text()
                    .collect::<String>()
                    .len()
                    > 0
            })
            .map(|t| t.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "Sem título".to_string());

        // Find title near this link's parent
        let title = find_nearby_title(media, &title_sel).unwrap_or(title);

        let category = find_nearby_category(media, &category_sel);
        let poster = media
            .select(&poster_sel)
            .next()
            .and_then(|img| {
                img.value()
                    .attr("data-src")
                    .or_else(|| img.value().attr("src"))
                    .map(|s| SushiClient::normalize_url(s))
            });

        items.push(CatalogItem {
            title,
            url,
            poster,
            category,
        });
    }

    // Alternative layout: list items with title links
    if items.is_empty() {
        let link_sel = Selector::parse("a.list-title, a[href*='/anime/'], a[href*='/filme/'], a[href*='/assistir/']").unwrap();
        for link in document.select(&link_sel) {
            let href = link.value().attr("href").unwrap_or("");
            if !is_catalog_media_href(href) {
                continue;
            }
            let url = SushiClient::normalize_url(href);
            if !seen.insert(url.clone()) {
                continue;
            }
            let title = link.text().collect::<String>().trim().to_string();
            if title.is_empty() {
                continue;
            }
            items.push(CatalogItem {
                title,
                url,
                poster: None,
                category: None,
            });
        }
    }

    let has_next = html.contains(&format!("page={}", page + 1))
        || html.contains("pagination-next")
        || html.contains("Próxima")
        || html.contains("Proxima");

    Ok(CatalogPage {
        items,
        page,
        has_next,
    })
}

fn find_nearby_title(
    media: scraper::ElementRef<'_>,
    title_sel: &Selector,
) -> Option<String> {
    let parent = media.parent()?.parent()?;
    let el_ref = scraper::ElementRef::wrap(parent)?;
    el_ref
        .select(title_sel)
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string())
}

fn find_nearby_category(
    media: scraper::ElementRef<'_>,
    category_sel: &Selector,
) -> Option<String> {
    let parent = media.parent()?.parent()?;
    let el_ref = scraper::ElementRef::wrap(parent)?;
    el_ref
        .select(category_sel)
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string())
}

fn extract_poster(document: &Html, _html: &str) -> Option<String> {
    if let Ok(cover_sel) = Selector::parse(".media-cover") {
        if let Some(cover) = document.select(&cover_sel).next() {
            if let Some(src) = cover
                .value()
                .attr("data-src")
                .or_else(|| cover.value().attr("src"))
            {
                return Some(SushiClient::normalize_url(src));
            }
        }
    }
    extract_meta(document, "og:image")
}

fn extract_meta(document: &Html, property: &str) -> Option<String> {
    let sel = Selector::parse(&format!("meta[property='{property}'], meta[name='{property}']"))
        .ok()?;
    document
        .select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_title_tag(document: &Html) -> Option<String> {
    let sel = Selector::parse("title").ok()?;
    document
        .select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
}

fn clean_anime_title(title: &str) -> String {
    title
        .replace("Assisitir ", "")
        .replace("Assistir ", "")
        .split('–')
        .next()
        .unwrap_or(title)
        .split('-')
        .next()
        .unwrap_or(title)
        .trim()
        .to_string()
}

pub fn parse_episode_url(url: &str) -> Option<(u32, u32)> {
    // SushiAnimes URL: {slug}-{season}-season-{episode}-episode
    let re = Regex::new(r"-(\d+)-season-(\d+)-episode").ok()?;
    let mut last = None;
    for caps in re.captures_iter(url) {
        let season: u32 = caps.get(1)?.as_str().parse().ok()?;
        let episode: u32 = caps.get(2)?.as_str().parse().ok()?;
        last = Some((season, episode));
    }
    last
}

fn extract_episode_number(title: &str) -> Option<u32> {
    let re = Regex::new(r"(\d+)").ok()?;
    re.captures(title)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sushi::client::SushiClient;
    use crate::sushi::embed::parse_stream_from_embed;

    #[tokio::test]
    async fn catalog_animes_have_posters() {
        let client = SushiClient::new().unwrap();
        let page = browse_catalog(&client, CatalogType::Animes, 1, None)
            .await
            .expect("browse");
        assert!(!page.items.is_empty());
        let with_poster = page.items.iter().filter(|i| i.poster.is_some()).count();
        assert!(
            with_poster > 0,
            "expected some catalog items with poster, got {with_poster}/{}",
            page.items.len()
        );
    }

    #[test]
    fn parse_episode_url_reads_season_then_episode() {
        let url = "https://sushianimes.com.br/anime/re-zero-kara-hajimeru-isekai-seikatsu-dublado-989-1-season-5-episode";
        let (season, episode) = parse_episode_url(url).unwrap();
        assert_eq!(season, 1);
        assert_eq!(episode, 5);
    }

    #[test]
    fn parse_episode_url_season_two_episode_two() {
        let url = "https://sushianimes.com.br/anime/show-10-2-season-2-episode";
        let (season, episode) = parse_episode_url(url).unwrap();
        assert_eq!(season, 2);
        assert_eq!(episode, 2);
    }

    #[tokio::test]
    async fn parse_owari_season_one_has_multiple_eps_same_season() {
        let client = SushiClient::new().unwrap();
        let url = "https://sushianimes.com.br/anime/owari-no-seraph-1052";
        let info = parse_anime_page(&client, url).await.expect("parse owari");
        let s1 = info
            .seasons
            .iter()
            .find(|s| s.number == 1)
            .expect("season 1");
        assert!(
            s1.episodes.len() >= 2,
            "season 1 should have multiple episodes"
        );
        for ep in &s1.episodes {
            assert_eq!(
                ep.season, 1,
                "episode {} should be in season 1, not {}",
                ep.number, ep.season
            );
        }
    }

    #[tokio::test]
    async fn parse_overlord_lists_episodes() {
        let client = SushiClient::new().unwrap();
        let url = "https://sushianimes.com.br/anime/overlord-175";
        let info = parse_anime_page(&client, url).await.expect("parse overlord");
        let total: usize = info.seasons.iter().map(|s| s.episodes.len()).sum();
        assert!(total >= 4, "overlord should list episodes, got {total}");
    }

    #[test]
    fn is_catalog_media_href_rejects_episode_links() {
        assert!(!is_catalog_media_href("/anime/foo-1-season-1-episode"));
        assert!(is_catalog_media_href("/anime/overlord-175"));
        assert!(is_catalog_media_href("/filme/some-movie"));
        assert!(is_catalog_media_href("/assistir/some-movie"));
    }

    #[tokio::test]
    async fn parse_movie_page_has_single_episode() {
        let client = SushiClient::new().unwrap();
        let url = "https://sushianimes.com.br/assistir/sword-art-online-progressive-scherzo-of-deep-night-legendado-1234";
        if client.get(url).await.is_err() {
            return;
        }
        let info = parse_anime_page(&client, url).await.expect("parse movie");
        assert_eq!(info.seasons.len(), 1);
        assert_eq!(info.seasons[0].episodes.len(), 1);
    }

    #[tokio::test]
    async fn resolve_rezero_stream_is_hls_or_mp4() {
        let client = SushiClient::new().unwrap();
        let url = "https://sushianimes.com.br/anime/re-zero-kara-hajimeru-isekai-seikatsu-dublado-989-1-season-1-episode";
        let embed_id = parse_episode_embed_id(&client, url).await.unwrap();
        let stream = crate::sushi::resolve_stream_url(&client, &embed_id)
            .await
            .unwrap();
        assert!(matches!(
            stream.kind,
            crate::sushi::StreamKind::Hls | crate::sushi::StreamKind::Mp4
        ));
    }

    #[test]
    fn embed_parser_finds_m3u8() {
        let html = r#"<source src="https://cdn.example.com/master.m3u8">"#;
        let info = parse_stream_from_embed(html).unwrap();
        assert!(info.url.contains(".m3u8"));
    }
}

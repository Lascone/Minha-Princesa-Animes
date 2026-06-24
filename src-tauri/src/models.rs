use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AnimeSourceId {
    #[default]
    Sushianimes,
    Goyabu,
    Meusanimes,
    Animesonlinecc,
    Animesdigital,
}

impl AnimeSourceId {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sushianimes => "Sushi Animes",
            Self::Goyabu => "Goyabu",
            Self::Meusanimes => "Meus Animes",
            Self::Animesonlinecc => "Animes Online CC",
            Self::Animesdigital => "Animes Digital",
        }
    }

    pub fn detect_from_url(url: &str) -> Option<Self> {
        let lower = url.to_lowercase();
        if lower.contains("meusanimes.blog") || lower.contains("meusanimes.") {
            return Some(Self::Meusanimes);
        }
        if lower.contains("animesonlinecc.to") {
            return Some(Self::Animesonlinecc);
        }
        if lower.contains("animesdigital.org") {
            return Some(Self::Animesdigital);
        }
        if lower.contains("goyabu.io") || lower.contains("goyabu.") {
            return Some(Self::Goyabu);
        }
        if lower.contains("sushianimes.com.br") {
            return Some(Self::Sushianimes);
        }
        None
    }
}

pub const APP_DATA_DIR: &str = "Minha Princesa Animes";
pub const APP_DOWNLOAD_DIR: &str = "Minha Princesa Animes";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeInfo {
    pub title: String,
    pub url: String,
    pub poster: Option<String>,
    pub synopsis: Option<String>,
    pub seasons: Vec<SeasonInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonInfo {
    pub number: u32,
    pub episodes: Vec<EpisodeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeInfo {
    pub number: u32,
    pub season: u32,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub title: String,
    pub url: String,
    pub poster: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage {
    pub items: Vec<CatalogItem>,
    pub page: u32,
    pub has_next: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInfo {
    pub name: String,
    pub slug: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub id: String,
    pub anime_title: String,
    pub episode_label: String,
    pub episode: EpisodeInfo,
    pub status: DownloadStatus,
    pub progress: f64,
    pub speed: String,
    pub output_path: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub download_folder: String,
    pub naming_template: String,
    pub max_concurrent: u32,
    pub ffmpeg_path: String,
    pub overwrite: bool,
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_folder = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(APP_DOWNLOAD_DIR)
            .to_string_lossy()
            .to_string();

        Self {
            download_folder,
            naming_template: "{anime}/Season {season}/{anime} - S{season:02}E{episode:02} - {title}.mp4"
                .to_string(),
            max_concurrent: 3,
            ffmpeg_path: String::new(),
            overwrite: false,
            theme: "dark".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub anime_title: String,
    pub episodes: Vec<EpisodeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CatalogType {
    Animes,
    Filmes,
    Category,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaFilter {
    #[default]
    All,
    Anime,
    Filme,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CatalogSort {
    #[default]
    Default,
    TitleAsc,
    TitleDesc,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CatalogFilters {
    #[serde(default)]
    pub media_filter: MediaFilter,
    #[serde(default)]
    pub sort: CatalogSort,
    pub category: Option<String>,
    pub title_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub page: u32,
    #[serde(default)]
    pub filters: CatalogFilters,
    #[serde(default)]
    pub source: AnimeSourceId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseRequest {
    pub catalog_type: CatalogType,
    pub page: u32,
    pub category_slug: Option<String>,
    #[serde(default)]
    pub filters: CatalogFilters,
    #[serde(default)]
    pub source: AnimeSourceId,
}

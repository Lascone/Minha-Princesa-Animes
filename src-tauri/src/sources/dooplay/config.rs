use crate::models::AnimeSourceId;

#[derive(Debug, Clone, Copy)]
pub struct DooplaySiteConfig {
    pub source_id: AnimeSourceId,
    pub base_url: &'static str,
    pub anime_prefix: &'static str,
    pub episode_prefix: &'static str,
    pub catalog_list_path: &'static str,
    pub catalog_prefix: &'static str,
    pub genres_path: &'static str,
}

pub const MEUSANIMES_CONFIG: DooplaySiteConfig = DooplaySiteConfig {
    source_id: AnimeSourceId::Meusanimes,
    base_url: "https://meusanimes.blog",
    anime_prefix: "/a/",
    episode_prefix: "/e/",
    catalog_list_path: "/g/legendado/",
    catalog_prefix: "/g/legendado/page/",
    genres_path: "/genero/",
};

pub const AOCC_CONFIG: DooplaySiteConfig = DooplaySiteConfig {
    source_id: AnimeSourceId::Animesonlinecc,
    base_url: "https://animesonlinecc.to",
    anime_prefix: "/anime/",
    episode_prefix: "/episodio/",
    catalog_list_path: "/anime/",
    catalog_prefix: "/anime/page/",
    genres_path: "/generos/",
};

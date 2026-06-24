pub mod client;
pub mod embed;
pub mod parser;

pub use embed::{resolve_stream_url, StreamKind};
pub use parser::{
    apply_catalog_filters, browse_catalog, parse_anime_page, parse_categories,
    parse_episode_embed_id, search_catalog,
};

use super::{AnimeSource, SourceError, SourceId};
use super::{goyabu, sushianimes};

pub struct SourceRegistry;

impl SourceRegistry {
    pub fn get(id: SourceId) -> Box<dyn AnimeSource + Send + Sync> {
        match id {
            SourceId::Sushianimes => Box::new(sushianimes::SushiSource),
            SourceId::Goyabu => Box::new(goyabu::GoyabuSource),
        }
    }

    pub fn for_url(url: &str) -> Result<SourceId, SourceError> {
        SourceId::detect_from_url(url).ok_or(SourceError::UnknownSource)
    }
}

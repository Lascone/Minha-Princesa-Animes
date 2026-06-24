use crate::models::{AppSettings, EpisodeInfo};
use regex::Regex;
use std::path::{Path, PathBuf};

pub fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*]"#).unwrap();
    re.replace_all(name, "")
        .replace('\0', "")
        .trim()
        .to_string()
}

pub fn build_output_path(
    settings: &AppSettings,
    anime_title: &str,
    episode: &EpisodeInfo,
) -> PathBuf {
    let anime = sanitize_filename(anime_title);
    let title = sanitize_filename(&episode.title);
    let season = episode.season;
    let ep = episode.number;

    let mut filename = settings.naming_template.clone();
    filename = filename.replace("{anime}", &anime);
    filename = filename.replace("{title}", &title);
    filename = apply_numeric_placeholder(&filename, "season", season);
    filename = apply_numeric_placeholder(&filename, "episode", ep);

    let base = Path::new(&settings.download_folder);
    let path = Path::new(&filename);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn apply_numeric_placeholder(template: &str, name: &str, value: u32) -> String {
    let plain = format!("{{{name}}}");
    let padded = format!("{{{name}:02}}");
    template
        .replace(&padded, &format!("{value:02}"))
        .replace(&plain, &value.to_string())
}

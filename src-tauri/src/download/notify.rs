use crate::models::{AppSettings, DownloadItem, DownloadStatus};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

fn notifications_enabled(settings: &AppSettings) -> bool {
    settings.notifications
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

pub fn on_status_changed(
    app: &AppHandle,
    settings: &AppSettings,
    item: &DownloadItem,
    previous: DownloadStatus,
) {
    if !notifications_enabled(settings) || previous == item.status {
        return;
    }

    match item.status {
        DownloadStatus::Completed => notify_completed(app, item),
        DownloadStatus::Failed => notify_failed(app, item),
        _ => {}
    }
}

pub fn notify_completed(app: &AppHandle, item: &DownloadItem) {
    let body = format!("{} — {}", item.anime_title, item.episode_label);
    let _ = app
        .notification()
        .builder()
        .title("Download concluído")
        .body(body)
        .show();

    let _ = app.emit("download-notification", serde_json::json!({
        "kind": "completed",
        "animeTitle": item.anime_title,
        "episodeLabel": item.episode_label,
    }));
}

pub fn notify_failed(app: &AppHandle, item: &DownloadItem) {
    let error = item
        .error
        .as_deref()
        .unwrap_or("Erro desconhecido");
    let body = format!(
        "{} — {}\n{}",
        item.anime_title,
        item.episode_label,
        truncate(error, 140)
    );
    let _ = app
        .notification()
        .builder()
        .title("Download falhou")
        .body(body)
        .show();

    let _ = app.emit("download-notification", serde_json::json!({
        "kind": "failed",
        "animeTitle": item.anime_title,
        "episodeLabel": item.episode_label,
        "error": error,
    }));
}

pub fn notify_queue_idle(app: &AppHandle, has_failures: bool) {
    let (title, body) = if has_failures {
        (
            "Fila de downloads finalizada",
            "Não há mais downloads em andamento. Verifique os episódios com erro na biblioteca.",
        )
    } else {
        (
            "Downloads finalizados",
            "Todos os episódios da fila foram baixados com sucesso.",
        )
    };

    let _ = app.notification().builder().title(title).body(body).show();

    let _ = app.emit("download-notification", serde_json::json!({
        "kind": "queue_idle",
        "hasFailures": has_failures,
    }));
}

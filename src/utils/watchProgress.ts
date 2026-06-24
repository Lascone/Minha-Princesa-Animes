import type { DownloadItem } from "../types";

const STORAGE_KEY = "minha-princesa-watch-progress";

export interface WatchProgress {
  outputPath: string;
  animeTitle: string;
  episodeLabel: string;
  downloadId: string;
  position: number;
  duration: number;
  updatedAt: number;
}

function readAll(): Record<string, WatchProgress> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, WatchProgress>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function writeAll(data: Record<string, WatchProgress>) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
}

export function watchProgressKey(item: DownloadItem): string {
  return item.outputPath ?? item.id;
}

export function loadWatchProgress(key: string): WatchProgress | null {
  return readAll()[key] ?? null;
}

export function saveWatchProgress(
  item: DownloadItem,
  position: number,
  duration: number
) {
  if (!item.outputPath || duration <= 0) return;

  const ratio = position / duration;
  if (ratio >= 0.92) {
    clearWatchProgress(watchProgressKey(item));
    return;
  }

  const key = watchProgressKey(item);
  const all = readAll();
  all[key] = {
    outputPath: item.outputPath,
    animeTitle: item.animeTitle,
    episodeLabel: item.episodeLabel,
    downloadId: item.id,
    position,
    duration,
    updatedAt: Date.now(),
  };
  writeAll(all);
}

export function clearWatchProgress(key: string) {
  const all = readAll();
  if (!(key in all)) return;
  delete all[key];
  writeAll(all);
}

export function formatPlaybackTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function findContinueWatching(
  library: DownloadItem[]
): { item: DownloadItem; progress: WatchProgress } | null {
  const all = readAll();
  const entries = Object.values(all)
    .filter((p) => {
      const ratio = p.duration > 0 ? p.position / p.duration : 0;
      return ratio > 0.02 && ratio < 0.92;
    })
    .sort((a, b) => b.updatedAt - a.updatedAt);

  for (const progress of entries) {
    const item = library.find(
      (d) =>
        d.status === "completed" &&
        d.outputPath === progress.outputPath &&
        d.id === progress.downloadId
    );
    if (item) return { item, progress };
  }
  return null;
}

import type { DownloadItem } from "../types";

const STORAGE_KEY = "minha-princesa-watch-progress";
const DISMISSED_KEY = "minha-princesa-watch-dismissed";
const PLAYER_PREFS_KEY = "minha-princesa-player-prefs";

export interface WatchProgress {
  outputPath: string;
  animeTitle: string;
  episodeLabel: string;
  downloadId: string;
  position: number;
  duration: number;
  updatedAt: number;
}

export interface PlayerPrefs {
  volume: number;
  speed: number;
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

function readDismissed(): Set<string> {
  try {
    const raw = localStorage.getItem(DISMISSED_KEY);
    if (!raw) return new Set();
    const parsed = JSON.parse(raw) as string[];
    return new Set(Array.isArray(parsed) ? parsed : []);
  } catch {
    return new Set();
  }
}

function writeDismissed(keys: Set<string>) {
  localStorage.setItem(DISMISSED_KEY, JSON.stringify([...keys]));
}

export function watchProgressKey(item: DownloadItem): string {
  return item.outputPath ?? item.id;
}

export function loadWatchProgress(key: string): WatchProgress | null {
  return readAll()[key] ?? null;
}

export function getSavedPosition(key: string): number | null {
  const progress = loadWatchProgress(key);
  if (!progress || progress.duration <= 0) return null;
  if (!shouldAutoResume(progress)) return null;
  if (readDismissed().has(key)) return null;
  return progress.position;
}

export function shouldAutoResume(progress: WatchProgress): boolean {
  if (progress.duration <= 0) return false;
  const ratio = progress.position / progress.duration;
  return ratio > 0.02 && ratio < 0.92;
}

export function dismissResumeForSession(key: string) {
  const dismissed = readDismissed();
  dismissed.add(key);
  writeDismissed(dismissed);
}

export function clearDismissedResume(key: string) {
  const dismissed = readDismissed();
  if (!dismissed.delete(key)) return;
  writeDismissed(dismissed);
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
    clearDismissedResume(watchProgressKey(item));
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

export function loadPlayerPrefs(): PlayerPrefs {
  try {
    const raw = localStorage.getItem(PLAYER_PREFS_KEY);
    if (!raw) return { volume: 1, speed: 1 };
    const parsed = JSON.parse(raw) as PlayerPrefs;
    return {
      volume: typeof parsed.volume === "number" ? parsed.volume : 1,
      speed: typeof parsed.speed === "number" ? parsed.speed : 1,
    };
  } catch {
    return { volume: 1, speed: 1 };
  }
}

export function savePlayerPrefs(prefs: Partial<PlayerPrefs>) {
  const current = loadPlayerPrefs();
  localStorage.setItem(
    PLAYER_PREFS_KEY,
    JSON.stringify({ ...current, ...prefs })
  );
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
  const dismissed = readDismissed();
  const all = readAll();
  const entries = Object.values(all)
    .filter((p) => shouldAutoResume(p) && !dismissed.has(p.outputPath))
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

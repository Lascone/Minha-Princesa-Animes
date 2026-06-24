import type { DownloadItem } from "../types";

export function sortEpisodes(a: DownloadItem, b: DownloadItem): number {
  if (a.episode.season !== b.episode.season) {
    return a.episode.season - b.episode.season;
  }
  return a.episode.number - b.episode.number;
}

export function findNextEpisode(
  current: DownloadItem,
  library: DownloadItem[]
): DownloadItem | null {
  const playable = library
    .filter(
      (d) =>
        d.animeTitle === current.animeTitle &&
        d.status === "completed" &&
        !!d.outputPath
    )
    .sort(sortEpisodes);

  const idx = playable.findIndex((d) => d.id === current.id);
  if (idx < 0 || idx >= playable.length - 1) return null;
  return playable[idx + 1] ?? null;
}

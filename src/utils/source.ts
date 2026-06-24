import type { AnimeSourceId } from "../types";

export function detectSourceFromUrl(url: string): AnimeSourceId | null {
  const lower = url.trim().toLowerCase();
  if (lower.includes("goyabu.io") || lower.includes("goyabu.")) {
    return "goyabu";
  }
  if (lower.includes("sushianimes.com.br")) {
    return "sushianimes";
  }
  return null;
}

export function sourceLabel(source: AnimeSourceId): string {
  switch (source) {
    case "goyabu":
      return "Goyabu";
    case "sushianimes":
      return "Sushi Animes";
  }
}

export function sourceSupportsFilmes(source: AnimeSourceId): boolean {
  return source === "sushianimes";
}

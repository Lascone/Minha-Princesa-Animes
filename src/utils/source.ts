import type { AnimeSourceId } from "../types";

export const ALL_SOURCES: AnimeSourceId[] = [
  "sushianimes",
  "goyabu",
  "meusanimes",
  "animesonlinecc",
  "animesdigital",
];

export function detectSourceFromUrl(url: string): AnimeSourceId | null {
  const lower = url.trim().toLowerCase();
  if (lower.includes("meusanimes.blog") || lower.includes("meusanimes.")) {
    return "meusanimes";
  }
  if (lower.includes("animesonlinecc.to")) {
    return "animesonlinecc";
  }
  if (lower.includes("animesdigital.org")) {
    return "animesdigital";
  }
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
    case "meusanimes":
      return "Meus Animes";
    case "animesonlinecc":
      return "Animes Online CC";
    case "animesdigital":
      return "Animes Digital";
    case "goyabu":
      return "Goyabu";
    case "sushianimes":
      return "Sushi Animes";
  }
}

export function sourceSupportsFilmes(source: AnimeSourceId): boolean {
  return source === "sushianimes" || source === "animesdigital";
}

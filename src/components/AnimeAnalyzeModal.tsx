import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AnimeInfo, CatalogItem, EpisodeInfo } from "../types";
import { EpisodePicker, episodeKey } from "./EpisodePicker";
import { Icon } from "./Icon";
import { PosterImage } from "./PosterImage";

const analyzeCache = new Map<string, AnimeInfo>();

interface AnimeAnalyzeModalProps {
  item: CatalogItem;
  onClose: () => void;
  onDownloadStarted?: () => void;
}

export function AnimeAnalyzeModal({
  item,
  onClose,
  onDownloadStarted,
}: AnimeAnalyzeModalProps) {
  const [loading, setLoading] = useState(!analyzeCache.has(item.url));
  const [error, setError] = useState("");
  const [anime, setAnime] = useState<AnimeInfo | null>(
    analyzeCache.get(item.url) ?? null
  );
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [downloading, setDownloading] = useState(false);

  const load = useCallback(async () => {
    const cached = analyzeCache.get(item.url);
    if (cached) {
      setAnime(cached);
      setLoading(false);
      const all = new Set<string>();
      cached.seasons.forEach((s) =>
        s.episodes.forEach((ep) => all.add(episodeKey(ep)))
      );
      setSelected(all);
      return;
    }

    setLoading(true);
    setError("");
    try {
      const info = await invoke<AnimeInfo>("parse_anime_url", { url: item.url });
      analyzeCache.set(item.url, info);
      setAnime(info);
      const all = new Set<string>();
      info.seasons.forEach((s) =>
        s.episodes.forEach((ep) => all.add(episodeKey(ep)))
      );
      setSelected(all);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [item.url]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const getSelectedEpisodes = (): EpisodeInfo[] => {
    if (!anime) return [];
    const eps: EpisodeInfo[] = [];
    anime.seasons.forEach((s) =>
      s.episodes.forEach((ep) => {
        if (selected.has(episodeKey(ep))) eps.push(ep);
      })
    );
    return eps;
  };

  const startDownload = async () => {
    if (!anime) return;
    const episodes = getSelectedEpisodes();
    if (episodes.length === 0) {
      setError("Selecione pelo menos um episódio");
      return;
    }
    setDownloading(true);
    setError("");
    try {
      await invoke("start_downloads", {
        request: {
          animeTitle: anime.title,
          episodes,
          posterUrl: anime.poster,
          animeUrl: anime.url,
        },
      });
      onDownloadStarted?.();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setDownloading(false);
    }
  };

  const toggleEpisode = (ep: EpisodeInfo) => {
    setSelected((prev) => {
      const next = new Set(prev);
      const key = episodeKey(ep);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const totalEpisodes =
    anime?.seasons.reduce((a, s) => a + s.episodes.length, 0) ?? 0;

  return (
    <div className="modal-overlay" onClick={onClose} role="presentation">
      <div
        className="modal-panel analyze-modal"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="analyze-modal-title"
      >
        <div className="modal-header">
          <h3 id="analyze-modal-title">
            <Icon name="fa-bolt" /> Analisar
          </h3>
          <button type="button" className="btn-ghost btn-icon" onClick={onClose}>
            <Icon name="fa-xmark" />
          </button>
        </div>

        {loading && (
          <div className="loading">
            <Icon name="fa-spinner" spin /> Carregando episódios...
          </div>
        )}

        {error && (
          <div className="error-banner">
            <Icon name="fa-circle-exclamation" /> {error}
          </div>
        )}

        {anime && !loading && (
          <>
            <div className="anime-preview-header modal-anime-header">
              <PosterImage
                src={anime.poster ?? item.poster}
                alt={anime.title}
                className="anime-poster"
              />
              <div className="anime-preview-body">
                <h3>{anime.title}</h3>
                <div className="anime-meta-chips">
                  <span className="chip-meta">
                    <Icon name="fa-layer-group" /> {anime.seasons.length} temporada(s)
                  </span>
                  <span className="chip-meta">
                    <Icon name="fa-film" /> {totalEpisodes} episódio(s)
                  </span>
                </div>
                {anime.synopsis && (
                  <p className="synopsis synopsis-clamp">{anime.synopsis}</p>
                )}
              </div>
            </div>

            <EpisodePicker
              seasons={anime.seasons}
              selected={selected}
              onToggle={toggleEpisode}
              onSelectAll={() => {
                const all = new Set<string>();
                anime.seasons.forEach((s) =>
                  s.episodes.forEach((ep) => all.add(episodeKey(ep)))
                );
                setSelected(all);
              }}
              onSelectNone={() => setSelected(new Set())}
              onSelectSeason={(seasonNum) => {
                setSelected((prev) => {
                  const next = new Set(prev);
                  anime.seasons
                    .find((s) => s.number === seasonNum)
                    ?.episodes.forEach((ep) => next.add(episodeKey(ep)));
                  return next;
                });
              }}
            />

            <div className="modal-footer">
              <button type="button" className="btn-ghost" onClick={onClose}>
                Fechar
              </button>
              <button
                type="button"
                className="btn-primary"
                onClick={startDownload}
                disabled={downloading || selected.size === 0}
              >
                {downloading ? (
                  <>
                    <Icon name="fa-spinner" spin /> Iniciando...
                  </>
                ) : (
                  <>
                    <Icon name="fa-download" /> Baixar {selected.size} episódio(s)
                  </>
                )}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

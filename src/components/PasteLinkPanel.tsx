import { useMemo, useState } from "react";

import { invoke } from "@tauri-apps/api/core";

import type { AnimeInfo, EpisodeInfo } from "../types";

import { detectSourceFromUrl, sourceLabel } from "../utils/source";

import { EpisodePicker, episodeKey } from "./EpisodePicker";

import { Icon } from "./Icon";

import { PosterImage } from "./PosterImage";



interface PasteLinkPanelProps {

  onDownloadStarted: () => void;

  initialUrl?: string;

}



export function PasteLinkPanel({ onDownloadStarted, initialUrl = "" }: PasteLinkPanelProps) {

  const [url, setUrl] = useState(initialUrl);

  const [loading, setLoading] = useState(false);

  const [error, setError] = useState("");

  const [anime, setAnime] = useState<AnimeInfo | null>(null);

  const [selected, setSelected] = useState<Set<string>>(new Set());

  const [downloading, setDownloading] = useState(false);

  const detectedSource = useMemo(() => detectSourceFromUrl(url), [url]);



  const analyze = async () => {

    if (!url.trim()) return;

    setLoading(true);

    setError("");

    setAnime(null);

    try {

      const info = await invoke<AnimeInfo>("parse_anime_url", { url: url.trim() });

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

  };



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

        request: { animeTitle: anime.title, episodes },

      });

      onDownloadStarted();

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



  const totalEpisodes = anime?.seasons.reduce((a, s) => a + s.episodes.length, 0) ?? 0;



  return (

    <div className="paste-panel">

      <div className="paste-header">

        <h2>

          <Icon name="fa-link" /> Colar link do anime ou filme

        </h2>

        <p>Cole o link de qualquer fonte suportada — anime, filme ou episódio</p>

      </div>



      <div className="paste-input-row">

        <div className="input-with-icon">

          <Icon name="fa-magnifying-glass" />

          <input

            type="url"

            placeholder="https://sushianimes.com.br/... | goyabu.io/... | meusanimes.blog/... | animesonlinecc.to/... | animesdigital.org/..."

            value={url}

            onChange={(e) => setUrl(e.target.value)}

            onKeyDown={(e) => e.key === "Enter" && analyze()}

          />

        </div>

        <button type="button" className="btn-primary" onClick={analyze} disabled={loading}>

          {loading ? (

            <>

              <Icon name="fa-spinner" spin /> Analisando...

            </>

          ) : (

            <>

              <Icon name="fa-wand-magic-sparkles" /> Analisar

            </>

          )}

        </button>

      </div>



      {detectedSource && (

        <div className="paste-source-badge">

          <Icon name="fa-database" /> Fonte: {sourceLabel(detectedSource)}

        </div>

      )}



      {error && (

        <div className="error-banner">

          <Icon name="fa-circle-exclamation" /> {error}

        </div>

      )}



      {anime && (

        <div className="anime-preview">

          <div className="anime-preview-header">

            <PosterImage

              src={anime.poster}

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

                  <Icon name="fa-film" />{" "}

                  {totalEpisodes === 1 &&

                  anime.seasons[0]?.episodes[0]?.description === "Filme"

                    ? "Filme"

                    : `${totalEpisodes} episódio(s)`}

                </span>

              </div>

              {anime.synopsis && <p className="synopsis">{anime.synopsis}</p>}

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



          <div className="download-action">

            <button

              type="button"

              className="btn-primary btn-large"

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

        </div>

      )}

    </div>

  );

}


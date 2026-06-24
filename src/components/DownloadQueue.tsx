import { useEffect, useMemo, useState } from "react";

import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

import type { DownloadFilter, DownloadItem } from "../types";

import { AppLogo } from "./AppLogo";

import { Icon } from "./Icon";

import { VideoPlayer } from "./VideoPlayer";
import { findNextEpisode } from "../utils/library";



interface DownloadQueueProps {
  downloads: DownloadItem[];
  onCancel: (id: string) => void;
  onRetry: (id: string) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onPauseAnime: (title: string) => void;
  onResumeAnime: (title: string) => void;
  onCancelAnime: (title: string) => void;
  onRemove: (id: string) => void;
}



const STATUS_LABEL: Record<string, string> = {

  queued: "Pendente",

  downloading: "Em curso",

  paused: "Pausado",

  completed: "Completo",

  failed: "Erro",

  cancelled: "Cancelado",

};



const STATUS_ICON: Record<string, string> = {

  queued: "fa-clock",

  downloading: "fa-spinner",

  paused: "fa-pause",

  completed: "fa-circle-check",

  failed: "fa-circle-xmark",

  cancelled: "fa-ban",

};



const FILTER_TABS: {

  id: DownloadFilter;

  label: string;

  icon: string;

  match: (d: DownloadItem) => boolean;

}[] = [

  { id: "all", label: "Todos", icon: "fa-list", match: () => true },

  {

    id: "active",

    label: "Em curso",

    icon: "fa-bolt",

    match: (d) => d.status === "downloading",

  },

  {

    id: "queued",

    label: "Pendentes",

    icon: "fa-hourglass-half",

    match: (d) => d.status === "queued",

  },

  {

    id: "completed",

    label: "Salvos",

    icon: "fa-circle-check",

    match: (d) => d.status === "completed",

  },

  {

    id: "failed",

    label: "Erros",

    icon: "fa-triangle-exclamation",

    match: (d) => d.status === "failed" || d.status === "cancelled",

  },

];



type AnimeGroup = {

  key: string;

  animeTitle: string;

  episodes: DownloadItem[];

};



function groupStatus(episodes: DownloadItem[]): string {
  if (episodes.some((e) => e.status === "downloading")) return "downloading";
  if (episodes.some((e) => e.status === "queued")) return "queued";
  if (episodes.some((e) => e.status === "paused")) return "paused";
  if (episodes.some((e) => e.status === "failed" || e.status === "cancelled")) {
    return "failed";
  }
  if (episodes.every((e) => e.status === "completed")) return "completed";
  return "mixed";
}



function groupProgress(episodes: DownloadItem[]): number {

  const active = episodes.filter(

    (e) => e.status === "downloading" || e.status === "queued"

  );

  if (active.length === 0) {

    const done = episodes.filter((e) => e.status === "completed").length;

    return episodes.length > 0 ? (done / episodes.length) * 100 : 0;

  }

  const sum = active.reduce((acc, e) => acc + e.progress, 0);

  return sum / active.length;

}



function matchesSearch(item: DownloadItem, query: string): boolean {

  const q = query.trim().toLowerCase();

  if (!q) return true;

  return (

    item.animeTitle.toLowerCase().includes(q) ||

    item.episodeLabel.toLowerCase().includes(q) ||

    item.episode.title.toLowerCase().includes(q)

  );

}



export function DownloadQueue({
  downloads,
  onCancel,
  onRetry,
  onPause,
  onResume,
  onPauseAnime,
  onResumeAnime,
  onCancelAnime,
  onRemove,
}: DownloadQueueProps) {

  const [filter, setFilter] = useState<DownloadFilter>("all");

  const [search, setSearch] = useState("");

  const [playing, setPlaying] = useState<DownloadItem | null>(null);

  const [expanded, setExpanded] = useState<Set<string>>(new Set());



  const counts = useMemo(() => {

    const map: Record<DownloadFilter, number> = {

      all: downloads.length,

      active: 0,

      queued: 0,

      completed: 0,

      failed: 0,

    };

    for (const d of downloads) {

      if (d.status === "downloading") map.active++;

      if (d.status === "queued") map.queued++;

      if (d.status === "completed") map.completed++;

      if (d.status === "failed" || d.status === "cancelled") map.failed++;

    }

    return map;

  }, [downloads]);



  const filtered = useMemo(() => {

    const tab = FILTER_TABS.find((t) => t.id === filter) ?? FILTER_TABS[0];

    return downloads.filter(

      (d) => tab.match(d) && matchesSearch(d, search)

    );

  }, [downloads, filter, search]);



  const groups = useMemo(() => {

    const map = new Map<string, AnimeGroup>();

    for (const item of filtered) {

      const key = item.animeTitle;

      const existing = map.get(key);

      if (existing) {

        existing.episodes.push(item);

      } else {

        map.set(key, { key, animeTitle: key, episodes: [item] });

      }

    }

    const list = Array.from(map.values());

    for (const g of list) {

      g.episodes.sort((a, b) => {

        if (a.episode.season !== b.episode.season) {

          return a.episode.season - b.episode.season;

        }

        return a.episode.number - b.episode.number;

      });

    }

    list.sort((a, b) => {

      const aMax = Math.max(...a.episodes.map((e) => e.updatedAt ?? 0));

      const bMax = Math.max(...b.episodes.map((e) => e.updatedAt ?? 0));

      return bMax - aMax;

    });

    return list;

  }, [filtered]);



  useEffect(() => {

    setExpanded((prev) => {

      const next = new Set(prev);

      for (const g of groups) {

        const status = groupStatus(g.episodes);

        if (status === "downloading" || status === "queued") {

          next.add(g.key);

        }

      }

      return next;

    });

  }, [groups]);



  const toggleGroup = (key: string) => {

    setExpanded((prev) => {

      const next = new Set(prev);

      if (next.has(key)) next.delete(key);

      else next.add(key);

      return next;

    });

  };



  const completedTotal = downloads.filter((d) => d.status === "completed").length;

  const playingNext = useMemo(
    () => (playing ? findNextEpisode(playing, downloads) : null),
    [playing, downloads]
  );



  return (

    <div className="downloads-panel">

      <div className="downloads-header">

        <h2>

          <Icon name="fa-download" /> Biblioteca

        </h2>

        <span className="downloads-summary">

          {completedTotal} episódio(s) salvo(s) · {counts.active + counts.queued} na fila

        </span>

      </div>



      {playing && (

        <VideoPlayer
          item={playing}
          nextEpisode={playingNext}
          onNextEpisode={setPlaying}
          onClose={() => setPlaying(null)}
        />

      )}



      <div className="downloads-toolbar">

        <div className="downloads-search">

          <Icon name="fa-magnifying-glass" />

          <input

            type="search"

            placeholder="Buscar anime ou episódio..."

            value={search}

            onChange={(e) => setSearch(e.target.value)}

          />

        </div>

      </div>



      <div className="download-filter-tabs" role="tablist">

        {FILTER_TABS.map((tab) => (

          <button

            key={tab.id}

            type="button"

            role="tab"

            aria-selected={filter === tab.id}

            className={`download-filter-tab ${filter === tab.id ? "active" : ""}`}

            onClick={() => setFilter(tab.id)}

          >

            <Icon name={tab.icon} spin={tab.id === "active" && counts.active > 0} />

            {tab.label}

            <span className="tab-count">{counts[tab.id]}</span>

          </button>

        ))}

      </div>



      {downloads.length === 0 && (

        <p className="empty-state empty-state-logo">

          <AppLogo size={64} />

          Nenhum download ainda. Cole um link na aba Início para começar.

        </p>

      )}



      {downloads.length > 0 && groups.length === 0 && (

        <p className="empty-state">

          <Icon name="fa-inbox" /> Nenhum item encontrado.

        </p>

      )}



      <div className="download-group-list">

        {groups.map((group) => (

          <AnimeDownloadGroup

            key={group.key}

            group={group}

            expanded={expanded.has(group.key)}

            onToggle={() => toggleGroup(group.key)}

            onCancel={onCancel}
            onRetry={onRetry}
            onPause={onPause}
            onResume={onResume}
            onPauseAnime={onPauseAnime}
            onResumeAnime={onResumeAnime}
            onCancelAnime={onCancelAnime}
            onRemove={onRemove}

            onPlay={setPlaying}

            playingId={playing?.id ?? null}

            library={downloads}

          />

        ))}

      </div>

    </div>

  );

}



function AnimeDownloadGroup({
  group,
  expanded,
  onToggle,
  onCancel,
  onRetry,
  onPause,
  onResume,
  onPauseAnime,
  onResumeAnime,
  onCancelAnime,
  onRemove,
  onPlay,
  playingId,
  library,
}: {
  group: AnimeGroup;
  expanded: boolean;
  onToggle: () => void;
  onCancel: (id: string) => void;
  onRetry: (id: string) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onPauseAnime: (title: string) => void;
  onResumeAnime: (title: string) => void;
  onCancelAnime: (title: string) => void;
  onRemove: (id: string) => void;
  onPlay: (item: DownloadItem) => void;
  playingId: string | null;
  library: DownloadItem[];
}) {

  const status = groupStatus(group.episodes);

  const progress = groupProgress(group.episodes);

  const completed = group.episodes.filter((e) => e.status === "completed").length;

  const total = group.episodes.length;

  const statusIcon = STATUS_ICON[status] ?? "fa-folder";

  const activeEp = group.episodes.find((e) => e.status === "downloading");

  const hasActive = group.episodes.some(
    (e) => e.status === "downloading" || e.status === "queued"
  );
  const hasPaused = group.episodes.some((e) => e.status === "paused");

  return (

    <div className={`download-group status-${status}`}>

      <button

        type="button"

        className="download-group-header"

        onClick={onToggle}

        aria-expanded={expanded}

      >

        <Icon name={expanded ? "fa-chevron-down" : "fa-chevron-right"} />

        <div className="download-group-info">

          <strong>{group.animeTitle}</strong>

          <span className="download-group-meta">

            {completed}/{total} episódio(s)

            {activeEp && (

              <span className="download-group-active">

                {" "}

                · baixando {activeEp.episodeLabel}

              </span>

            )}

          </span>

        </div>

        <span className="status-tag">

          <Icon name={statusIcon} spin={status === "downloading"} />

          {status === "downloading"

            ? "Baixando"

            : status === "queued"

              ? "Na fila"

              : status === "completed"

                ? "Completo"

                : status === "failed"

                  ? "Com erros"

                  : "Biblioteca"}

        </span>

        {(status === "downloading" || status === "queued") && (

          <div className="download-group-progress">

            <div className="progress-bar">

              <div className="progress-fill" style={{ width: `${progress}%` }} />

            </div>

            <span className="progress-label">{Math.round(progress)}%</span>

          </div>

        )}

      </button>

      {(hasActive || hasPaused) && (
        <div className="download-group-actions">
          {hasActive && (
            <button
              type="button"
              className="btn-ghost btn-sm"
              onClick={(e) => {
                e.stopPropagation();
                onPauseAnime(group.animeTitle);
              }}
            >
              <Icon name="fa-pause" /> Pausar tudo
            </button>
          )}
          {hasPaused && (
            <button
              type="button"
              className="btn-ghost btn-sm"
              onClick={(e) => {
                e.stopPropagation();
                onResumeAnime(group.animeTitle);
              }}
            >
              <Icon name="fa-play" /> Retomar tudo
            </button>
          )}
          {(hasActive || hasPaused) && (
            <button
              type="button"
              className="btn-danger btn-sm"
              onClick={(e) => {
                e.stopPropagation();
                onCancelAnime(group.animeTitle);
              }}
            >
              <Icon name="fa-xmark" /> Cancelar tudo
            </button>
          )}
        </div>
      )}

      {expanded && (

        <div className="download-group-episodes">

          {group.episodes.map((item) => (

            <DownloadRow

              key={item.id}

              item={item}

              onCancel={onCancel}
              onRetry={onRetry}
              onPause={onPause}
              onResume={onResume}
              onRemove={onRemove}

              onPlayItem={onPlay}
              isPlaying={playingId === item.id}
              library={library}

            />

          ))}

        </div>

      )}

    </div>

  );

}



function DownloadRow({
  item,
  onCancel,
  onRetry,
  onPause,
  onResume,
  onRemove,
  onPlayItem,
  isPlaying,
  library,
}: {
  item: DownloadItem;
  onCancel: (id: string) => void;
  onRetry: (id: string) => void;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onRemove: (id: string) => void;
  onPlayItem: (item: DownloadItem) => void;
  isPlaying: boolean;
  library: DownloadItem[];
}) {
  const canCancel = item.status === "queued" || item.status === "downloading";
  const canPause = item.status === "queued" || item.status === "downloading";
  const canResume = item.status === "paused";
  const canRetry = item.status === "failed" || item.status === "cancelled";
  const canRemove =
    item.status === "completed" ||
    item.status === "cancelled" ||
    item.status === "failed";
  const canPlay = item.status === "completed" && !!item.outputPath;
  const nextEpisode = canPlay ? findNextEpisode(item, library) : null;
  const statusIcon = STATUS_ICON[item.status] ?? "fa-circle";



  return (

    <div className={`download-row status-${item.status} ${isPlaying ? "is-playing" : ""}`}>

      <div className="download-row-main">

        <div className="download-info">

          <strong>{item.episodeLabel}</strong>

          {item.episode.title && item.episode.title !== item.episodeLabel && (

            <span>{item.episode.title}</span>

          )}

          <span className="status-tag">

            <Icon name={statusIcon} spin={item.status === "downloading"} />

            {STATUS_LABEL[item.status] ?? item.status}

            {item.speed && item.status === "downloading" && (

              <span className="speed-tag"> · {item.speed}</span>

            )}

          </span>

        </div>



        {(item.status === "downloading" || item.status === "queued") && (

          <div className="progress-block">

            <div className="progress-bar">

              <div className="progress-fill" style={{ width: `${item.progress}%` }} />

            </div>

            <span className="progress-label">{Math.round(item.progress)}%</span>

          </div>

        )}



        {item.error && (

          <div className="download-error">

            <Icon name="fa-circle-exclamation" />

            <span>{item.error}</span>

          </div>

        )}

      </div>



      <div className="download-actions">

        {canPlay && (

          <>

            <button type="button" className="btn-primary btn-sm" onClick={() => onPlayItem(item)}>

              <Icon name="fa-circle-play" /> Assistir

            </button>

            {nextEpisode && (
              <button
                type="button"
                className="btn-ghost btn-sm"
                onClick={() => onPlayItem(nextEpisode)}
                title={`Próximo: ${nextEpisode.episodeLabel}`}
              >
                <Icon name="fa-forward-step" /> Próximo
              </button>
            )}

            <button

              type="button"

              className="btn-ghost btn-sm"

              onClick={() => revealItemInDir(item.outputPath!)}

            >

              <Icon name="fa-folder-open" /> Pasta

            </button>

            <button

              type="button"

              className="btn-ghost btn-sm"

              onClick={() => openPath(item.outputPath!)}

            >

              <Icon name="fa-up-right-from-square" /> Externo

            </button>

          </>

        )}

        {canRetry && (

          <button type="button" className="btn-primary btn-sm" onClick={() => onRetry(item.id)}>

            <Icon name="fa-rotate-right" /> Tentar novamente

          </button>

        )}

        {canResume && (
          <button type="button" className="btn-primary btn-sm" onClick={() => onResume(item.id)}>
            <Icon name="fa-play" /> Retomar
          </button>
        )}

        {canPause && (
          <button type="button" className="btn-ghost btn-sm" onClick={() => onPause(item.id)}>
            <Icon name="fa-pause" /> Pausar
          </button>
        )}

        {canCancel && (

          <button type="button" className="btn-danger btn-sm" onClick={() => onCancel(item.id)}>

            <Icon name="fa-xmark" /> Cancelar

          </button>

        )}

        {canRemove && (
          <button
            type="button"
            className="btn-ghost btn-sm"
            onClick={() => onRemove(item.id)}
            title="Remover da biblioteca"
          >
            <Icon name="fa-trash" /> Remover
          </button>
        )}

      </div>

    </div>

  );

}



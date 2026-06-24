import { useMemo, useState } from "react";
import type { EpisodeInfo, SeasonInfo } from "../types";
import { Icon } from "./Icon";

interface EpisodePickerProps {
  seasons: SeasonInfo[];
  selected: Set<string>;
  onToggle: (episode: EpisodeInfo) => void;
  onSelectAll: () => void;
  onSelectNone: () => void;
  onSelectSeason: (season: number) => void;
}

function episodeKey(ep: EpisodeInfo) {
  return `${ep.season}-${ep.number}`;
}

export function EpisodePicker({
  seasons,
  selected,
  onToggle,
  onSelectAll,
  onSelectNone,
  onSelectSeason,
}: EpisodePickerProps) {
  const [activeSeason, setActiveSeason] = useState(seasons[0]?.number ?? 1);

  const total = seasons.reduce((acc, s) => acc + s.episodes.length, 0);
  const selectedCount = selected.size;

  const currentSeason = useMemo(
    () => seasons.find((s) => s.number === activeSeason) ?? seasons[0],
    [seasons, activeSeason]
  );

  const seasonSelectedCount = (season: SeasonInfo) =>
    season.episodes.filter((ep) => selected.has(episodeKey(ep))).length;

  const deselectSeason = (season: SeasonInfo) => {
    // Parent only has onSelectSeason for select - we toggle via onToggle in batch
    season.episodes.forEach((ep) => {
      if (selected.has(episodeKey(ep))) onToggle(ep);
    });
  };

  const handleSeasonToggle = (season: SeasonInfo) => {
    const count = seasonSelectedCount(season);
    if (count === season.episodes.length) {
      deselectSeason(season);
    } else {
      onSelectSeason(season.number);
    }
  };

  if (!currentSeason) return null;

  return (
    <div className="episode-picker">
      <div className="picker-toolbar">
        <div className="picker-stats">
          <Icon name="fa-list-check" />
          <span>
            <strong>{selectedCount}</strong> de <strong>{total}</strong> episódios
          </span>
        </div>
        <div className="picker-actions">
          <button type="button" className="btn-ghost btn-sm" onClick={onSelectAll}>
            <Icon name="fa-check-double" /> Todos
          </button>
          <button type="button" className="btn-ghost btn-sm" onClick={onSelectNone}>
            <Icon name="fa-xmark" /> Limpar
          </button>
        </div>
      </div>

      {seasons.length > 1 && (
        <div className="season-tabs" role="tablist">
          {seasons.map((season) => {
            const count = seasonSelectedCount(season);
            const isActive = season.number === activeSeason;
            return (
              <button
                key={season.number}
                type="button"
                role="tab"
                aria-selected={isActive}
                className={`season-tab ${isActive ? "active" : ""}`}
                onClick={() => setActiveSeason(season.number)}
              >
                <span>Temp. {season.number}</span>
                <span className="season-tab-meta">
                  {count}/{season.episodes.length}
                </span>
              </button>
            );
          })}
        </div>
      )}

      <div className="season-panel">
        <div className="season-header">
          <div className="season-title">
            <Icon name="fa-layer-group" />
            <h3>
              {seasons.length === 1
                ? `Temporada ${currentSeason.number}`
                : `Episódios — Temporada ${currentSeason.number}`}
            </h3>
            <span className="season-count">
              {currentSeason.episodes.length} eps
            </span>
          </div>
          <button
            type="button"
            className="btn-ghost btn-sm"
            onClick={() => handleSeasonToggle(currentSeason)}
          >
            <Icon
              name={
                seasonSelectedCount(currentSeason) === currentSeason.episodes.length
                  ? "fa-square-minus"
                  : "fa-square-check"
              }
            />
            {seasonSelectedCount(currentSeason) === currentSeason.episodes.length
              ? "Desmarcar temporada"
              : "Marcar temporada"}
          </button>
        </div>

        <div className="episode-grid">
          {currentSeason.episodes.map((ep) => {
            const key = episodeKey(ep);
            const isSelected = selected.has(key);
            return (
              <label
                key={key}
                className={`episode-card ${isSelected ? "selected" : ""}`}
              >
                <input
                  type="checkbox"
                  checked={isSelected}
                  onChange={() => onToggle(ep)}
                />
                <div className="episode-num">
                  <span>{String(ep.number).padStart(2, "0")}</span>
                </div>
                <div className="episode-info">
                  <strong>{ep.title}</strong>
                  {ep.description && ep.description !== ep.title && (
                    <small>{ep.description}</small>
                  )}
                </div>
                {isSelected && (
                  <span className="episode-check">
                    <Icon name="fa-heart" />
                  </span>
                )}
              </label>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export { episodeKey };

import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { DownloadItem } from "../types";
import { Icon } from "./Icon";

interface VideoPlayerProps {
  item: DownloadItem;
  nextEpisode?: DownloadItem | null;
  onNextEpisode?: (item: DownloadItem) => void;
  onClose: () => void;
}

export function VideoPlayer({
  item,
  nextEpisode,
  onNextEpisode,
  onClose,
}: VideoPlayerProps) {
  const [src, setSrc] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    if (!item.outputPath) {
      setError("Arquivo não encontrado");
      return;
    }
    try {
      setSrc(convertFileSrc(item.outputPath));
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }, [item.outputPath]);

  return (
    <div className="video-player-panel">
      <div className="video-player-header">
        <div>
          <strong>{item.animeTitle}</strong>
          <span>{item.episodeLabel}</span>
        </div>
        <div className="video-player-actions">
          {nextEpisode && onNextEpisode && (
            <button
              type="button"
              className="btn-primary btn-sm"
              onClick={() => onNextEpisode(nextEpisode)}
            >
              <Icon name="fa-forward-step" /> Próximo episódio
            </button>
          )}
          <button
            type="button"
            className="btn-ghost btn-sm"
            onClick={onClose}
            aria-label="Fechar player"
          >
            <Icon name="fa-xmark" />
          </button>
        </div>
      </div>
      {error ? (
        <div className="video-player-error">
          <Icon name="fa-circle-exclamation" /> {error}
        </div>
      ) : (
        <video
          key={src}
          className="video-player"
          src={src}
          controls
          autoPlay
          playsInline
          preload="metadata"
          onEnded={() => {
            if (nextEpisode && onNextEpisode) {
              onNextEpisode(nextEpisode);
            }
          }}
        >
          Seu navegador não suporta reprodução de vídeo.
        </video>
      )}
    </div>
  );
}

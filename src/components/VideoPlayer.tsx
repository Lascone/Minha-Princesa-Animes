import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import Plyr from "plyr";
import "plyr/dist/plyr.css";
import type { DownloadItem } from "../types";
import {
  clearWatchProgress,
  formatPlaybackTime,
  loadWatchProgress,
  saveWatchProgress,
  watchProgressKey,
} from "../utils/watchProgress";
import { Icon } from "./Icon";

const PLYR_I18N = {
  restart: "Reiniciar",
  rewind: "Voltar {seektime}s",
  play: "Reproduzir",
  pause: "Pausar",
  fastForward: "Avançar {seektime}s",
  seekLabel: "{currentTime} de {duration}",
  played: "Reproduzido",
  buffered: "Carregado",
  currentTime: "Tempo atual",
  duration: "Duração",
  volume: "Volume",
  mute: "Mudo",
  unmute: "Ativar som",
  enableCaptions: "Ativar legendas",
  disableCaptions: "Desativar legendas",
  download: "Baixar",
  enterFullscreen: "Tela cheia",
  exitFullscreen: "Sair da tela cheia",
  frameTitle: "Player de {title}",
  captions: "Legendas",
  settings: "Configurações",
  pip: "Picture-in-picture",
  menuBack: "Voltar ao menu anterior",
  speed: "Velocidade",
  normal: "Normal",
  quality: "Qualidade",
  loop: "Repetir",
  start: "Início",
  end: "Fim",
  all: "Tudo",
  reset: "Redefinir",
  disabled: "Desativado",
  advertisement: "Anúncio",
  qualityBadge: {
    2160: "4K",
    1440: "HD",
    1080: "HD",
    720: "HD",
    576: "SD",
    480: "SD",
  },
};

const AUTO_NEXT_SECONDS = 5;

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
  const videoRef = useRef<HTMLVideoElement>(null);
  const playerRef = useRef<Plyr | null>(null);
  const saveTimerRef = useRef<number | null>(null);
  const autoNextTimerRef = useRef<number | null>(null);

  const [src, setSrc] = useState("");
  const [error, setError] = useState("");
  const [resumeHint, setResumeHint] = useState<string | null>(null);
  const [autoNextCountdown, setAutoNextCountdown] = useState<number | null>(
    null
  );

  const progressKey = watchProgressKey(item);

  useEffect(() => {
    if (!item.outputPath) {
      setError("Arquivo não encontrado");
      setSrc("");
      return;
    }
    try {
      setSrc(convertFileSrc(item.outputPath));
      setError("");
    } catch (e) {
      setError(String(e));
      setSrc("");
    }
  }, [item.outputPath]);

  useEffect(() => {
    setResumeHint(null);
    setAutoNextCountdown(null);
    if (autoNextTimerRef.current) {
      window.clearInterval(autoNextTimerRef.current);
      autoNextTimerRef.current = null;
    }
  }, [item.id]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video || !src || error) return;

    const saved = loadWatchProgress(progressKey);

    const player = new Plyr(video, {
      autoplay: true,
      clickToPlay: true,
      hideControls: true,
      resetOnEnd: false,
      keyboard: { focused: true, global: false },
      tooltips: { controls: true, seek: true },
      speed: { selected: 1, options: [0.5, 0.75, 1, 1.25, 1.5, 2] },
      i18n: PLYR_I18N,
    });

    playerRef.current = player;

    const onLoadedMetadata = () => {
      if (!saved || saved.duration <= 0) return;
      const ratio = saved.position / saved.duration;
      if (ratio > 0.02 && ratio < 0.92) {
        player.currentTime = saved.position;
        setResumeHint(
          `Continuando de ${formatPlaybackTime(saved.position)}`
        );
        window.setTimeout(() => setResumeHint(null), 4000);
      }
    };

    const scheduleSave = () => {
      if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
      saveTimerRef.current = window.setTimeout(() => {
        saveWatchProgress(item, player.currentTime, player.duration);
      }, 1500);
    };

    const onEnded = () => {
      clearWatchProgress(progressKey);
      if (!nextEpisode || !onNextEpisode) return;

      let remaining = AUTO_NEXT_SECONDS;
      setAutoNextCountdown(remaining);
      autoNextTimerRef.current = window.setInterval(() => {
        remaining -= 1;
        if (remaining <= 0) {
          if (autoNextTimerRef.current) {
            window.clearInterval(autoNextTimerRef.current);
            autoNextTimerRef.current = null;
          }
          setAutoNextCountdown(null);
          onNextEpisode(nextEpisode);
          return;
        }
        setAutoNextCountdown(remaining);
      }, 1000);
    };

    const onTimeUpdate = () => scheduleSave();
    const onPause = () => {
      saveWatchProgress(item, player.currentTime, player.duration);
    };

    video.addEventListener("loadedmetadata", onLoadedMetadata);
    player.on("ended", onEnded);
    player.on("timeupdate", onTimeUpdate);
    player.on("pause", onPause);

    return () => {
      if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
      if (autoNextTimerRef.current) {
        window.clearInterval(autoNextTimerRef.current);
      }
      saveWatchProgress(item, player.currentTime, player.duration);
      video.removeEventListener("loadedmetadata", onLoadedMetadata);
      player.off("ended", onEnded);
      player.off("timeupdate", onTimeUpdate);
      player.off("pause", onPause);
      player.destroy();
      playerRef.current = null;
    };
  }, [src, error, item, progressKey, nextEpisode, onNextEpisode]);

  const cancelAutoNext = () => {
    if (autoNextTimerRef.current) {
      window.clearInterval(autoNextTimerRef.current);
      autoNextTimerRef.current = null;
    }
    setAutoNextCountdown(null);
  };

  const playNextNow = () => {
    cancelAutoNext();
    if (nextEpisode && onNextEpisode) onNextEpisode(nextEpisode);
  };

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
        <div className="video-player-shell plyr-princesa">
          {resumeHint && (
            <div className="video-player-resume-hint">
              <Icon name="fa-clock-rotate-left" /> {resumeHint}
            </div>
          )}

          {autoNextCountdown !== null && nextEpisode && (
            <div className="video-player-autonext">
              <div className="video-player-autonext-card">
                <strong>Próximo episódio em {autoNextCountdown}s</strong>
                <span>
                  {nextEpisode.animeTitle} · {nextEpisode.episodeLabel}
                </span>
                <div className="video-player-autonext-actions">
                  <button
                    type="button"
                    className="btn-primary btn-sm"
                    onClick={playNextNow}
                  >
                    <Icon name="fa-play" /> Assistir agora
                  </button>
                  <button
                    type="button"
                    className="btn-ghost btn-sm"
                    onClick={cancelAutoNext}
                  >
                    Cancelar
                  </button>
                </div>
              </div>
            </div>
          )}

          <video
            ref={videoRef}
            key={item.id}
            className="video-player"
            src={src}
            playsInline
            preload="metadata"
          />
        </div>
      )}
    </div>
  );
}

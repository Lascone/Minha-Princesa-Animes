import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import Plyr from "plyr";
import type { DownloadItem } from "../types";
import {
  clearWatchProgress,
  dismissResumeForSession,
  formatPlaybackTime,
  getSavedPosition,
  loadPlayerPrefs,
  savePlayerPrefs,
  saveWatchProgress,
  watchProgressKey,
} from "../utils/watchProgress";

export const PLYR_I18N = {
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
const UP_NEXT_THRESHOLD = 0.9;
const UP_NEXT_REMAINING_SEC = 45;

export interface UsePlyrPlayerOptions {
  item: DownloadItem;
  nextEpisode?: DownloadItem | null;
  onNextEpisode?: (item: DownloadItem) => void;
}

export function usePlyrPlayer({
  item,
  nextEpisode,
  onNextEpisode,
}: UsePlyrPlayerOptions) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const playerRef = useRef<Plyr | null>(null);
  const saveTimerRef = useRef<number | null>(null);
  const autoNextTimerRef = useRef<number | null>(null);
  const resumeAppliedRef = useRef(false);
  const userSeekedRef = useRef(false);
  const upNextDismissedRef = useRef(false);

  const nextEpisodeRef = useRef(nextEpisode);
  const onNextEpisodeRef = useRef(onNextEpisode);
  nextEpisodeRef.current = nextEpisode;
  onNextEpisodeRef.current = onNextEpisode;

  const progressKey = watchProgressKey(item);

  const [src, setSrc] = useState("");
  const [error, setError] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [resumeHint, setResumeHint] = useState<string | null>(null);
  const [showResumeChoice, setShowResumeChoice] = useState(false);
  const [pendingResumeTime, setPendingResumeTime] = useState(0);
  const [playbackProgress, setPlaybackProgress] = useState(0);
  const [remainingSeconds, setRemainingSeconds] = useState(0);
  const [showUpNext, setShowUpNext] = useState(false);
  const [autoNextCountdown, setAutoNextCountdown] = useState<number | null>(
    null
  );

  useEffect(() => {
    if (!item.outputPath) {
      setError("Arquivo não encontrado");
      setSrc("");
      setIsLoading(false);
      return;
    }
    try {
      setSrc(convertFileSrc(item.outputPath));
      setError("");
      setIsLoading(true);
    } catch (e) {
      setError(String(e));
      setSrc("");
      setIsLoading(false);
    }
  }, [item.outputPath]);

  const cancelAutoNext = () => {
    if (autoNextTimerRef.current) {
      window.clearInterval(autoNextTimerRef.current);
      autoNextTimerRef.current = null;
    }
    setAutoNextCountdown(null);
  };

  const playNextNow = () => {
    cancelAutoNext();
    const next = nextEpisodeRef.current;
    const onNext = onNextEpisodeRef.current;
    if (next && onNext) onNext(next);
  };

  const dismissUpNext = () => {
    upNextDismissedRef.current = true;
    setShowUpNext(false);
  };

  const startFromBeginning = () => {
    dismissResumeForSession(progressKey);
    userSeekedRef.current = true;
    setShowResumeChoice(false);
    setResumeHint(null);
    const player = playerRef.current;
    if (player) {
      player.currentTime = 0;
      void player.play();
    }
  };

  const applyResume = () => {
    const player = playerRef.current;
    if (!player || pendingResumeTime <= 0) {
      setShowResumeChoice(false);
      return;
    }
    player.currentTime = pendingResumeTime;
    setResumeHint(`Continuando de ${formatPlaybackTime(pendingResumeTime)}`);
    setShowResumeChoice(false);
    resumeAppliedRef.current = true;
    window.setTimeout(() => setResumeHint(null), 4000);
    void player.play();
  };

  useEffect(() => {
    const video = videoRef.current;
    if (!video || !src || error) return;

    resumeAppliedRef.current = false;
    userSeekedRef.current = false;
    upNextDismissedRef.current = false;
    setShowUpNext(false);
    setShowResumeChoice(false);
    setPlaybackProgress(0);
    setRemainingSeconds(0);
    cancelAutoNext();

    const prefs = loadPlayerPrefs();
    const savedPosition = getSavedPosition(progressKey);

    const player = new Plyr(video, {
      autoplay: false,
      clickToPlay: true,
      hideControls: true,
      resetOnEnd: false,
      keyboard: { focused: true, global: false },
      tooltips: { controls: true, seek: true },
      speed: { selected: prefs.speed, options: [0.5, 0.75, 1, 1.25, 1.5, 2] },
      volume: prefs.volume,
      i18n: PLYR_I18N,
    });

    playerRef.current = player;

    const onLoadStart = () => setIsLoading(true);
    const onCanPlay = () => setIsLoading(false);

    const onLoadedMetadata = () => {
      if (
        savedPosition &&
        savedPosition > 0 &&
        !resumeAppliedRef.current &&
        !userSeekedRef.current
      ) {
        setPendingResumeTime(savedPosition);
        setShowResumeChoice(true);
      } else {
        void player.play();
      }
    };

    const scheduleSave = () => {
      if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
      saveTimerRef.current = window.setTimeout(() => {
        saveWatchProgress(item, player.currentTime, player.duration);
      }, 1500);
    };

    const updateProgressUi = () => {
      const duration = player.duration;
      const current = player.currentTime;
      if (!Number.isFinite(duration) || duration <= 0) return;

      const progress = current / duration;
      const remaining = Math.max(0, duration - current);
      setPlaybackProgress(progress);
      setRemainingSeconds(remaining);

      if (
        !upNextDismissedRef.current &&
        nextEpisodeRef.current &&
        (progress >= UP_NEXT_THRESHOLD || remaining <= UP_NEXT_REMAINING_SEC)
      ) {
        setShowUpNext(true);
      }
    };

    const onEnded = () => {
      clearWatchProgress(progressKey);
      setShowUpNext(false);
      const next = nextEpisodeRef.current;
      const onNext = onNextEpisodeRef.current;
      if (!next || !onNext) return;

      let remaining = AUTO_NEXT_SECONDS;
      setAutoNextCountdown(remaining);
      autoNextTimerRef.current = window.setInterval(() => {
        remaining -= 1;
        if (remaining <= 0) {
          cancelAutoNext();
          onNext(next);
          return;
        }
        setAutoNextCountdown(remaining);
      }, 1000);
    };

    const onSeeked = () => {
      userSeekedRef.current = true;
      setShowResumeChoice(false);
      scheduleSave();
    };

    const onTimeUpdate = () => {
      scheduleSave();
      updateProgressUi();
    };

    const onPause = () => {
      saveWatchProgress(item, player.currentTime, player.duration);
    };

    const onVolumeChange = () => {
      savePlayerPrefs({ volume: player.volume });
    };

    const onRateChange = () => {
      savePlayerPrefs({ speed: player.speed });
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() !== "n" || e.repeat) return;
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }
      const next = nextEpisodeRef.current;
      const onNext = onNextEpisodeRef.current;
      if (next && onNext) {
        e.preventDefault();
        onNext(next);
      }
    };

    video.addEventListener("loadstart", onLoadStart);
    video.addEventListener("canplay", onCanPlay);
    video.addEventListener("loadedmetadata", onLoadedMetadata);
    player.on("ended", onEnded);
    player.on("seeked", onSeeked);
    player.on("timeupdate", onTimeUpdate);
    player.on("pause", onPause);
    player.on("volumechange", onVolumeChange);
    player.on("ratechange", onRateChange);
    window.addEventListener("keydown", onKeyDown);

    return () => {
      window.removeEventListener("keydown", onKeyDown);
      if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
      cancelAutoNext();
      if (playerRef.current) {
        saveWatchProgress(
          item,
          playerRef.current.currentTime,
          playerRef.current.duration
        );
      }
      video.removeEventListener("loadstart", onLoadStart);
      video.removeEventListener("canplay", onCanPlay);
      video.removeEventListener("loadedmetadata", onLoadedMetadata);
      player.off("ended", onEnded);
      player.off("seeked", onSeeked);
      player.off("timeupdate", onTimeUpdate);
      player.off("pause", onPause);
      player.off("volumechange", onVolumeChange);
      player.off("ratechange", onRateChange);
      player.destroy();
      playerRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- one Plyr instance per mount; episode switch via parent key
  }, [src, error, item, progressKey]);

  return {
    videoRef,
    src,
    error,
    isLoading,
    resumeHint,
    showResumeChoice,
    pendingResumeTime,
    playbackProgress,
    remainingSeconds,
    showUpNext,
    autoNextCountdown,
    applyResume,
    startFromBeginning,
    playNextNow,
    cancelAutoNext,
    dismissUpNext,
  };
}

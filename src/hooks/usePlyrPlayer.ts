import { useCallback, useEffect, useRef, useState } from "react";
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
const END_THRESHOLD = 0.985;

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
  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const playerRef = useRef<Plyr | null>(null);
  const mountedRef = useRef(true);
  const saveTimerRef = useRef<number | null>(null);
  const autoNextTimerRef = useRef<number | null>(null);
  const resumeAppliedRef = useRef(false);
  const userSeekedRef = useRef(false);
  const upNextDismissedRef = useRef(false);
  const endedOnceRef = useRef(false);
  const itemRef = useRef(item);
  const loadTokenRef = useRef(0);

  const nextEpisodeRef = useRef(nextEpisode);
  const onNextEpisodeRef = useRef(onNextEpisode);
  nextEpisodeRef.current = nextEpisode;
  onNextEpisodeRef.current = onNextEpisode;
  itemRef.current = item;

  const progressKey = watchProgressKey(item);

  const [src, setSrc] = useState("");
  const [error, setError] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [playerReady, setPlayerReady] = useState(false);
  const [resumeHint, setResumeHint] = useState<string | null>(null);
  const [showResumeChoice, setShowResumeChoice] = useState(false);
  const [pendingResumeTime, setPendingResumeTime] = useState(0);
  const [showUpNext, setShowUpNext] = useState(false);
  const [autoNextCountdown, setAutoNextCountdown] = useState<number | null>(
    null
  );

  const safeSet = useCallback(<T,>(setter: (v: T) => void, value: T) => {
    if (mountedRef.current) setter(value);
  }, []);

  const cancelAutoNext = useCallback(() => {
    if (autoNextTimerRef.current) {
      window.clearInterval(autoNextTimerRef.current);
      autoNextTimerRef.current = null;
    }
    safeSet(setAutoNextCountdown, null);
  }, [safeSet]);

  const resetEpisodeUi = useCallback(() => {
    resumeAppliedRef.current = false;
    userSeekedRef.current = false;
    upNextDismissedRef.current = false;
    endedOnceRef.current = false;
    safeSet(setShowUpNext, false);
    safeSet(setShowResumeChoice, false);
    safeSet(setResumeHint, null);
    safeSet(setPendingResumeTime, 0);
    cancelAutoNext();
  }, [cancelAutoNext, safeSet]);

  const playNextNow = useCallback(() => {
    cancelAutoNext();
    const next = nextEpisodeRef.current;
    const onNext = onNextEpisodeRef.current;
    if (next && onNext) {
      window.setTimeout(() => onNext(next), 0);
    }
  }, [cancelAutoNext]);

  const dismissUpNext = useCallback(() => {
    upNextDismissedRef.current = true;
    safeSet(setShowUpNext, false);
  }, [safeSet]);

  const startFromBeginning = useCallback(() => {
    dismissResumeForSession(progressKey);
    userSeekedRef.current = true;
    safeSet(setShowResumeChoice, false);
    safeSet(setResumeHint, null);
    const player = playerRef.current;
    if (player) {
      player.currentTime = 0;
      const playPromise = player.play();
      if (playPromise !== undefined) {
        playPromise.catch(() => undefined);
      }
    }
  }, [progressKey, safeSet]);

  const applyResume = useCallback(() => {
    const player = playerRef.current;
    if (!player || pendingResumeTime <= 0) {
      safeSet(setShowResumeChoice, false);
      return;
    }
    player.currentTime = pendingResumeTime;
    safeSet(setResumeHint, `Continuando de ${formatPlaybackTime(pendingResumeTime)}`);
    safeSet(setShowResumeChoice, false);
    resumeAppliedRef.current = true;
    window.setTimeout(() => safeSet(setResumeHint, null), 4000);
    const playPromise = player.play();
    if (playPromise !== undefined) {
      playPromise.catch(() => undefined);
    }
  }, [pendingResumeTime, safeSet]);

  const scheduleSave = useCallback((player: Plyr) => {
    if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
    saveTimerRef.current = window.setTimeout(() => {
      if (!mountedRef.current) return;
      saveWatchProgress(itemRef.current, player.currentTime, player.duration);
    }, 1500);
  }, []);

  const handleEnded = useCallback(() => {
    if (endedOnceRef.current) return;
    endedOnceRef.current = true;

    const currentItem = itemRef.current;
    clearWatchProgress(watchProgressKey(currentItem));
    safeSet(setShowUpNext, false);

    const next = nextEpisodeRef.current;
    const onNext = onNextEpisodeRef.current;
    if (!next || !onNext) return;

    let remaining = AUTO_NEXT_SECONDS;
    safeSet(setAutoNextCountdown, remaining);
    autoNextTimerRef.current = window.setInterval(() => {
      if (!mountedRef.current) return;
      remaining -= 1;
      if (remaining <= 0) {
        cancelAutoNext();
        window.setTimeout(() => {
          if (mountedRef.current) onNext(next);
        }, 0);
        return;
      }
      safeSet(setAutoNextCountdown, remaining);
    }, 1000);
  }, [cancelAutoNext, safeSet]);

  const updateProgressUi = useCallback(
    (player: Plyr) => {
      const duration = player.duration;
      const current = player.currentTime;
      if (!Number.isFinite(duration) || duration <= 0) return;

      const progress = current / duration;
      const remaining = Math.max(0, duration - current);

      if (progress >= END_THRESHOLD && !endedOnceRef.current) {
        handleEnded();
        return;
      }

      if (
        !upNextDismissedRef.current &&
        !endedOnceRef.current &&
        nextEpisodeRef.current &&
        (progress >= UP_NEXT_THRESHOLD || remaining <= UP_NEXT_REMAINING_SEC)
      ) {
        safeSet(setShowUpNext, true);
      }
    },
    [handleEnded, safeSet]
  );

  const loadEpisode = useCallback(
    (player: Plyr, episodeSrc: string, episodeItem: DownloadItem) => {
      const token = ++loadTokenRef.current;
      const key = watchProgressKey(episodeItem);
      const savedPosition = getSavedPosition(key);

      resetEpisodeUi();
      safeSet(setIsLoading, true);
      safeSet(setError, "");

      const onReady = () => {
        window.clearTimeout(loadTimeout);
        player.off("loadeddata", onReady);
        player.off("error", onVideoError);
        if (token !== loadTokenRef.current || !mountedRef.current) return;

        safeSet(setIsLoading, false);

        if (
          savedPosition &&
          savedPosition > 0 &&
          !resumeAppliedRef.current &&
          !userSeekedRef.current
        ) {
          safeSet(setPendingResumeTime, savedPosition);
          safeSet(setShowResumeChoice, true);
        } else {
          const playPromise = player.play();
          if (playPromise !== undefined) {
            playPromise.catch(() => undefined);
          }
        }
      };

      const onVideoError = () => {
        window.clearTimeout(loadTimeout);
        player.off("loadeddata", onReady);
        player.off("error", onVideoError);
        if (token !== loadTokenRef.current || !mountedRef.current) return;
        safeSet(setError, "Não foi possível reproduzir este episódio");
        safeSet(setIsLoading, false);
      };

      const loadTimeout = window.setTimeout(() => {
        player.off("loadeddata", onReady);
        player.off("error", onVideoError);
        if (token !== loadTokenRef.current || !mountedRef.current) return;
        if (player.duration > 0) {
          safeSet(setIsLoading, false);
          return;
        }
        safeSet(setError, "O vídeo demorou demais para carregar. Tente fechar e abrir de novo.");
        safeSet(setIsLoading, false);
      }, 15000);

      player.on("loadeddata", onReady);
      player.on("error", onVideoError);

      try {
        player.source = {
          type: "video",
          title: episodeItem.episodeLabel,
          sources: [{ src: episodeSrc, type: "video/mp4" }],
        };
      } catch {
        window.clearTimeout(loadTimeout);
        player.off("loadeddata", onReady);
        player.off("error", onVideoError);
        if (token !== loadTokenRef.current) return;
        safeSet(setError, "Não foi possível carregar o vídeo");
        safeSet(setIsLoading, false);
      }
    },
    [resetEpisodeUi, safeSet]
  );

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
      cancelAutoNext();
    };
  }, [cancelAutoNext]);

  useEffect(() => {
    if (!item.outputPath) {
      safeSet(setError, "Arquivo não encontrado");
      safeSet(setSrc, "");
      safeSet(setIsLoading, false);
      return;
    }
    try {
      const nextSrc = convertFileSrc(item.outputPath);
      safeSet(setSrc, nextSrc);
      safeSet(setError, "");

      const player = playerRef.current;
      if (player && playerReady) {
        loadEpisode(player, nextSrc, item);
      } else {
        resetEpisodeUi();
        safeSet(setIsLoading, true);
      }
    } catch (e) {
      safeSet(setError, String(e));
      safeSet(setSrc, "");
      safeSet(setIsLoading, false);
    }
  }, [item.id, item.outputPath, playerReady, loadEpisode, resetEpisodeUi, safeSet]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    let cancelled = false;
    let player: Plyr | null = null;

    const initFrame = requestAnimationFrame(() => {
      if (cancelled || !video.isConnected) return;

      const prefs = loadPlayerPrefs();
      player = new Plyr(video, {
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

      const onLoadStart = () => safeSet(setIsLoading, true);
      const onCanPlay = () => safeSet(setIsLoading, false);

      const onSeeked = () => {
        userSeekedRef.current = true;
        safeSet(setShowResumeChoice, false);
        if (player) scheduleSave(player);
      };

      const onTimeUpdate = () => {
        if (!player) return;
        scheduleSave(player);
        updateProgressUi(player);
      };

      const onPause = () => {
        if (!player) return;
        saveWatchProgress(itemRef.current, player.currentTime, player.duration);
      };

      const onVolumeChange = () => {
        if (player) savePlayerPrefs({ volume: player.volume });
      };

      const onRateChange = () => {
        if (player) savePlayerPrefs({ speed: player.speed });
      };

      const onEnded = () => handleEnded();

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
          window.setTimeout(() => onNext(next), 0);
        }
      };

      video.addEventListener("loadstart", onLoadStart);
      video.addEventListener("canplay", onCanPlay);
      player.on("ended", onEnded);
      player.on("seeked", onSeeked);
      player.on("timeupdate", onTimeUpdate);
      player.on("pause", onPause);
      player.on("volumechange", onVolumeChange);
      player.on("ratechange", onRateChange);
      window.addEventListener("keydown", onKeyDown);

      safeSet(setPlayerReady, true);

      const currentItem = itemRef.current;
      if (currentItem.outputPath) {
        try {
          const initialSrc = convertFileSrc(currentItem.outputPath);
          loadEpisode(player, initialSrc, currentItem);
        } catch (e) {
          safeSet(setError, String(e));
        }
      }

      (player as Plyr & { __cleanup?: () => void }).__cleanup = () => {
        window.removeEventListener("keydown", onKeyDown);
        video.removeEventListener("loadstart", onLoadStart);
        video.removeEventListener("canplay", onCanPlay);
        player?.off("ended", onEnded);
        player?.off("seeked", onSeeked);
        player?.off("timeupdate", onTimeUpdate);
        player?.off("pause", onPause);
        player?.off("volumechange", onVolumeChange);
        player?.off("ratechange", onRateChange);
      };
    });

    return () => {
      cancelled = true;
      cancelAnimationFrame(initFrame);
      loadTokenRef.current += 1;

      const active = playerRef.current as (Plyr & { __cleanup?: () => void }) | null;
      if (active) {
        try {
          saveWatchProgress(
            itemRef.current,
            active.currentTime,
            active.duration
          );
        } catch {
          // player may already be torn down
        }
        active.__cleanup?.();
        try {
          active.destroy();
        } catch {
          // ignore destroy errors
        }
        playerRef.current = null;
      }
      safeSet(setPlayerReady, false);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- Plyr init once per player mount
  }, []);

  return {
    containerRef,
    videoRef,
    src,
    error,
    isLoading,
    resumeHint,
    showResumeChoice,
    pendingResumeTime,
    showUpNext,
    autoNextCountdown,
    applyResume,
    startFromBeginning,
    playNextNow,
    cancelAutoNext,
    dismissUpNext,
  };
};

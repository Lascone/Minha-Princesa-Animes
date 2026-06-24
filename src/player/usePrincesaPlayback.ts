import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
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
import { PrincesaPlayer } from "./createPrincesaPlayer";

const AUTO_NEXT_SECONDS = 5;
const UP_NEXT_THRESHOLD = 0.9;
const UP_NEXT_REMAINING_SEC = 45;
const END_THRESHOLD = 0.985;

export interface UsePrincesaPlaybackOptions {
  item: DownloadItem;
  nextEpisode?: DownloadItem | null;
  onNextEpisode?: (item: DownloadItem) => void;
}

interface PendingLoad {
  token: number;
  savedPosition: number | null;
}

export function usePrincesaPlayback({
  item,
  nextEpisode,
  onNextEpisode,
}: UsePrincesaPlaybackOptions) {
  const videoEl = PrincesaPlayer.useMedia() as HTMLVideoElement | null;

  const mountedRef = useRef(true);
  const saveTimerRef = useRef<number | null>(null);
  const autoNextTimerRef = useRef<number | null>(null);
  const loadTimeoutRef = useRef<number | null>(null);
  const pendingLoadRef = useRef<PendingLoad | null>(null);
  const resumeAppliedRef = useRef(false);
  const userSeekedRef = useRef(false);
  const upNextDismissedRef = useRef(false);
  const endedOnceRef = useRef(false);
  const itemRef = useRef(item);
  const loadTokenRef = useRef(0);
  const prefsAppliedRef = useRef(false);

  const nextEpisodeRef = useRef(nextEpisode);
  const onNextEpisodeRef = useRef(onNextEpisode);
  nextEpisodeRef.current = nextEpisode;
  onNextEpisodeRef.current = onNextEpisode;
  itemRef.current = item;

  const progressKey = watchProgressKey(item);

  const [src, setSrc] = useState("");
  const [error, setError] = useState("");
  const [isLoading, setIsLoading] = useState(true);
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

  const clearLoadTimeout = useCallback(() => {
    if (loadTimeoutRef.current) {
      window.clearTimeout(loadTimeoutRef.current);
      loadTimeoutRef.current = null;
    }
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

  const playVideo = useCallback((video: HTMLVideoElement) => {
    const promise = video.play();
    if (promise !== undefined) {
      promise.catch(() => undefined);
    }
  }, []);

  const startFromBeginning = useCallback(() => {
    dismissResumeForSession(progressKey);
    userSeekedRef.current = true;
    safeSet(setShowResumeChoice, false);
    safeSet(setResumeHint, null);
    if (videoEl) {
      videoEl.currentTime = 0;
      playVideo(videoEl);
    }
  }, [playVideo, progressKey, safeSet, videoEl]);

  const applyResume = useCallback(() => {
    if (pendingResumeTime <= 0 || !videoEl) {
      safeSet(setShowResumeChoice, false);
      return;
    }
    videoEl.currentTime = pendingResumeTime;
    safeSet(
      setResumeHint,
      `Continuando de ${formatPlaybackTime(pendingResumeTime)}`
    );
    safeSet(setShowResumeChoice, false);
    resumeAppliedRef.current = true;
    window.setTimeout(() => safeSet(setResumeHint, null), 4000);
    playVideo(videoEl);
  }, [pendingResumeTime, playVideo, safeSet, videoEl]);

  const scheduleSave = useCallback((current: number, duration: number) => {
    if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
    saveTimerRef.current = window.setTimeout(() => {
      if (!mountedRef.current) return;
      saveWatchProgress(itemRef.current, current, duration);
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
    (current: number, duration: number) => {
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

  const finishPendingLoad = useCallback(() => {
    const pending = pendingLoadRef.current;
    if (!pending || pending.token !== loadTokenRef.current) return;

    pendingLoadRef.current = null;
    clearLoadTimeout();
    safeSet(setIsLoading, false);

    if (!videoEl) return;

    if (
      pending.savedPosition &&
      pending.savedPosition > 0 &&
      !resumeAppliedRef.current &&
      !userSeekedRef.current
    ) {
      safeSet(setPendingResumeTime, pending.savedPosition);
      safeSet(setShowResumeChoice, true);
    } else {
      playVideo(videoEl);
    }
  }, [clearLoadTimeout, playVideo, safeSet, videoEl]);

  const beginEpisode = useCallback(
    (episodeItem: DownloadItem) => {
      const token = ++loadTokenRef.current;
      const key = watchProgressKey(episodeItem);
      const savedPosition = getSavedPosition(key);

      resetEpisodeUi();
      clearLoadTimeout();
      safeSet(setIsLoading, true);
      safeSet(setError, "");

      pendingLoadRef.current = {
        token,
        savedPosition: savedPosition ?? null,
      };

      loadTimeoutRef.current = window.setTimeout(() => {
        if (token !== loadTokenRef.current || !mountedRef.current) return;
        const duration = videoEl?.duration ?? 0;
        if (duration > 0) {
          finishPendingLoad();
          return;
        }
        pendingLoadRef.current = null;
        safeSet(
          setError,
          "O vídeo demorou demais para carregar. Tente fechar e abrir de novo."
        );
        safeSet(setIsLoading, false);
      }, 15000);
    },
    [clearLoadTimeout, finishPendingLoad, resetEpisodeUi, safeSet, videoEl]
  );

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
      clearLoadTimeout();
      cancelAutoNext();
      if (videoEl) {
        saveWatchProgress(
          itemRef.current,
          videoEl.currentTime,
          videoEl.duration
        );
      }
    };
  }, [cancelAutoNext, clearLoadTimeout, videoEl]);

  useEffect(() => {
    if (!videoEl || prefsAppliedRef.current) return;
    const prefs = loadPlayerPrefs();
    videoEl.volume = prefs.volume;
    videoEl.playbackRate = prefs.speed;
    prefsAppliedRef.current = true;
  }, [videoEl]);

  useEffect(() => {
    if (!item.outputPath) {
      safeSet(setError, "Arquivo não encontrado");
      safeSet(setSrc, "");
      safeSet(setIsLoading, false);
      pendingLoadRef.current = null;
      return;
    }

    try {
      const nextSrc = convertFileSrc(item.outputPath);
      beginEpisode(item);
      setSrc(nextSrc);
      setError("");
    } catch (e) {
      safeSet(setError, String(e));
      safeSet(setSrc, "");
      safeSet(setIsLoading, false);
      pendingLoadRef.current = null;
    }
  }, [item.id, item.outputPath, beginEpisode, safeSet]);

  useEffect(() => {
    const video = videoEl;
    if (!video || !src) return;

    const onLoadedData = () => {
      if (pendingLoadRef.current) {
        finishPendingLoad();
      } else {
        safeSet(setIsLoading, false);
      }
    };

    const onCanPlay = () => {
      if (pendingLoadRef.current) {
        finishPendingLoad();
      }
    };

    const onVideoError = () => {
      pendingLoadRef.current = null;
      clearLoadTimeout();
      safeSet(setError, "Não foi possível reproduzir este episódio");
      safeSet(setIsLoading, false);
    };

    const onTimeUpdate = () => {
      scheduleSave(video.currentTime, video.duration);
      updateProgressUi(video.currentTime, video.duration);
    };

    const onSeeked = () => {
      userSeekedRef.current = true;
      safeSet(setShowResumeChoice, false);
    };

    const onPause = () => {
      saveWatchProgress(itemRef.current, video.currentTime, video.duration);
    };

    const onEnded = () => {
      handleEnded();
    };

    const onVolumeChange = () => {
      savePlayerPrefs({ volume: video.muted ? 0 : video.volume });
    };

    const onRateChange = () => {
      savePlayerPrefs({ speed: video.playbackRate });
    };

    video.addEventListener("loadeddata", onLoadedData);
    video.addEventListener("canplay", onCanPlay);
    video.addEventListener("error", onVideoError);
    video.addEventListener("timeupdate", onTimeUpdate);
    video.addEventListener("seeked", onSeeked);
    video.addEventListener("pause", onPause);
    video.addEventListener("ended", onEnded);
    video.addEventListener("volumechange", onVolumeChange);
    video.addEventListener("ratechange", onRateChange);

    if (video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA) {
      onLoadedData();
    }

    return () => {
      video.removeEventListener("loadeddata", onLoadedData);
      video.removeEventListener("canplay", onCanPlay);
      video.removeEventListener("error", onVideoError);
      video.removeEventListener("timeupdate", onTimeUpdate);
      video.removeEventListener("seeked", onSeeked);
      video.removeEventListener("pause", onPause);
      video.removeEventListener("ended", onEnded);
      video.removeEventListener("volumechange", onVolumeChange);
      video.removeEventListener("ratechange", onRateChange);
    };
  }, [
    videoEl,
    src,
    clearLoadTimeout,
    finishPendingLoad,
    handleEnded,
    safeSet,
    scheduleSave,
    updateProgressUi,
  ]);

  useEffect(() => {
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

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return {
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
}

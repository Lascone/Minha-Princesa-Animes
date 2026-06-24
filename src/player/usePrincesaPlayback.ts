import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  selectError,
  selectPlayback,
  selectPlaybackRate,
  selectTime,
  selectVolume,
} from "@videojs/react";
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
  const store = PrincesaPlayer.usePlayer();
  const time = PrincesaPlayer.usePlayer(selectTime);
  const playback = PrincesaPlayer.usePlayer(selectPlayback);
  const volume = PrincesaPlayer.usePlayer(selectVolume);
  const playbackRate = PrincesaPlayer.usePlayer(selectPlaybackRate);
  const playerError = PrincesaPlayer.usePlayer(selectError);
  const canPlay = PrincesaPlayer.usePlayer((s) => s.canPlay);

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
  const prevSeekingRef = useRef(false);
  const prevPausedRef = useRef(true);

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

  const startFromBeginning = useCallback(() => {
    dismissResumeForSession(progressKey);
    userSeekedRef.current = true;
    safeSet(setShowResumeChoice, false);
    safeSet(setResumeHint, null);
    void store.seek(0);
    void store.play().catch(() => undefined);
  }, [progressKey, safeSet, store]);

  const applyResume = useCallback(() => {
    if (pendingResumeTime <= 0) {
      safeSet(setShowResumeChoice, false);
      return;
    }
    void store.seek(pendingResumeTime);
    safeSet(
      setResumeHint,
      `Continuando de ${formatPlaybackTime(pendingResumeTime)}`
    );
    safeSet(setShowResumeChoice, false);
    resumeAppliedRef.current = true;
    window.setTimeout(() => safeSet(setResumeHint, null), 4000);
    void store.play().catch(() => undefined);
  }, [pendingResumeTime, safeSet, store]);

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

    if (
      pending.savedPosition &&
      pending.savedPosition > 0 &&
      !resumeAppliedRef.current &&
      !userSeekedRef.current
    ) {
      safeSet(setPendingResumeTime, pending.savedPosition);
      safeSet(setShowResumeChoice, true);
    } else {
      void store.play().catch(() => undefined);
    }
  }, [clearLoadTimeout, safeSet, store]);

  const loadEpisode = useCallback(
    (episodeSrc: string, episodeItem: DownloadItem) => {
      const token = ++loadTokenRef.current;
      const key = watchProgressKey(episodeItem);
      const savedPosition = getSavedPosition(key);

      resetEpisodeUi();
      safeSet(setIsLoading, true);
      safeSet(setError, "");
      clearLoadTimeout();

      pendingLoadRef.current = {
        token,
        savedPosition: savedPosition ?? null,
      };

      loadTimeoutRef.current = window.setTimeout(() => {
        if (token !== loadTokenRef.current || !mountedRef.current) return;
        const duration = time?.duration ?? 0;
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

      try {
        store.loadSource(episodeSrc);
      } catch {
        clearLoadTimeout();
        pendingLoadRef.current = null;
        if (token !== loadTokenRef.current) return;
        safeSet(setError, "Não foi possível carregar o vídeo");
        safeSet(setIsLoading, false);
      }
    },
    [
      clearLoadTimeout,
      finishPendingLoad,
      resetEpisodeUi,
      safeSet,
      store,
      time?.duration,
    ]
  );

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
      clearLoadTimeout();
      cancelAutoNext();
      if (time) {
        saveWatchProgress(
          itemRef.current,
          time.currentTime,
          time.duration
        );
      }
    };
  }, [cancelAutoNext, clearLoadTimeout, time]);

  useEffect(() => {
    if (!prefsAppliedRef.current) {
      const prefs = loadPlayerPrefs();
      store.setVolume(prefs.volume);
      playbackRate?.setPlaybackRate(prefs.speed);
      prefsAppliedRef.current = true;
    }
  }, [playbackRate, store]);

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
      loadEpisode(nextSrc, item);
    } catch (e) {
      safeSet(setError, String(e));
      safeSet(setSrc, "");
      safeSet(setIsLoading, false);
    }
  }, [item.id, item.outputPath, loadEpisode, safeSet]);

  useEffect(() => {
    if (canPlay && pendingLoadRef.current) {
      finishPendingLoad();
    }
  }, [canPlay, finishPendingLoad]);

  useEffect(() => {
    if (!time) return;
    scheduleSave(time.currentTime, time.duration);
    updateProgressUi(time.currentTime, time.duration);
  }, [time?.currentTime, time?.duration, scheduleSave, updateProgressUi, time]);

  useEffect(() => {
    if (!playback?.ended) return;
    handleEnded();
  }, [playback?.ended, handleEnded]);

  useEffect(() => {
    const seeking = time?.seeking ?? false;
    if (!seeking && prevSeekingRef.current) {
      userSeekedRef.current = true;
      safeSet(setShowResumeChoice, false);
    }
    prevSeekingRef.current = seeking;
  }, [time?.seeking, safeSet]);

  useEffect(() => {
    const paused = playback?.paused ?? true;
    if (paused && !prevPausedRef.current && time) {
      saveWatchProgress(itemRef.current, time.currentTime, time.duration);
    }
    prevPausedRef.current = paused;
  }, [playback?.paused, time]);

  useEffect(() => {
    if (volume) {
      savePlayerPrefs({ volume: volume.muted ? 0 : volume.volume });
    }
  }, [volume?.volume, volume?.muted]);

  useEffect(() => {
    if (playbackRate) {
      savePlayerPrefs({ speed: playbackRate.playbackRate });
    }
  }, [playbackRate?.playbackRate]);

  useEffect(() => {
    if (playerError?.error && !error) {
      safeSet(setError, "Não foi possível reproduzir este episódio");
      safeSet(setIsLoading, false);
    }
  }, [playerError?.error, error, safeSet]);

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

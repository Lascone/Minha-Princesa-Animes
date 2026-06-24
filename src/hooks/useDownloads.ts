import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DownloadItem, DownloadStatus } from "../types";

function patchDownload(
  items: DownloadItem[],
  id: string,
  patch: Partial<DownloadItem>
): DownloadItem[] {
  return items.map((d) => (d.id === id ? { ...d, ...patch } : d));
}

function patchAnimeDownloads(
  items: DownloadItem[],
  title: string,
  statuses: DownloadStatus[],
  patch: Partial<DownloadItem>
): DownloadItem[] {
  return items.map((d) =>
    d.animeTitle === title && statuses.includes(d.status) ? { ...d, ...patch } : d
  );
}

export function useDownloads() {
  const [downloads, setDownloads] = useState<DownloadItem[]>([]);
  const optimisticUntil = useRef(0);
  const lastAwakeSync = useRef(0);
  const awakeRetryTimer = useRef<number | null>(null);

  const pullFromBackend = useCallback(async () => {
    return invoke<DownloadItem[]>("get_downloads");
  }, []);

  const refresh = useCallback(async () => {
    if (Date.now() < optimisticUntil.current) return;
    const items = await pullFromBackend();
    setDownloads(items);
  }, [pullFromBackend]);

  const syncAfterAwake = useCallback(async () => {
    const now = Date.now();
    if (now - lastAwakeSync.current < 2000) return;
    lastAwakeSync.current = now;
    optimisticUntil.current = 0;

    const items = await pullFromBackend();
    setDownloads(items);

    if (awakeRetryTimer.current !== null) {
      window.clearTimeout(awakeRetryTimer.current);
    }
    awakeRetryTimer.current = window.setTimeout(async () => {
      const retryItems = await pullFromBackend();
      setDownloads(retryItems);
      awakeRetryTimer.current = null;
    }, 500);
  }, [pullFromBackend]);

  useEffect(() => {
    void refresh();

    const unlistenProgress = listen<DownloadItem>("download-progress", (event) => {
      if (Date.now() < optimisticUntil.current) return;
      setDownloads((prev) => {
        const idx = prev.findIndex((d) => d.id === event.payload.id);
        if (idx >= 0) {
          const next = [...prev];
          next[idx] = event.payload;
          return next;
        }
        return [...prev, event.payload];
      });
    });

    const unlistenAwake = listen("window-awake", () => {
      void syncAfterAwake();
    });

    const interval = window.setInterval(() => {
      if (document.visibilityState !== "hidden") {
        void refresh();
      }
    }, 3000);

    const onVisibility = () => {
      if (document.visibilityState === "visible") {
        void syncAfterAwake();
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      window.clearInterval(interval);
      if (awakeRetryTimer.current !== null) {
        window.clearTimeout(awakeRetryTimer.current);
      }
      document.removeEventListener("visibilitychange", onVisibility);
      unlistenProgress.then((fn) => fn());
      unlistenAwake.then((fn) => fn());
    };
  }, [refresh, syncAfterAwake]);

  const withOptimistic = (action: () => Promise<void>) => {
    optimisticUntil.current = Date.now() + 1500;
    return action().catch(() => refresh());
  };

  const cancel = (id: string) =>
    withOptimistic(async () => {
      setDownloads((prev) =>
        patchDownload(prev, id, { status: "cancelled", speed: "", progress: 0 })
      );
      await invoke("cancel_download", { id });
    });

  const pause = (id: string) =>
    withOptimistic(async () => {
      setDownloads((prev) =>
        patchDownload(prev, id, { status: "paused", speed: "", progress: 0 })
      );
      await invoke("pause_download", { id });
    });

  const resume = (id: string) =>
    withOptimistic(async () => {
      setDownloads((prev) =>
        patchDownload(prev, id, {
          status: "queued",
          speed: "",
          progress: 0,
          error: undefined,
        })
      );
      await invoke("resume_download", { id });
    });

  const pauseAnime = (title: string) =>
    withOptimistic(async () => {
      setDownloads((prev) =>
        patchAnimeDownloads(prev, title, ["downloading", "queued"], {
          status: "paused",
          speed: "",
          progress: 0,
        })
      );
      await invoke("pause_anime", { title });
    });

  const resumeAnime = (title: string) =>
    withOptimistic(async () => {
      setDownloads((prev) =>
        patchAnimeDownloads(prev, title, ["paused"], {
          status: "queued",
          speed: "",
          progress: 0,
          error: undefined,
        })
      );
      await invoke("resume_anime", { title });
    });

  const cancelAnime = (title: string) =>
    withOptimistic(async () => {
      setDownloads((prev) =>
        patchAnimeDownloads(prev, title, ["downloading", "queued", "paused"], {
          status: "cancelled",
          speed: "",
          progress: 0,
        })
      );
      await invoke("cancel_anime", { title });
    });

  const remove = (id: string) =>
    withOptimistic(async () => {
      setDownloads((prev) => prev.filter((d) => d.id !== id));
      await invoke("delete_download", { id });
    });

  const retry = (id: string) =>
    withOptimistic(async () => {
      setDownloads((prev) =>
        patchDownload(prev, id, {
          status: "queued",
          speed: "",
          progress: 0,
          error: undefined,
        })
      );
      await invoke("retry_download", { id });
    });

  return {
    downloads,
    refresh,
    cancel,
    pause,
    resume,
    pauseAnime,
    resumeAnime,
    cancelAnime,
    remove,
    retry,
  };
}

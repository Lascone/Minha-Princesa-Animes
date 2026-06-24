import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
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

/** Evita o WebView2 congelar timers/eventos ao perder foco (workaround Windows). */
function useBackgroundKeepAlive() {
  useEffect(() => {
    const locks = navigator.locks;
    if (!locks?.request) return;

    const controller = new AbortController();

    locks
      .request(
        "minha-princesa-downloads",
        { mode: "shared", signal: controller.signal },
        () =>
          new Promise<void>((resolve) => {
            controller.signal.addEventListener("abort", () => resolve(), {
              once: true,
            });
          })
      )
      .catch(() => undefined);

    return () => controller.abort();
  }, []);
}

export function useDownloads() {
  const [downloads, setDownloads] = useState<DownloadItem[]>([]);
  const optimisticUntil = useRef(0);

  const refresh = useCallback(async () => {
    const items = await invoke<DownloadItem[]>("get_downloads");
    if (Date.now() < optimisticUntil.current) return;
    setDownloads(items);
  }, []);

  const applySnapshot = useCallback((items: DownloadItem[]) => {
    if (Date.now() < optimisticUntil.current) return;
    setDownloads(items);
  }, []);

  useBackgroundKeepAlive();

  useEffect(() => {
    refresh();
    const restoreTimer = setTimeout(refresh, 600);

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

    const unlistenSnapshot = listen<DownloadItem[]>("downloads-snapshot", (event) => {
      applySnapshot(event.payload);
    });

    const onVisibility = () => {
      if (document.visibilityState === "visible") refresh();
    };
    document.addEventListener("visibilitychange", onVisibility);

    // Sempre ativo — perder foco (Alt+Tab) NÃO deve parar o sync.
    const interval = setInterval(refresh, 3000);

    const win = getCurrentWindow();
    const unlistenFocus = win.onFocusChanged(({ payload: focused }) => {
      if (focused) refresh();
    });

    return () => {
      clearTimeout(restoreTimer);
      clearInterval(interval);
      document.removeEventListener("visibilitychange", onVisibility);
      unlistenProgress.then((fn) => fn());
      unlistenSnapshot.then((fn) => fn());
      unlistenFocus.then((fn) => fn());
    };
  }, [refresh, applySnapshot]);

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

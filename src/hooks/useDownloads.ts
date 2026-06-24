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
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    const locks = navigator.locks;
    if (!locks?.request) return;

    const controller = new AbortController();
    abortRef.current = controller;

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
  const visibleRef = useRef(true);

  const refresh = useCallback(async () => {
    const items = await invoke<DownloadItem[]>("get_downloads");
    setDownloads(items);
  }, []);

  useBackgroundKeepAlive();

  useEffect(() => {
    refresh();
    const restoreTimer = setTimeout(refresh, 600);

    const unlistenProgress = listen<DownloadItem>("download-progress", (event) => {
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

    const onVisibility = () => {
      visibleRef.current = document.visibilityState === "visible";
      if (visibleRef.current) refresh();
    };
    document.addEventListener("visibilitychange", onVisibility);

    const interval = setInterval(() => {
      if (visibleRef.current) refresh();
    }, 5000);

    const win = getCurrentWindow();
    const unlistenFocus = win.onFocusChanged(({ payload: focused }) => {
      visibleRef.current = focused;
      if (focused) refresh();
    });

    return () => {
      clearTimeout(restoreTimer);
      clearInterval(interval);
      document.removeEventListener("visibilitychange", onVisibility);
      unlistenProgress.then((fn) => fn());
      unlistenFocus.then((fn) => fn());
    };
  }, [refresh]);

  const cancel = async (id: string) => {
    setDownloads((prev) =>
      patchDownload(prev, id, { status: "cancelled", speed: "", progress: 0 })
    );
    try {
      await invoke("cancel_download", { id });
    } catch {
      await refresh();
    }
  };

  const pause = async (id: string) => {
    setDownloads((prev) =>
      patchDownload(prev, id, { status: "paused", speed: "", progress: 0 })
    );
    try {
      await invoke("pause_download", { id });
    } catch {
      await refresh();
    }
  };

  const resume = async (id: string) => {
    setDownloads((prev) =>
      patchDownload(prev, id, {
        status: "queued",
        speed: "",
        progress: 0,
        error: undefined,
      })
    );
    try {
      await invoke("resume_download", { id });
    } catch {
      await refresh();
    }
  };

  const pauseAnime = async (title: string) => {
    setDownloads((prev) =>
      patchAnimeDownloads(prev, title, ["downloading", "queued"], {
        status: "paused",
        speed: "",
        progress: 0,
      })
    );
    try {
      await invoke("pause_anime", { title });
    } catch {
      await refresh();
    }
  };

  const resumeAnime = async (title: string) => {
    setDownloads((prev) =>
      patchAnimeDownloads(prev, title, ["paused"], {
        status: "queued",
        speed: "",
        progress: 0,
        error: undefined,
      })
    );
    try {
      await invoke("resume_anime", { title });
    } catch {
      await refresh();
    }
  };

  const cancelAnime = async (title: string) => {
    setDownloads((prev) =>
      patchAnimeDownloads(prev, title, ["downloading", "queued", "paused"], {
        status: "cancelled",
        speed: "",
        progress: 0,
      })
    );
    try {
      await invoke("cancel_anime", { title });
    } catch {
      await refresh();
    }
  };

  const remove = async (id: string) => {
    setDownloads((prev) => prev.filter((d) => d.id !== id));
    try {
      await invoke("delete_download", { id });
    } catch {
      await refresh();
    }
  };

  const retry = async (id: string) => {
    setDownloads((prev) =>
      patchDownload(prev, id, {
        status: "queued",
        speed: "",
        progress: 0,
        error: undefined,
      })
    );
    try {
      await invoke("retry_download", { id });
    } catch {
      await refresh();
    }
  };

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

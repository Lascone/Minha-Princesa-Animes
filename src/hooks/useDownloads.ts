import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { DownloadItem } from "../types";

export function useDownloads() {
  const [downloads, setDownloads] = useState<DownloadItem[]>([]);

  const refresh = useCallback(async () => {
    const items = await invoke<DownloadItem[]>("get_downloads");
    setDownloads(items);
  }, []);

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

    const interval = setInterval(refresh, 5000);

    const win = getCurrentWindow();
    const unlistenFocus = win.onFocusChanged(({ payload: focused }) => {
      if (focused) refresh();
    });

    return () => {
      clearTimeout(restoreTimer);
      clearInterval(interval);
      unlistenProgress.then((fn) => fn());
      unlistenFocus.then((fn) => fn());
    };
  }, [refresh]);

  const cancel = async (id: string) => {
    await invoke("cancel_download", { id });
    await refresh();
  };

  const pause = async (id: string) => {
    await invoke("pause_download", { id });
    await refresh();
  };

  const resume = async (id: string) => {
    await invoke("resume_download", { id });
    await refresh();
  };

  const pauseAnime = async (title: string) => {
    await invoke("pause_anime", { title });
    await refresh();
  };

  const resumeAnime = async (title: string) => {
    await invoke("resume_anime", { title });
    await refresh();
  };

  const cancelAnime = async (title: string) => {
    await invoke("cancel_anime", { title });
    await refresh();
  };

  const remove = async (id: string) => {
    await invoke("delete_download", { id });
    await refresh();
  };

  const retry = async (id: string) => {
    await invoke("retry_download", { id });
    await refresh();
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

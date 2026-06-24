import { useCallback, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

function friendlyUpdateError(message: string): string {
  const lower = message.toLowerCase();
  if (
    lower.includes("valid release json") ||
    lower.includes("404") ||
    lower.includes("could not fetch")
  ) {
    return "Nenhum release publicado no GitHub ainda (ou o build ainda está em andamento). Tente novamente em alguns minutos.";
  }
  if (lower.includes("network") || lower.includes("fetch")) {
    return "Sem conexão com o GitHub. Verifique sua internet e tente de novo.";
  }
  return message;
}

export type UpdateStatus =
  | "idle"
  | "checking"
  | "uptodate"
  | "available"
  | "downloading"
  | "installing"
  | "error";

export function useUpdater() {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [currentVersion, setCurrentVersion] = useState<string>("…");
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const [notes, setNotes] = useState<string>("");
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [pendingUpdate, setPendingUpdate] = useState<Update | null>(null);

  const loadVersion = useCallback(async () => {
    try {
      const v = await getVersion();
      setCurrentVersion(v);
    } catch {
      setCurrentVersion("0.1.0");
    }
  }, []);

  const checkForUpdates = useCallback(async () => {
    setStatus("checking");
    setError(null);
    setProgress(0);
    try {
      await loadVersion();
      const update = await check();
      if (!update) {
        setStatus("uptodate");
        setAvailableVersion(null);
        setPendingUpdate(null);
        return null;
      }
      setPendingUpdate(update);
      setAvailableVersion(update.version);
      setNotes(update.body ?? "");
      setStatus("available");
      return update;
    } catch (e) {
      const message = friendlyUpdateError(
        e instanceof Error ? e.message : String(e)
      );
      setError(message);
      setStatus("error");
      return null;
    }
  }, [loadVersion]);

  const installUpdate = useCallback(async () => {
    const update = pendingUpdate ?? (await check());
    if (!update) {
      setStatus("uptodate");
      return;
    }

    setPendingUpdate(update);
    setStatus("downloading");
    setError(null);
    setProgress(0);

    try {
      let downloaded = 0;
      let total = 0;

      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          setStatus("downloading");
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total > 0) {
            setProgress(Math.min(100, Math.round((downloaded / total) * 100)));
          }
        } else if (event.event === "Finished") {
          setProgress(100);
        }
      });

      setStatus("installing");
      await relaunch();
    } catch (e) {
      const message = friendlyUpdateError(
        e instanceof Error ? e.message : String(e)
      );
      setError(message);
      setStatus("error");
    }
  }, [pendingUpdate]);

  return {
    status,
    currentVersion,
    availableVersion,
    notes,
    progress,
    error,
    loadVersion,
    checkForUpdates,
    installUpdate,
  };
}

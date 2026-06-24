import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Icon } from "./Icon";

type ToastKind = "completed" | "failed" | "queue_idle";

interface DownloadToast {
  id: number;
  kind: ToastKind;
  title: string;
  message: string;
}

let toastId = 0;

export function DownloadNotifications() {
  const [toasts, setToasts] = useState<DownloadToast[]>([]);

  useEffect(() => {
    const unlisten = listen<{
      kind: ToastKind;
      animeTitle?: string;
      episodeLabel?: string;
      error?: string;
      hasFailures?: boolean;
    }>("download-notification", (event) => {
      const payload = event.payload;
      let title = "";
      let message = "";

      if (payload.kind === "completed") {
        title = "Download concluído";
        message = `${payload.animeTitle ?? "Anime"} — ${payload.episodeLabel ?? ""}`;
      } else if (payload.kind === "failed") {
        title = "Download falhou";
        message = `${payload.animeTitle ?? "Anime"} — ${payload.episodeLabel ?? ""}`;
        if (payload.error) message += `\n${payload.error}`;
      } else {
        title = payload.hasFailures
          ? "Fila finalizada com erros"
          : "Downloads finalizados";
        message = payload.hasFailures
          ? "Confira os episódios com erro na biblioteca."
          : "Todos os episódios da fila foram baixados.";
      }

      const id = ++toastId;
      setToasts((prev) => [...prev, { id, kind: payload.kind, title, message }]);
      window.setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== id));
      }, 6000);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div className="download-toast-stack" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className={`download-toast download-toast--${toast.kind}`}>
          <Icon
            name={
              toast.kind === "completed"
                ? "fa-circle-check"
                : toast.kind === "failed"
                  ? "fa-circle-exclamation"
                  : "fa-bell"
            }
          />
          <div>
            <strong>{toast.title}</strong>
            <span>{toast.message}</span>
          </div>
        </div>
      ))}
    </div>
  );
}

import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";

export function TrayCloseModal() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    getCurrentWindow()
      .onCloseRequested((event) => {
        event.preventDefault();
        setOpen(true);
      })
      .then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => {});

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const minimizeToTray = async () => {
    setOpen(false);
    await invoke("hide_window_to_tray");
  };

  const exitApp = async () => {
    setOpen(false);
    await invoke("exit_app");
  };

  if (!open) return null;

  return (
    <div className="modal-overlay" role="dialog" aria-modal="true" aria-labelledby="tray-close-title">
      <div className="modal-panel tray-close-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3 id="tray-close-title">
            <Icon name="fa-window-minimize" /> Fechar aplicativo?
          </h3>
        </div>
        <div className="tray-close-body">
          <p>
            Os downloads continuam em segundo plano. Você pode minimizar para a{" "}
            <strong>bandeja do sistema</strong> (ícone perto do relógio) ou sair completamente.
          </p>
          <p className="tray-close-hint">
            <Icon name="fa-circle-info" /> Clique no ícone na bandeja para abrir o app novamente.
          </p>
        </div>
        <div className="modal-footer tray-close-footer">
          <button type="button" className="btn-ghost" onClick={() => setOpen(false)}>
            Cancelar
          </button>
          <button type="button" className="btn-ghost btn-danger-text" onClick={exitApp}>
            <Icon name="fa-power-off" /> Sair completamente
          </button>
          <button type="button" className="btn-primary" onClick={minimizeToTray}>
            <Icon name="fa-down-left-and-up-right-to-center" /> Minimizar para bandeja
          </button>
        </div>
      </div>
    </div>
  );
}

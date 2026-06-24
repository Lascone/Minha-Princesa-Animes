import { useEffect } from "react";
import { useUpdater } from "../hooks/useUpdater";
import { Icon } from "./Icon";

const STATUS_TEXT: Record<string, string> = {
  idle: "Verifique se há uma versão mais recente no GitHub.",
  checking: "Consultando GitHub Releases…",
  uptodate: "Você já está na versão mais recente.",
  available: "Nova versão disponível para instalar.",
  downloading: "Baixando atualização…",
  installing: "Instalando… o app vai reiniciar.",
  error: "Não foi possível verificar ou instalar a atualização.",
};

export function UpdateSection() {
  const {
    status,
    currentVersion,
    availableVersion,
    notes,
    progress,
    error,
    loadVersion,
    checkForUpdates,
    installUpdate,
  } = useUpdater();

  useEffect(() => {
    loadVersion();
  }, [loadVersion]);

  const busy = status === "checking" || status === "downloading" || status === "installing";

  return (
    <div className="setting-group update-section">
      <label>
        <Icon name="fa-cloud-arrow-down" /> Atualizações
      </label>
      <p className="update-version">
        Versão instalada: <strong>v{currentVersion}</strong>
      </p>
      <p className="update-hint">{STATUS_TEXT[status] ?? STATUS_TEXT.idle}</p>

      {availableVersion && status === "available" && (
        <div className="update-available">
          <span className="chip-meta">
            <Icon name="fa-gift" /> v{availableVersion} disponível
          </span>
          {notes && <p className="update-notes">{notes}</p>}
        </div>
      )}

      {status === "downloading" && (
        <div className="progress-block">
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${progress}%` }} />
          </div>
          <span className="progress-label">{progress}%</span>
        </div>
      )}

      {(error || status === "error") && error && (
        <small className="error-text">
          <Icon name="fa-circle-exclamation" /> {error}
        </small>
      )}

      <div className="update-actions">
        <button
          type="button"
          className="btn-ghost"
          onClick={() => checkForUpdates()}
          disabled={busy}
        >
          {status === "checking" ? (
            <>
              <Icon name="fa-spinner" spin /> Verificando…
            </>
          ) : (
            <>
              <Icon name="fa-rotate" /> Verificar atualizações
            </>
          )}
        </button>

        {status === "available" && (
          <button type="button" className="btn-primary" onClick={() => installUpdate()} disabled={busy}>
            <Icon name="fa-download" /> Instalar v{availableVersion}
          </button>
        )}
      </div>

      <small>
        As atualizações vêm de{" "}
        <a
          href="https://github.com/Lascone/Minha-Princesa-Animes/releases"
          target="_blank"
          rel="noreferrer"
        >
          GitHub Releases
        </a>
        . Builds de desenvolvimento não recebem auto-update.
      </small>
    </div>
  );
}

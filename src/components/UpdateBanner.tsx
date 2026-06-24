import { useEffect, useState } from "react";
import { useUpdater } from "../hooks/useUpdater";
import { Icon } from "./Icon";

interface UpdateBannerProps {
  onOpenSettings?: () => void;
}

export function UpdateBanner({ onOpenSettings }: UpdateBannerProps) {
  const { status, availableVersion, notes, checkForUpdates, installUpdate } = useUpdater();
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    checkForUpdates();
  }, [checkForUpdates]);

  if (dismissed || status !== "available" || !availableVersion) {
    return null;
  }

  return (
    <div className="update-banner" role="status">
      <div className="update-banner-text">
        <Icon name="fa-sparkles" />
        <div>
          <strong>Nova versão v{availableVersion} disponível</strong>
          {notes && <span className="update-banner-notes">{notes.slice(0, 120)}</span>}
        </div>
      </div>
      <div className="update-banner-actions">
        <button type="button" className="btn-primary btn-sm" onClick={() => installUpdate()}>
          <Icon name="fa-download" /> Atualizar agora
        </button>
        {onOpenSettings && (
          <button type="button" className="btn-ghost btn-sm" onClick={onOpenSettings}>
            Detalhes
          </button>
        )}
        <button
          type="button"
          className="btn-ghost btn-sm btn-icon"
          onClick={() => setDismissed(true)}
          aria-label="Fechar"
        >
          <Icon name="fa-xmark" />
        </button>
      </div>
    </div>
  );
}

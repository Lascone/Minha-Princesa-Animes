import type { DownloadItem } from "../types";
import { Icon } from "./Icon";

interface PlayerUpNextProps {
  nextEpisode: DownloadItem;
  autoNextCountdown: number | null;
  onPlayNow: () => void;
  onDismiss: () => void;
  onCancelCountdown: () => void;
}

export function PlayerUpNext({
  nextEpisode,
  autoNextCountdown,
  onPlayNow,
  onDismiss,
  onCancelCountdown,
}: PlayerUpNextProps) {
  const isCountingDown = autoNextCountdown !== null;
  const countdownProgress =
    autoNextCountdown !== null
      ? ((5 - autoNextCountdown) / 5) * 100
      : 0;

  return (
    <div className={`player-up-next ${isCountingDown ? "is-counting" : ""}`}>
      <div className="player-up-next-card">
        <button
          type="button"
          className="player-up-next-dismiss"
          onClick={isCountingDown ? onCancelCountdown : onDismiss}
          aria-label="Fechar preview"
        >
          <Icon name="fa-xmark" />
        </button>

        <span className="player-up-next-label">
          <Icon name="fa-forward-step" /> A seguir
        </span>

        <strong className="player-up-next-episode">{nextEpisode.episodeLabel}</strong>
        <span className="player-up-next-anime">{nextEpisode.animeTitle}</span>

        {isCountingDown ? (
          <>
            <p className="player-up-next-countdown-text">
              Iniciando em {autoNextCountdown}s…
            </p>
            <div className="player-up-next-countdown-bar">
              <div
                className="player-up-next-countdown-fill"
                style={{ width: `${countdownProgress}%` }}
              />
            </div>
            <button
              type="button"
              className="btn-ghost btn-sm player-up-next-btn"
              onClick={onCancelCountdown}
            >
              Cancelar
            </button>
          </>
        ) : (
          <button
            type="button"
            className="btn-primary btn-sm player-up-next-btn"
            onClick={onPlayNow}
          >
            <Icon name="fa-play" /> Assistir agora
          </button>
        )}
      </div>
    </div>
  );
}

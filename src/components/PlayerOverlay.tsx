import { Icon } from "./Icon";
import { formatPlaybackTime } from "../utils/watchProgress";

interface PlayerOverlayProps {
  animeTitle: string;
  episodeLabel: string;
  onClose: () => void;
  resumeHint: string | null;
  showResumeChoice: boolean;
  pendingResumeTime: number;
  onApplyResume: () => void;
  onStartFromBeginning: () => void;
}

export function PlayerOverlay({
  animeTitle,
  episodeLabel,
  onClose,
  resumeHint,
  showResumeChoice,
  pendingResumeTime,
  onApplyResume,
  onStartFromBeginning,
}: PlayerOverlayProps) {
  return (
    <>
      <div className="player-cinema-top">
        <div className="player-cinema-titles">
          <strong>{animeTitle}</strong>
          <span>{episodeLabel}</span>
        </div>
        <div className="player-cinema-top-actions">
          <span className="player-shortcut-hint" title="Atalhos">
            <Icon name="fa-keyboard" /> N próximo
          </span>
          <button
            type="button"
            className="player-cinema-close"
            onClick={onClose}
            aria-label="Fechar player"
          >
            <Icon name="fa-xmark" />
          </button>
        </div>
      </div>

      {showResumeChoice && pendingResumeTime > 0 && (
        <div className="player-resume-choice">
          <div className="player-resume-choice-card">
            <Icon name="fa-clock-rotate-left" />
            <div>
              <strong>Continuar de {formatPlaybackTime(pendingResumeTime)}?</strong>
              <span>Ou assistir do início</span>
            </div>
            <div className="player-resume-choice-actions">
              <button
                type="button"
                className="btn-primary btn-sm"
                onClick={onApplyResume}
              >
                Retomar
              </button>
              <button
                type="button"
                className="btn-ghost btn-sm"
                onClick={onStartFromBeginning}
              >
                Do início
              </button>
            </div>
          </div>
        </div>
      )}

      {resumeHint && !showResumeChoice && (
        <div className="video-player-resume-hint">
          <Icon name="fa-clock-rotate-left" /> {resumeHint}
        </div>
      )}
    </>
  );
}

import "plyr/dist/plyr.css";
import type { DownloadItem } from "../types";
import { usePlyrPlayer } from "../hooks/usePlyrPlayer";
import { Icon } from "./Icon";
import { PlayerOverlay } from "./PlayerOverlay";
import { PlayerUpNext } from "./PlayerUpNext";

interface VideoPlayerProps {
  item: DownloadItem;
  nextEpisode?: DownloadItem | null;
  onNextEpisode?: (item: DownloadItem) => void;
  onClose: () => void;
}

export function VideoPlayer({
  item,
  nextEpisode,
  onNextEpisode,
  onClose,
}: VideoPlayerProps) {
  const {
    containerRef,
    videoRef,
    error,
    isLoading,
    resumeHint,
    showResumeChoice,
    pendingResumeTime,
    showUpNext,
    autoNextCountdown,
    applyResume,
    startFromBeginning,
    playNextNow,
    cancelAutoNext,
    dismissUpNext,
  } = usePlyrPlayer({ item, nextEpisode, onNextEpisode });

  if (error) {
    return (
      <div className="video-player-panel video-player-panel--error">
        <div className="video-player-error">
          <Icon name="fa-circle-exclamation" /> {error}
        </div>
        <button type="button" className="btn-ghost btn-sm" onClick={onClose}>
          <Icon name="fa-xmark" /> Fechar
        </button>
      </div>
    );
  }

  return (
    <div className="video-player-panel video-player-panel--cinema">
      <div ref={containerRef} className="video-player-cinema-frame plyr-princesa">
        <div className="video-player-aspect">
          {isLoading && (
            <div className="video-player-loading" aria-hidden="true">
              <div className="video-player-loading-glow" />
              <Icon name="fa-spinner" spin />
              <span>Carregando episódio…</span>
            </div>
          )}

          <PlayerOverlay
            animeTitle={item.animeTitle}
            episodeLabel={item.episodeLabel}
            onClose={onClose}
            resumeHint={resumeHint}
            showResumeChoice={showResumeChoice}
            pendingResumeTime={pendingResumeTime}
            onApplyResume={applyResume}
            onStartFromBeginning={startFromBeginning}
          />

          {showUpNext && nextEpisode && (
            <PlayerUpNext
              nextEpisode={nextEpisode}
              autoNextCountdown={autoNextCountdown}
              onPlayNow={playNextNow}
              onDismiss={dismissUpNext}
              onCancelCountdown={cancelAutoNext}
            />
          )}

          <video
            ref={videoRef}
            className="video-player"
            playsInline
            preload="metadata"
          />
        </div>
      </div>
    </div>
  );
}

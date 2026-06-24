import type { DownloadItem } from "../types";
import { Video } from "@videojs/react/video";
import { Icon } from "./Icon";
import { PlayerOverlay } from "./PlayerOverlay";
import { PlayerUpNext } from "./PlayerUpNext";
import { PrincesaPlayer } from "../player/createPrincesaPlayer";
import { PrincesaVideoSkin } from "../player/PrincesaVideoSkin";
import { usePrincesaPlayback } from "../player/usePrincesaPlayback";

interface VideoPlayerProps {
  item: DownloadItem;
  nextEpisode?: DownloadItem | null;
  onNextEpisode?: (item: DownloadItem) => void;
  onClose: () => void;
}

function VideoPlayerInner({
  item,
  nextEpisode,
  onNextEpisode,
  onClose,
}: VideoPlayerProps) {
  const {
    src,
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
  } = usePrincesaPlayback({ item, nextEpisode, onNextEpisode });

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
      <div className="video-player-cinema-frame">
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

          <PrincesaVideoSkin>
            <Video
              src={src}
              className="video-player"
              playsInline
              preload="metadata"
            />
          </PrincesaVideoSkin>
        </div>
      </div>
    </div>
  );
}

export function VideoPlayer(props: VideoPlayerProps) {
  if (!props.item.outputPath) {
    return (
      <div className="video-player-panel video-player-panel--error">
        <div className="video-player-error">
          <Icon name="fa-circle-exclamation" /> Arquivo não encontrado
        </div>
        <button type="button" className="btn-ghost btn-sm" onClick={props.onClose}>
          <Icon name="fa-xmark" /> Fechar
        </button>
      </div>
    );
  }

  return (
    <PrincesaPlayer.Provider>
      <VideoPlayerInner {...props} />
    </PrincesaPlayer.Provider>
  );
}

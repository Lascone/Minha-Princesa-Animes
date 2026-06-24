import { Component, type ErrorInfo, type ReactNode } from "react";
import type { DownloadItem } from "../types";
import { Video, VideoSkin } from "@videojs/react/video";
import "@videojs/react/video/skin.css";
import "../player/PrincesaVideoSkin.css";
import { Icon } from "./Icon";
import { PlayerOverlay } from "./PlayerOverlay";
import { PlayerUpNext } from "./PlayerUpNext";
import { PrincesaPlayer } from "../player/createPrincesaPlayer";
import { usePrincesaPlayback } from "../player/usePrincesaPlayback";

interface VideoPlayerProps {
  item: DownloadItem;
  nextEpisode?: DownloadItem | null;
  onNextEpisode?: (item: DownloadItem) => void;
  onClose: () => void;
}

interface PlayerErrorBoundaryProps {
  children: ReactNode;
  onClose: () => void;
}

interface PlayerErrorBoundaryState {
  error: string | null;
}

class PlayerErrorBoundary extends Component<
  PlayerErrorBoundaryProps,
  PlayerErrorBoundaryState
> {
  state: PlayerErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error: error.message || "Erro no player" };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("VideoPlayer crash:", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="video-player-panel video-player-panel--error">
          <div className="video-player-error">
            <Icon name="fa-circle-exclamation" /> {this.state.error}
          </div>
          <button
            type="button"
            className="btn-ghost btn-sm"
            onClick={this.props.onClose}
          >
            <Icon name="fa-xmark" /> Fechar
          </button>
        </div>
      );
    }
    return this.props.children;
  }
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

          {src ? (
            <VideoSkin className="media-princesa-skin player-princesa-skin">
              <Video
                src={src}
                className="video-player"
                playsInline
                preload="auto"
              />
            </VideoSkin>
          ) : null}
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
    <PlayerErrorBoundary onClose={props.onClose}>
      <PrincesaPlayer.Provider>
        <VideoPlayerInner {...props} />
      </PrincesaPlayer.Provider>
    </PlayerErrorBoundary>
  );
}

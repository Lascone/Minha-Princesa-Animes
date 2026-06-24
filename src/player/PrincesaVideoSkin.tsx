import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import {
  AlertDialog,
  BufferingIndicator,
  CaptionsButton,
  CastButton,
  Container,
  Controls,
  ErrorDialog,
  FullscreenButton,
  Gesture,
  Hotkey,
  Menu,
  MuteButton,
  PiPButton,
  PlayButton,
  PlaybackRateMenu,
  Popover,
  Poster,
  SeekButton,
  SeekIndicator,
  Slider,
  StatusAnnouncer,
  StatusIndicator,
  Time,
  TimeSlider,
  Tooltip,
  VolumeIndicator,
  VolumeSlider,
  usePlaybackRateMenu,
  usePlayer,
} from "@videojs/react";
import {
  CaptionsOffIcon,
  CaptionsOnIcon,
  CastEnterIcon,
  CastExitIcon,
  CheckIcon,
  ChevronIcon,
  FullscreenEnterIcon,
  FullscreenExitIcon,
  PauseIcon,
  PipEnterIcon,
  PipExitIcon,
  PlayIcon,
  RestartIcon,
  SeekIcon,
  SpinnerIcon,
  VolumeHighIcon,
  VolumeLowIcon,
  VolumeOffIcon,
} from "@videojs/react/icons";
import { cn } from "@videojs/utils/style";
import "./PrincesaVideoSkin.css";

const SEEK_TIME = 10;
const TOP_STATUS_ACTIONS = [
  "toggleSubtitles",
  "toggleFullscreen",
  "togglePictureInPicture",
] as const;
const CENTER_STATUS_ACTIONS = ["togglePaused"] as const;

type SkinButtonProps = ButtonHTMLAttributes<HTMLButtonElement>;

const SkinButton = forwardRef<HTMLButtonElement, SkinButtonProps>(
  function SkinButton({ className, ...props }, ref) {
    return (
      <button
        ref={ref}
        type="button"
        className={cn(
          "media-button media-button--subtle media-button--icon",
          className
        )}
        {...props}
      />
    );
  }
);

function VolumePopover() {
  const volumeUnsupported = usePlayer((s) => s.volumeAvailability === "unsupported");

  const muteButton = (
    <MuteButton
      className="media-button--mute"
      render={<SkinButton aria-label="Volume" />}
    >
      <VolumeOffIcon className="media-icon media-icon--volume-off" />
      <VolumeLowIcon className="media-icon media-icon--volume-low" />
      <VolumeHighIcon className="media-icon media-icon--volume-high" />
    </MuteButton>
  );

  if (volumeUnsupported) return muteButton;

  return (
    <Popover.Root openOnHover delay={200} closeDelay={100} side="top">
      <Popover.Trigger render={muteButton} />
      <Popover.Popup className="media-surface media-popover media-popover--volume">
        <VolumeSlider.Root
          className="media-slider"
          orientation="vertical"
          thumbAlignment="edge"
        >
          <Slider.Track className="media-slider__track">
            <Slider.Fill className="media-slider__fill" />
          </Slider.Track>
          <Slider.Thumb className="media-slider__thumb media-slider__thumb--persistent" />
        </VolumeSlider.Root>
      </Popover.Popup>
    </Popover.Root>
  );
}

function PlaybackRateMenuItems() {
  const { options, setValue, value } = usePlaybackRateMenu();

  return (
    <Menu.RadioGroup
      className="media-menu__group"
      value={value}
      onValueChange={setValue}
      label="Velocidade"
    >
      {options.map((option) => (
        <Menu.RadioItem
          key={option.value}
          className="media-menu__item"
          value={option.value}
          disabled={option.disabled}
        >
          <span>{option.label}</span>
          <Menu.ItemIndicator
            checked={option.value === value}
            forceMount
            className="media-menu__indicator"
          >
            <CheckIcon className="media-icon" />
          </Menu.ItemIndicator>
        </Menu.RadioItem>
      ))}
    </Menu.RadioGroup>
  );
}

export interface PrincesaVideoSkinProps {
  children: ReactNode;
  className?: string;
  poster?: string;
}

export function PrincesaVideoSkin({
  children,
  className,
  poster,
}: PrincesaVideoSkinProps) {
  return (
    <Container
      className={cn(
        "media-default-skin media-default-skin--video media-princesa-skin",
        className
      )}
    >
      {children}

      {poster ? <Poster src={poster} /> : null}

      <BufferingIndicator
        render={(props) => (
          <div {...props} className="media-buffering-indicator">
            <div className="media-surface">
              <SpinnerIcon className="media-icon" />
            </div>
          </div>
        )}
      />

      <ErrorDialog.Root>
        <AlertDialog.Popup className="media-error">
          <div className="media-error__dialog media-surface">
            <div className="media-error__content">
              <AlertDialog.Title className="media-error__title">
                Não foi possível reproduzir
              </AlertDialog.Title>
              <ErrorDialog.Description className="media-error__description" />
            </div>
            <div className="media-error__actions">
              <AlertDialog.Close className="media-button media-button--primary">
                Fechar
              </AlertDialog.Close>
            </div>
          </div>
        </AlertDialog.Popup>
      </ErrorDialog.Root>

      <Controls.Root className="media-surface media-controls media-princesa-controls">
        <Tooltip.Provider>
          <div className="media-button-group">
            <Tooltip.Root side="top">
              <Tooltip.Trigger
                render={
                  <PlayButton
                    className="media-button--play"
                    render={<SkinButton aria-label="Reproduzir" />}
                  >
                    <RestartIcon className="media-icon media-icon--restart" />
                    <PlayIcon className="media-icon media-icon--play" />
                    <PauseIcon className="media-icon media-icon--pause" />
                  </PlayButton>
                }
              />
              <Tooltip.Popup className="media-surface media-tooltip" />
            </Tooltip.Root>

            <Tooltip.Root side="top">
              <Tooltip.Trigger
                render={
                  <SeekButton
                    seconds={-SEEK_TIME}
                    className="media-button--seek"
                    render={<SkinButton aria-label={`Voltar ${SEEK_TIME}s`} />}
                  >
                    <span className="media-icon__container">
                      <SeekIcon className="media-icon media-icon--seek media-icon--flipped" />
                      <span className="media-icon__label">{SEEK_TIME}</span>
                    </span>
                  </SeekButton>
                }
              />
              <Tooltip.Popup className="media-surface media-tooltip" />
            </Tooltip.Root>

            <Tooltip.Root side="top">
              <Tooltip.Trigger
                render={
                  <SeekButton
                    seconds={SEEK_TIME}
                    className="media-button--seek"
                    render={<SkinButton aria-label={`Avançar ${SEEK_TIME}s`} />}
                  >
                    <span className="media-icon__container">
                      <SeekIcon className="media-icon media-icon--seek" />
                      <span className="media-icon__label">{SEEK_TIME}</span>
                    </span>
                  </SeekButton>
                }
              />
              <Tooltip.Popup className="media-surface media-tooltip" />
            </Tooltip.Root>
          </div>

          <div className="media-time-controls">
            <Time.Value type="current" className="media-time" />
            <TimeSlider.Root className="media-slider media-princesa-time-slider">
              <Slider.Track className="media-slider__track">
                <Slider.Fill className="media-slider__fill" />
                <Slider.Buffer className="media-slider__buffer" />
              </Slider.Track>
              <Slider.Thumb className="media-slider__thumb" />
              <div className="media-surface media-preview media-slider__preview">
                <Slider.Thumbnail className="media-preview__thumbnail" />
                <Slider.Value
                  type="pointer"
                  className="media-time media-preview__time"
                />
                <SpinnerIcon className="media-preview__spinner media-icon" />
              </div>
            </TimeSlider.Root>
            <Time.Value type="duration" className="media-time" />
          </div>

          <div className="media-button-group">
            <PlaybackRateMenu.Root side="top" align="center">
              <PlaybackRateMenu.Trigger
                className="media-button--playback-rate"
                render={<SkinButton aria-label="Velocidade" />}
              />
              <PlaybackRateMenu.Content className="media-surface media-popover media-menu media-menu--playback-rate">
                <PlaybackRateMenuItems />
              </PlaybackRateMenu.Content>
            </PlaybackRateMenu.Root>

            <VolumePopover />

            <Tooltip.Root side="top">
              <Tooltip.Trigger
                render={
                  <CaptionsButton
                    className="media-button--captions"
                    render={<SkinButton aria-label="Legendas" />}
                  >
                    <CaptionsOffIcon className="media-icon media-icon--captions-off" />
                    <CaptionsOnIcon className="media-icon media-icon--captions-on" />
                  </CaptionsButton>
                }
              />
              <Tooltip.Popup className="media-surface media-tooltip" />
            </Tooltip.Root>

            <Tooltip.Root side="top">
              <Tooltip.Trigger
                render={
                  <CastButton
                    className="media-button--cast"
                    render={<SkinButton aria-label="Transmitir" />}
                  >
                    <CastEnterIcon className="media-icon media-icon--cast-enter" />
                    <CastExitIcon className="media-icon media-icon--cast-exit" />
                  </CastButton>
                }
              />
              <Tooltip.Popup className="media-surface media-tooltip" />
            </Tooltip.Root>

            <Tooltip.Root side="top">
              <Tooltip.Trigger
                render={
                  <PiPButton
                    className="media-button--pip"
                    render={<SkinButton aria-label="Picture-in-picture" />}
                  >
                    <PipEnterIcon className="media-icon media-icon--pip-enter" />
                    <PipExitIcon className="media-icon media-icon--pip-exit" />
                  </PiPButton>
                }
              />
              <Tooltip.Popup className="media-surface media-tooltip" />
            </Tooltip.Root>

            <Tooltip.Root side="top">
              <Tooltip.Trigger
                render={
                  <FullscreenButton
                    className="media-button--fullscreen"
                    render={<SkinButton aria-label="Tela cheia" />}
                  >
                    <FullscreenEnterIcon className="media-icon media-icon--fullscreen-enter" />
                    <FullscreenExitIcon className="media-icon media-icon--fullscreen-exit" />
                  </FullscreenButton>
                }
              />
              <Tooltip.Popup className="media-surface media-tooltip" />
            </Tooltip.Root>
          </div>
        </Tooltip.Provider>
      </Controls.Root>

      <div className="media-overlay" />

      <Hotkey keys="Space" action="togglePaused" />
      <Hotkey keys="k" action="togglePaused" />
      <Hotkey keys="m" action="toggleMuted" />
      <Hotkey keys="f" action="toggleFullscreen" />
      <Hotkey keys="c" action="toggleSubtitles" />
      <Hotkey keys="i" action="togglePictureInPicture" />
      <Hotkey keys="ArrowRight" action="seekStep" value={SEEK_TIME / 2} />
      <Hotkey keys="ArrowLeft" action="seekStep" value={-(SEEK_TIME / 2)} />
      <Hotkey keys="l" action="seekStep" value={SEEK_TIME} />
      <Hotkey keys="j" action="seekStep" value={-SEEK_TIME} />
      <Hotkey keys="ArrowUp" action="volumeStep" value={0.05} />
      <Hotkey keys="ArrowDown" action="volumeStep" value={-0.05} />
      <Hotkey keys="0-9" action="seekToPercent" />
      <Hotkey keys="Home" action="seekToPercent" value={0} />
      <Hotkey keys="End" action="seekToPercent" value={100} />
      <Hotkey keys=">" action="speedUp" />
      <Hotkey keys="<" action="speedDown" />

      <Gesture type="tap" action="togglePaused" pointer="mouse" region="center" />
      <Gesture type="tap" action="toggleControls" pointer="touch" />
      <Gesture
        type="doubletap"
        action="seekStep"
        value={-SEEK_TIME}
        region="left"
      />
      <Gesture type="doubletap" action="toggleFullscreen" region="center" />
      <Gesture
        type="doubletap"
        action="seekStep"
        value={SEEK_TIME}
        region="right"
      />

      <StatusAnnouncer />

      <div className="media-input-feedback">
        <VolumeIndicator.Root className="media-surface media-input-feedback-island media-input-feedback-island--volume">
          <VolumeIndicator.Fill className="media-input-feedback-island__content">
            <VolumeHighIcon className="media-icon media-icon--volume-high" />
            <VolumeLowIcon className="media-icon media-icon--volume-low" />
            <VolumeOffIcon className="media-icon media-icon--volume-off" />
            <VolumeIndicator.Value className="media-input-feedback-island__value" />
          </VolumeIndicator.Fill>
        </VolumeIndicator.Root>

        <StatusIndicator.Root
          actions={[...TOP_STATUS_ACTIONS]}
          className="media-surface media-input-feedback-island media-input-feedback-island--status"
        >
          <div className="media-input-feedback-island__content">
            <CaptionsOnIcon className="media-icon media-icon--captions-on" />
            <CaptionsOffIcon className="media-icon media-icon--captions-off" />
            <FullscreenEnterIcon className="media-icon media-icon--fullscreen-enter" />
            <FullscreenExitIcon className="media-icon media-icon--fullscreen-exit" />
            <PipEnterIcon className="media-icon media-icon--pip-enter" />
            <PipExitIcon className="media-icon media-icon--pip-exit" />
            <StatusIndicator.Value className="media-input-feedback-island__value" />
          </div>
        </StatusIndicator.Root>

        <SeekIndicator.Root className="media-input-feedback-bubble">
          <ChevronIcon className="media-icon media-icon--seek" />
          <SeekIndicator.Value className="media-time" />
        </SeekIndicator.Root>

        <StatusIndicator.Root
          actions={[...CENTER_STATUS_ACTIONS]}
          className="media-input-feedback-bubble"
        >
          <PlayIcon className="media-icon media-icon--play" />
          <PauseIcon className="media-icon media-icon--pause" />
        </StatusIndicator.Root>
      </div>
    </Container>
  );
}

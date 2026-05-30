import {
  ChevronDown,
  Heart,
  ListMusic,
  Maximize2,
  Minimize,
  PanelRightClose,
  PanelRightOpen,
  Pause,
  Play,
  Repeat,
  Repeat1,
  Shuffle,
  SkipBack,
  SkipForward,
  Video,
  VideoOff,
  X,
} from "lucide-react";
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
} from "react";
import turntablePlayer from "../../assets/turntable-player.png";
import { toAssetUrl } from "../../lib/assets";
import { defaultArtworkSource } from "../../lib/defaultArtwork";
import { formatCount, formatDuration, formatFileSize } from "../../lib/format";
import { runWindowAction, startWindowDrag } from "../../lib/window";
import { useCoverPalette } from "../../hooks/useCoverPalette";
import type {
  AudioTimeline,
  PlaybackMode,
  PlayerOpenSourceRect,
  TrackSummary,
} from "../../types/domain";

interface FullPlayerOverlayProps {
  open: boolean;
  track: TrackSummary | null;
  timeline: AudioTimeline | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  musicVolume: number;
  playbackMode: PlaybackMode;
  isFavorite: boolean;
  closing: boolean;
  openSourceRect: PlayerOpenSourceRect | null;
  onClose: () => void;
  onPlayPause: () => void;
  onPrevious: () => void;
  onNext: () => void;
  onSeek: (time: number) => void;
  onMusicVolumeChange: (volume: number) => void;
  onCyclePlaybackMode: () => void;
  onToggleFavorite: () => void;
}

export function FullPlayerOverlay({
  open,
  track,
  timeline,
  isPlaying,
  currentTime,
  duration,
  musicVolume,
  playbackMode,
  isFavorite,
  closing,
  openSourceRect,
  onClose,
  onPlayPause,
  onPrevious,
  onNext,
  onSeek,
  onMusicVolumeChange,
  onCyclePlaybackMode,
  onToggleFavorite,
}: FullPlayerOverlayProps) {
  const turntableRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const [isOpening, setIsOpening] = useState(false);
  const [entryStyle, setEntryStyle] = useState<CSSProperties | undefined>();
  const [videoEnabled, setVideoEnabled] = useState(true);
  const [videoInfoOpen, setVideoInfoOpen] = useState(false);
  const fallbackCover = defaultArtworkSource(track?.game, "cover");
  const palette = useCoverPalette(track?.coverPath ?? fallbackCover);
  const cover = toAssetUrl(track?.coverPath) ?? fallbackCover;
  const videoSource = toAssetUrl(track?.videoPath);
  const hasVideo = Boolean(track?.hasVideo && videoSource);
  const videoActive = Boolean(hasVideo && videoEnabled);
  const parked = !open && !closing;
  const safeDuration = duration || timeline?.duration || track?.duration || 0;
  const seekMax = Math.max(1, safeDuration);
  const progressStyle = {
    "--progress": `${Math.min(100, Math.max(0, (currentTime / seekMax) * 100))}%`,
  } as CSSProperties;
  const infoItems = [
    { label: "曲名", value: track?.title ?? "--" },
    { label: "作曲", value: track?.artist ?? "--", highlight: true },
    { label: "谱师", value: track?.author ?? "--" },
    { label: "BPM", value: track ? `${Math.round(track.bpm)}` : "--" },
    { label: "时长", value: formatDuration(safeDuration) },
    { label: "打拍音", value: `${formatCount(timeline?.hitEvents.length ?? 0)} 个` },
    { label: "谱面音效", value: `${formatCount(timeline?.playSoundEvents.length ?? 0)} 个` },
    { label: "长按音", value: `${formatCount(timeline?.holdSoundEvents.length ?? 0)} 个` },
    { label: "音乐文件", value: formatFileSize(track?.audioFileSize) },
    { label: "视频", value: track?.hasVideo ? "有" : "无" },
    { label: "谱面文件", value: fileName(track?.chartPath) },
    { label: "音频文件", value: fileName(track?.audioPath) },
    { label: "封面", value: fileName(track?.coverPath) },
    { label: "图标", value: fileName(track?.iconPath) },
    { label: "视频文件", value: fileName(track?.videoPath) },
  ];

  const style = {
    "--player-accent": palette.accent,
    "--player-accent-text": palette.accentText,
    "--player-bg-a": palette.backgroundA,
    "--player-bg-b": palette.backgroundB,
    "--player-soft": palette.soft,
  } as CSSProperties;
  useLayoutEffect(() => {
    if (!open || !openSourceRect || closing || !turntableRef.current) {
      setIsOpening(false);
      setEntryStyle(undefined);
      return;
    }

    const target = turntableRef.current.getBoundingClientRect();
    const sourceCenterX = openSourceRect.left + openSourceRect.width / 2;
    const sourceCenterY = openSourceRect.top + openSourceRect.height / 2;
    const targetCenterX = target.left + target.width / 2;
    const targetCenterY = target.top + target.height / 2;
    const scale = Math.max(0.14, Math.min(0.24, openSourceRect.width / Math.max(1, target.width)));

    setEntryStyle({
      "--open-x": `${sourceCenterX - targetCenterX}px`,
      "--open-y": `${sourceCenterY - targetCenterY}px`,
      "--open-scale": `${scale}`,
    } as CSSProperties);
    setIsOpening(true);

    const frame = requestAnimationFrame(() => setIsOpening(false));
    return () => cancelAnimationFrame(frame);
  }, [closing, open, openSourceRect, track?.id]);

  useEffect(() => {
    setVideoEnabled(true);
    setVideoInfoOpen(false);
  }, [track?.id]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) {
      return;
    }

    if (!videoActive || !videoSource) {
      video.pause();
      return;
    }

    const pitch = Math.max(0.05, timeline?.pitch ?? 1);
    const songOffsetSec = (timeline?.songOffsetMs ?? 0) / 1000;
    const rawTarget = currentTime * pitch - songOffsetSec + (track?.videoOffsetSec ?? 0);
    const shouldPlayVideo = rawTarget >= 0;
    let targetTime = Math.max(0, rawTarget);
    if (Number.isFinite(video.duration) && video.duration > 0) {
      targetTime = track?.loopVideo ? targetTime % video.duration : Math.min(targetTime, video.duration);
    }

    video.playbackRate = pitch;
    const drift = Math.abs(video.currentTime - targetTime);
    if (drift > (isPlaying ? 0.08 : 0.02)) {
      try {
        video.currentTime = targetTime;
      } catch {
        // Metadata may not be ready yet; the next tick will try again.
      }
    }

    if (isPlaying && shouldPlayVideo) {
      void video.play().catch(() => undefined);
    } else {
      video.pause();
    }
  }, [
    currentTime,
    isPlaying,
    timeline?.pitch,
    timeline?.songOffsetMs,
    track?.loopVideo,
    track?.videoOffsetSec,
    videoActive,
    videoSource,
  ]);

  function handleTopbarMouseDown(event: MouseEvent<HTMLElement>) {
    if (event.button !== 0 || shouldSkipWindowDrag(event.target)) {
      return;
    }
    event.preventDefault();
    startWindowDrag();
  }

  function handleTopbarDoubleClick(event: MouseEvent<HTMLElement>) {
    if (shouldSkipWindowDrag(event.target)) {
      return;
    }
    void runWindowAction("maximize");
  }

  return (
    <section
      className={[
        "player-overlay",
        parked ? "parked" : "",
        closing ? "closing" : "",
        videoActive ? "video-active" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      style={style}
      aria-hidden={parked}
    >
      {videoActive && videoSource && (
        <>
          <video
            ref={videoRef}
            className="player-video-bg"
            src={videoSource}
            muted
            playsInline
            loop={Boolean(track?.loopVideo)}
            preload="auto"
          />
          <div className="player-video-scrim" aria-hidden="true" />
        </>
      )}

      <header
        className="player-topbar"
        onMouseDown={handleTopbarMouseDown}
        onDoubleClick={handleTopbarDoubleClick}
      >
        <button className="overlay-close" type="button" onClick={onClose} title="返回" data-no-window-drag>
          <ChevronDown aria-hidden="true" />
        </button>
        <div className="overlay-drag-region" />
        <div className="window-controls overlay-window-controls" data-no-window-drag>
          <button type="button" title="最小化" onClick={() => void runWindowAction("minimize")}>
            <Minimize aria-hidden="true" />
          </button>
          <button type="button" title="最大化" onClick={() => void runWindowAction("maximize")}>
            <Maximize2 aria-hidden="true" />
          </button>
          <button type="button" title="关闭" onClick={() => void runWindowAction("close")}>
            <X aria-hidden="true" />
          </button>
        </div>
      </header>

      <div className={videoActive ? "player-stage video-stage" : "player-stage"}>
        {!videoActive && (
          <div
            ref={turntableRef}
            className={isOpening ? "turntable-card opening" : "turntable-card"}
            style={entryStyle}
          >
            <img className="turntable-image" src={turntablePlayer} alt="" aria-hidden="true" />
            <div
              className={isPlaying ? "turntable-artwork spinning" : "turntable-artwork"}
            >
              <img src={cover} alt={`${track?.title ?? "音乐"} 封面`} />
            </div>
          </div>
        )}

        {!videoActive && <TrackInfoPanel infoItems={infoItems} track={track} />}
      </div>

      {videoActive && (
        <>
          <button
            className={videoInfoOpen ? "video-info-tab hidden" : "video-info-tab"}
            type="button"
            onClick={() => setVideoInfoOpen(true)}
            title="展开谱面信息"
          >
            <PanelRightOpen aria-hidden="true" />
            <span>谱面信息</span>
          </button>
          <aside className={videoInfoOpen ? "video-info-drawer open" : "video-info-drawer"}>
            <header>
              <div>
                <strong>{track?.title ?? "谱面信息"}</strong>
                <span>{track?.artist ?? "Music Box"}</span>
              </div>
              <button type="button" onClick={() => setVideoInfoOpen(false)} title="收起谱面信息">
                <PanelRightClose aria-hidden="true" />
              </button>
            </header>
            <TrackInfoPanel infoItems={infoItems} track={track} compact />
          </aside>
        </>
      )}

      <footer className="overlay-controls">
        <div className="overlay-track">
          <button
            className={isFavorite ? "heart-button active" : "heart-button"}
            type="button"
            onClick={onToggleFavorite}
            title={isFavorite ? "取消喜欢" : "喜欢"}
            disabled={!track}
          >
            <Heart aria-hidden="true" />
          </button>
          <div>
            <strong>{track?.title ?? "还没有播放音乐"}</strong>
            <span>{track?.artist ?? "添加本地谱面后开始播放"}</span>
          </div>
        </div>
        <div className="overlay-transport">
          <button className="plain-icon" type="button" onClick={onCyclePlaybackMode} title={playbackModeLabel(playbackMode)}>
            {playbackModeIcon(playbackMode)}
          </button>
          <button className="plain-icon" type="button" onClick={onPrevious} title="上一首">
            <SkipBack aria-hidden="true" />
          </button>
          <button className="main-play overlay" type="button" onClick={onPlayPause} title={isPlaying ? "暂停" : "播放"}>
            {isPlaying ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
          </button>
          <button className="plain-icon" type="button" onClick={onNext} title="下一首">
            <SkipForward aria-hidden="true" />
          </button>
        </div>
        <div className="overlay-progress">
          <span>{formatDuration(currentTime)}</span>
          <input
            type="range"
            min="0"
            max={seekMax}
            step="0.01"
            value={Math.min(currentTime, seekMax)}
            style={progressStyle}
            onChange={(event) => onSeek(Number(event.currentTarget.value))}
            aria-label="播放进度"
          />
          <span>{formatDuration(safeDuration)}</span>
        </div>
        <div className="overlay-right">
          {hasVideo && (
            <button
              className={videoEnabled ? "plain-icon active" : "plain-icon"}
              type="button"
              onClick={() => setVideoEnabled((enabled) => !enabled)}
              title={videoEnabled ? "关闭视频" : "打开视频"}
            >
              {videoEnabled ? <Video aria-hidden="true" /> : <VideoOff aria-hidden="true" />}
            </button>
          )}
          <label className="overlay-volume">
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={musicVolume}
              onChange={(event) => onMusicVolumeChange(Number(event.currentTarget.value))}
              aria-label="音乐音量"
            />
          </label>
        </div>
      </footer>
    </section>
  );
}

interface TrackInfoPanelProps {
  track: TrackSummary | null;
  infoItems: Array<{ label: string; value: string; highlight?: boolean }>;
  compact?: boolean;
}

function TrackInfoPanel({ track, infoItems, compact = false }: TrackInfoPanelProps) {
  return (
    <div className={compact ? "player-info compact" : "player-info"}>
      {!compact && (
        <div className="info-title">
          <h1>{track?.title ?? "还没有播放音乐"}</h1>
          <span>{track?.artist ?? "Music Box"}</span>
        </div>
      )}
      <div className="info-stream">
        {infoItems.map((item, index) => (
          <InfoLine
            key={`${item.label}-${index}`}
            label={item.label}
            value={item.value}
            highlight={item.highlight}
          />
        ))}
      </div>
    </div>
  );
}

function shouldSkipWindowDrag(target: EventTarget) {
  return target instanceof Element && Boolean(target.closest("[data-no-window-drag]"));
}

interface InfoLineProps {
  label: string;
  value: string;
  highlight?: boolean;
}

function InfoLine({ label, value, highlight = false }: InfoLineProps) {
  const className = ["info-line", highlight ? "highlight" : ""]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={className}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function fileName(path?: string | null) {
  if (!path) {
    return "无";
  }
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function playbackModeLabel(mode: PlaybackMode) {
  switch (mode) {
    case "sequence":
      return "顺序播放";
    case "repeatAll":
      return "循环播放";
    case "repeatOne":
      return "单曲循环";
    case "shuffle":
      return "随机播放";
  }
}

function playbackModeIcon(mode: PlaybackMode) {
  switch (mode) {
    case "sequence":
      return <ListMusic aria-hidden="true" />;
    case "repeatAll":
      return <Repeat aria-hidden="true" />;
    case "repeatOne":
      return <Repeat1 aria-hidden="true" />;
    case "shuffle":
      return <Shuffle aria-hidden="true" />;
  }
}

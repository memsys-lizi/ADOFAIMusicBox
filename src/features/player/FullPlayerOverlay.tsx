import {
  ChevronDown,
  Disc3,
  Heart,
  ListMusic,
  Maximize2,
  Minimize,
  Pause,
  Play,
  Repeat,
  Repeat1,
  Shuffle,
  SkipBack,
  SkipForward,
  Volume2,
  Waves,
  X,
} from "lucide-react";
import type { CSSProperties, MouseEvent } from "react";
import { toAssetUrl } from "../../lib/assets";
import { formatCount, formatDuration, formatFileSize } from "../../lib/format";
import { runWindowAction, startWindowDrag } from "../../lib/window";
import { useCoverPalette } from "../../hooks/useCoverPalette";
import type { AudioTimeline, PlaybackMode, TrackSummary } from "../../types/domain";

interface FullPlayerOverlayProps {
  track: TrackSummary | null;
  timeline: AudioTimeline | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  musicVolume: number;
  playbackMode: PlaybackMode;
  isFavorite: boolean;
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
  track,
  timeline,
  isPlaying,
  currentTime,
  duration,
  musicVolume,
  playbackMode,
  isFavorite,
  onClose,
  onPlayPause,
  onPrevious,
  onNext,
  onSeek,
  onMusicVolumeChange,
  onCyclePlaybackMode,
  onToggleFavorite,
}: FullPlayerOverlayProps) {
  const palette = useCoverPalette(track?.coverPath);
  const cover = toAssetUrl(track?.coverPath);
  const safeDuration = duration || timeline?.duration || track?.duration || 0;
  const seekMax = Math.max(1, safeDuration);
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
    { label: "谱面文件", value: fileName(track?.adofaiPath) },
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
    <section className="player-overlay" style={style}>
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

      <div className="player-stage">
        <div className="turntable-card">
          <div className={isPlaying ? "vinyl spinning" : "vinyl"}>
            <div className="vinyl-rings" />
            <div className="vinyl-label">
              {cover ? <img src={cover} alt={`${track?.title ?? "音乐"} 封面`} /> : <Disc3 aria-hidden="true" />}
            </div>
          </div>
          <div className="tonearm">
            <span />
            <b />
          </div>
        </div>

        <div className="player-info">
          <div className="info-title">
            <h1>{track?.title ?? "还没有播放音乐"}</h1>
            <span>{track?.artist ?? "ADOFAI Music Box"}</span>
          </div>
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
      </div>

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
          <button className="plain-icon" type="button" title="音量">
            <Volume2 aria-hidden="true" />
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
            onChange={(event) => onSeek(Number(event.currentTarget.value))}
            aria-label="播放进度"
          />
          <span>{formatDuration(safeDuration)}</span>
        </div>
        <div className="overlay-right">
          <button className="plain-icon labeled active" type="button" title="打拍音">
            <Waves aria-hidden="true" />
          </button>
          <button className="plain-icon labeled" type="button" title="队列">
            <ListMusic aria-hidden="true" />
          </button>
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

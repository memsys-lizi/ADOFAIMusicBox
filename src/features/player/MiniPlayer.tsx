import {
  Heart,
  ListMusic,
  Pause,
  Play,
  Repeat,
  Repeat1,
  Shuffle,
  SkipBack,
  SkipForward,
  SlidersHorizontal,
} from "lucide-react";
import { type CSSProperties, useRef, useState } from "react";
import { EmptyArtwork } from "../../components/EmptyArtwork";
import { formatDuration } from "../../lib/format";
import type { PlaybackMode, PlayerOpenSourceRect, TrackSummary } from "../../types/domain";
import { VolumeMixer } from "./VolumeMixer";

interface MiniPlayerProps {
  track: TrackSummary | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  masterVolume: number;
  musicVolume: number;
  hitSoundVolume: number;
  playSoundVolume: number;
  playbackMode: PlaybackMode;
  isFavorite: boolean;
  onOpenPlayer: (sourceRect?: PlayerOpenSourceRect) => void;
  onPlayPause: () => void;
  onPrevious: () => void;
  onNext: () => void;
  onSeek: (time: number) => void;
  onMasterVolumeChange: (volume: number) => void;
  onMusicVolumeChange: (volume: number) => void;
  onHitSoundVolumeChange: (volume: number) => void;
  onPlaySoundVolumeChange: (volume: number) => void;
  onCyclePlaybackMode: () => void;
  onToggleFavorite: () => void;
}

export function MiniPlayer({
  track,
  isPlaying,
  currentTime,
  duration,
  masterVolume,
  musicVolume,
  hitSoundVolume,
  playSoundVolume,
  playbackMode,
  isFavorite,
  onOpenPlayer,
  onPlayPause,
  onPrevious,
  onNext,
  onSeek,
  onMasterVolumeChange,
  onMusicVolumeChange,
  onHitSoundVolumeChange,
  onPlaySoundVolumeChange,
  onCyclePlaybackMode,
  onToggleFavorite,
}: MiniPlayerProps) {
  const [mixOpen, setMixOpen] = useState(false);
  const coverRef = useRef<HTMLButtonElement>(null);
  const safeDuration = Math.max(0, duration);
  const seekMax = Math.max(1, safeDuration);
  const progressStyle = {
    "--progress": `${Math.min(100, Math.max(0, (currentTime / seekMax) * 100))}%`,
  } as CSSProperties;

  function handleOpenPlayer() {
    const rect = coverRef.current?.getBoundingClientRect();
    onOpenPlayer(rect ? rectToSource(rect) : undefined);
  }

  return (
    <footer className="mini-player">
      <div className="mini-track">
        <button
          ref={coverRef}
          className="mini-cover"
          type="button"
          onClick={handleOpenPlayer}
          title="打开播放页"
        >
          <EmptyArtwork
            title={track?.title ?? "默认封面"}
            imagePath={track?.coverPath}
            game={track?.game}
            size="sm"
            fallback="cover"
          />
        </button>
        <button className="mini-title" type="button" onClick={handleOpenPlayer}>
          <strong>{track?.title ?? "还没有播放音乐"}</strong>
          <small>{track ? track.artist : "添加本地谱面后开始播放"}</small>
        </button>
        <button
          className={isFavorite ? "heart-button active" : "heart-button"}
          type="button"
          onClick={onToggleFavorite}
          title={isFavorite ? "取消喜欢" : "喜欢"}
          disabled={!track}
        >
          <Heart aria-hidden="true" />
        </button>
      </div>

      <div className="mini-center">
        <div className="transport-row">
          <button className="plain-icon" type="button" onClick={onCyclePlaybackMode} title={playbackModeLabel(playbackMode)}>
            {playbackModeIcon(playbackMode)}
          </button>
          <button className="plain-icon" type="button" onClick={onPrevious} title="上一首">
            <SkipBack aria-hidden="true" />
          </button>
          <button className="main-play" type="button" onClick={onPlayPause} title={isPlaying ? "暂停" : "播放"}>
            {isPlaying ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
          </button>
          <button className="plain-icon" type="button" onClick={onNext} title="下一首">
            <SkipForward aria-hidden="true" />
          </button>
        </div>
        <div className="mini-progress">
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
      </div>

      <div className="mini-actions">
        <button className="plain-icon labeled" type="button" onClick={() => setMixOpen((open) => !open)} title="音量">
          <SlidersHorizontal aria-hidden="true" />
        </button>
      </div>

      {mixOpen && (
        <div className="mix-popover">
          <VolumeMixer
            masterVolume={masterVolume}
            musicVolume={musicVolume}
            hitSoundVolume={hitSoundVolume}
            playSoundVolume={playSoundVolume}
            onMasterVolumeChange={onMasterVolumeChange}
            onMusicVolumeChange={onMusicVolumeChange}
            onHitSoundVolumeChange={onHitSoundVolumeChange}
            onPlaySoundVolumeChange={onPlaySoundVolumeChange}
          />
        </div>
      )}
    </footer>
  );
}

function rectToSource(rect: DOMRect): PlayerOpenSourceRect {
  return {
    left: rect.left,
    top: rect.top,
    width: rect.width,
    height: rect.height,
  };
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

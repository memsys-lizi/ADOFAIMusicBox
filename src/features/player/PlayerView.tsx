import {
  Disc3,
  FileAudio,
  Pause,
  Play,
  SkipBack,
  SkipForward,
  Video,
  Volume2,
  Waves,
} from "lucide-react";
import type { ReactNode } from "react";
import { EmptyArtwork } from "../../components/EmptyArtwork";
import { StatPill } from "../../components/StatPill";
import { toAssetUrl } from "../../lib/assets";
import { formatCount, formatDuration } from "../../lib/format";
import type { AudioTimeline, TrackSummary } from "../../types/domain";

interface PlayerViewProps {
  track: TrackSummary | null;
  timeline: AudioTimeline | null;
  currentTime: number;
  duration: number;
  isPlaying: boolean;
  hitSoundsEnabled: boolean;
  playSoundsEnabled: boolean;
  videoEnabled: boolean;
  onPlayPause: () => void;
  onPrevious: () => void;
  onNext: () => void;
  onSeek: (time: number) => void;
  onToggleHitSounds: (enabled: boolean) => void;
  onTogglePlaySounds: (enabled: boolean) => void;
  onToggleVideo: (enabled: boolean) => void;
}

export function PlayerView({
  track,
  timeline,
  currentTime,
  duration,
  isPlaying,
  hitSoundsEnabled,
  playSoundsEnabled,
  videoEnabled,
  onPlayPause,
  onPrevious,
  onNext,
  onSeek,
  onToggleHitSounds,
  onTogglePlaySounds,
  onToggleVideo,
}: PlayerViewProps) {
  if (!track) {
    return (
      <section className="page player-page">
        <div className="empty-state">
          <Disc3 aria-hidden="true" />
          <h3>还没有正在播放的谱面</h3>
        </div>
      </section>
    );
  }

  const safeDuration = duration || timeline?.duration || track.duration || 0;
  const progress = safeDuration > 0 ? currentTime / safeDuration : 0;

  return (
    <section className="page player-page">
      <div className="player-hero">
        <div className="now-artwork-wrap">
          <EmptyArtwork title={track.title} imagePath={track.coverPath} size="lg" />
        </div>
        <div className="now-info">
          <p className="eyebrow">Now Playing</p>
          <h2>{track.title}</h2>
          <p>{track.artist} · 谱师 {track.author}</p>
          <div className="library-stats inline">
            <StatPill label="BPM" value={`${Math.round(track.bpm)}`} />
            <StatPill label="打拍音" value={formatCount(timeline?.hitEvents.length ?? 0)} />
            <StatPill label="谱面音效" value={formatCount(timeline?.playSoundEvents.length ?? 0)} />
          </div>
        </div>
      </div>

      {track.videoPath && videoEnabled && (
        <div className="video-preview">
          <video src={toAssetUrl(track.videoPath) ?? undefined} muted loop controls />
        </div>
      )}

      <div className="wave-panel">
        <div className="waveform" aria-hidden="true">
          {Array.from({ length: 96 }, (_, index) => (
            <span
              className={index / 96 <= progress ? "active" : ""}
              key={index}
              style={{ height: `${18 + ((index * 17) % 44)}px` }}
            />
          ))}
        </div>
        <input
          className="seek-slider"
          type="range"
          min="0"
          max={Math.max(1, safeDuration)}
          step="0.01"
          value={Math.min(currentTime, Math.max(1, safeDuration))}
          onChange={(event) => onSeek(Number(event.currentTarget.value))}
          aria-label="播放进度"
        />
        <div className="time-row">
          <span>{formatDuration(currentTime)}</span>
          <span>{formatDuration(safeDuration)}</span>
        </div>
      </div>

      <div className="transport-panel">
        <button className="icon-button" type="button" title="上一首" onClick={onPrevious}>
          <SkipBack aria-hidden="true" />
        </button>
        <button className="play-button" type="button" onClick={onPlayPause}>
          {isPlaying ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
        </button>
        <button className="icon-button" type="button" title="下一首" onClick={onNext}>
          <SkipForward aria-hidden="true" />
        </button>
      </div>

      <div className="mix-grid">
        <TogglePanel
          icon={<Volume2 aria-hidden="true" />}
          title="打拍音"
          enabled={hitSoundsEnabled}
          onChange={onToggleHitSounds}
          value={`${formatCount(timeline?.hitEvents.length ?? 0)} 个`}
        />
        <TogglePanel
          icon={<FileAudio aria-hidden="true" />}
          title="谱面音效"
          enabled={playSoundsEnabled}
          onChange={onTogglePlaySounds}
          value={`${formatCount(timeline?.playSoundEvents.length ?? 0)} 个`}
        />
        <TogglePanel
          icon={<Video aria-hidden="true" />}
          title="视频"
          enabled={videoEnabled}
          onChange={onToggleVideo}
          value={track.hasVideo ? "存在" : "无"}
        />
        <div className="settings-panel">
          <div className="panel-title">
            <Waves aria-hidden="true" />
            <strong>节拍轨道</strong>
          </div>
          <ResourceNumber label="长按音" value={timeline?.holdSoundEvents.length ?? 0} />
          <ResourceNumber label="倒计时" value={timeline?.countdownEvents.length ?? 0} />
        </div>
      </div>
    </section>
  );
}

interface TogglePanelProps {
  icon: ReactNode;
  title: string;
  enabled: boolean;
  value: string;
  onChange: (enabled: boolean) => void;
}

function TogglePanel({ icon, title, enabled, value, onChange }: TogglePanelProps) {
  return (
    <div className="settings-panel">
      <div className="panel-title">
        {icon}
        <strong>{title}</strong>
      </div>
      <label className="switch-row">
        <span>{value}</span>
        <input
          type="checkbox"
          checked={enabled}
          onChange={(event) => onChange(event.currentTarget.checked)}
        />
      </label>
    </div>
  );
}

interface ResourceNumberProps {
  label: string;
  value: number;
}

function ResourceNumber({ label, value }: ResourceNumberProps) {
  return (
    <div className="resource-row">
      <span>{label}</span>
      <strong>{formatCount(value)}</strong>
    </div>
  );
}

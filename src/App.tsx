import {
  AlertCircle,
  ListOrdered,
  Pause,
  Play,
  Repeat,
  Repeat1,
  Shuffle,
  SkipBack,
  SkipForward,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import "./App.css";
import { useAdoAudio } from "./audio/useAdoAudio";
import { AppShell } from "./components/AppShell";
import { EmptyArtwork } from "./components/EmptyArtwork";
import { LevelDetailView } from "./features/level-detail/LevelDetailView";
import { LibraryView } from "./features/library/LibraryView";
import { PlayerView } from "./features/player/PlayerView";
import { SettingsView } from "./features/settings/SettingsView";
import { formatDuration } from "./lib/format";
import { cleanDisplayText } from "./lib/text";
import {
  addLibraryFolder,
  buildAudioTimeline,
  getSettings,
  listTracks,
  saveSettings,
  scanLibrary,
} from "./lib/tauri";
import type {
  AppView,
  AudioTimeline,
  LibrarySettings,
  PlaybackMode,
  TrackSummary,
} from "./types/domain";

function App() {
  const [activeView, setActiveView] = useState<AppView>("library");
  const [settings, setSettings] = useState<LibrarySettings | null>(null);
  const [tracks, setTracks] = useState<TrackSummary[]>([]);
  const [selectedTrack, setSelectedTrack] = useState<TrackSummary | null>(null);
  const [timeline, setTimeline] = useState<AudioTimeline | null>(null);
  const [loadedTrackId, setLoadedTrackId] = useState<string | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [videoEnabled, setVideoEnabled] = useState(false);

  const audio = useAdoAudio({
    musicVolume: settings?.musicVolume ?? 1,
    hitSoundVolume: settings?.hitSoundVolume ?? 0.82,
    playSoundVolume: settings?.playSoundVolume ?? 0.78,
    onEnded: handlePlaybackEnded,
  });

  const theme = settings?.theme ?? "dark";
  const resolvedTheme = useMemo(() => {
    if (theme !== "system") {
      return theme;
    }
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
  }, [resolvedTheme]);

  useEffect(() => {
    void bootstrap();
  }, []);

  useEffect(() => {
    setTracks((current) => normalizeTracks(current));
    setSelectedTrack((current) => (current ? normalizeTrack(current) : current));
  }, []);

  async function bootstrap() {
    try {
      const nextSettings = await getSettings();
      setSettings(nextSettings);
      const cachedTracks = normalizeTracks(await listTracks());
      setTracks(cachedTracks);
      if (cachedTracks.length === 0 && nextSettings.folders.length > 0) {
        await refreshLibrary();
      }
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function refreshLibrary() {
    setIsScanning(true);
    setError(null);
    try {
      const nextTracks = normalizeTracks(await scanLibrary());
      setTracks(nextTracks);
      if (!selectedTrack && nextTracks.length > 0) {
        setSelectedTrack(nextTracks[0]);
      } else if (
        selectedTrack &&
        !nextTracks.some((track) => track.id === selectedTrack.id)
      ) {
        setSelectedTrack(nextTracks[0] ?? null);
        setTimeline(null);
        setLoadedTrackId(null);
      }
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setIsScanning(false);
    }
  }

  async function handleAddFolder(folder: string) {
    try {
      const next = await addLibraryFolder(folder);
      setSettings(next);
      await refreshLibrary();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleSaveSettings(next: LibrarySettings) {
    try {
      const saved = await saveSettings(next);
      setSettings(saved);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  function patchSettings(patch: Partial<LibrarySettings>) {
    if (!settings) {
      return;
    }
    const next = { ...settings, ...patch };
    setSettings(next);
    void saveSettings(next).catch((err: unknown) => setError(errorMessage(err)));
  }

  async function handlePlayTrack(track: TrackSummary) {
    setError(null);
    setSelectedTrack(track);
    setActiveView("nowPlaying");
    try {
      await loadTrack(track);
      await audio.play();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function loadTrack(track: TrackSummary) {
    const nextTimeline = await buildAudioTimeline(track.adofaiPath);
    await audio.load(track, nextTimeline);
    setTimeline(nextTimeline);
    setLoadedTrackId(track.id);
  }

  function handlePreviousTrack() {
    if (tracks.length === 0) {
      return;
    }
    const index = selectedTrack
      ? tracks.findIndex((track) => track.id === selectedTrack.id)
      : 0;
    const nextIndex = index <= 0 ? tracks.length - 1 : index - 1;
    void handlePlayTrack(tracks[nextIndex]);
  }

  function handleNextTrack() {
    if (tracks.length === 0) {
      return;
    }
    const index = selectedTrack
      ? tracks.findIndex((track) => track.id === selectedTrack.id)
      : -1;
    const nextIndex = settings?.playbackMode === "shuffle"
      ? randomTrackIndex(index, tracks.length)
      : index >= tracks.length - 1 ? 0 : index + 1;
    void handlePlayTrack(tracks[nextIndex]);
  }

  function handlePlaybackEnded() {
    if (tracks.length === 0) {
      return;
    }
    const mode = settings?.playbackMode ?? "sequence";
    const current = selectedTrack ?? tracks[0];
    const index = Math.max(0, tracks.findIndex((track) => track.id === current.id));

    if (mode === "repeatOne") {
      void handlePlayTrack(current);
      return;
    }

    if (mode === "shuffle") {
      void handlePlayTrack(tracks[randomTrackIndex(index, tracks.length)]);
      return;
    }

    if (index < tracks.length - 1) {
      void handlePlayTrack(tracks[index + 1]);
      return;
    }

    if (mode === "repeatAll") {
      void handlePlayTrack(tracks[0]);
    }
  }

  function cyclePlaybackMode() {
    const modes: PlaybackMode[] = ["sequence", "repeatAll", "repeatOne", "shuffle"];
    const current = settings?.playbackMode ?? "sequence";
    const index = modes.indexOf(current);
    patchSettings({ playbackMode: modes[(index + 1) % modes.length] });
  }

  function handlePlayPause() {
    const track = selectedTrack ?? tracks[0] ?? null;
    if (!track) {
      return;
    }
    if (audio.isPlaying && loadedTrackId === track.id) {
      audio.pause();
    } else {
      void (async () => {
        setError(null);
        setSelectedTrack(track);
        if (!timeline || loadedTrackId !== track.id) {
          await loadTrack(track);
        }
        await audio.play();
      })().catch((err: unknown) => setError(errorMessage(err)));
    }
  }

  return (
    <>
      <AppShell
        activeView={activeView}
        settings={settings}
        tracks={tracks}
        selectedTrack={selectedTrack}
        isScanning={isScanning}
        onViewChange={setActiveView}
        onScan={() => void refreshLibrary()}
      >
        {error && (
          <div className="notice error">
            <AlertCircle aria-hidden="true" />
            <span>{error}</span>
          </div>
        )}

        {activeView === "library" && (
          <LibraryView
            tracks={tracks}
            settings={settings}
            selectedTrack={selectedTrack}
            onAddFolder={handleAddFolder}
            onSelectTrack={setSelectedTrack}
            onPlayTrack={(track) => void handlePlayTrack(track)}
          />
        )}
        {activeView === "nowPlaying" && (
          <PlayerView
            track={selectedTrack}
            timeline={timeline}
            currentTime={audio.currentTime}
            duration={audio.duration}
            isPlaying={audio.isPlaying}
            hitSoundsEnabled={audio.hitSoundsEnabled}
            playSoundsEnabled={audio.playSoundsEnabled}
            videoEnabled={videoEnabled}
            onPlayPause={handlePlayPause}
            onPrevious={handlePreviousTrack}
            onNext={handleNextTrack}
            onSeek={audio.seek}
            onToggleHitSounds={audio.setHitSoundsEnabled}
            onTogglePlaySounds={audio.setPlaySoundsEnabled}
            onToggleVideo={setVideoEnabled}
          />
        )}
        {activeView === "detail" && <LevelDetailView track={selectedTrack} />}
        {activeView === "settings" && (
          <SettingsView settings={settings} onSave={handleSaveSettings} />
        )}
      </AppShell>

      <MiniPlayer
        track={selectedTrack}
        isPlaying={audio.isPlaying}
        currentTime={audio.currentTime}
        duration={audio.duration || timeline?.duration || selectedTrack?.duration || 0}
        musicVolume={settings?.musicVolume ?? 1}
        hitSoundVolume={settings?.hitSoundVolume ?? 0.82}
        playSoundVolume={settings?.playSoundVolume ?? 0.78}
        playbackMode={settings?.playbackMode ?? "sequence"}
        onPlayPause={handlePlayPause}
        onPrevious={handlePreviousTrack}
        onNext={handleNextTrack}
        onSeek={audio.seek}
        onMusicVolumeChange={(musicVolume) => patchSettings({ musicVolume })}
        onHitSoundVolumeChange={(hitSoundVolume) => patchSettings({ hitSoundVolume })}
        onPlaySoundVolumeChange={(playSoundVolume) => patchSettings({ playSoundVolume })}
        onCyclePlaybackMode={cyclePlaybackMode}
        onOpenPlayer={() => setActiveView("nowPlaying")}
      />
    </>
  );
}

interface MiniPlayerProps {
  track: TrackSummary | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  musicVolume: number;
  hitSoundVolume: number;
  playSoundVolume: number;
  playbackMode: PlaybackMode;
  onPlayPause: () => void;
  onPrevious: () => void;
  onNext: () => void;
  onSeek: (time: number) => void;
  onMusicVolumeChange: (volume: number) => void;
  onHitSoundVolumeChange: (volume: number) => void;
  onPlaySoundVolumeChange: (volume: number) => void;
  onCyclePlaybackMode: () => void;
  onOpenPlayer: () => void;
}

function MiniPlayer({
  track,
  isPlaying,
  currentTime,
  duration,
  musicVolume,
  hitSoundVolume,
  playSoundVolume,
  playbackMode,
  onPlayPause,
  onPrevious,
  onNext,
  onSeek,
  onMusicVolumeChange,
  onHitSoundVolumeChange,
  onPlaySoundVolumeChange,
  onCyclePlaybackMode,
  onOpenPlayer,
}: MiniPlayerProps) {
  const safeDuration = Math.max(0, duration);
  const seekMax = Math.max(1, safeDuration);

  return (
    <footer className="mini-player">
      <button className="mini-track" type="button" onClick={onOpenPlayer}>
        <EmptyArtwork title={track?.title ?? "默认封面"} imagePath={track?.coverPath} size="sm" />
        <span>
          <strong>{track?.title ?? "未选择曲目"}</strong>
          <small>{track?.artist ?? "ADOFAI Music Box"}</small>
        </span>
      </button>
      <div className="mini-center">
        <button className="icon-button compact" type="button" onClick={onPrevious} title="上一曲">
          <SkipBack aria-hidden="true" />
        </button>
        <button className="play-button compact" type="button" onClick={onPlayPause}>
          {isPlaying ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
        </button>
        <button className="icon-button compact" type="button" onClick={onNext} title="下一曲">
          <SkipForward aria-hidden="true" />
        </button>
        <div className="mini-progress">
          <span>{formatDuration(currentTime)}</span>
          <input
            className="mini-seek"
            type="range"
            min="0"
            max={seekMax}
            step="0.01"
            value={Math.min(currentTime, seekMax)}
            onChange={(event) => onSeek(Number(event.currentTarget.value))}
            aria-label="播放进度"
          />
          <span>{formatDuration(duration)}</span>
        </div>
      </div>
      <div className="mini-tools">
        <button
          className="mode-button"
          type="button"
          onClick={onCyclePlaybackMode}
          title={playbackModeLabel(playbackMode)}
        >
          {playbackModeIcon(playbackMode)}
          <span>{playbackModeLabel(playbackMode)}</span>
        </button>
        <VolumeControl label="音乐" value={musicVolume} onChange={onMusicVolumeChange} />
        <VolumeControl label="打拍" value={hitSoundVolume} onChange={onHitSoundVolumeChange} />
        <VolumeControl label="音效" value={playSoundVolume} onChange={onPlaySoundVolumeChange} />
      </div>
    </footer>
  );
}

interface VolumeControlProps {
  label: string;
  value: number;
  onChange: (volume: number) => void;
}

function VolumeControl({ label, value, onChange }: VolumeControlProps) {
  return (
    <label className="volume-control">
      <span>{label}</span>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={value}
        onChange={(event) => onChange(Number(event.currentTarget.value))}
      />
      <b>{Math.round(value * 100)}</b>
    </label>
  );
}

function errorMessage(err: unknown) {
  return err instanceof Error ? err.message : String(err);
}

function normalizeTracks(tracks: TrackSummary[]) {
  return tracks.map(normalizeTrack);
}

function normalizeTrack(track: TrackSummary): TrackSummary {
  return {
    ...track,
    title: cleanDisplayText(track.title),
    artist: cleanDisplayText(track.artist),
    author: cleanDisplayText(track.author),
  };
}

function randomTrackIndex(currentIndex: number, length: number) {
  if (length <= 1) {
    return 0;
  }
  let next = Math.floor(Math.random() * length);
  if (next === currentIndex) {
    next = (next + 1) % length;
  }
  return next;
}

function playbackModeLabel(mode: PlaybackMode) {
  switch (mode) {
    case "sequence":
      return "顺序";
    case "repeatAll":
      return "列表循环";
    case "repeatOne":
      return "单曲循环";
    case "shuffle":
      return "随机";
  }
}

function playbackModeIcon(mode: PlaybackMode) {
  switch (mode) {
    case "sequence":
      return <ListOrdered aria-hidden="true" />;
    case "repeatAll":
      return <Repeat aria-hidden="true" />;
    case "repeatOne":
      return <Repeat1 aria-hidden="true" />;
    case "shuffle":
      return <Shuffle aria-hidden="true" />;
  }
}

export default App;

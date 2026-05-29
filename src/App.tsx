import { AlertCircle, Pause, Play, SkipBack, SkipForward } from "lucide-react";
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
    hitSoundVolume: settings?.hitSoundVolume ?? 0.82,
    playSoundVolume: settings?.playSoundVolume ?? 0.78,
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
    const nextIndex = index >= tracks.length - 1 ? 0 : index + 1;
    void handlePlayTrack(tracks[nextIndex]);
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
        onPlayPause={handlePlayPause}
        onPrevious={handlePreviousTrack}
        onNext={handleNextTrack}
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
  onPlayPause: () => void;
  onPrevious: () => void;
  onNext: () => void;
  onOpenPlayer: () => void;
}

function MiniPlayer({
  track,
  isPlaying,
  currentTime,
  duration,
  onPlayPause,
  onPrevious,
  onNext,
  onOpenPlayer,
}: MiniPlayerProps) {
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
          <div>
            <i style={{ width: `${duration > 0 ? Math.min(100, (currentTime / duration) * 100) : 0}%` }} />
          </div>
          <span>{formatDuration(duration)}</span>
        </div>
      </div>
      <div className="mini-meta">
        <strong>{track ? `${Math.round(track.bpm)} BPM` : "--"}</strong>
      </div>
    </footer>
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

export default App;

import { open } from "@tauri-apps/plugin-dialog";
import { AlertCircle } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/shell.css";
import "./styles/library.css";
import "./styles/player.css";
import "./styles/modal.css";
import { useAdoAudio } from "./audio/useAdoAudio";
import { AppShell } from "./components/AppShell";
import { FolderManager } from "./components/FolderManager";
import { LibraryView } from "./features/library/LibraryView";
import { FullPlayerOverlay } from "./features/player/FullPlayerOverlay";
import { MiniPlayer } from "./features/player/MiniPlayer";
import { cleanDisplayText } from "./lib/text";
import {
  addLibraryFile,
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
  PlayerOpenSourceRect,
  PlaybackMode,
  TrackSummary,
} from "./types/domain";

function App() {
  const [activeView, setActiveView] = useState<AppView>("local");
  const [settings, setSettings] = useState<LibrarySettings | null>(null);
  const [tracks, setTracks] = useState<TrackSummary[]>([]);
  const [selectedTrack, setSelectedTrack] = useState<TrackSummary | null>(null);
  const [timeline, setTimeline] = useState<AudioTimeline | null>(null);
  const [loadedTrackId, setLoadedTrackId] = useState<string | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [folderManagerOpen, setFolderManagerOpen] = useState(false);
  const [playerOpen, setPlayerOpen] = useState(false);
  const [playerClosing, setPlayerClosing] = useState(false);
  const [playerOpenSource, setPlayerOpenSource] = useState<PlayerOpenSourceRect | null>(null);
  const [playbackView, setPlaybackView] = useState<AppView>("local");
  const [locateRequest, setLocateRequest] = useState(0);

  const audio = useAdoAudio({
    musicVolume: settings?.musicVolume ?? 1,
    hitSoundVolume: settings?.hitSoundVolume ?? 0.82,
    playSoundVolume: settings?.playSoundVolume ?? 0.78,
    onEnded: handlePlaybackEnded,
  });

  const favoriteIds = useMemo(
    () => new Set(settings?.favoriteTrackIds ?? []),
    [settings?.favoriteTrackIds],
  );

  const recentIds = settings?.recentTrackIds ?? [];

  const favoriteTrackCount = useMemo(
    () => tracks.filter((track) => favoriteIds.has(track.id)).length,
    [favoriteIds, tracks],
  );

  const recentTrackCount = useMemo(() => {
    const trackIds = new Set(tracks.map((track) => track.id));
    return recentIds.filter((id) => trackIds.has(id)).length;
  }, [recentIds, tracks]);

  const viewTracks = useMemo(() => {
    if (activeView === "favorites") {
      return tracks.filter((track) => favoriteIds.has(track.id));
    }
    if (activeView === "recent") {
      const byId = new Map(tracks.map((track) => [track.id, track]));
      return recentIds.map((id) => byId.get(id)).filter((track): track is TrackSummary => Boolean(track));
    }
    return tracks;
  }, [activeView, favoriteIds, recentIds, tracks]);

  const selectedIsFavorite = Boolean(selectedTrack && favoriteIds.has(selectedTrack.id));
  const duration = audio.duration || timeline?.duration || selectedTrack?.duration || 0;

  useEffect(() => {
    document.documentElement.dataset.theme = "light";
  }, []);

  useEffect(() => {
    void bootstrap();
  }, []);

  async function bootstrap() {
    try {
      const nextSettings = normalizeSettings(await getSettings());
      setSettings(nextSettings);
      const cachedTracks = normalizeTracks(await listTracks());
      setTracks(cachedTracks);
      if (cachedTracks.length === 0 && nextSettings.folders.length > 0) {
        await refreshLibrary();
      } else if (cachedTracks.length > 0) {
        setSelectedTrack(cachedTracks[0]);
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

  async function chooseAndAddFolder() {
    try {
      const result = await open({ directory: true, multiple: false });
      if (typeof result === "string") {
        await handleAddFolder(result);
      }
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function chooseAndAddFile() {
    try {
      const result = await open({
        directory: false,
        multiple: false,
        filters: [{ name: "ADOFAI 谱面", extensions: ["adofai"] }],
      });
      if (typeof result === "string") {
        await handleAddFile(result);
      }
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleAddFolder(folder: string) {
    try {
      const next = normalizeSettings(await addLibraryFolder(folder));
      setSettings(next);
      await refreshLibrary();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleAddFile(file: string) {
    try {
      const next = normalizeSettings(await addLibraryFile(file));
      setSettings(next);
      await refreshLibrary();
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function handleRemoveFolder(folder: string) {
    if (!settings) {
      return;
    }
    const next = normalizeSettings({
      ...settings,
      folders: settings.folders.filter((item) => item !== folder),
    });
    await persistSettings(next);
    await refreshLibrary();
  }

  async function handleRemoveFile(file: string) {
    if (!settings) {
      return;
    }
    const next = normalizeSettings({
      ...settings,
      adofaiFiles: settings.adofaiFiles.filter((item) => item !== file),
    });
    await persistSettings(next);
    await refreshLibrary();
  }

  async function persistSettings(next: LibrarySettings) {
    try {
      const saved = normalizeSettings(await saveSettings(next));
      setSettings(saved);
      return saved;
    } catch (err) {
      setError(errorMessage(err));
      return next;
    }
  }

  function patchSettings(patch: Partial<LibrarySettings>) {
    if (!settings) {
      return;
    }
    const next = normalizeSettings({ ...settings, ...patch });
    setSettings(next);
    void saveSettings(next).catch((err: unknown) => setError(errorMessage(err)));
  }

  function rememberRecent(trackId: string) {
    if (!settings) {
      return;
    }
    const recentTrackIds = [trackId, ...settings.recentTrackIds.filter((id) => id !== trackId)].slice(0, 500);
    patchSettings({ recentTrackIds });
  }

  function handleToggleFavorite(track: TrackSummary | null = selectedTrack) {
    if (!track || !settings) {
      return;
    }
    const nextIds = new Set(settings.favoriteTrackIds);
    if (nextIds.has(track.id)) {
      nextIds.delete(track.id);
    } else {
      nextIds.add(track.id);
    }
    patchSettings({ favoriteTrackIds: [...nextIds] });
  }

  async function handlePlayTrack(track: TrackSummary, sourceView: AppView = activeView) {
    setError(null);
    setSelectedTrack(track);
    setPlaybackView(sourceView);
    rememberRecent(track.id);
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

  function handlePlayAll() {
    const first = filteredBySearch(viewTracks, searchQuery)[0] ?? viewTracks[0] ?? tracks[0];
    if (first) {
      void handlePlayTrack(first, activeView);
    }
  }

  function handlePreviousTrack() {
    const queue = queueTracks(viewTracks, tracks);
    if (queue.length === 0) {
      return;
    }
    const index = selectedTrack
      ? queue.findIndex((track) => track.id === selectedTrack.id)
      : 0;
    const nextIndex = index <= 0 ? queue.length - 1 : index - 1;
    void handlePlayTrack(queue[nextIndex], activeView);
  }

  function handleNextTrack() {
    const queue = queueTracks(viewTracks, tracks);
    if (queue.length === 0) {
      return;
    }
    const index = selectedTrack
      ? queue.findIndex((track) => track.id === selectedTrack.id)
      : -1;
    const nextIndex =
      settings?.playbackMode === "shuffle"
        ? randomTrackIndex(index, queue.length)
        : index >= queue.length - 1
          ? 0
          : index + 1;
    void handlePlayTrack(queue[nextIndex], activeView);
  }

  function handlePlaybackEnded() {
    const queue = queueTracks(viewTracks, tracks);
    if (queue.length === 0) {
      return;
    }
    const mode = settings?.playbackMode ?? "sequence";
    const current = selectedTrack ?? queue[0];
    const index = Math.max(0, queue.findIndex((track) => track.id === current.id));

    if (mode === "repeatOne") {
      void handlePlayTrack(current, playbackView);
      return;
    }
    if (mode === "shuffle") {
      void handlePlayTrack(queue[randomTrackIndex(index, queue.length)], playbackView);
      return;
    }
    if (index < queue.length - 1) {
      void handlePlayTrack(queue[index + 1], playbackView);
      return;
    }
    if (mode === "repeatAll") {
      void handlePlayTrack(queue[0], playbackView);
    }
  }

  function cyclePlaybackMode() {
    const modes: PlaybackMode[] = ["sequence", "repeatAll", "repeatOne", "shuffle"];
    const current = settings?.playbackMode ?? "sequence";
    const index = modes.indexOf(current);
    patchSettings({ playbackMode: modes[(index + 1) % modes.length] });
  }

  function handlePlayPause() {
    const track = selectedTrack ?? viewTracks[0] ?? tracks[0] ?? null;
    if (!track) {
      return;
    }
    if (audio.isPlaying && loadedTrackId === track.id) {
      audio.pause();
      return;
    }
    void (async () => {
      setError(null);
      setSelectedTrack(track);
      rememberRecent(track.id);
      if (!timeline || loadedTrackId !== track.id) {
        await loadTrack(track);
      }
      await audio.play();
    })().catch((err: unknown) => setError(errorMessage(err)));
  }

  function handleLocatePlaying() {
    if (!selectedTrack) {
      return;
    }

    const targetView = locateViewForTrack(selectedTrack, playbackView, favoriteIds, recentIds);
    setActiveView(targetView);
    if (!trackMatchesSearch(selectedTrack, searchQuery)) {
      setSearchQuery("");
    }
    setLocateRequest((request) => request + 1);
  }

  function handleOpenPlayer(sourceRect?: PlayerOpenSourceRect) {
    const showPlayer = () => {
      setPlayerOpenSource(sourceRect ?? null);
      setPlayerClosing(false);
      setPlayerOpen(true);
    };

    const transitionDocument = document as Document & {
      startViewTransition?: (callback: () => void) => void;
    };

    if (!playerOpen && transitionDocument.startViewTransition) {
      transitionDocument.startViewTransition(showPlayer);
      return;
    }

    showPlayer();
  }

  function handleClosePlayer() {
    setPlayerClosing(true);
    window.setTimeout(() => {
      setPlayerOpen(false);
      setPlayerClosing(false);
      setPlayerOpenSource(null);
    }, 240);
  }

  return (
    <>
      <AppShell
        activeView={activeView}
        settings={settings}
        tracks={tracks}
        favoriteCount={favoriteTrackCount}
        recentCount={recentTrackCount}
        searchQuery={searchQuery}
        isScanning={isScanning}
        onViewChange={setActiveView}
        onSearchChange={setSearchQuery}
        onOpenFolderManager={() => setFolderManagerOpen(true)}
      >
        {error && (
          <div className="notice error">
            <AlertCircle aria-hidden="true" />
            <span>{error}</span>
          </div>
        )}
        <LibraryView
          activeView={activeView}
          tracks={viewTracks}
          allTrackCount={tracks.length}
          favoriteCount={favoriteTrackCount}
          recentCount={recentTrackCount}
          selectedTrack={selectedTrack}
          favoriteIds={favoriteIds}
          query={searchQuery}
          isScanning={isScanning}
          locateRequest={locateRequest}
          canLocatePlaying={Boolean(loadedTrackId && selectedTrack)}
          onViewChange={setActiveView}
          onPlayTrack={(track) => void handlePlayTrack(track, activeView)}
          onLocatePlaying={handleLocatePlaying}
          onPlayAll={handlePlayAll}
          onToggleFavorite={handleToggleFavorite}
          onAddFolder={() => void chooseAndAddFolder()}
          onAddFile={() => void chooseAndAddFile()}
          onOpenFolderManager={() => setFolderManagerOpen(true)}
          onScan={() => void refreshLibrary()}
        />
      </AppShell>

      <MiniPlayer
        track={selectedTrack}
        isPlaying={audio.isPlaying}
        currentTime={audio.currentTime}
        duration={duration}
        musicVolume={settings?.musicVolume ?? 1}
        hitSoundVolume={settings?.hitSoundVolume ?? 0.82}
        playSoundVolume={settings?.playSoundVolume ?? 0.78}
        playbackMode={settings?.playbackMode ?? "sequence"}
        hitSoundsEnabled={audio.hitSoundsEnabled}
        playSoundsEnabled={audio.playSoundsEnabled}
        isFavorite={selectedIsFavorite}
        isPlayerOpen={playerOpen && !playerClosing}
        onOpenPlayer={handleOpenPlayer}
        onPlayPause={handlePlayPause}
        onPrevious={handlePreviousTrack}
        onNext={handleNextTrack}
        onSeek={audio.seek}
        onMusicVolumeChange={(musicVolume) => patchSettings({ musicVolume })}
        onHitSoundVolumeChange={(hitSoundVolume) => patchSettings({ hitSoundVolume })}
        onPlaySoundVolumeChange={(playSoundVolume) => patchSettings({ playSoundVolume })}
        onToggleHitSounds={audio.setHitSoundsEnabled}
        onTogglePlaySounds={audio.setPlaySoundsEnabled}
        onCyclePlaybackMode={cyclePlaybackMode}
        onToggleFavorite={() => handleToggleFavorite()}
      />

      {playerOpen && (
        <FullPlayerOverlay
          track={selectedTrack}
          timeline={timeline}
          isPlaying={audio.isPlaying}
          currentTime={audio.currentTime}
          duration={duration}
          musicVolume={settings?.musicVolume ?? 1}
          playbackMode={settings?.playbackMode ?? "sequence"}
          isFavorite={selectedIsFavorite}
          closing={playerClosing}
          openSourceRect={playerOpenSource}
          onClose={handleClosePlayer}
          onPlayPause={handlePlayPause}
          onPrevious={handlePreviousTrack}
          onNext={handleNextTrack}
          onSeek={audio.seek}
          onMusicVolumeChange={(musicVolume) => patchSettings({ musicVolume })}
          onCyclePlaybackMode={cyclePlaybackMode}
          onToggleFavorite={() => handleToggleFavorite()}
        />
      )}

      <FolderManager
        open={folderManagerOpen}
        folders={settings?.folders ?? []}
        files={settings?.adofaiFiles ?? []}
        isScanning={isScanning}
        onClose={() => setFolderManagerOpen(false)}
        onAddFolder={() => void chooseAndAddFolder()}
        onAddFile={() => void chooseAndAddFile()}
        onRemoveFolder={(folder) => void handleRemoveFolder(folder)}
        onRemoveFile={(file) => void handleRemoveFile(file)}
        onScan={() => void refreshLibrary()}
      />
    </>
  );
}

function errorMessage(err: unknown) {
  const message = err instanceof Error ? err.message : String(err);
  if (/JSON|control character|line terminator|谱面解析失败|宽松解析|标准 JSON/i.test(message)) {
    return "这个谱面文件格式异常，暂时无法播放。";
  }
  return message;
}

function normalizeSettings(settings: LibrarySettings): LibrarySettings {
  return {
    ...settings,
    adofaiFiles: Array.isArray(settings.adofaiFiles) ? settings.adofaiFiles : [],
    theme: "light",
    musicVolume: settings.musicVolume ?? 1,
    hitSoundVolume: settings.hitSoundVolume ?? 0.82,
    playSoundVolume: settings.playSoundVolume ?? 0.78,
    playbackMode: settings.playbackMode ?? "sequence",
    favoriteTrackIds: Array.isArray(settings.favoriteTrackIds) ? settings.favoriteTrackIds : [],
    recentTrackIds: Array.isArray(settings.recentTrackIds) ? settings.recentTrackIds : [],
  };
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

function filteredBySearch(tracks: TrackSummary[], query: string) {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return tracks;
  }
  return tracks.filter((track) => trackMatchesSearch(track, normalized));
}

function trackMatchesSearch(track: TrackSummary, query: string) {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return true;
  }
  return [track.title, track.artist, track.author, track.folderPath]
    .join(" ")
    .toLowerCase()
    .includes(normalized);
}

function locateViewForTrack(
  track: TrackSummary,
  preferredView: AppView,
  favoriteIds: Set<string>,
  recentIds: string[],
): AppView {
  if (preferredView === "favorites" && favoriteIds.has(track.id)) {
    return "favorites";
  }
  if (preferredView === "recent" && recentIds.includes(track.id)) {
    return "recent";
  }
  return "local";
}

function queueTracks(viewTracks: TrackSummary[], allTracks: TrackSummary[]) {
  return viewTracks.length > 0 ? viewTracks : allTracks;
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

export default App;

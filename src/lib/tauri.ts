import { invoke } from "@tauri-apps/api/core";
import type {
  AudioTimeline,
  GameMode,
  LibrarySettings,
  LibraryProfile,
  TrackDetail,
  TrackSummary,
} from "../types/domain";

const emptyProfile = (): LibraryProfile => ({
  folders: [],
  files: [],
  favoriteTrackIds: [],
  recentTrackIds: [],
  hiddenTrackIds: [],
  hiddenTracks: [],
});

const browserPreviewSettings: LibrarySettings = {
  libraries: {
    adofai: emptyProfile(),
    rhythmDoctor: emptyProfile(),
  },
  folders: [],
  adofaiFiles: [],
  theme: "light",
  musicVolume: 1,
  hitSoundVolume: 0.82,
  playSoundVolume: 0.78,
  playbackMode: "sequence",
  defaultCoverMode: "generated",
  audioAssetRoot: null,
  lenientParsing: true,
  favoriteTrackIds: [],
  recentTrackIds: [],
  hiddenTrackIds: [],
  hiddenTracks: [],
};

export function hasTauriBridge() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function getSettings(): Promise<LibrarySettings> {
  if (!hasTauriBridge()) {
    return Promise.resolve(browserPreviewSettings);
  }
  return invoke("get_settings");
}

export function saveSettings(settings: LibrarySettings): Promise<LibrarySettings> {
  if (!hasTauriBridge()) {
    return Promise.resolve(settings);
  }
  return invoke("save_settings", { settings });
}

export function addLibraryFolder(mode: GameMode, folder: string): Promise<LibrarySettings> {
  if (!hasTauriBridge()) {
    return Promise.resolve({
      ...browserPreviewSettings,
      libraries: {
        ...browserPreviewSettings.libraries,
        [mode]: { ...emptyProfile(), folders: [folder] },
      },
    });
  }
  return invoke("add_library_folder", { mode, folder });
}

export function addLibraryFile(mode: GameMode, file: string): Promise<LibrarySettings> {
  if (!hasTauriBridge()) {
    return Promise.resolve({
      ...browserPreviewSettings,
      libraries: {
        ...browserPreviewSettings.libraries,
        [mode]: { ...emptyProfile(), files: [file] },
      },
    });
  }
  return invoke("add_library_file", { mode, file });
}

export function scanLibrary(mode: GameMode): Promise<TrackSummary[]> {
  if (!hasTauriBridge()) {
    return Promise.resolve([]);
  }
  return invoke("scan_library", { mode });
}

export function listTracks(mode: GameMode): Promise<TrackSummary[]> {
  if (!hasTauriBridge()) {
    return Promise.resolve([]);
  }
  return invoke("list_tracks", { mode });
}

export function saveTrackCache(mode: GameMode, tracks: TrackSummary[]): Promise<void> {
  if (!hasTauriBridge()) {
    return Promise.resolve();
  }
  return invoke("save_track_cache", { mode, tracks });
}

export function getTrackDetail(mode: GameMode, chartPath: string): Promise<TrackDetail> {
  if (!hasTauriBridge()) {
    return Promise.reject(new Error("请在桌面应用中打开本地曲目"));
  }
  return invoke("get_track_detail", { mode, chartPath });
}

export function getTrackSummary(mode: GameMode, chartPath: string): Promise<TrackSummary> {
  if (!hasTauriBridge()) {
    return Promise.reject(new Error("请在桌面应用中打开本地曲目"));
  }
  return invoke("get_track_summary", { mode, chartPath });
}

export function buildAudioTimeline(mode: GameMode, chartPath: string): Promise<AudioTimeline> {
  if (!hasTauriBridge()) {
    return Promise.reject(new Error("请在桌面应用中播放本地曲目"));
  }
  return invoke("build_audio_timeline", { mode, chartPath });
}

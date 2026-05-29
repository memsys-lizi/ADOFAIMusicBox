import { invoke } from "@tauri-apps/api/core";
import type {
  AudioTimeline,
  LibrarySettings,
  TrackDetail,
  TrackSummary,
} from "../types/domain";

const browserPreviewSettings: LibrarySettings = {
  folders: [],
  theme: "dark",
  hitSoundVolume: 0.82,
  playSoundVolume: 0.78,
  defaultCoverMode: "generated",
  audioAssetRoot: null,
  lenientParsing: true,
};

function hasTauriBridge() {
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

export function addLibraryFolder(folder: string): Promise<LibrarySettings> {
  if (!hasTauriBridge()) {
    return Promise.resolve({ ...browserPreviewSettings, folders: [folder] });
  }
  return invoke("add_library_folder", { folder });
}

export function scanLibrary(): Promise<TrackSummary[]> {
  if (!hasTauriBridge()) {
    return Promise.resolve([]);
  }
  return invoke("scan_library");
}

export function listTracks(): Promise<TrackSummary[]> {
  if (!hasTauriBridge()) {
    return Promise.resolve([]);
  }
  return invoke("list_tracks");
}

export function getTrackDetail(adofaiPath: string): Promise<TrackDetail> {
  if (!hasTauriBridge()) {
    return Promise.reject(new Error("请在桌面应用中打开本地曲目"));
  }
  return invoke("get_track_detail", { adofaiPath });
}

export function buildAudioTimeline(adofaiPath: string): Promise<AudioTimeline> {
  if (!hasTauriBridge()) {
    return Promise.reject(new Error("请在桌面应用中播放本地曲目"));
  }
  return invoke("build_audio_timeline", { adofaiPath });
}

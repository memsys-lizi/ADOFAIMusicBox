export type ThemeMode = "dark" | "light" | "system";
export type DefaultCoverMode = "generated" | "minimal";
export type ParseStatus = "ok" | "lenient" | "warning" | "error";
export type AppView = "local" | "favorites" | "recent";
export type PlaybackMode = "sequence" | "repeatAll" | "repeatOne" | "shuffle";
export type GameMode = "adofai" | "rhythmDoctor";

export interface PlayerOpenSourceRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface HiddenTrack {
  id: string;
  title: string;
  artist: string;
  author: string;
  game: GameMode;
  chartPath: string;
  adofaiPath: string;
  folderPath: string;
  removedAt: string;
}

export interface LibraryProfile {
  folders: string[];
  files: string[];
  favoriteTrackIds: string[];
  recentTrackIds: string[];
  hiddenTrackIds: string[];
  hiddenTracks: HiddenTrack[];
}

export interface LibraryProfiles {
  adofai: LibraryProfile;
  rhythmDoctor: LibraryProfile;
}

export interface LibrarySettings {
  libraries: LibraryProfiles;
  folders: string[];
  adofaiFiles: string[];
  theme: ThemeMode;
  musicVolume: number;
  hitSoundVolume: number;
  playSoundVolume: number;
  playbackMode: PlaybackMode;
  defaultCoverMode: DefaultCoverMode;
  audioAssetRoot?: string | null;
  lenientParsing: boolean;
  favoriteTrackIds: string[];
  recentTrackIds: string[];
  hiddenTrackIds: string[];
  hiddenTracks: HiddenTrack[];
}

export interface TrackSummary {
  id: string;
  game: GameMode;
  chartPath: string;
  adofaiPath: string;
  folderPath: string;
  title: string;
  artist: string;
  author: string;
  bpm: number;
  duration: number;
  coverPath?: string | null;
  iconPath?: string | null;
  audioPath?: string | null;
  audioFileSize?: number | null;
  videoPath?: string | null;
  hasVideo: boolean;
  videoOffsetSec: number;
  loopVideo: boolean;
  parseStatus: ParseStatus;
}

export interface ResourceStatus {
  audioExists: boolean;
  coverExists: boolean;
  iconExists: boolean;
  videoExists: boolean;
  missing: string[];
}

export interface TrackDetail {
  summary: TrackSummary;
  settings: Record<string, unknown>;
  resourceStatus: ResourceStatus;
  eventCounts: Record<string, number>;
  warnings: string[];
  rawParseMode: string;
  supportedAudioEvents: string[];
}

export interface HitEvent {
  timeSec: number;
  endTimeSec?: number | null;
  soundName: string;
  volume: number;
  pitch: number;
  sourceFloor: number;
  kind: string;
}

export interface AudioTimeline {
  songOffsetMs: number;
  countdownLeadInSec: number;
  pitch: number;
  duration: number;
  hitEvents: HitEvent[];
  playSoundEvents: HitEvent[];
  holdSoundEvents: HitEvent[];
  warnings: string[];
}

export interface PlayerState {
  currentTrack: TrackSummary | null;
  timeline: AudioTimeline | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  hitSoundsEnabled: boolean;
  playSoundsEnabled: boolean;
  videoEnabled: boolean;
}

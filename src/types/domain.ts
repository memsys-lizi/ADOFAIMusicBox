export type ThemeMode = "dark" | "light" | "system";
export type DefaultCoverMode = "generated" | "minimal";
export type ParseStatus = "ok" | "lenient" | "warning" | "error";
export type AppView = "local" | "favorites" | "recent";
export type PlaybackMode = "sequence" | "repeatAll" | "repeatOne" | "shuffle";

export interface PlayerOpenSourceRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export interface LibrarySettings {
  folders: string[];
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
}

export interface TrackSummary {
  id: string;
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

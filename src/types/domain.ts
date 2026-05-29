export type ThemeMode = "dark" | "light" | "system";
export type DefaultCoverMode = "generated" | "minimal";
export type ParseStatus = "ok" | "lenient" | "warning" | "error";
export type AppView = "library" | "nowPlaying" | "detail" | "settings";

export interface LibrarySettings {
  folders: string[];
  theme: ThemeMode;
  hitSoundVolume: number;
  playSoundVolume: number;
  defaultCoverMode: DefaultCoverMode;
  audioAssetRoot?: string | null;
  lenientParsing: boolean;
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
  countdownEvents: HitEvent[];
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

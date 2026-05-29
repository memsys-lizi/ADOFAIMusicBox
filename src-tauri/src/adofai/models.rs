use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySettings {
    pub folders: Vec<String>,
    #[serde(default)]
    pub adofai_files: Vec<String>,
    pub theme: ThemeMode,
    #[serde(default = "default_music_volume")]
    pub music_volume: f32,
    pub hit_sound_volume: f32,
    pub play_sound_volume: f32,
    #[serde(default)]
    pub playback_mode: PlaybackMode,
    pub default_cover_mode: DefaultCoverMode,
    pub audio_asset_root: Option<String>,
    pub lenient_parsing: bool,
    #[serde(default)]
    pub favorite_track_ids: Vec<String>,
    #[serde(default)]
    pub recent_track_ids: Vec<String>,
}

impl Default for LibrarySettings {
    fn default() -> Self {
        Self {
            folders: Vec::new(),
            adofai_files: Vec::new(),
            theme: ThemeMode::Light,
            music_volume: default_music_volume(),
            hit_sound_volume: 0.82,
            play_sound_volume: 0.78,
            playback_mode: PlaybackMode::Sequence,
            default_cover_mode: DefaultCoverMode::Generated,
            audio_asset_root: None,
            lenient_parsing: true,
            favorite_track_ids: Vec::new(),
            recent_track_ids: Vec::new(),
        }
    }
}

fn default_music_volume() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DefaultCoverMode {
    Generated,
    Minimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackMode {
    #[default]
    Sequence,
    RepeatAll,
    RepeatOne,
    Shuffle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSummary {
    pub id: String,
    pub adofai_path: String,
    pub folder_path: String,
    pub title: String,
    pub artist: String,
    pub author: String,
    pub bpm: f64,
    pub duration: f64,
    pub cover_path: Option<String>,
    pub icon_path: Option<String>,
    pub audio_path: Option<String>,
    pub audio_file_size: Option<u64>,
    pub video_path: Option<String>,
    pub has_video: bool,
    pub parse_status: ParseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParseStatus {
    Ok,
    Lenient,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackDetail {
    pub summary: TrackSummary,
    pub settings: BTreeMap<String, serde_json::Value>,
    pub resource_status: ResourceStatus,
    pub event_counts: BTreeMap<String, usize>,
    pub warnings: Vec<String>,
    pub raw_parse_mode: String,
    pub supported_audio_events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStatus {
    pub audio_exists: bool,
    pub cover_exists: bool,
    pub icon_exists: bool,
    pub video_exists: bool,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTimeline {
    pub song_offset_ms: f64,
    pub pitch: f64,
    pub duration: f64,
    pub hit_events: Vec<HitEvent>,
    pub play_sound_events: Vec<HitEvent>,
    pub hold_sound_events: Vec<HitEvent>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HitEvent {
    pub time_sec: f64,
    pub end_time_sec: Option<f64>,
    pub sound_name: String,
    pub volume: f32,
    pub pitch: f32,
    pub source_floor: usize,
    pub kind: String,
}

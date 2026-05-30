use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameMode {
    #[serde(rename = "adofai")]
    AdoFai,
    #[serde(rename = "rhythmDoctor")]
    RhythmDoctor,
}

impl Default for GameMode {
    fn default() -> Self {
        Self::AdoFai
    }
}

impl GameMode {
    pub fn chart_extension(self) -> &'static str {
        match self {
            Self::AdoFai => "adofai",
            Self::RhythmDoctor => "rdlevel",
        }
    }

    pub fn cache_suffix(self) -> &'static str {
        match self {
            Self::AdoFai => "adofai",
            Self::RhythmDoctor => "rhythm-doctor",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::AdoFai => "ADOFAI",
            Self::RhythmDoctor => "Rhythm Doctor",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenTrack {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub game: GameMode,
    #[serde(default)]
    pub chart_path: String,
    #[serde(default)]
    pub adofai_path: String,
    #[serde(default)]
    pub folder_path: String,
    #[serde(default)]
    pub removed_at: String,
}

impl HiddenTrack {
    pub fn effective_chart_path(&self) -> &str {
        if !self.chart_path.is_empty() {
            &self.chart_path
        } else {
            &self.adofai_path
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryProfile {
    #[serde(default)]
    pub folders: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub favorite_track_ids: Vec<String>,
    #[serde(default)]
    pub recent_track_ids: Vec<String>,
    #[serde(default)]
    pub hidden_track_ids: Vec<String>,
    #[serde(default)]
    pub hidden_tracks: Vec<HiddenTrack>,
}

impl LibraryProfile {
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
            && self.files.is_empty()
            && self.favorite_track_ids.is_empty()
            && self.recent_track_ids.is_empty()
            && self.hidden_track_ids.is_empty()
            && self.hidden_tracks.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryProfiles {
    #[serde(default)]
    pub adofai: LibraryProfile,
    #[serde(default)]
    pub rhythm_doctor: LibraryProfile,
}

impl LibraryProfiles {
    pub fn profile(&self, mode: GameMode) -> &LibraryProfile {
        match mode {
            GameMode::AdoFai => &self.adofai,
            GameMode::RhythmDoctor => &self.rhythm_doctor,
        }
    }

    pub fn profile_mut(&mut self, mode: GameMode) -> &mut LibraryProfile {
        match mode {
            GameMode::AdoFai => &mut self.adofai,
            GameMode::RhythmDoctor => &mut self.rhythm_doctor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySettings {
    #[serde(default)]
    pub libraries: LibraryProfiles,

    #[serde(default)]
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
    #[serde(default)]
    pub hidden_track_ids: Vec<String>,
    #[serde(default)]
    pub hidden_tracks: Vec<HiddenTrack>,
}

impl LibrarySettings {
    pub fn normalize_legacy(mut self) -> Self {
        let has_legacy_profile = !self.folders.is_empty()
            || !self.adofai_files.is_empty()
            || !self.favorite_track_ids.is_empty()
            || !self.recent_track_ids.is_empty()
            || !self.hidden_track_ids.is_empty()
            || !self.hidden_tracks.is_empty();

        if self.libraries.adofai.is_empty() && has_legacy_profile {
            self.libraries.adofai = LibraryProfile {
                folders: self.folders.clone(),
                files: self.adofai_files.clone(),
                favorite_track_ids: self.favorite_track_ids.clone(),
                recent_track_ids: self.recent_track_ids.clone(),
                hidden_track_ids: self.hidden_track_ids.clone(),
                hidden_tracks: self
                    .hidden_tracks
                    .iter()
                    .cloned()
                    .map(|mut track| {
                        track.game = GameMode::AdoFai;
                        if track.chart_path.is_empty() {
                            track.chart_path = track.adofai_path.clone();
                        }
                        track
                    })
                    .collect(),
            };
        }

        self.sync_legacy_adofai_fields();
        self
    }

    pub fn sync_legacy_adofai_fields(&mut self) {
        let adofai = &self.libraries.adofai;
        self.folders = adofai.folders.clone();
        self.adofai_files = adofai.files.clone();
        self.favorite_track_ids = adofai.favorite_track_ids.clone();
        self.recent_track_ids = adofai.recent_track_ids.clone();
        self.hidden_track_ids = adofai.hidden_track_ids.clone();
        self.hidden_tracks = adofai.hidden_tracks.clone();
    }

    pub fn profile(&self, mode: GameMode) -> &LibraryProfile {
        self.libraries.profile(mode)
    }

    pub fn profile_mut(&mut self, mode: GameMode) -> &mut LibraryProfile {
        self.libraries.profile_mut(mode)
    }
}

impl Default for LibrarySettings {
    fn default() -> Self {
        let mut settings = Self {
            libraries: LibraryProfiles::default(),
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
            hidden_track_ids: Vec::new(),
            hidden_tracks: Vec::new(),
        };
        settings.sync_legacy_adofai_fields();
        settings
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
    #[serde(default)]
    pub game: GameMode,
    #[serde(default)]
    pub chart_path: String,
    #[serde(default)]
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
    pub video_offset_sec: f64,
    pub loop_video: bool,
    pub parse_status: ParseStatus,
}

impl TrackSummary {
    pub fn effective_chart_path(&self) -> &str {
        if !self.chart_path.is_empty() {
            &self.chart_path
        } else {
            &self.adofai_path
        }
    }
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
    pub countdown_lead_in_sec: f64,
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

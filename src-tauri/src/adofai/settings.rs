use crate::library::{GameMode, LibrarySettings, TrackSummary};
use std::fs;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let base = dirs::config_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ADOFAI Music Box")
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

fn tracks_cache_path(mode: GameMode) -> PathBuf {
    data_dir().join(format!("tracks-{}.json", mode.cache_suffix()))
}

fn legacy_tracks_cache_path() -> PathBuf {
    data_dir().join("tracks.json")
}

pub fn load_settings() -> LibrarySettings {
    let path = settings_path();
    let Ok(text) = fs::read_to_string(path) else {
        return LibrarySettings::default();
    };
    serde_json::from_str::<LibrarySettings>(&text)
        .map(LibrarySettings::normalize_legacy)
        .unwrap_or_default()
}

pub fn save_settings(settings: &LibrarySettings) -> Result<(), String> {
    let mut settings = settings.clone();
    settings.sync_legacy_adofai_fields();
    let path = settings_path();
    let Some(parent) = path.parent() else {
        return Err("无法定位设置目录".to_string());
    };
    fs::create_dir_all(parent).map_err(|err| format!("创建设置目录失败: {err}"))?;
    let text = serde_json::to_string_pretty(&settings).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| format!("保存设置失败: {err}"))
}

pub fn load_tracks_cache(mode: GameMode) -> Vec<TrackSummary> {
    let path = tracks_cache_path(mode);
    if !path.exists() && mode == GameMode::AdoFai {
        return read_tracks_cache(legacy_tracks_cache_path(), mode);
    }
    read_tracks_cache(path, mode)
}

fn read_tracks_cache(path: PathBuf, mode: GameMode) -> Vec<TrackSummary> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut tracks = serde_json::from_str::<Vec<TrackSummary>>(&text).unwrap_or_default();
    for track in &mut tracks {
        track.game = mode;
        if track.chart_path.is_empty() {
            track.chart_path = track.adofai_path.clone();
        }
        if track.adofai_path.is_empty() {
            track.adofai_path = track.chart_path.clone();
        }
    }
    tracks
}

pub fn save_tracks_cache(mode: GameMode, tracks: &[TrackSummary]) -> Result<(), String> {
    let path = tracks_cache_path(mode);
    let Some(parent) = path.parent() else {
        return Err("无法定位曲库缓存目录".to_string());
    };
    fs::create_dir_all(parent).map_err(|err| format!("创建缓存目录失败: {err}"))?;
    let text = serde_json::to_string(tracks).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| format!("保存曲库缓存失败: {err}"))
}

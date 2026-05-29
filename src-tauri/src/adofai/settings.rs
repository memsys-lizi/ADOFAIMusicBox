use super::models::{LibrarySettings, TrackSummary};
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

fn tracks_cache_path() -> PathBuf {
    data_dir().join("tracks.json")
}

pub fn load_settings() -> LibrarySettings {
    let path = settings_path();
    let Ok(text) = fs::read_to_string(path) else {
        return LibrarySettings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_settings(settings: &LibrarySettings) -> Result<(), String> {
    let path = settings_path();
    let Some(parent) = path.parent() else {
        return Err("无法定位设置目录".to_string());
    };
    fs::create_dir_all(parent).map_err(|err| format!("创建设置目录失败: {err}"))?;
    let text = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| format!("保存设置失败: {err}"))
}

pub fn load_tracks_cache() -> Vec<TrackSummary> {
    let path = tracks_cache_path();
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_tracks_cache(tracks: &[TrackSummary]) -> Result<(), String> {
    let path = tracks_cache_path();
    let Some(parent) = path.parent() else {
        return Err("无法定位曲库缓存目录".to_string());
    };
    fs::create_dir_all(parent).map_err(|err| format!("创建缓存目录失败: {err}"))?;
    let text = serde_json::to_string(tracks).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| format!("保存曲库缓存失败: {err}"))
}

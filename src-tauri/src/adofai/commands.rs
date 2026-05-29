use super::models::{AudioTimeline, LibrarySettings, TrackDetail, TrackSummary};
use super::scanner::{detail_from_path, scan_sources};
use super::settings::{load_settings, save_settings as persist_settings};
use super::timeline::build_timeline_from_path;
use std::path::Path;
use std::sync::Mutex;

pub struct AppState {
    settings: Mutex<LibrarySettings>,
    tracks: Mutex<Vec<TrackSummary>>,
}

impl AppState {
    pub fn load() -> Self {
        let settings = load_settings();
        let tracks = scan_sources(
            &settings.folders,
            &settings.adofai_files,
            settings.lenient_parsing,
        );
        Self {
            settings: Mutex::new(settings),
            tracks: Mutex::new(tracks),
        }
    }
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Result<LibrarySettings, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| "设置状态已损坏".to_string())
}

#[tauri::command]
pub fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: LibrarySettings,
) -> Result<LibrarySettings, String> {
    persist_settings(&settings)?;
    let mut stored = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?;
    *stored = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn add_library_folder(
    state: tauri::State<'_, AppState>,
    folder: String,
) -> Result<LibrarySettings, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?;
    if !Path::new(&folder).exists() {
        return Err("文件夹不存在".to_string());
    }
    if !settings
        .folders
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&folder))
    {
        settings.folders.push(folder);
        persist_settings(&settings)?;
    }
    Ok(settings.clone())
}

#[tauri::command]
pub fn add_library_file(
    state: tauri::State<'_, AppState>,
    file: String,
) -> Result<LibrarySettings, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?;
    let path = Path::new(&file);
    if !path.exists() || !path.is_file() {
        return Err("谱面文件不存在".to_string());
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("adofai"))
    {
        return Err("请选择 ADOFAI 谱面文件".to_string());
    }
    if !settings
        .adofai_files
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&file))
    {
        settings.adofai_files.push(file);
        persist_settings(&settings)?;
    }
    Ok(settings.clone())
}

#[tauri::command]
pub fn scan_library(state: tauri::State<'_, AppState>) -> Result<Vec<TrackSummary>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .clone();
    let tracks = scan_sources(
        &settings.folders,
        &settings.adofai_files,
        settings.lenient_parsing,
    );
    let mut stored = state
        .tracks
        .lock()
        .map_err(|_| "曲库状态已损坏".to_string())?;
    *stored = tracks.clone();
    Ok(tracks)
}

#[tauri::command]
pub fn list_tracks(state: tauri::State<'_, AppState>) -> Result<Vec<TrackSummary>, String> {
    state
        .tracks
        .lock()
        .map(|tracks| tracks.clone())
        .map_err(|_| "曲库状态已损坏".to_string())
}

#[tauri::command]
pub fn get_track_detail(
    state: tauri::State<'_, AppState>,
    adofai_path: String,
) -> Result<TrackDetail, String> {
    let lenient = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .lenient_parsing;
    detail_from_path(Path::new(&adofai_path), lenient)
}

#[tauri::command]
pub fn build_audio_timeline(
    state: tauri::State<'_, AppState>,
    adofai_path: String,
) -> Result<AudioTimeline, String> {
    let lenient = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .lenient_parsing;
    build_timeline_from_path(Path::new(&adofai_path), lenient)
}

#[tauri::command]
pub fn resolve_asset_path(path: String) -> Result<String, String> {
    let asset = Path::new(&path);
    if asset.exists() {
        Ok(asset.to_string_lossy().to_string())
    } else {
        Err("资源文件不存在".to_string())
    }
}

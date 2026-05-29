use super::models::{AudioTimeline, LibrarySettings, TrackDetail, TrackSummary};
use super::scanner::{detail_from_path, scan_sources, summary_from_path};
use super::settings::{
    load_settings, load_tracks_cache, save_settings as persist_settings,
    save_tracks_cache as persist_tracks_cache,
};
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
        let tracks = filter_cached_tracks(load_tracks_cache(), &settings);
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
    let mut tracks = state
        .tracks
        .lock()
        .map_err(|_| "曲库状态已损坏".to_string())?;
    *tracks = filter_cached_tracks(tracks.clone(), &settings);
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
pub async fn scan_library(state: tauri::State<'_, AppState>) -> Result<Vec<TrackSummary>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .clone();
    let folders = settings.folders.clone();
    let adofai_files = settings.adofai_files.clone();
    let hidden_track_ids = settings.hidden_track_ids.clone();
    let hidden_tracks = settings.hidden_tracks.clone();
    let lenient_parsing = settings.lenient_parsing;
    let tracks = tauri::async_runtime::spawn_blocking(move || {
        scan_sources(
            &folders,
            &adofai_files,
            &hidden_track_ids,
            &hidden_tracks,
            lenient_parsing,
        )
    })
    .await
    .map_err(|err| format!("扫描曲库失败: {err}"))?;
    let mut stored = state
        .tracks
        .lock()
        .map_err(|_| "曲库状态已损坏".to_string())?;
    *stored = tracks.clone();
    persist_tracks_cache(&tracks)?;
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
pub fn save_track_cache(
    state: tauri::State<'_, AppState>,
    tracks: Vec<TrackSummary>,
) -> Result<(), String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .clone();
    let tracks = filter_cached_tracks(tracks, &settings);
    let mut stored = state
        .tracks
        .lock()
        .map_err(|_| "曲库状态已损坏".to_string())?;
    *stored = tracks.clone();
    persist_tracks_cache(&tracks)
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
pub fn get_track_summary(
    state: tauri::State<'_, AppState>,
    adofai_path: String,
) -> Result<TrackSummary, String> {
    let path = Path::new(&adofai_path);
    if !path.exists() || !path.is_file() {
        return Err("谱面文件不存在".to_string());
    }
    let lenient = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .lenient_parsing;
    Ok(summary_from_path(path, lenient))
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

fn filter_cached_tracks(tracks: Vec<TrackSummary>, settings: &LibrarySettings) -> Vec<TrackSummary> {
    let mut tracks: Vec<TrackSummary> = tracks
        .into_iter()
        .filter(|track| is_from_enabled_source(track, settings) && !is_hidden_track(track, settings))
        .collect();
    tracks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    tracks.dedup_by(|a, b| same_path(&a.adofai_path, &b.adofai_path) || a.id == b.id);
    tracks
}

fn is_from_enabled_source(track: &TrackSummary, settings: &LibrarySettings) -> bool {
    settings
        .adofai_files
        .iter()
        .any(|file| same_path(file, &track.adofai_path))
        || settings
            .folders
            .iter()
            .any(|folder| path_is_inside(&track.adofai_path, folder))
}

fn is_hidden_track(track: &TrackSummary, settings: &LibrarySettings) -> bool {
    settings
        .hidden_track_ids
        .iter()
        .any(|id| id.eq_ignore_ascii_case(&track.id))
        || settings.hidden_tracks.iter().any(|hidden| {
            hidden.id.eq_ignore_ascii_case(&track.id)
                || (!hidden.adofai_path.is_empty() && same_path(&hidden.adofai_path, &track.adofai_path))
        })
}

fn same_path(left: &str, right: &str) -> bool {
    normalize_path(left) == normalize_path(right)
}

fn path_is_inside(path: &str, folder: &str) -> bool {
    let path = normalize_path(path);
    let mut folder = normalize_path(folder);
    if !folder.ends_with('\\') {
        folder.push('\\');
    }
    path.starts_with(&folder)
}

fn normalize_path(path: &str) -> String {
    path.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

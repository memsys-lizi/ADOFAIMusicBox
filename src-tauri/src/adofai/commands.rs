use super::scanner as adofai_scanner;
use super::settings::{
    load_settings, load_tracks_cache, save_settings as persist_settings,
    save_tracks_cache as persist_tracks_cache,
};
use super::timeline as adofai_timeline;
use crate::library::{AudioTimeline, GameMode, LibrarySettings, TrackDetail, TrackSummary};
use crate::rhythm_doctor::{scanner as rd_scanner, timeline as rd_timeline};
use std::path::Path;
use std::sync::Mutex;

pub struct AppState {
    settings: Mutex<LibrarySettings>,
    adofai_tracks: Mutex<Vec<TrackSummary>>,
    rhythm_doctor_tracks: Mutex<Vec<TrackSummary>>,
}

impl AppState {
    pub fn load() -> Self {
        let settings = load_settings();
        let adofai_tracks = filter_cached_tracks(
            load_tracks_cache(GameMode::AdoFai),
            GameMode::AdoFai,
            &settings,
        );
        let rhythm_doctor_tracks = filter_cached_tracks(
            load_tracks_cache(GameMode::RhythmDoctor),
            GameMode::RhythmDoctor,
            &settings,
        );
        Self {
            settings: Mutex::new(settings),
            adofai_tracks: Mutex::new(adofai_tracks),
            rhythm_doctor_tracks: Mutex::new(rhythm_doctor_tracks),
        }
    }

    fn tracks_for_mode(&self, mode: GameMode) -> &Mutex<Vec<TrackSummary>> {
        match mode {
            GameMode::AdoFai => &self.adofai_tracks,
            GameMode::RhythmDoctor => &self.rhythm_doctor_tracks,
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
    let settings = settings.normalize_legacy();
    persist_settings(&settings)?;
    let mut stored = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?;
    *stored = settings.clone();
    drop(stored);

    refilter_mode_cache(&state, GameMode::AdoFai, &settings)?;
    refilter_mode_cache(&state, GameMode::RhythmDoctor, &settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn add_library_folder(
    state: tauri::State<'_, AppState>,
    mode: GameMode,
    folder: String,
) -> Result<LibrarySettings, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?;
    if !Path::new(&folder).exists() {
        return Err("文件夹不存在".to_string());
    }
    let profile = settings.profile_mut(mode);
    if !profile
        .folders
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&folder))
    {
        profile.folders.push(folder);
        persist_settings(&settings)?;
    }
    Ok(settings.clone())
}

#[tauri::command]
pub fn add_library_file(
    state: tauri::State<'_, AppState>,
    mode: GameMode,
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
        .is_none_or(|ext| !ext.eq_ignore_ascii_case(mode.chart_extension()))
    {
        return Err(format!("请选择 {} 谱面文件", mode.display_name()));
    }
    let profile = settings.profile_mut(mode);
    if !profile
        .files
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&file))
    {
        profile.files.push(file);
        persist_settings(&settings)?;
    }
    Ok(settings.clone())
}

#[tauri::command]
pub async fn scan_library(
    state: tauri::State<'_, AppState>,
    mode: GameMode,
) -> Result<Vec<TrackSummary>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .clone();
    let profile = settings.profile(mode).clone();
    let lenient_parsing = settings.lenient_parsing;
    let tracks = tauri::async_runtime::spawn_blocking(move || match mode {
        GameMode::AdoFai => adofai_scanner::scan_sources(
            &profile.folders,
            &profile.files,
            &profile.hidden_track_ids,
            &profile.hidden_tracks,
            lenient_parsing,
        ),
        GameMode::RhythmDoctor => rd_scanner::scan_sources(
            &profile.folders,
            &profile.files,
            &profile.hidden_track_ids,
            &profile.hidden_tracks,
            lenient_parsing,
        ),
    })
    .await
    .map_err(|err| format!("扫描曲库失败: {err}"))?;
    let mut stored = state
        .tracks_for_mode(mode)
        .lock()
        .map_err(|_| "曲库状态已损坏".to_string())?;
    *stored = tracks.clone();
    persist_tracks_cache(mode, &tracks)?;
    Ok(tracks)
}

#[tauri::command]
pub fn list_tracks(
    state: tauri::State<'_, AppState>,
    mode: GameMode,
) -> Result<Vec<TrackSummary>, String> {
    state
        .tracks_for_mode(mode)
        .lock()
        .map(|tracks| tracks.clone())
        .map_err(|_| "曲库状态已损坏".to_string())
}

#[tauri::command]
pub fn save_track_cache(
    state: tauri::State<'_, AppState>,
    mode: GameMode,
    tracks: Vec<TrackSummary>,
) -> Result<(), String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .clone();
    let tracks = filter_cached_tracks(tracks, mode, &settings);
    let mut stored = state
        .tracks_for_mode(mode)
        .lock()
        .map_err(|_| "曲库状态已损坏".to_string())?;
    *stored = tracks.clone();
    persist_tracks_cache(mode, &tracks)
}

#[tauri::command]
pub fn get_track_detail(
    state: tauri::State<'_, AppState>,
    mode: GameMode,
    chart_path: String,
) -> Result<TrackDetail, String> {
    let lenient = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .lenient_parsing;
    match mode {
        GameMode::AdoFai => adofai_scanner::detail_from_path(Path::new(&chart_path), lenient),
        GameMode::RhythmDoctor => rd_scanner::detail_from_path(Path::new(&chart_path), lenient),
    }
}

#[tauri::command]
pub fn get_track_summary(
    state: tauri::State<'_, AppState>,
    mode: GameMode,
    chart_path: String,
) -> Result<TrackSummary, String> {
    let path = Path::new(&chart_path);
    if !path.exists() || !path.is_file() {
        return Err("谱面文件不存在".to_string());
    }
    let lenient = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .lenient_parsing;
    Ok(match mode {
        GameMode::AdoFai => adofai_scanner::summary_from_path(path, lenient),
        GameMode::RhythmDoctor => rd_scanner::summary_from_path(path, lenient),
    })
}

#[tauri::command]
pub fn build_audio_timeline(
    state: tauri::State<'_, AppState>,
    mode: GameMode,
    chart_path: String,
) -> Result<AudioTimeline, String> {
    let lenient = state
        .settings
        .lock()
        .map_err(|_| "设置状态已损坏".to_string())?
        .lenient_parsing;
    match mode {
        GameMode::AdoFai => {
            adofai_timeline::build_timeline_from_path(Path::new(&chart_path), lenient)
        }
        GameMode::RhythmDoctor => {
            rd_timeline::build_timeline_from_path(Path::new(&chart_path), lenient)
        }
    }
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

fn refilter_mode_cache(
    state: &tauri::State<'_, AppState>,
    mode: GameMode,
    settings: &LibrarySettings,
) -> Result<(), String> {
    let mut tracks = state
        .tracks_for_mode(mode)
        .lock()
        .map_err(|_| "曲库状态已损坏".to_string())?;
    *tracks = filter_cached_tracks(tracks.clone(), mode, settings);
    Ok(())
}

fn filter_cached_tracks(
    tracks: Vec<TrackSummary>,
    mode: GameMode,
    settings: &LibrarySettings,
) -> Vec<TrackSummary> {
    let mut tracks: Vec<TrackSummary> = tracks
        .into_iter()
        .filter_map(|mut track| {
            if track.chart_path.is_empty() {
                track.chart_path = track.adofai_path.clone();
            }
            if track.adofai_path.is_empty() {
                track.adofai_path = track.chart_path.clone();
            }
            if track.game != mode {
                return None;
            }
            (is_from_enabled_source(&track, mode, settings)
                && !is_hidden_track(&track, mode, settings))
            .then_some(track)
        })
        .collect();
    tracks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    tracks.dedup_by(|a, b| {
        same_path(a.effective_chart_path(), b.effective_chart_path()) || a.id == b.id
    });
    tracks
}

fn is_from_enabled_source(
    track: &TrackSummary,
    mode: GameMode,
    settings: &LibrarySettings,
) -> bool {
    let profile = settings.profile(mode);
    profile
        .files
        .iter()
        .any(|file| same_path(file, track.effective_chart_path()))
        || profile
            .folders
            .iter()
            .any(|folder| path_is_inside(track.effective_chart_path(), folder))
}

fn is_hidden_track(track: &TrackSummary, mode: GameMode, settings: &LibrarySettings) -> bool {
    let profile = settings.profile(mode);
    profile
        .hidden_track_ids
        .iter()
        .any(|id| id.eq_ignore_ascii_case(&track.id))
        || profile.hidden_tracks.iter().any(|hidden| {
            hidden.game == mode
                && (hidden.id.eq_ignore_ascii_case(&track.id)
                    || (!hidden.effective_chart_path().is_empty()
                        && same_path(hidden.effective_chart_path(), track.effective_chart_path())))
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
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

mod adofai;
mod library;
mod rhythm_doctor;

use adofai::commands::{
    add_library_file, add_library_folder, build_audio_timeline, get_settings, get_track_detail,
    get_track_summary, list_tracks, resolve_asset_path, save_settings, save_track_cache,
    scan_library, AppState,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::load())
        .invoke_handler(tauri::generate_handler![
            add_library_folder,
            add_library_file,
            scan_library,
            list_tracks,
            get_track_detail,
            get_track_summary,
            build_audio_timeline,
            resolve_asset_path,
            get_settings,
            save_settings,
            save_track_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

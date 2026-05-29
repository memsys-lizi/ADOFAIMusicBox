mod adofai;

use adofai::commands::{
    add_library_folder, build_audio_timeline, get_settings, get_track_detail, list_tracks,
    resolve_asset_path, save_settings, scan_library, AppState,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::load())
        .invoke_handler(tauri::generate_handler![
            add_library_folder,
            scan_library,
            list_tracks,
            get_track_detail,
            build_audio_timeline,
            resolve_asset_path,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

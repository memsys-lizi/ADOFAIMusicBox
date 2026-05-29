use super::models::LibrarySettings;
use std::fs;
use std::path::PathBuf;

fn settings_path() -> PathBuf {
    let base = dirs::config_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ADOFAI Music Box").join("settings.json")
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

use super::conditions::ConditionContext;
use super::parser::{
    array_at, clean_display_text, event_type, filename_stem, number_setting, parse_level_file,
    resolve_sibling, settings_map, sound_ref_at, string_setting,
};
use super::timeline::build_timeline_from_root;
use crate::library::{
    GameMode, HiddenTrack, ParseStatus, ResourceStatus, TrackDetail, TrackSummary,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &["ogg", "mp3", "wav", "aif", "aiff", "flac"];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

pub fn scan_sources(
    folders: &[String],
    rd_files: &[String],
    hidden_track_ids: &[String],
    hidden_tracks: &[HiddenTrack],
    lenient: bool,
) -> Vec<TrackSummary> {
    let mut tracks = Vec::new();
    let mut seen = HashSet::new();
    for folder in folders {
        let root = Path::new(folder);
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !is_rdlevel(path) || is_backup_or_hidden(path) {
                continue;
            }
            push_track_once(&mut tracks, &mut seen, path, lenient);
        }
    }

    for file in rd_files {
        let path = Path::new(file);
        if path.exists() && is_rdlevel(path) && !is_backup_or_hidden(path) {
            push_track_once(&mut tracks, &mut seen, path, lenient);
        }
    }

    if !hidden_track_ids.is_empty() || !hidden_tracks.is_empty() {
        let mut hidden_ids: HashSet<&str> = hidden_track_ids.iter().map(String::as_str).collect();
        hidden_ids.extend(hidden_tracks.iter().map(|track| track.id.as_str()));
        let hidden_paths: HashSet<String> = hidden_tracks
            .iter()
            .filter(|track| {
                track.game == GameMode::RhythmDoctor && !track.effective_chart_path().is_empty()
            })
            .map(|track| normalize_path_key(track.effective_chart_path()))
            .collect();
        tracks.retain(|track| {
            !hidden_ids.contains(track.id.as_str())
                && !hidden_paths.contains(&normalize_path_key(track.effective_chart_path()))
        });
    }

    tracks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    tracks
}

pub fn detail_from_path(path: &Path, lenient: bool) -> Result<TrackDetail, String> {
    let parsed = parse_level_file(path, lenient)?;
    let settings = settings_map(&parsed.root);
    let mut warnings = parsed.warnings.clone();
    let summary = summary_from_parsed(path, &parsed.root, &parsed.parse_mode, &mut warnings);
    let resource_status = resource_status(path, &settings, &summary);
    let event_counts = event_counts(&parsed.root);
    let supported_audio_events = supported_audio_events(&event_counts);

    Ok(TrackDetail {
        summary,
        settings,
        resource_status,
        event_counts,
        warnings,
        raw_parse_mode: parsed.parse_mode,
        supported_audio_events,
    })
}

pub fn summary_from_path(path: &Path, lenient: bool) -> TrackSummary {
    match parse_level_file(path, lenient) {
        Ok(parsed) => {
            let mut warnings = parsed.warnings.clone();
            summary_from_parsed(path, &parsed.root, &parsed.parse_mode, &mut warnings)
        }
        Err(err) => {
            let fallback = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("无法解析的谱面")
                .to_string();
            let chart_path = path.to_string_lossy().to_string();
            TrackSummary {
                id: stable_id(path),
                game: GameMode::RhythmDoctor,
                chart_path: chart_path.clone(),
                adofai_path: chart_path,
                folder_path: path
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .to_string_lossy()
                    .to_string(),
                title: clean_display_text(&fallback),
                artist: "未知艺术家".to_string(),
                author: "未知谱师".to_string(),
                bpm: 0.0,
                duration: 0.0,
                cover_path: None,
                icon_path: None,
                audio_path: None,
                audio_file_size: None,
                video_path: None,
                has_video: false,
                video_offset_sec: 0.0,
                loop_video: false,
                parse_status: {
                    let _ = err;
                    ParseStatus::Error
                },
            }
        }
    }
}

fn summary_from_parsed(
    path: &Path,
    root: &Value,
    parse_mode: &str,
    warnings: &mut Vec<String>,
) -> TrackSummary {
    let settings = settings_map(root);
    let condition_context = ConditionContext::single_player(root);
    let song_ref = first_play_song(root, &condition_context)
        .map(|event| sound_ref_at(event, "song", "sndOrientalTechno"))
        .unwrap_or_else(|| super::parser::SoundRef::new("sndOrientalTechno"));
    let audio_path = resolve_level_audio(path, &song_ref.filename)
        .or_else(|| {
            resolve_level_audio(
                path,
                string_setting(&settings, "previewSong")
                    .as_deref()
                    .unwrap_or_default(),
            )
        })
        .or_else(|| find_first_with_extensions(path.parent(), AUDIO_EXTENSIONS));
    let audio_file_size = audio_path
        .as_ref()
        .filter(|path| !path.to_string_lossy().starts_with("rd:"))
        .and_then(|path| path.metadata().ok())
        .map(|metadata| metadata.len());
    let cover_path = resolve_sibling(path, string_setting(&settings, "previewImage").as_deref())
        .filter(|path| path.exists())
        .or_else(|| find_first_named_image(path.parent(), &["cover", "preview", "jacket"]));
    let icon_path = resolve_sibling(path, string_setting(&settings, "syringeIcon").as_deref())
        .filter(|path| path.exists())
        .or_else(|| find_first_named_image(path.parent(), &["icon", "syringe"]));

    if audio_path
        .as_ref()
        .is_none_or(|path| !path.exists() && !path.to_string_lossy().starts_with("rd:"))
    {
        warnings.push("未找到谱面引用的音乐文件".to_string());
    }

    let fallback_title = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("未命名谱面");
    let title = clean_display_text(
        &string_setting(&settings, "song").unwrap_or_else(|| fallback_title.to_string()),
    );
    let artist = clean_display_text(
        &string_setting(&settings, "artist").unwrap_or_else(|| "未知艺术家".to_string()),
    );
    let author = clean_display_text(
        &string_setting(&settings, "author").unwrap_or_else(|| "未知谱师".to_string()),
    );
    let bpm = first_play_song(root, &condition_context)
        .map(|event| super::parser::value_as_f64(event.get("beatsPerMinute"), 100.0))
        .unwrap_or_else(|| number_setting(&settings, "bpm", 100.0));
    let duration = build_timeline_from_root(root, path, false)
        .map(|timeline| timeline.duration)
        .unwrap_or(0.0);
    let chart_path = path.to_string_lossy().to_string();

    TrackSummary {
        id: stable_id(path),
        game: GameMode::RhythmDoctor,
        chart_path: chart_path.clone(),
        adofai_path: chart_path,
        folder_path: path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_string_lossy()
            .to_string(),
        title,
        artist,
        author,
        bpm,
        duration,
        cover_path: path_string(cover_path),
        icon_path: path_string(icon_path),
        audio_path: path_string(audio_path),
        audio_file_size,
        video_path: None,
        has_video: false,
        video_offset_sec: 0.0,
        loop_video: false,
        parse_status: if !warnings.is_empty() {
            ParseStatus::Warning
        } else if parse_mode == "strict-json" {
            ParseStatus::Ok
        } else {
            ParseStatus::Lenient
        },
    }
}

fn resource_status(
    level_path: &Path,
    settings: &BTreeMap<String, Value>,
    summary: &TrackSummary,
) -> ResourceStatus {
    let mut missing = Vec::new();
    let audio_exists = summary
        .audio_path
        .as_ref()
        .map(|path| path.starts_with("rd:") || Path::new(path).exists())
        .unwrap_or(false);
    let cover_exists = summary
        .cover_path
        .as_ref()
        .map(|path| Path::new(path).exists())
        .unwrap_or(false);
    let icon_exists = summary
        .icon_path
        .as_ref()
        .map(|path| Path::new(path).exists())
        .unwrap_or(false);

    if !audio_exists {
        missing.push("音乐文件".to_string());
    }
    if string_setting(settings, "previewImage").is_some() && !cover_exists {
        missing.push("封面 previewImage".to_string());
    }
    if string_setting(settings, "syringeIcon").is_some() && !icon_exists {
        missing.push("图标 syringeIcon".to_string());
    }
    if !level_path.exists() {
        missing.push("谱面文件".to_string());
    }

    ResourceStatus {
        audio_exists,
        cover_exists,
        icon_exists,
        video_exists: false,
        missing,
    }
}

fn event_counts(root: &Value) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for event in array_at(root, "events") {
        if let Some(event_type) = event_type(event) {
            *counts.entry(event_type.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn supported_audio_events(counts: &BTreeMap<String, usize>) -> Vec<String> {
    [
        "PlaySong",
        "PlaySound",
        "SetBeatSound",
        "SetClapSounds",
        "SetGameSound",
        "SetCountingSound",
        "AddClassicBeat",
        "AddOneshotBeat",
        "AddFreeTimeBeat",
        "PulseFreeTimeBeat",
        "SayReadyGetSetGo",
    ]
    .into_iter()
    .filter(|key| counts.contains_key(*key))
    .map(ToOwned::to_owned)
    .collect()
}

fn first_play_song<'a>(root: &'a Value, condition_context: &ConditionContext) -> Option<&'a Value> {
    array_at(root, "events")
        .into_iter()
        .find(|event| condition_context.event_runs(event) && event_type(event) == Some("PlaySong"))
}

fn resolve_level_audio(level_path: &Path, filename: &str) -> Option<PathBuf> {
    let filename = filename.trim();
    if filename.is_empty() || filename == "None" {
        return None;
    }
    if let Some(path) = resolve_sibling(level_path, Some(filename)).filter(|path| path.exists()) {
        return Some(path);
    }
    if !super::parser::has_audio_extension(filename) {
        if let Some(path) = find_sibling_by_stem(level_path.parent(), filename) {
            return Some(path);
        }
    }
    Some(PathBuf::from(format!("rd:{}", filename_stem(filename))))
}

fn is_rdlevel(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rdlevel"))
}

fn push_track_once(
    tracks: &mut Vec<TrackSummary>,
    seen: &mut HashSet<String>,
    path: &Path,
    lenient: bool,
) {
    let key = path.to_string_lossy().to_lowercase();
    if seen.insert(key) {
        tracks.push(summary_from_path(path, lenient));
    }
}

fn is_backup_or_hidden(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    name == "backup.rdlevel" || name.starts_with('.')
}

fn stable_id(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    "rhythm-doctor".hash(&mut hasher);
    path.to_string_lossy().to_lowercase().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn find_first_with_extensions(folder: Option<&Path>, exts: &[&str]) -> Option<PathBuf> {
    let folder = folder?;
    let entries = std::fs::read_dir(folder).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|ext| exts.iter().any(|item| ext.eq_ignore_ascii_case(item)))
        })
}

fn find_sibling_by_stem(folder: Option<&Path>, stem: &str) -> Option<PathBuf> {
    let folder = folder?;
    let normalized = stem.to_ascii_lowercase();
    let entries = std::fs::read_dir(folder).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case(&normalized))
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|ext| {
                        AUDIO_EXTENSIONS
                            .iter()
                            .any(|item| ext.eq_ignore_ascii_case(item))
                    })
        })
}

fn find_first_named_image(folder: Option<&Path>, names: &[&str]) -> Option<PathBuf> {
    let folder = folder?;
    let entries = std::fs::read_dir(folder).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_lowercase();
            let is_image = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| {
                    IMAGE_EXTENSIONS
                        .iter()
                        .any(|item| ext.eq_ignore_ascii_case(item))
                });
            is_image && names.iter().any(|name| stem.contains(name))
        })
}

fn path_string(path: Option<PathBuf>) -> Option<String> {
    path.map(|path| path.to_string_lossy().to_string())
}

fn normalize_path_key(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

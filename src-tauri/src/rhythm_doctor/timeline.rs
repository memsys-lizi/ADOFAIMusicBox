use super::conditions::ConditionContext;
use super::parser::{
    array_at, event_bar, event_beat, event_sort_order, event_type, filename_stem, parse_level_file,
    sound_ref_at, value_as_bool, value_as_f64, value_as_i64, value_as_string, SoundRef,
};
use crate::library::{AudioTimeline, HitEvent};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
struct CpbChange {
    bar: i32,
    cpb: i32,
}

#[derive(Debug, Clone)]
struct BpmChange {
    beat: f64,
    bpm: f64,
}

#[derive(Debug, Clone)]
struct BeatClock {
    cpb_changes: Vec<CpbChange>,
    bpm_changes: Vec<BpmChange>,
}

#[derive(Debug, Clone)]
struct TimelineState {
    rows: HashMap<i64, RowState>,
    game_sounds: BTreeMap<String, SoundRef>,
    next_hold_pulse_alt: bool,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            rows: HashMap::new(),
            game_sounds: default_game_sounds(),
            next_hold_pulse_alt: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Classic,
    Oneshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowPlayer {
    P1,
    P2,
    Cpu,
}

#[derive(Debug, Clone)]
struct RowState {
    row_type: RowKind,
    player: RowPlayer,
    pulse_sounds: Vec<SoundRef>,
    show: [bool; 6],
    synco_beat: i32,
    synco_swing: f64,
    synco_volume: f32,
    synco_pitch: f32,
    synco_style: String,
    counting_enabled: bool,
    counting_sounds: Vec<SoundRef>,
    counting_subdiv_offset: f64,
    muted: bool,
}

impl Default for RowState {
    fn default() -> Self {
        let mut row = Self {
            row_type: RowKind::Classic,
            player: RowPlayer::P1,
            pulse_sounds: vec![SoundRef::new("sndKick"); 7],
            show: [true; 6],
            synco_beat: -1,
            synco_swing: 0.0,
            synco_volume: 0.7,
            synco_pitch: 1.0,
            synco_style: "Chirp".to_string(),
            counting_enabled: false,
            counting_sounds: Vec::new(),
            counting_subdiv_offset: 0.5,
            muted: false,
        };
        row.set_counting_sounds("JyiCount", 1.0);
        row
    }
}

impl RowState {
    fn set_pulse_sounds(&mut self, sound: SoundRef) {
        self.pulse_sounds = vec![sound; 7];
    }

    fn pulse_sound(&self, beatbox_number: usize) -> SoundRef {
        self.pulse_sounds
            .get(beatbox_number.saturating_sub(1))
            .cloned()
            .unwrap_or_else(|| SoundRef::new("sndKick"))
    }

    fn set_counting_sounds(&mut self, voice_source: &str, volume: f32) {
        let count = if self.row_type == RowKind::Oneshot {
            10
        } else {
            7
        };
        let voice = if self.row_type == RowKind::Oneshot && voice_source == "JyiCount" {
            "JyiCountEnglish"
        } else {
            voice_source
        };
        let prefix = counting_voice_prefix(voice);
        self.counting_sounds = (1..=count)
            .map(|index| {
                let mut sound = SoundRef::new(&format!("{prefix}{index}"));
                sound.volume = volume;
                sound
            })
            .collect();
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RdAudioMetadata {
    sound_offsets: HashMap<String, RdSoundOffset>,
    game_sounds: HashMap<String, RdGameSound>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RdSoundOffset {
    offset_ms: f64,
    volume: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RdGameSound {
    filename: String,
    volume: f32,
    max_pitch: f32,
}

static RD_AUDIO_METADATA: OnceLock<RdAudioMetadata> = OnceLock::new();

pub fn build_timeline_from_path(path: &Path, lenient: bool) -> Result<AudioTimeline, String> {
    let parsed = parse_level_file(path, lenient)?;
    let mut timeline = build_timeline_from_root(&parsed.root, path, true)?;
    timeline.warnings.extend(parsed.warnings);
    Ok(timeline)
}

pub fn build_timeline_from_root(
    root: &Value,
    level_path: &Path,
    include_events: bool,
) -> Result<AudioTimeline, String> {
    let condition_context = ConditionContext::single_player(root);
    let cpb_changes = collect_cpb_changes(root, &condition_context);
    let bpm_changes = collect_bpm_changes(root, &cpb_changes, &condition_context);
    let clock = BeatClock {
        cpb_changes,
        bpm_changes,
    };
    let song_ref = first_play_song(root, &condition_context)
        .map(|event| sound_ref_at(event, "song", "sndOrientalTechno"))
        .unwrap_or_else(|| SoundRef::new("sndOrientalTechno"));
    let pitch = song_ref.pitch.max(0.05) as f64;
    let mut timeline = AudioTimeline {
        song_offset_ms: song_ref.offset_ms,
        countdown_lead_in_sec: 0.0,
        pitch,
        duration: 0.0,
        hit_events: Vec::new(),
        play_sound_events: Vec::new(),
        hold_sound_events: Vec::new(),
        warnings: Vec::new(),
    };

    if include_events {
        append_events(&mut timeline, root, level_path, &clock, &condition_context);
    }

    let max_event_time = timeline
        .hit_events
        .iter()
        .chain(timeline.play_sound_events.iter())
        .chain(timeline.hold_sound_events.iter())
        .map(|event| event.end_time_sec.unwrap_or(event.time_sec))
        .fold(0.0, f64::max);
    timeline.duration = max_event_time.max(last_event_time(root, &clock, &condition_context) + 4.0);
    Ok(timeline)
}

fn append_events(
    timeline: &mut AudioTimeline,
    root: &Value,
    level_path: &Path,
    clock: &BeatClock,
    condition_context: &ConditionContext,
) {
    let mut events = array_at(root, "events");
    events.sort_by(|left, right| event_sort_key(left).cmp(&event_sort_key(right)));
    let mut state = TimelineState::default();
    apply_initial_rows(&mut state, root);
    let mut saw_runtime_filters = false;

    for (index, event) in events.iter().enumerate() {
        if ConditionContext::event_has_filter(event) {
            saw_runtime_filters = true;
        }
        if !condition_context.event_runs(event) {
            continue;
        }
        match event_type(event) {
            Some("MakeRow") => apply_make_row(&mut state, event),
            Some("SetBeatSound") => {
                let row = value_as_i64(event.get("row"), 0);
                state
                    .row_mut(row)
                    .set_pulse_sounds(sound_ref_at(event, "sound", "Shaker"));
            }
            Some("SetClapSounds") => apply_clap_sounds(&mut state, event),
            Some("SetGameSound") => apply_game_sound(&mut state, event),
            Some("SetCountingSound") => apply_counting_sound(&mut state, event),
            Some("SetRowXs") => apply_row_xs(&mut state, event),
            Some("PlaySound") => append_play_sound(timeline, event, level_path, clock),
            Some("AddClassicBeat") => {
                append_classic_beat(timeline, event, level_path, clock, &mut state)
            }
            Some("AddFreeTimeBeat") => append_free_time_beat(
                timeline,
                event,
                &events,
                index,
                level_path,
                clock,
                &mut state,
                condition_context,
            ),
            Some("PulseFreeTimeBeat") => {}
            Some("AddOneshotBeat") => {
                append_oneshot_beat(timeline, event, level_path, clock, &state)
            }
            Some("SayReadyGetSetGo") => append_ready_get_set_go(timeline, event, clock),
            Some("ReadNarration") | Some("NarrateRowInfo") => {
                append_narration_hint(timeline, event, clock)
            }
            _ => {}
        }
    }

    if saw_runtime_filters {
        timeline
            .warnings
            .push("已按单人模式过滤 RD 条件与标签事件。".to_string());
    }
}

fn apply_initial_rows(state: &mut TimelineState, root: &Value) {
    for row in array_at(root, "rows") {
        apply_make_row(state, row);
    }
}

fn collect_cpb_changes(root: &Value, condition_context: &ConditionContext) -> Vec<CpbChange> {
    let mut changes = vec![CpbChange { bar: 1, cpb: 8 }];
    for event in array_at(root, "events") {
        if !condition_context.event_runs(event) || event_type(event) != Some("SetCrotchetsPerBar") {
            continue;
        }
        changes.push(CpbChange {
            bar: event_bar(event).max(1),
            cpb: value_as_i64(event.get("crotchetsPerBar"), 8).clamp(1, 64) as i32,
        });
    }
    changes.sort_by_key(|change| change.bar);
    dedup_cpb_changes(changes)
}

fn dedup_cpb_changes(changes: Vec<CpbChange>) -> Vec<CpbChange> {
    let mut by_bar = BTreeMap::new();
    for change in changes {
        by_bar.insert(change.bar, change.cpb);
    }
    by_bar
        .into_iter()
        .map(|(bar, cpb)| CpbChange { bar, cpb })
        .collect()
}

fn collect_bpm_changes(
    root: &Value,
    cpb_changes: &[CpbChange],
    condition_context: &ConditionContext,
) -> Vec<BpmChange> {
    let base_bpm = first_play_song(root, condition_context)
        .map(|event| event_bpm(event, 100.0))
        .unwrap_or(100.0)
        .max(1.0);
    let mut changes = vec![BpmChange {
        beat: 0.0,
        bpm: base_bpm,
    }];
    for event in array_at(root, "events") {
        if !condition_context.event_runs(event) {
            continue;
        }
        if event_type(event) == Some("SetBeatsPerMinute") {
            changes.push(BpmChange {
                beat: bar_beat_to_abs(event_bar(event), event_beat(event), cpb_changes),
                bpm: event_bpm(event, base_bpm).max(1.0),
            });
        }
    }
    changes.sort_by(|left, right| {
        left.beat
            .partial_cmp(&right.beat)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    dedup_bpm_changes(changes)
}

fn event_bpm(event: &Value, fallback: f64) -> f64 {
    value_as_f64(
        event.get("beatsPerMinute"),
        value_as_f64(event.get("bpm"), fallback),
    )
}

fn dedup_bpm_changes(changes: Vec<BpmChange>) -> Vec<BpmChange> {
    let mut deduped: Vec<BpmChange> = Vec::new();
    for change in changes {
        if let Some(previous) = deduped.last_mut() {
            if (previous.beat - change.beat).abs() < 0.0001 {
                *previous = change;
                continue;
            }
        }
        deduped.push(change);
    }
    deduped
}

fn append_play_sound(
    timeline: &mut AudioTimeline,
    event: &Value,
    level_path: &Path,
    clock: &BeatClock,
) {
    let sound = sound_ref_at(event, "sound", "Shaker");
    append_sound_ref(
        &mut timeline.play_sound_events,
        clock.time_at(event_abs(event, clock)),
        None,
        &sound,
        level_path,
        event_bar(event) as usize,
        "rd-play-sound",
    );
}

fn append_classic_beat(
    timeline: &mut AudioTimeline,
    event: &Value,
    level_path: &Path,
    clock: &BeatClock,
    state: &mut TimelineState,
) {
    let row_id = value_as_i64(event.get("row"), 0);
    let row = state.row(row_id).clone();
    if row.muted {
        return;
    }
    let tick = value_as_f64(event.get("tick"), 1.0).max(0.0);
    let swing = value_as_f64(event.get("swing"), 0.0).clamp(0.0, tick * 2.0);
    let hold = value_as_f64(event.get("hold"), 0.0).max(0.0);
    let length = value_as_i64(event.get("length"), 7).clamp(1, 7) as usize;
    let start = event_abs(event, clock);
    let custom_sound = if event.get("sound").is_some() {
        Some(sound_ref_at(event, "sound", "Shaker"))
    } else {
        None
    };
    let beats = classic_beat_times(
        start,
        tick,
        swing,
        value_as_bool(event.get("legacy"), false),
        hold,
        length,
        row.synco_beat,
        row.synco_swing,
    );
    append_classic_flexible(
        timeline,
        &beats,
        &row,
        custom_sound.as_ref(),
        state,
        level_path,
        clock,
        event_bar(event) as usize,
    );
}

#[derive(Debug, Clone)]
struct ClassicBeatTime {
    beatbox_number: i32,
    press_abs: f64,
    release_abs: f64,
}

fn classic_beat_times(
    start: f64,
    tick: f64,
    swing: f64,
    legacy_swing: bool,
    hold: f64,
    length: usize,
    synco_beat: i32,
    synco_swing: f64,
) -> Vec<ClassicBeatTime> {
    (0..length)
        .map(|index| {
            let one_based = index + 1;
            let mut synco_offset = 0.0;
            if (0..=5).contains(&synco_beat) && index as i32 > synco_beat {
                synco_offset = -tick * if synco_swing > 0.0 { synco_swing } else { 0.5 };
            }
            let mut press = tick * index as f64 + start;
            if swing != 0.0 {
                if legacy_swing {
                    let parity = (start / tick + index as f64).round() % 2.0;
                    if (parity - 1.0).abs() < 0.0001 {
                        press = tick * one_based as f64 + start - swing;
                    }
                } else if one_based % 2 == 0 {
                    press = start + tick * one_based as f64 - swing;
                }
            }
            let press_abs = press + synco_offset;
            ClassicBeatTime {
                beatbox_number: 8 - length as i32 + index as i32,
                press_abs,
                release_abs: press_abs + hold,
            }
        })
        .collect()
}

fn append_classic_flexible(
    timeline: &mut AudioTimeline,
    beats: &[ClassicBeatTime],
    row: &RowState,
    custom_sound: Option<&SoundRef>,
    state: &mut TimelineState,
    level_path: &Path,
    clock: &BeatClock,
    source_floor: usize,
) {
    let has_held_pulses = beats
        .iter()
        .any(|beat| (beat.release_abs - beat.press_abs).abs() > 0.0001);

    for (index, beat) in beats.iter().enumerate() {
        let beatbox_number = beat.beatbox_number;
        if beatbox_number == -99 {
            break;
        }
        if beatbox_number <= 0 {
            continue;
        }
        let is_seventh = beatbox_number == 7;
        let visible = !is_seventh
            && row
                .show
                .get((beatbox_number - 1) as usize)
                .copied()
                .unwrap_or(true);
        if !(visible || is_seventh) {
            continue;
        }

        let press_time = clock.time_at(beat.press_abs);
        let release_time = clock.time_at(beat.release_abs);
        if is_seventh {
            if has_held_pulses {
                append_held_clap(
                    timeline,
                    state,
                    level_path,
                    row,
                    press_time,
                    release_time,
                    false,
                    source_floor,
                    "rd-classic-held-clap",
                );
            } else {
                append_game_sound_to(
                    &mut timeline.hit_events,
                    state,
                    clap_sound_type(row, false),
                    press_time,
                    None,
                    level_path,
                    source_floor,
                    "rd-classic-clap",
                );
            }
        } else if has_held_pulses && (beat.release_abs - beat.press_abs).abs() > 0.0001 {
            append_held_pulse(
                timeline,
                state,
                level_path,
                press_time,
                release_time,
                source_floor,
                "rd-classic-held-pulse",
            );
        } else {
            let sound = custom_sound
                .cloned()
                .unwrap_or_else(|| row.pulse_sound(beatbox_number as usize));
            append_sound_ref(
                &mut timeline.hit_events,
                press_time,
                None,
                &sound,
                level_path,
                source_floor,
                "rd-classic-pulse",
            );
        }

        if row.counting_enabled {
            if let Some(count) = row.counting_sounds.get((beatbox_number - 1) as usize) {
                append_sound_ref(
                    &mut timeline.play_sound_events,
                    press_time,
                    None,
                    count,
                    level_path,
                    source_floor,
                    "rd-counting",
                );
            }
        }
        append_synco_sound(timeline, row, index, press_time, source_floor);
    }
}

fn append_synco_sound(
    timeline: &mut AudioTimeline,
    row: &RowState,
    index: usize,
    press_time: f64,
    source_floor: usize,
) {
    if !(0..=5).contains(&row.synco_beat) {
        return;
    }
    let sound = match row.synco_style.as_str() {
        "Chirp" if index as i32 == row.synco_beat => Some("sndSyncoPull"),
        "Chirp" if index as i32 == row.synco_beat + 1 => Some("sndSyncoRelease"),
        "Chirp" if index == 0 => Some("sndSyncoFirstPulse"),
        "Beep" if index == 6 => Some("sndSyncopationBeepHigh"),
        "Beep" if index as i32 == row.synco_beat + 1 => Some("sndSyncopationBeepMid"),
        "Beep" if index % 2 == 0 && index as i32 <= row.synco_beat => Some("sndSyncopationBeepLow"),
        _ => None,
    };
    if let Some(sound) = sound {
        append_builtin(
            &mut timeline.play_sound_events,
            press_time,
            None,
            sound,
            row.synco_volume,
            row.synco_pitch,
            source_floor,
            "rd-synco",
        );
    }
}

fn append_held_pulse(
    timeline: &mut AudioTimeline,
    state: &mut TimelineState,
    level_path: &Path,
    start: f64,
    end: f64,
    source_floor: usize,
    kind: &str,
) {
    let duration = (end - start).max(0.0);
    let is_short = duration < game_sound_offset(state, "PulseSoundHoldEnd");
    let use_alt = state.next_hold_pulse_alt;
    state.next_hold_pulse_alt = !state.next_hold_pulse_alt;
    let start_type = if use_alt {
        "PulseSoundHoldStartAlt"
    } else {
        "PulseSoundHoldStart"
    };
    let end_type = match (use_alt, is_short) {
        (true, true) => "PulseSoundHoldShortEndAlt",
        (true, false) => "PulseSoundHoldEndAlt",
        (false, true) => "PulseSoundHoldShortEnd",
        (false, false) => "PulseSoundHoldEnd",
    };
    append_game_sound_to(
        &mut timeline.hold_sound_events,
        state,
        start_type,
        start,
        Some(end),
        level_path,
        source_floor,
        kind,
    );
    append_game_sound_to(
        &mut timeline.hold_sound_events,
        state,
        end_type,
        end,
        None,
        level_path,
        source_floor,
        "rd-held-pulse-end",
    );
}

fn append_held_clap(
    timeline: &mut AudioTimeline,
    state: &TimelineState,
    level_path: &Path,
    row: &RowState,
    start: f64,
    end: f64,
    oneshot: bool,
    source_floor: usize,
    kind: &str,
) {
    let duration = (end - start).max(0.0);
    let is_p2_classic = !oneshot && row.player == RowPlayer::P2;
    let long_end = if oneshot {
        "HoldshotSoundClapLongEnd"
    } else if is_p2_classic {
        "ClapSoundHoldLongEndP2"
    } else {
        "ClapSoundHoldLongEnd"
    };
    let short_end = if oneshot {
        "HoldshotSoundClapShortEnd"
    } else if is_p2_classic {
        "ClapSoundHoldShortEndP2"
    } else {
        "ClapSoundHoldShortEnd"
    };
    let is_short = duration < game_sound_offset(state, long_end);
    let start_type = match (oneshot, is_p2_classic, is_short) {
        (true, _, _) => "HoldshotSoundClapStart",
        (false, true, true) => "ClapSoundHoldShortStartP2",
        (false, true, false) => "ClapSoundHoldLongStartP2",
        (false, false, true) => "ClapSoundHoldShortStart",
        (false, false, false) => "ClapSoundHoldLongStart",
    };
    let end_type = if is_short { short_end } else { long_end };
    append_game_sound_to(
        &mut timeline.hold_sound_events,
        state,
        start_type,
        start,
        Some(end),
        level_path,
        source_floor,
        kind,
    );
    append_game_sound_to(
        &mut timeline.hold_sound_events,
        state,
        end_type,
        end,
        None,
        level_path,
        source_floor,
        "rd-held-clap-end",
    );
}

fn append_free_time_beat(
    timeline: &mut AudioTimeline,
    event: &Value,
    events: &[&Value],
    event_index: usize,
    level_path: &Path,
    clock: &BeatClock,
    state: &mut TimelineState,
    condition_context: &ConditionContext,
) {
    let row_id = value_as_i64(event.get("row"), 0);
    let row = state.row(row_id).clone();
    if row.muted {
        return;
    }
    let mut pulse = value_as_i64(event.get("pulse"), 0).clamp(0, 6) as i32;
    let mut beats = vec![ClassicBeatTime {
        beatbox_number: pulse + 1,
        press_abs: event_abs(event, clock),
        release_abs: event_abs(event, clock) + value_as_f64(event.get("hold"), 0.0).max(0.0),
    }];

    if pulse != 6 {
        for next in events.iter().skip(event_index + 1) {
            if !condition_context.event_runs(next)
                || event_type(next) != Some("PulseFreeTimeBeat")
                || value_as_i64(next.get("row"), 0) != row_id
            {
                continue;
            }
            let action = value_as_string(next.get("action"), "Increment");
            if action == "Remove" {
                beats.push(ClassicBeatTime {
                    beatbox_number: -99,
                    press_abs: event_abs(next, clock),
                    release_abs: event_abs(next, clock),
                });
                break;
            }
            pulse = match action.as_str() {
                "Decrement" => pulse - 1,
                "Custom" => value_as_i64(next.get("customPulse"), pulse as i64) as i32,
                _ => pulse + 1,
            }
            .clamp(0, 6);
            let press_abs = event_abs(next, clock);
            beats.push(ClassicBeatTime {
                beatbox_number: pulse + 1,
                press_abs,
                release_abs: press_abs + value_as_f64(next.get("hold"), 0.0).max(0.0),
            });
            if pulse == 6 {
                break;
            }
        }
    }

    append_classic_flexible(
        timeline,
        &beats,
        &row,
        None,
        state,
        level_path,
        clock,
        event_bar(event) as usize,
    );
}

fn append_oneshot_beat(
    timeline: &mut AudioTimeline,
    event: &Value,
    level_path: &Path,
    clock: &BeatClock,
    state: &TimelineState,
) {
    let row_id = value_as_i64(event.get("row"), 0);
    let row = state.row(row_id).clone();
    if row.muted {
        return;
    }
    let pulse_type = value_as_string(event.get("pulseType"), "Wave");
    let freeze_burn = value_as_string(event.get("freezeBurnMode"), "None");
    let tick = value_as_f64(event.get("tick"), 1.0).max(0.0);
    let delay = if freeze_burn == "Freezeshot" {
        value_as_f64(event.get("delay"), 0.5).max(0.0)
    } else {
        0.0
    };
    let loops = value_as_i64(event.get("loops"), 0).max(0) as usize;
    let hold = value_as_bool(event.get("hold"), false);
    let hold_cue = value_as_string(event.get("holdCue"), "Auto");
    let interval = if freeze_burn != "None"
        || loops > 0
        || value_as_bool(event.get("skipshot"), false)
        || hold
    {
        value_as_f64(event.get("interval"), 2.0).max(0.0)
    } else {
        (2.0 * tick).max(0.0)
    };
    let freeze_offset = if freeze_burn == "Freezeshot" || (hold && hold_cue != "Late") {
        (interval - tick).max(0.0)
    } else if freeze_burn == "Burnshot" {
        interval.max(0.0)
    } else {
        0.0
    };
    let start = (event_abs(event, clock) - freeze_offset).max(0.0);
    let subdivisions = match pulse_type.as_str() {
        "Square" => 1,
        "Triangle" => value_as_i64(event.get("subdivisions"), 1).clamp(1, 10) as usize,
        _ => 0,
    };
    let subdiv_sound = value_as_bool(event.get("subdivSound"), true);
    let subdiv_tick_override = if subdivisions > 1 && freeze_burn == "Burnshot" {
        value_as_f64(event.get("subdivTickOverride"), 0.0).max(0.0)
    } else {
        0.0
    };
    let custom_sound = if event.get("sound").is_some() {
        sound_ref_at(event, "sound", "Shaker")
    } else {
        row.pulse_sound(1)
    };

    for repeat in 0..=loops {
        let repeat_start = start + interval * repeat as f64;
        append_oneshot_counting_sound(
            timeline,
            clock,
            repeat_start,
            tick,
            interval,
            subdivisions,
            subdiv_tick_override,
            &freeze_burn,
            &row,
            level_path,
            event_bar(event) as usize,
        );
        if value_as_bool(event.get("skipshot"), false) {
            let subdiv_offset = if pulse_type == "Triangle" && subdivisions > 1 {
                tick * (1.0 - 1.0 / subdivisions as f64)
            } else {
                0.0
            };
            append_game_sound(
                timeline,
                clock,
                repeat_start + freeze_offset + tick + delay + subdiv_offset,
                "Skipshot",
                state,
                level_path,
                event_bar(event) as usize,
                "rd-skipshot",
            );
        }

        if pulse_type == "Triangle" && subdivisions > 1 {
            let subdiv_tick = if freeze_burn == "Burnshot" && subdiv_tick_override > 0.0 {
                subdiv_tick_override
            } else if freeze_burn == "Burnshot" {
                interval / 2.0
            } else {
                tick
            };
            let step = subdiv_tick / subdivisions as f64;
            let mut hold_cue_offset = hold_cue_offset(
                hold,
                &hold_cue,
                freeze_burn == "Burnshot",
                interval,
                tick,
                clock,
                repeat_start,
            );
            hold_cue_offset += step * (subdivisions - 1) as f64;
            for index in 0..subdivisions {
                let sound = if subdiv_sound {
                    SoundRef::new(&format!("sndTriangleshot{}", index + 1))
                } else {
                    custom_sound.clone()
                };
                let hold_duration = if hold && index + 1 == subdivisions {
                    (interval - (tick + delay + step * index as f64)).max(0.0)
                } else {
                    0.0
                };
                append_oneshot_instance(
                    timeline,
                    state,
                    level_path,
                    clock,
                    repeat_start + step * index as f64,
                    tick,
                    &sound,
                    &row,
                    delay,
                    freeze_offset,
                    freeze_burn == "Burnshot",
                    hold_duration,
                    hold_cue_offset,
                    index > 0,
                    event_bar(event) as usize,
                );
            }
        } else {
            let sound = if pulse_type == "Square" && subdiv_sound {
                SoundRef::new("sndSquareshot")
            } else {
                custom_sound.clone()
            };
            let hold_duration = if hold {
                (interval - (tick + delay)).max(0.0)
            } else {
                0.0
            };
            let hold_cue_offset = hold_cue_offset(
                hold,
                &hold_cue,
                freeze_burn == "Burnshot",
                interval,
                tick,
                clock,
                repeat_start,
            );
            append_oneshot_instance(
                timeline,
                state,
                level_path,
                clock,
                repeat_start,
                tick,
                &sound,
                &row,
                delay,
                freeze_offset,
                freeze_burn == "Burnshot",
                hold_duration,
                hold_cue_offset,
                false,
                event_bar(event) as usize,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_oneshot_instance(
    timeline: &mut AudioTimeline,
    state: &TimelineState,
    level_path: &Path,
    clock: &BeatClock,
    boom_start: f64,
    tick: f64,
    hit_sound: &SoundRef,
    row: &RowState,
    delay: f64,
    delay_beat_offset: f64,
    is_burnshot: bool,
    hold_duration: f64,
    hold_cue_offset: f64,
    second_subdivision_or_later: bool,
    source_floor: usize,
) {
    let freezeshot = delay > 0.0;
    let boom = boom_start + delay_beat_offset;
    let chak = boom + tick + delay;
    let pre_delay_hit = boom + tick;
    let release = chak + hold_duration;
    let first_cue = boom_start;
    let second_cue = boom_start + if freezeshot { delay } else { tick };

    append_sound_ref(
        &mut timeline.hit_events,
        clock.time_at(boom),
        None,
        hit_sound,
        level_path,
        source_floor,
        "rd-oneshot-boom",
    );
    append_game_sound_to(
        &mut timeline.hit_events,
        state,
        clap_sound_type(row, true),
        clock.time_at(chak),
        None,
        level_path,
        source_floor,
        "rd-oneshot-clap",
    );

    if !second_subdivision_or_later {
        if freezeshot {
            for (sound_type, beat) in [
                ("FreezeshotSoundCueLow", first_cue),
                ("FreezeshotSoundCueHigh", second_cue),
                ("FreezeshotSoundCueHigh", pre_delay_hit),
                ("FreezeshotSoundRiser", pre_delay_hit - 0.74),
                ("FreezeshotSoundCueLow", chak),
                ("FreezeshotSoundCymbal", chak),
            ] {
                append_game_sound(
                    timeline,
                    clock,
                    beat,
                    sound_type,
                    state,
                    level_path,
                    source_floor,
                    "rd-freezeshot",
                );
            }
        } else if is_burnshot {
            for (sound_type, beat) in [
                ("BurnshotSoundCueLow", first_cue),
                ("BurnshotSoundCueHigh", second_cue),
                ("BurnshotSoundCueHigh", boom),
                ("BurnshotSoundRiser", boom),
                ("BurnshotSoundCueLow", chak),
                ("BurnshotSoundCymbal", chak),
            ] {
                append_game_sound(
                    timeline,
                    clock,
                    beat,
                    sound_type,
                    state,
                    level_path,
                    source_floor,
                    "rd-burnshot",
                );
            }
        }
    }

    if hold_duration > 0.0 {
        append_held_clap(
            timeline,
            state,
            level_path,
            row,
            clock.time_at(chak),
            clock.time_at(release),
            true,
            source_floor,
            "rd-holdshot-clap",
        );
        append_game_sound(
            timeline,
            clock,
            boom - hold_cue_offset,
            "HoldshotSoundCue",
            state,
            level_path,
            source_floor,
            "rd-holdshot-cue",
        );
    }
}

fn hold_cue_offset(
    hold: bool,
    hold_cue: &str,
    is_burnshot: bool,
    interval: f64,
    tick: f64,
    clock: &BeatClock,
    start: f64,
) -> f64 {
    if !hold {
        return 0.0;
    }
    let auto_early = clock.duration_between(start, tick) <= 0.666;
    if hold_cue == "Early" || (hold_cue == "Auto" && auto_early) {
        if is_burnshot {
            interval
        } else {
            (interval - tick).max(0.0)
        }
    } else {
        0.0
    }
}

#[allow(clippy::too_many_arguments)]
fn append_oneshot_counting_sound(
    timeline: &mut AudioTimeline,
    clock: &BeatClock,
    start: f64,
    tick: f64,
    interval: f64,
    subdivisions: usize,
    subdiv_tick_override: f64,
    freeze_burn: &str,
    row: &RowState,
    level_path: &Path,
    source_floor: usize,
) {
    if !row.counting_enabled || subdivisions == 0 {
        return;
    }
    let count_tick = if freeze_burn != "Burnshot" {
        tick
    } else if subdiv_tick_override > 0.0 {
        subdiv_tick_override
    } else {
        interval / 2.0
    };
    let mut count_offset = if subdivisions == 1 {
        0.0
    } else {
        row.counting_subdiv_offset * count_tick
    };
    if freeze_burn == "Freezeshot" {
        count_offset -= interval - tick;
    } else if freeze_burn == "Burnshot" {
        count_offset -= interval;
    }
    if let Some(sound) = row.counting_sounds.get(subdivisions.saturating_sub(1)) {
        append_sound_ref(
            &mut timeline.play_sound_events,
            clock.time_at(start - count_offset),
            None,
            sound,
            level_path,
            source_floor,
            "rd-counting",
        );
    }
}

fn append_ready_get_set_go(timeline: &mut AudioTimeline, event: &Value, clock: &BeatClock) {
    let phrase = value_as_string(event.get("phraseToSay"), "SayReaDyGetSetGoNew");
    let voice = value_as_string(event.get("voiceSource"), "Nurse");
    let tick = value_as_f64(event.get("tick"), 1.0).max(0.0);
    let volume = (value_as_f64(event.get("volume"), 100.0) / 100.0) as f32;
    let base = event_abs(event, clock);
    let words = phrase_words(&phrase);
    let words = if words.is_empty() {
        vec![phrase.as_str()]
    } else {
        words
    };
    let mut index = 0usize;
    for word in words {
        if phrase == "SayReadyGetSetGo" && index == 1 {
            index += 1;
        }
        let beat = base + tick * index as f64;
        append_ready_word(
            timeline,
            clock,
            beat,
            word,
            &voice,
            volume,
            event_bar(event) as usize,
        );
        index += 1;
    }
}

fn append_ready_word(
    timeline: &mut AudioTimeline,
    clock: &BeatClock,
    beat: f64,
    word: &str,
    voice: &str,
    volume: f32,
    source_floor: usize,
) {
    let click = match word {
        "JustSayRea" | "JustSayDy" | "JustSayReady" => Some("sndClickLow"),
        "JustSayGet" | "JustSaySet" => Some("sndClickMid"),
        "JustSayGo" | "JustSayStop" => Some("sndClickHi"),
        _ => None,
    };
    if let Some(click) = click {
        append_builtin(
            &mut timeline.play_sound_events,
            clock.time_at(beat),
            None,
            click,
            1.0,
            1.0,
            source_floor,
            "rd-rdgs-click",
        );
    }
    if let Some(sound) = ready_voice_sound(word, voice) {
        append_builtin(
            &mut timeline.play_sound_events,
            clock.time_at(beat),
            None,
            &sound,
            volume,
            1.0,
            source_floor,
            "rd-rdgs-voice",
        );
    }
}

fn append_narration_hint(timeline: &mut AudioTimeline, event: &Value, clock: &BeatClock) {
    let sound = match value_as_string(event.get("type"), "").as_str() {
        "NarrateRowInfo" => Some("sndPatientUpdate"),
        "ReadNarration" => Some("sndSpeechGeneric"),
        _ => None,
    };
    if let Some(sound) = sound {
        append_builtin(
            &mut timeline.play_sound_events,
            clock.time_at(event_abs(event, clock)),
            None,
            sound,
            0.8,
            1.0,
            event_bar(event) as usize,
            "rd-narration",
        );
    }
}

fn apply_clap_sounds(state: &mut TimelineState, event: &Value) {
    let row_type = value_as_string(event.get("rowType"), "Classic");
    let oneshot = row_type == "Oneshot";
    for (key, classic_type, oneshot_type) in [
        ("p1Sound", "ClapSoundP1Classic", "ClapSoundP1Oneshot"),
        ("p2Sound", "ClapSoundP2Classic", "ClapSoundP2Oneshot"),
        ("cpuSound", "ClapSoundCPUClassic", "ClapSoundCPUOneshot"),
    ] {
        if event.get(key).is_some() {
            let sound = sound_ref_at(event, key, "");
            if sound.used {
                state.game_sounds.insert(
                    (if oneshot { oneshot_type } else { classic_type }).to_string(),
                    sound,
                );
            }
        }
    }
}

fn apply_game_sound(state: &mut TimelineState, event: &Value) {
    let sound_type = value_as_string(event.get("soundType"), "SmallMistake");
    if let Some(sounds) = event.get("sounds").and_then(Value::as_array) {
        let group = game_sound_group(&sound_type);
        if sounds.len() == group.len() {
            for (sound, subtype) in sounds.iter().zip(group.iter()) {
                let decoded = super::parser::decode_sound_ref(Some(sound), "");
                if decoded.used {
                    state.game_sounds.insert((*subtype).to_string(), decoded);
                }
            }
        } else if let Some(first) = sounds.first() {
            let decoded = super::parser::decode_sound_ref(Some(first), "");
            if decoded.used {
                state.game_sounds.insert(sound_type, decoded);
            }
        }
        return;
    }
    let sound = if event.get("sound").is_some() {
        sound_ref_at(event, "sound", "")
    } else if event.get("filename").is_some() {
        super::parser::decode_sound_ref(Some(event), "")
    } else {
        SoundRef::new("")
    };
    if !sound.filename.is_empty() {
        state.game_sounds.insert(sound_type, sound);
    }
}

fn apply_make_row(state: &mut TimelineState, event: &Value) {
    let row_id = value_as_i64(event.get("row"), 0);
    let row = state.row_mut(row_id);
    row.row_type = if value_as_string(event.get("rowType"), "Classic") == "Oneshot" {
        RowKind::Oneshot
    } else {
        RowKind::Classic
    };
    row.player = row_player_from_event(event);
    row.muted =
        value_as_bool(event.get("muteBeats"), false) || value_as_bool(event.get("muteIn1P"), false);
    row.set_pulse_sounds(sound_ref_from_keys(
        event,
        "pulseSound",
        "pulseSoundVolume",
        "pulseSoundPitch",
        "pulseSoundOffset",
        "Shaker",
    ));
    if row.counting_sounds.is_empty() {
        row.set_counting_sounds("JyiCount", 1.0);
    }
}

fn row_player_from_event(event: &Value) -> RowPlayer {
    match value_as_string(event.get("player"), "P1").as_str() {
        "P2" => RowPlayer::P2,
        "CPU" => RowPlayer::Cpu,
        _ => RowPlayer::P1,
    }
}

fn clap_sound_type(row: &RowState, oneshot: bool) -> &'static str {
    match (oneshot, row.player) {
        (true, RowPlayer::P1) => "ClapSoundP1Oneshot",
        (true, RowPlayer::P2) => "ClapSoundP2Oneshot",
        (true, RowPlayer::Cpu) => "ClapSoundCPUOneshot",
        (false, RowPlayer::P1) => "ClapSoundP1Classic",
        (false, RowPlayer::P2) => "ClapSoundP2Classic",
        (false, RowPlayer::Cpu) => "ClapSoundCPUClassic",
    }
}

fn apply_counting_sound(state: &mut TimelineState, event: &Value) {
    let row = state.row_mut(value_as_i64(event.get("row"), 0));
    row.counting_enabled = value_as_bool(event.get("enabled"), true);
    row.counting_subdiv_offset = value_as_f64(event.get("subdivOffset"), 0.5);
    if !row.counting_enabled {
        return;
    }
    let volume = (value_as_f64(event.get("volume"), 100.0) / 100.0) as f32;
    if value_as_string(event.get("voiceSource"), "JyiCount") == "Custom" {
        if let Some(sounds) = event.get("sounds").and_then(Value::as_array) {
            row.counting_sounds = sounds
                .iter()
                .map(|sound| {
                    let mut decoded = super::parser::decode_sound_ref(Some(sound), "");
                    decoded.volume *= volume;
                    decoded
                })
                .collect();
        }
    } else {
        let voice = value_as_string(event.get("voiceSource"), "JyiCount");
        row.set_counting_sounds(&voice, volume);
    }
}

fn apply_row_xs(state: &mut TimelineState, event: &Value) {
    let row = state.row_mut(value_as_i64(event.get("row"), 0));
    row.show = show_beats_from_pattern(&value_as_string(event.get("pattern"), "------"));
    row.synco_beat = value_as_i64(event.get("syncoBeat"), -1) as i32;
    row.synco_swing = value_as_f64(event.get("syncoSwing"), 0.0).clamp(0.0, 1.0);
    row.synco_volume = (value_as_f64(event.get("syncoVolume"), 70.0) / 100.0) as f32;
    row.synco_pitch = (value_as_f64(event.get("syncoPitch"), 100.0) / 100.0) as f32;
    row.synco_style = value_as_string(event.get("syncoStyle"), "Chirp");
}

fn append_game_sound_to(
    events: &mut Vec<HitEvent>,
    state: &TimelineState,
    sound_type: &str,
    time_sec: f64,
    end_time_sec: Option<f64>,
    level_path: &Path,
    source_floor: usize,
    kind: &str,
) {
    let sound = state
        .game_sounds
        .get(sound_type)
        .cloned()
        .unwrap_or_else(|| SoundRef::new(sound_type));
    append_sound_ref(
        events,
        time_sec,
        end_time_sec,
        &sound,
        level_path,
        source_floor,
        kind,
    );
}

fn append_game_sound(
    timeline: &mut AudioTimeline,
    clock: &BeatClock,
    beat: f64,
    sound_type: &str,
    state: &TimelineState,
    level_path: &Path,
    source_floor: usize,
    kind: &str,
) {
    let sound = state
        .game_sounds
        .get(sound_type)
        .cloned()
        .unwrap_or_else(|| SoundRef::new(sound_type));
    append_sound_ref(
        &mut timeline.play_sound_events,
        clock.time_at(beat),
        None,
        &sound,
        level_path,
        source_floor,
        kind,
    );
}

fn first_play_song<'a>(root: &'a Value, condition_context: &ConditionContext) -> Option<&'a Value> {
    array_at(root, "events")
        .into_iter()
        .find(|event| condition_context.event_runs(event) && event_type(event) == Some("PlaySong"))
}

fn last_event_time(root: &Value, clock: &BeatClock, condition_context: &ConditionContext) -> f64 {
    array_at(root, "events")
        .into_iter()
        .filter(|event| condition_context.event_runs(event))
        .map(|event| clock.time_at(event_abs(event, clock)))
        .fold(0.0, f64::max)
}

fn event_sort_key(event: &Value) -> (i32, i64, i32) {
    (
        event_bar(event),
        (event_beat(event) * 10000.0).round() as i64,
        event_sort_order(event),
    )
}

fn event_abs(event: &Value, clock: &BeatClock) -> f64 {
    bar_beat_to_abs(event_bar(event), event_beat(event), &clock.cpb_changes)
}

fn bar_beat_to_abs(bar: i32, beat: f64, cpb_changes: &[CpbChange]) -> f64 {
    let target_bar = bar.max(1);
    let mut total = 0.0;
    for current_bar in 1..target_bar {
        total += cpb_for_bar(current_bar, cpb_changes) as f64;
    }
    total + (beat - 1.0).max(0.0)
}

fn cpb_for_bar(bar: i32, cpb_changes: &[CpbChange]) -> i32 {
    let mut cpb = 8;
    for change in cpb_changes {
        if change.bar > bar {
            break;
        }
        cpb = change.cpb;
    }
    cpb
}

impl BeatClock {
    fn time_at(&self, beat: f64) -> f64 {
        if self.bpm_changes.is_empty() {
            return beat.max(0.0) * 0.6;
        }
        let target = beat.max(0.0);
        let mut time = 0.0;
        let mut previous_beat = 0.0;
        let mut bpm = self.bpm_changes[0].bpm.max(1.0);
        for change in self.bpm_changes.iter().skip(1) {
            if change.beat >= target {
                break;
            }
            time += (change.beat - previous_beat).max(0.0) * 60.0 / bpm;
            previous_beat = change.beat;
            bpm = change.bpm.max(1.0);
        }
        time + (target - previous_beat).max(0.0) * 60.0 / bpm
    }

    fn duration_between(&self, start_beat: f64, duration_beats: f64) -> f64 {
        (self.time_at(start_beat + duration_beats) - self.time_at(start_beat)).max(0.0)
    }
}

impl TimelineState {
    fn row(&self, row_id: i64) -> RowState {
        self.rows.get(&row_id).cloned().unwrap_or_default()
    }

    fn row_mut(&mut self, row_id: i64) -> &mut RowState {
        self.rows.entry(row_id).or_default()
    }
}

fn append_sound_ref(
    events: &mut Vec<HitEvent>,
    time_sec: f64,
    end_time_sec: Option<f64>,
    sound: &SoundRef,
    level_path: &Path,
    source_floor: usize,
    kind: &str,
) {
    if !sound.used || sound.filename.trim().is_empty() || sound.filename == "None" {
        return;
    }
    let filename = sound.filename.trim();
    let pitch = sound.pitch.max(0.05);
    let mut volume = sound.volume;
    let mut offset_sec = sound.offset_ms / 1000.0;
    let sound_name = if let Some(path) = super::parser::resolve_sibling_audio(level_path, filename)
    {
        path.to_string_lossy().to_string()
    } else {
        let builtin = builtin_sound_name(filename);
        if offset_sec == 0.0 {
            offset_sec = sound_offset(&builtin);
        }
        volume *= sound_volume(&builtin);
        format!("rd:{builtin}")
    };
    events.push(HitEvent {
        time_sec: (time_sec - offset_sec / pitch as f64).max(0.0),
        end_time_sec,
        sound_name,
        volume,
        pitch,
        source_floor,
        kind: kind.to_string(),
    });
}

fn append_builtin(
    events: &mut Vec<HitEvent>,
    time_sec: f64,
    end_time_sec: Option<f64>,
    sound_name: &str,
    volume: f32,
    pitch: f32,
    source_floor: usize,
    kind: &str,
) {
    append_sound_ref(
        events,
        time_sec,
        end_time_sec,
        &SoundRef {
            filename: sound_name.trim().to_string(),
            volume,
            pitch,
            offset_ms: 0.0,
            used: true,
        },
        Path::new(""),
        source_floor,
        kind,
    );
}

fn sound_ref_from_keys(
    event: &Value,
    filename_key: &str,
    volume_key: &str,
    pitch_key: &str,
    offset_key: &str,
    fallback: &str,
) -> SoundRef {
    let mut sound = SoundRef::new(&value_as_string(event.get(filename_key), fallback));
    sound.volume = (value_as_f64(event.get(volume_key), 100.0) / 100.0) as f32;
    sound.pitch = (value_as_f64(event.get(pitch_key), 100.0) / 100.0) as f32;
    sound.offset_ms = value_as_f64(event.get(offset_key), 0.0);
    sound
}

fn show_beats_from_pattern(pattern: &str) -> [bool; 6] {
    let mut output = [true; 6];
    for (index, ch) in pattern.chars().take(6).enumerate() {
        output[index] = !matches!(ch, 'x' | 'X');
    }
    output
}

fn rd_audio_metadata() -> &'static RdAudioMetadata {
    RD_AUDIO_METADATA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../assets/rhythm-doctor-audio-metadata.json"
        ))
        .unwrap_or_else(|_| RdAudioMetadata {
            sound_offsets: HashMap::new(),
            game_sounds: HashMap::new(),
        })
    })
}

fn sound_offset(sound_name: &str) -> f64 {
    rd_audio_metadata()
        .sound_offsets
        .get(sound_name)
        .map(|meta| meta.offset_ms / 1000.0)
        .unwrap_or(0.0)
}

fn sound_volume(sound_name: &str) -> f32 {
    rd_audio_metadata()
        .sound_offsets
        .get(sound_name)
        .map(|meta| meta.volume)
        .unwrap_or(1.0)
}

fn game_sound_offset(state: &TimelineState, sound_type: &str) -> f64 {
    state
        .game_sounds
        .get(sound_type)
        .map(|sound| {
            sound_offset(&builtin_sound_name(&sound.filename)) / sound.pitch.max(0.05) as f64
        })
        .unwrap_or(0.0)
}

fn game_sound_group(sound_type: &str) -> &'static [&'static str] {
    match sound_type {
        "ClapSoundHold" => &[
            "ClapSoundHoldLongEnd",
            "ClapSoundHoldLongStart",
            "ClapSoundHoldShortEnd",
            "ClapSoundHoldShortStart",
        ],
        "PulseSoundHold" => &[
            "PulseSoundHoldStart",
            "PulseSoundHoldShortEnd",
            "PulseSoundHoldEnd",
            "PulseSoundHoldStartAlt",
            "PulseSoundHoldShortEndAlt",
            "PulseSoundHoldEndAlt",
        ],
        "ClapSoundHoldP2" => &[
            "ClapSoundHoldLongEndP2",
            "ClapSoundHoldLongStartP2",
            "ClapSoundHoldShortEndP2",
            "ClapSoundHoldShortStartP2",
        ],
        "PulseSoundHoldP2" => &[
            "PulseSoundHoldStartP2",
            "PulseSoundHoldShortEndP2",
            "PulseSoundHoldEndP2",
            "PulseSoundHoldStartAltP2",
            "PulseSoundHoldShortEndAltP2",
            "PulseSoundHoldEndAltP2",
        ],
        "FreezeshotSound" => &[
            "FreezeshotSoundCueLow",
            "FreezeshotSoundCueHigh",
            "FreezeshotSoundRiser",
            "FreezeshotSoundCymbal",
        ],
        "BurnshotSound" => &[
            "BurnshotSoundCueLow",
            "BurnshotSoundCueHigh",
            "BurnshotSoundRiser",
            "BurnshotSoundCymbal",
        ],
        "HoldshotSound" => &[
            "HoldshotSoundCue",
            "HoldshotSoundClapStart",
            "HoldshotSoundClapShortEnd",
            "HoldshotSoundClapLongEnd",
        ],
        _ => &[],
    }
}

fn builtin_sound_name(filename: &str) -> String {
    let stem = filename_stem(filename);
    if stem.starts_with("snd") {
        stem
    } else {
        format!("snd{stem}")
    }
}

fn phrase_words(phrase: &str) -> Vec<&'static str> {
    match phrase {
        "SayReaDyGetSetGoNew" => vec![
            "JustSayRea",
            "JustSayDy",
            "JustSayGet",
            "JustSaySet",
            "JustSayGo",
        ],
        "SayReadyGetSetGo" => vec!["JustSayReady", "JustSayGet", "JustSaySet", "JustSayGo"],
        "SayReaDyGetSetOne" => vec![
            "JustSayRea",
            "JustSayDy",
            "JustSayGet",
            "JustSaySet",
            "Count1",
        ],
        "SayGetSetOne" => vec!["JustSayGet", "JustSaySet", "Count1"],
        "SayGetSetGo" => vec!["JustSayGet", "JustSaySet", "JustSayGo"],
        _ => Vec::new(),
    }
}

fn ready_voice_sound(word: &str, voice: &str) -> Option<String> {
    let raw = match word {
        "JustSayReady" => "Ready",
        "JustSayRea" => "Rea",
        "JustSayDy" => "Dy",
        "JustSayGet" => "Get",
        "JustSaySet" => "Set",
        "JustSayGo" => "Go",
        "JustSayAnd" => "And",
        "JustSayStop" => "Stop",
        "JustSayAndStop" => "AndStop",
        "SaySwitch" => "Switch",
        "SayWatch" => "Watch",
        "SayListen" => "Listen",
        value if value.starts_with("Count") => value.trim_start_matches("Count"),
        _ => return None,
    };
    let prefix = ready_voice_prefix(voice, raw);
    let midfix = if word.starts_with("Count") {
        "CountEnglish"
    } else if matches!(raw, "And" | "Stop" | "AndStop") {
        "AndStop_"
    } else if matches!(raw, "Switch" | "Watch" | "Listen") {
        ""
    } else {
        "RDGS_"
    };
    Some(format!("{prefix}{midfix}{raw}"))
}

fn ready_voice_prefix(voice: &str, word: &str) -> &'static str {
    match voice {
        "NurseTired"
            if matches!(
                word,
                "Rea"
                    | "Dy"
                    | "Get"
                    | "Set"
                    | "Go"
                    | "AndStop"
                    | "And"
                    | "Stop"
                    | "Switch"
                    | "Watch"
                    | "Listen"
            ) =>
        {
            "sndJyiTired - "
        }
        "NurseSwing" if matches!(word, "Rea" | "Dy" | "Get" | "Set" | "Go") => "sndJyiSwing - ",
        "NurseSwingCalm" if matches!(word, "Rea" | "Dy" | "Get" | "Set" | "Go") => {
            "sndJyiSwingCalm - "
        }
        "IanExcited" => "sndIan - ",
        "IanCalm" if !matches!(word, "AndStop" | "And" | "Stop" | "Watch" | "Listen") => {
            "sndIanCalm - "
        }
        "IanSlow"
            if !matches!(
                word,
                "AndStop" | "And" | "Stop" | "Switch" | "Watch" | "Listen"
            ) =>
        {
            "sndIanSlow - "
        }
        "IanCalm" | "IanSlow" => "sndIan - ",
        _ => "sndJyi - ",
    }
}

fn counting_voice_prefix(voice: &str) -> &'static str {
    match voice {
        "BirdCount" => "sndBird",
        "CanaryCount" => "sndCanary",
        "IanCount" => "sndIan - Count",
        "IanCountCalm" => "sndIanCalm - Count",
        "IanCountFast" => "sndIan - ChineseCountFast",
        "IanCountSlow" => "sndIanSlow - Count",
        "IanCountSlower" => "sndIan - CountSlower",
        "IanCountEnglish" => "sndIan - CountEnglish",
        "IanCountEnglishFast" => "sndIan - CountEnglish",
        "IanCountEnglishCalm" => "sndIanCalm - CountEnglish",
        "IanCountEnglishSlow" => "sndIanSlow - CountEnglish",
        "JyiCount" => "sndJyi - ChineseCount",
        "JyiCountEnglish" => "sndJyi - CountEnglish",
        "JyiCountCalm" => "sndJyi - ChineseCountCalm",
        "JyiCountFast" => "sndJyi - ChineseCountFast",
        "JyiCountJapanese" => "sndJyi - JapaneseCount",
        "JyiCountLegacy" => "sndJyi - ChineseCountLegacy",
        "JyiCountTired" => "sndJyi - ChineseCountTired",
        "JyiCountVeryTired" => "sndJyi - ChineseCountVTired",
        "OrioleCount" => "sndOriole",
        "OwlCount" => "sndOwl",
        "ParrotCount" => "sndParrot",
        "SpearCount" => "sndSpear",
        "WhistleCount" => "sndWhistle",
        "WrenCount" => "sndWren",
        _ => "sndJyi - CountEnglish",
    }
}

fn default_game_sounds() -> BTreeMap<String, SoundRef> {
    rd_audio_metadata()
        .game_sounds
        .iter()
        .map(|(key, data)| {
            let mut sound = SoundRef::new(&data.filename);
            sound.volume = data.volume;
            sound.pitch = data.max_pitch;
            (key.clone(), sound)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn converts_bar_and_beat_with_cpb_changes() {
        let changes = vec![CpbChange { bar: 1, cpb: 8 }, CpbChange { bar: 3, cpb: 6 }];
        assert_eq!(bar_beat_to_abs(1, 1.0, &changes), 0.0);
        assert_eq!(bar_beat_to_abs(2, 1.0, &changes), 8.0);
        assert_eq!(bar_beat_to_abs(3, 1.0, &changes), 16.0);
        assert_eq!(bar_beat_to_abs(4, 1.0, &changes), 22.0);
    }

    #[test]
    fn builds_play_sound_timeline() {
        let root = json!({
            "events": [
                { "type": "PlaySong", "bar": 1, "beat": 1, "song": { "filename": "sndOrientalTechno" }, "beatsPerMinute": 120 },
                { "type": "PlaySound", "bar": 1, "beat": 3, "sound": { "filename": "Shaker", "volume": 50 } }
            ]
        });
        let timeline = build_timeline_from_root(&root, Path::new("test.rdlevel"), true).unwrap();
        assert_eq!(timeline.play_sound_events.len(), 1);
        assert!((timeline.play_sound_events[0].time_sec - 0.985).abs() < 0.001);
        assert_eq!(timeline.play_sound_events[0].sound_name, "rd:sndShaker");
    }

    #[test]
    fn inactive_play_sound_is_not_added() {
        let root = json!({
            "events": [
                { "type": "PlaySong", "bar": 1, "beat": 1, "song": { "filename": "music.ogg" }, "bpm": 120 },
                { "type": "PlaySound", "bar": 1, "beat": 2, "active": false, "sound": { "filename": "Muted" } },
                { "type": "PlaySound", "bar": 1, "beat": 3, "sound": { "filename": "Live" } }
            ]
        });
        let timeline = build_timeline_from_root(&root, Path::new("test.rdlevel"), true).unwrap();
        assert!(timeline
            .play_sound_events
            .iter()
            .all(|event| event.sound_name != "rd:sndMuted"));
        assert!(timeline
            .play_sound_events
            .iter()
            .any(|event| event.sound_name == "rd:sndLive"));
    }

    #[test]
    fn single_player_filters_two_player_branch_sounds() {
        let root = json!({
            "conditionals": [
                { "type": "PlayerMode", "id": 1, "tag": "1", "twoPlayerMode": true }
            ],
            "rows": [
                { "row": 1, "rowType": "Oneshot", "player": "P1", "pulseSound": "Kick" }
            ],
            "events": [
                { "type": "PlaySong", "bar": 1, "beat": 1, "song": { "filename": "music.ogg" }, "bpm": 120 },
                { "type": "SetClapSounds", "bar": 1, "beat": 1, "if": "1d0", "rowType": "Oneshot", "p1Sound": { "filename": "TwoPlayer" } },
                { "type": "SetClapSounds", "bar": 1, "beat": 1, "if": "~1d0", "rowType": "Oneshot", "p1Sound": { "filename": "SinglePlayer" } },
                { "type": "AddOneshotBeat", "bar": 1, "beat": 2, "row": 1, "pulseType": "Wave", "tick": 2 }
            ]
        });
        let timeline = build_timeline_from_root(&root, Path::new("test.rdlevel"), true).unwrap();
        assert!(timeline.hit_events.iter().any(
            |event| event.kind == "rd-oneshot-clap" && event.sound_name == "rd:sndSinglePlayer"
        ));
        assert!(!timeline
            .hit_events
            .iter()
            .any(|event| event.sound_name == "rd:sndTwoPlayer"));
    }

    #[test]
    fn tagged_events_do_not_run_without_run_tag() {
        let root = json!({
            "events": [
                { "type": "PlaySong", "bar": 1, "beat": 1, "song": { "filename": "music.ogg" }, "bpm": 120 },
                { "type": "PlaySound", "bar": 1, "beat": 2, "tag": "Later", "sound": { "filename": "Blocked" } },
                { "type": "PlaySound", "bar": 1, "beat": 3, "tag": "Now", "runTag": true, "sound": { "filename": "Allowed" } }
            ]
        });
        let timeline = build_timeline_from_root(&root, Path::new("test.rdlevel"), true).unwrap();
        assert!(!timeline
            .play_sound_events
            .iter()
            .any(|event| event.sound_name == "rd:sndBlocked"));
        assert!(timeline
            .play_sound_events
            .iter()
            .any(|event| event.sound_name == "rd:sndAllowed"));
    }

    #[test]
    fn classic_hold_uses_official_hold_sounds_per_pulse() {
        let root = json!({
            "events": [
                { "type": "PlaySong", "bar": 1, "beat": 1, "song": { "filename": "music.ogg" }, "bpm": 120 },
                { "type": "AddClassicBeat", "bar": 1, "beat": 1, "row": 0, "tick": 1, "hold": 1, "length": 7 }
            ]
        });
        let timeline = build_timeline_from_root(&root, Path::new("test.rdlevel"), true).unwrap();
        assert!(timeline
            .hold_sound_events
            .iter()
            .any(|event| event.sound_name == "rd:sndHoldStartTamb"));
        assert!(timeline
            .hold_sound_events
            .iter()
            .any(|event| event.sound_name == "rd:sndHoldWindupLongStart"));
    }

    #[test]
    fn oneshot_without_custom_sound_uses_row_sound() {
        let root = json!({
            "rows": [
                { "row": 1, "rowType": "Oneshot", "pulseSound": "KickChroma" }
            ],
            "events": [
                { "type": "PlaySong", "bar": 1, "beat": 1, "song": { "filename": "music.ogg" }, "bpm": 120 },
                { "type": "AddOneshotBeat", "bar": 1, "beat": 1, "row": 1, "pulseType": "Wave", "tick": 2 }
            ]
        });
        let timeline = build_timeline_from_root(&root, Path::new("test.rdlevel"), true).unwrap();
        assert!(
            timeline
                .hit_events
                .iter()
                .any(|event| event.kind == "rd-oneshot-boom"
                    && event.sound_name == "rd:sndKickChroma")
        );
        assert!(!timeline
            .hit_events
            .iter()
            .any(|event| event.kind == "rd-oneshot-boom" && event.sound_name == "rd:sndShaker"));
    }

    #[test]
    fn p2_oneshot_clap_can_use_custom_level_audio() {
        let dir =
            std::env::temp_dir().join(format!("adofai_music_box_rd_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sound_path = dir.join("hit.ogg");
        std::fs::write(&sound_path, b"placeholder").unwrap();
        let level_path = dir.join("level.rdlevel");
        let root = json!({
            "rows": [
                { "row": 2, "rowType": "Oneshot", "player": "P2", "pulseSound": "Kick" }
            ],
            "events": [
                { "type": "PlaySong", "bar": 1, "beat": 1, "song": { "filename": "music.ogg" }, "bpm": 120 },
                { "type": "SetClapSounds", "bar": 1, "beat": 1, "rowType": "Oneshot", "p2Sound": { "filename": "hit" } },
                { "type": "AddOneshotBeat", "bar": 1, "beat": 1, "row": 2, "pulseType": "Wave", "tick": 2 }
            ]
        });
        let timeline = build_timeline_from_root(&root, &level_path, true).unwrap();
        assert!(timeline.hit_events.iter().any(|event| {
            event.kind == "rd-oneshot-clap" && event.sound_name == sound_path.to_string_lossy()
        }));
        let _ = std::fs::remove_dir_all(dir);
    }
}

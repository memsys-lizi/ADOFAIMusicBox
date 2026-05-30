use super::parser::{
    array_at, event_bar, event_beat, event_is_active, event_sort_order, event_type, filename_stem,
    parse_level_file, sound_ref_at, value_as_bool, value_as_f64, value_as_i64, value_as_string,
    SoundRef,
};
use crate::library::{AudioTimeline, HitEvent};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

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
    row_beat_sounds: HashMap<i64, SoundRef>,
    row_patterns: HashMap<i64, String>,
    classic_clap: SoundRef,
    oneshot_clap: SoundRef,
    counting_voice: HashMap<i64, String>,
    game_sounds: BTreeMap<String, SoundRef>,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            row_beat_sounds: HashMap::new(),
            row_patterns: HashMap::new(),
            classic_clap: SoundRef::new("ClapHit"),
            oneshot_clap: SoundRef::new("ClapHit"),
            counting_voice: HashMap::new(),
            game_sounds: default_game_sounds(),
        }
    }
}

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
    let cpb_changes = collect_cpb_changes(root);
    let bpm_changes = collect_bpm_changes(root, &cpb_changes);
    let clock = BeatClock {
        cpb_changes,
        bpm_changes,
    };
    let song_ref = first_play_song(root)
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
        append_events(&mut timeline, root, level_path, &clock);
    }

    let max_event_time = timeline
        .hit_events
        .iter()
        .chain(timeline.play_sound_events.iter())
        .chain(timeline.hold_sound_events.iter())
        .map(|event| event.end_time_sec.unwrap_or(event.time_sec))
        .fold(0.0, f64::max);
    timeline.duration = max_event_time.max(last_event_time(root, &clock) + 4.0);
    Ok(timeline)
}

fn append_events(timeline: &mut AudioTimeline, root: &Value, level_path: &Path, clock: &BeatClock) {
    let mut events = array_at(root, "events");
    events.sort_by(|left, right| event_sort_key(left).cmp(&event_sort_key(right)));
    let mut state = TimelineState::default();
    let mut warned_conditions = false;

    for event in events {
        if !event_is_active(event) {
            continue;
        }
        if event.get("if").is_some() || event.get("tag").is_some() || event.get("runTag").is_some()
        {
            warned_conditions = true;
        }
        match event_type(event) {
            Some("SetBeatSound") => {
                let row = value_as_i64(event.get("row"), 0);
                state
                    .row_beat_sounds
                    .insert(row, sound_ref_at(event, "sound", "Shaker"));
            }
            Some("SetClapSounds") => apply_clap_sounds(&mut state, event),
            Some("SetGameSound") => apply_game_sound(&mut state, event),
            Some("SetCountingSound") => {
                let row = value_as_i64(event.get("row"), 0);
                state
                    .counting_voice
                    .insert(row, value_as_string(event.get("voiceSource"), "JyiCount"));
            }
            Some("SetRowXs") => {
                let row = value_as_i64(event.get("row"), 0);
                state
                    .row_patterns
                    .insert(row, value_as_string(event.get("pattern"), "------"));
            }
            Some("PlaySound") => append_play_sound(timeline, event, level_path, clock),
            Some("AddClassicBeat") => {
                append_classic_beat(timeline, event, level_path, clock, &state)
            }
            Some("AddFreeTimeBeat") => {
                append_free_time_beat(timeline, event, level_path, clock, &state)
            }
            Some("PulseFreeTimeBeat") => {
                append_free_time_pulse(timeline, event, level_path, clock, &state)
            }
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

    if warned_conditions {
        timeline
            .warnings
            .push("这个 RD 谱面包含条件或标签事件，播放器按静态时间线处理。".to_string());
    }
}

fn collect_cpb_changes(root: &Value) -> Vec<CpbChange> {
    let mut changes = vec![CpbChange { bar: 1, cpb: 8 }];
    for event in array_at(root, "events") {
        if !event_is_active(event) || event_type(event) != Some("SetCrotchetsPerBar") {
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

fn collect_bpm_changes(root: &Value, cpb_changes: &[CpbChange]) -> Vec<BpmChange> {
    let base_bpm = first_play_song(root)
        .map(|event| value_as_f64(event.get("beatsPerMinute"), 100.0))
        .unwrap_or(100.0)
        .max(1.0);
    let mut changes = vec![BpmChange {
        beat: 0.0,
        bpm: base_bpm,
    }];
    for event in array_at(root, "events") {
        if !event_is_active(event) {
            continue;
        }
        if event_type(event) == Some("SetBeatsPerMinute") {
            changes.push(BpmChange {
                beat: bar_beat_to_abs(event_bar(event), event_beat(event), cpb_changes),
                bpm: value_as_f64(event.get("beatsPerMinute"), base_bpm).max(1.0),
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
        clock.time_at(event_abs(event, clock) + sound.offset_ms / 1000.0),
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
    state: &TimelineState,
) {
    let row = value_as_i64(event.get("row"), 0);
    let tick = value_as_f64(event.get("tick"), 1.0).max(0.0);
    let swing = value_as_f64(event.get("swing"), 0.0).clamp(0.0, tick * 2.0);
    let hold = value_as_f64(event.get("hold"), 0.0).max(0.0);
    let length = value_as_i64(event.get("length"), 7).clamp(1, 7) as usize;
    let start = event_abs(event, clock);
    let sound = if event.get("sound").is_some() {
        sound_ref_at(event, "sound", "Shaker")
    } else {
        state
            .row_beat_sounds
            .get(&row)
            .cloned()
            .unwrap_or_else(|| SoundRef::new("Shaker"))
    };
    let pattern = state
        .row_patterns
        .get(&row)
        .map(String::as_str)
        .unwrap_or("------");

    if hold > 0.0 {
        let hold_start = start;
        let hold_end = start + hold;
        append_builtin(
            &mut timeline.hold_sound_events,
            clock.time_at(hold_start),
            None,
            "sndHoldStartTamb",
            1.0,
            1.0,
            event_bar(event) as usize,
            "rd-hold-start",
        );
        append_builtin(
            &mut timeline.hold_sound_events,
            clock.time_at(hold_start),
            Some(clock.time_at(hold_end)),
            "sndDrumrollBright",
            0.65,
            1.0,
            event_bar(event) as usize,
            "rd-hold-loop",
        );
        append_builtin(
            &mut timeline.hold_sound_events,
            clock.time_at(hold_end),
            None,
            "sndHoldEndTambShort",
            1.0,
            1.0,
            event_bar(event) as usize,
            "rd-hold-end",
        );
        append_sound_ref(
            &mut timeline.hit_events,
            clock.time_at(hold_end),
            None,
            &state.classic_clap,
            level_path,
            event_bar(event) as usize,
            "rd-classic-clap",
        );
        return;
    }

    for index in 0..length {
        if pattern_skip(pattern, index) {
            continue;
        }
        let beat = start + classic_offset(index, tick, swing, hold);
        if index + 1 == length {
            append_sound_ref(
                &mut timeline.hit_events,
                clock.time_at(beat),
                None,
                &state.classic_clap,
                level_path,
                event_bar(event) as usize,
                "rd-classic-clap",
            );
        } else {
            append_sound_ref(
                &mut timeline.hit_events,
                clock.time_at(beat),
                None,
                &sound,
                level_path,
                event_bar(event) as usize,
                "rd-classic-pulse",
            );
        }
    }
}

fn append_free_time_beat(
    timeline: &mut AudioTimeline,
    event: &Value,
    level_path: &Path,
    clock: &BeatClock,
    state: &TimelineState,
) {
    let row = value_as_i64(event.get("row"), 0);
    let sound = state
        .row_beat_sounds
        .get(&row)
        .cloned()
        .unwrap_or_else(|| SoundRef::new("Shaker"));
    append_sound_ref(
        &mut timeline.hit_events,
        clock.time_at(event_abs(event, clock)),
        None,
        &sound,
        level_path,
        event_bar(event) as usize,
        "rd-free-time",
    );
}

fn append_free_time_pulse(
    timeline: &mut AudioTimeline,
    event: &Value,
    level_path: &Path,
    clock: &BeatClock,
    state: &TimelineState,
) {
    let row = value_as_i64(event.get("row"), 0);
    let sound = state
        .row_beat_sounds
        .get(&row)
        .cloned()
        .unwrap_or_else(|| SoundRef::new("Shaker"));
    append_sound_ref(
        &mut timeline.hit_events,
        clock.time_at(event_abs(event, clock)),
        None,
        &sound,
        level_path,
        event_bar(event) as usize,
        "rd-free-time-pulse",
    );
}

fn append_oneshot_beat(
    timeline: &mut AudioTimeline,
    event: &Value,
    level_path: &Path,
    clock: &BeatClock,
    state: &TimelineState,
) {
    let row = value_as_i64(event.get("row"), 0);
    let pulse_type = value_as_string(event.get("pulseType"), "Wave");
    let freeze_burn = value_as_string(event.get("freezeBurnMode"), "None");
    let tick = value_as_f64(event.get("tick"), 1.0).max(0.0);
    let delay = if freeze_burn == "Freezeshot" {
        value_as_f64(event.get("delay"), 0.5).max(0.0)
    } else {
        0.0
    };
    let loops = value_as_i64(event.get("loops"), 0).max(0) as usize;
    let interval = if freeze_burn != "None"
        || loops > 0
        || value_as_bool(event.get("skipshot"), false)
        || value_as_bool(event.get("hold"), false)
    {
        value_as_f64(event.get("interval"), 2.0).max(0.0)
    } else {
        (2.0 * tick).max(0.0)
    };
    let freeze_offset = match freeze_burn.as_str() {
        "Freezeshot" => (interval - tick).max(0.0),
        "Burnshot" => interval.max(0.0),
        _ => 0.0,
    };
    let start = (event_abs(event, clock) - freeze_offset).max(0.0);
    let subdivisions = if pulse_type == "Square" {
        1
    } else if pulse_type == "Triangle" {
        value_as_i64(event.get("subdivisions"), 1).clamp(1, 10) as usize
    } else {
        0
    };
    let subdiv_sound = value_as_bool(event.get("subdivSound"), true);
    let custom_sound = if event.get("sound").is_some() {
        sound_ref_at(event, "sound", "Shaker")
    } else {
        SoundRef::new("Shaker")
    };

    for repeat in 0..=loops {
        let cue_abs = start + interval * repeat as f64;
        let hit_abs = cue_abs + freeze_offset + tick + delay;
        append_freeze_burn_cues(
            timeline,
            clock,
            cue_abs,
            hit_abs,
            &freeze_burn,
            event_bar(event) as usize,
        );
        append_counting_sound(
            timeline,
            clock,
            cue_abs,
            tick,
            subdivisions,
            row,
            state,
            event_bar(event) as usize,
        );

        if pulse_type == "Triangle" && subdivisions > 1 {
            let step = tick / subdivisions as f64;
            for index in 0..subdivisions {
                let beat = cue_abs + step * index as f64;
                let sound_name = if subdiv_sound {
                    format!("sndTriangleshot{}", index + 1)
                } else {
                    builtin_sound_name(&custom_sound.filename)
                };
                append_builtin(
                    &mut timeline.hit_events,
                    clock.time_at(beat),
                    None,
                    &sound_name,
                    custom_sound.volume,
                    custom_sound.pitch,
                    event_bar(event) as usize,
                    "rd-oneshot-subdivision",
                );
            }
        }

        let hit_sound = if pulse_type == "Square" && subdiv_sound {
            SoundRef::new("Squareshot")
        } else {
            custom_sound.clone()
        };
        append_sound_ref(
            &mut timeline.hit_events,
            clock.time_at(hit_abs),
            None,
            &hit_sound,
            level_path,
            event_bar(event) as usize,
            "rd-oneshot-hit",
        );

        append_sound_ref(
            &mut timeline.hit_events,
            clock.time_at(hit_abs),
            None,
            &state.oneshot_clap,
            level_path,
            event_bar(event) as usize,
            "rd-oneshot-clap",
        );

        if value_as_bool(event.get("skipshot"), false) {
            let skip_abs = cue_abs + interval;
            append_game_sound(
                timeline,
                clock,
                skip_abs,
                "Skipshot",
                state,
                event_bar(event) as usize,
                "rd-skipshot",
            );
        }

        if value_as_bool(event.get("hold"), false) {
            let end_abs = cue_abs + interval;
            append_game_sound(
                timeline,
                clock,
                hit_abs,
                "HoldshotSoundCue",
                state,
                event_bar(event) as usize,
                "rd-holdshot-cue",
            );
            append_builtin(
                &mut timeline.hold_sound_events,
                clock.time_at(hit_abs),
                Some(clock.time_at(end_abs)),
                "sndHoldStartTamb",
                0.8,
                1.0,
                event_bar(event) as usize,
                "rd-oneshot-hold",
            );
        }
    }
}

fn append_freeze_burn_cues(
    timeline: &mut AudioTimeline,
    clock: &BeatClock,
    cue_abs: f64,
    hit_abs: f64,
    mode: &str,
    source_floor: usize,
) {
    match mode {
        "Freezeshot" => {
            append_builtin(
                &mut timeline.play_sound_events,
                clock.time_at(cue_abs),
                None,
                "sndFreezeshotCueLow",
                1.0,
                1.0,
                source_floor,
                "rd-freezeshot-cue",
            );
            append_builtin(
                &mut timeline.play_sound_events,
                clock.time_at(hit_abs),
                None,
                "sndFreezeshotCymbal",
                1.0,
                1.0,
                source_floor,
                "rd-freezeshot-hit",
            );
        }
        "Burnshot" => {
            append_builtin(
                &mut timeline.play_sound_events,
                clock.time_at(cue_abs),
                None,
                "sndBurnshotCueLow",
                1.0,
                1.0,
                source_floor,
                "rd-burnshot-cue",
            );
            append_builtin(
                &mut timeline.play_sound_events,
                clock.time_at(hit_abs),
                None,
                "sndBurnshotCymbal",
                1.0,
                1.0,
                source_floor,
                "rd-burnshot-hit",
            );
        }
        _ => {}
    }
}

fn append_counting_sound(
    timeline: &mut AudioTimeline,
    clock: &BeatClock,
    cue_abs: f64,
    tick: f64,
    subdivisions: usize,
    row: i64,
    state: &TimelineState,
    source_floor: usize,
) {
    if subdivisions <= 1 {
        return;
    }
    let voice = state
        .counting_voice
        .get(&row)
        .map(String::as_str)
        .unwrap_or("JyiCountEnglish");
    let prefix = counting_voice_prefix(voice);
    let sound_name = format!("{prefix}{subdivisions}");
    append_builtin(
        &mut timeline.play_sound_events,
        clock.time_at((cue_abs - tick).max(0.0)),
        None,
        &sound_name,
        1.0,
        1.0,
        source_floor,
        "rd-counting",
    );
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
    let sound = sound_ref_at(event, "p1Sound", "ClapHit");
    if row_type == "Oneshot" {
        state.oneshot_clap = sound;
    } else {
        state.classic_clap = sound;
    }
}

fn apply_game_sound(state: &mut TimelineState, event: &Value) {
    let sound_type = value_as_string(event.get("soundType"), "SmallMistake");
    if let Some(sounds) = event.get("sounds").and_then(Value::as_array) {
        if let Some(first) = sounds.first() {
            state
                .game_sounds
                .insert(sound_type, super::parser::decode_sound_ref(Some(first), ""));
        }
        return;
    }
    let sound = sound_ref_at(event, "sound", "");
    if !sound.filename.is_empty() {
        state.game_sounds.insert(sound_type, sound);
    }
}

fn append_game_sound(
    timeline: &mut AudioTimeline,
    clock: &BeatClock,
    beat: f64,
    sound_type: &str,
    state: &TimelineState,
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
        Path::new(""),
        source_floor,
        kind,
    );
}

fn first_play_song(root: &Value) -> Option<&Value> {
    array_at(root, "events")
        .into_iter()
        .find(|event| event_is_active(event) && event_type(event) == Some("PlaySong"))
}

fn last_event_time(root: &Value, clock: &BeatClock) -> f64 {
    array_at(root, "events")
        .into_iter()
        .filter(|event| event_is_active(event))
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
    let sound_name = sound_name_for_ref(sound, level_path);
    events.push(HitEvent {
        time_sec: time_sec.max(0.0),
        end_time_sec,
        sound_name,
        volume: sound.volume,
        pitch: sound.pitch.max(0.05),
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
    events.push(HitEvent {
        time_sec: time_sec.max(0.0),
        end_time_sec,
        sound_name: format!("rd:{}", sound_name.trim()),
        volume,
        pitch: pitch.max(0.05),
        source_floor,
        kind: kind.to_string(),
    });
}

fn sound_name_for_ref(sound: &SoundRef, level_path: &Path) -> String {
    let filename = sound.filename.trim();
    if filename.contains('\\')
        || filename.contains('/')
        || super::parser::has_audio_extension(filename)
    {
        if let Some(path) =
            super::parser::resolve_sibling(level_path, Some(filename)).filter(|path| path.exists())
        {
            return path.to_string_lossy().to_string();
        }
    }
    format!("rd:{}", builtin_sound_name(filename))
}

fn builtin_sound_name(filename: &str) -> String {
    let stem = filename_stem(filename);
    if stem.starts_with("snd") {
        stem
    } else {
        format!("snd{stem}")
    }
}

fn classic_offset(index: usize, tick: f64, swing: f64, hold: f64) -> f64 {
    if swing != 0.0 && index % 2 == 1 && hold == 0.0 {
        tick * (index as f64 + 1.0) - swing
    } else {
        tick * index as f64
    }
}

fn pattern_skip(pattern: &str, index: usize) -> bool {
    let chars: Vec<char> = pattern.chars().collect();
    matches!(chars.get(index % chars.len().max(1)), Some('x') | Some('X'))
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
        "IanCount" => "sndIan - Count",
        "IanCountCalm" => "sndIanCalm - Count",
        "IanCountSlow" => "sndIanSlow - Count",
        "IanCountEnglish" => "sndIan - CountEnglish",
        "IanCountEnglishCalm" => "sndIanCalm - CountEnglish",
        "IanCountEnglishSlow" => "sndIanSlow - CountEnglish",
        "JyiCountEnglish" => "sndJyi - CountEnglish",
        "JyiCount" => "sndJyi - ChineseCount",
        "JyiCountCalm" => "sndJyi - ChineseCountCalm",
        "JyiCountFast" => "sndJyi - ChineseCountFast",
        _ => "sndJyi - CountEnglish",
    }
}

fn default_game_sounds() -> BTreeMap<String, SoundRef> {
    [
        ("Skipshot", "Vibraslap"),
        ("HoldshotSoundCue", "HoldshotCue"),
        ("FreezeshotSoundCueLow", "FreezeshotCueLow"),
        ("FreezeshotSoundCueHigh", "FreezeshotCueHigh"),
        ("FreezeshotSoundCymbal", "FreezeshotCymbal"),
        ("BurnshotSoundCueLow", "BurnshotCueLow"),
        ("BurnshotSoundCueHigh", "BurnshotCueHigh"),
        ("BurnshotSoundCymbal", "BurnshotCymbal"),
        ("SmallMistake", "MistakeSmall3"),
        ("BigMistake", "MistakeBig"),
    ]
    .into_iter()
    .map(|(key, sound)| (key.to_string(), SoundRef::new(sound)))
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
        assert!((timeline.play_sound_events[0].time_sec - 1.0).abs() < 0.001);
        assert_eq!(timeline.play_sound_events[0].sound_name, "rd:sndShaker");
    }
}

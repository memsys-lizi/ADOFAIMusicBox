use super::models::{AudioTimeline, HitEvent};
use super::parser::{
    array_at, event_floor, event_is_active, event_type, number_setting, parse_level_file,
    settings_map, value_as_f64, value_as_string,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;
use std::path::Path;

const TAU: f64 = PI * 2.0;

#[derive(Debug, Clone)]
struct Floor {
    seq_id: usize,
    entry_angle: f64,
    exit_angle: f64,
    is_ccw: bool,
    mid_spin: bool,
    speed: f64,
    extra_beats: f64,
    countdown_ticks: i32,
    hold_length: i32,
    num_planets: i32,
    taps_needed: i32,
    entry_time: f64,
    entry_time_pitch_adj: f64,
    set_hitsound: Option<SoundChange>,
    set_hold_sound: Option<HoldSoundChange>,
    freeroam_sound_on: Option<String>,
    freeroam_sound_off: Option<String>,
}

#[derive(Debug, Clone)]
struct SoundChange {
    game_sound: String,
    hitsound: String,
    volume: f32,
}

#[derive(Debug, Clone)]
struct HoldSoundChange {
    start: String,
    loop_sound: String,
    end: String,
    mid: String,
    mid_type: String,
    mid_delay_sec: f64,
    mid_timing: String,
    volume: f32,
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
    let settings = settings_map(root);
    let bpm = number_setting(&settings, "bpm", 100.0).max(1.0);
    let offset_ms = number_setting(&settings, "offset", 0.0);
    let pitch = (number_setting(&settings, "pitch", 100.0) / 100.0).max(0.01);
    let countdown_ticks = number_setting(&settings, "countdownTicks", 4.0);
    let countdown_speed_multiplier =
        number_setting(&settings, "countdownSpeedMultiplier", 1.0).max(0.01);
    let adjusted_countdown_ticks = countdown_ticks / countdown_speed_multiplier;
    let separate_countdown_time = bool_setting(&settings, "separateCountdownTime", true);
    let countdown_lead_in_sec = if separate_countdown_time {
        (60.0 / bpm) * adjusted_countdown_ticks
    } else {
        0.0
    };
    let song_start_delay = if separate_countdown_time {
        countdown_lead_in_sec / pitch
    } else {
        0.0
    };
    let default_hitsound = string_setting_or(&settings, "hitsound", "Kick");
    let hit_volume = (number_setting(&settings, "hitsoundVolume", 100.0) / 100.0) as f32;
    let mut warnings = Vec::new();

    let angles = read_angles(root, &mut warnings)?;
    let mut floors = instantiate_floors(&angles);
    apply_floor_events(root, &mut floors, bpm);
    if let Some(first) = floors.first_mut() {
        first.countdown_ticks = countdown_ticks as i32;
    }
    calculate_entry_times(&mut floors, bpm, pitch, adjusted_countdown_ticks);

    let mut timeline = AudioTimeline {
        song_offset_ms: offset_ms,
        countdown_lead_in_sec,
        pitch,
        duration: floors
            .last()
            .map(|floor| {
                event_time(
                    floor.entry_time_pitch_adj,
                    offset_ms,
                    pitch,
                    song_start_delay,
                )
            })
            .unwrap_or(0.0)
            .max(0.0),
        hit_events: Vec::new(),
        play_sound_events: Vec::new(),
        hold_sound_events: Vec::new(),
        warnings,
    };

    if include_events {
        append_hit_events(
            &mut timeline,
            root,
            level_path,
            &floors,
            bpm,
            pitch,
            song_start_delay,
            countdown_speed_multiplier,
            default_hitsound,
            hit_volume,
        );
    }

    Ok(timeline)
}

fn string_setting_or(settings: &BTreeMap<String, Value>, key: &str, fallback: &str) -> String {
    settings
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn bool_setting(settings: &BTreeMap<String, Value>, key: &str, fallback: bool) -> bool {
    settings
        .get(key)
        .and_then(|value| match value {
            Value::Bool(value) => Some(*value),
            Value::String(text) => text.parse::<bool>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn read_angles(root: &Value, warnings: &mut Vec<String>) -> Result<Vec<f64>, String> {
    if let Some(values) = root.get("angleData").and_then(Value::as_array) {
        return Ok(values
            .iter()
            .filter_map(|value| match value {
                Value::Number(number) => number.as_f64(),
                Value::String(text) => text.parse::<f64>().ok(),
                _ => None,
            })
            .collect());
    }

    if let Some(path_data) = root.get("pathData").and_then(Value::as_str) {
        warnings.push("谱面使用旧版 pathData，已转换为角度数据".to_string());
        return Ok(path_data_to_angles(path_data, warnings));
    }

    Err("谱面缺少 angleData/pathData".to_string())
}

fn instantiate_floors(angles: &[f64]) -> Vec<Floor> {
    let mut floors = Vec::with_capacity(angles.len() + 1);
    floors.push(Floor::new(0, 4.71238899230957));
    let mut previous = floors[0].clone();

    for (index, angle) in angles.iter().enumerate() {
        let exit_angle = if approx(*angle, 999.0) {
            previous.entry_angle
        } else {
            (-angle + 90.0).to_radians()
        };
        if let Some(floor) = floors.last_mut() {
            floor.exit_angle = exit_angle;
            if approx(*angle, 999.0) {
                floor.mid_spin = true;
            }
        }
        let mut next = Floor::new(index + 1, modulo(exit_angle + PI, TAU));
        next.exit_angle = next.entry_angle + PI;
        floors.push(next.clone());
        previous = next;
    }

    floors
}

impl Floor {
    fn new(seq_id: usize, entry_angle: f64) -> Self {
        Self {
            seq_id,
            entry_angle,
            exit_angle: entry_angle + PI,
            is_ccw: false,
            mid_spin: false,
            speed: 1.0,
            extra_beats: 0.0,
            countdown_ticks: 0,
            hold_length: -1,
            num_planets: 2,
            taps_needed: 1,
            entry_time: 0.0,
            entry_time_pitch_adj: 0.0,
            set_hitsound: None,
            set_hold_sound: None,
            freeroam_sound_on: None,
            freeroam_sound_off: None,
        }
    }
}

fn apply_floor_events(root: &Value, floors: &mut [Floor], base_bpm: f64) {
    let mut by_floor: BTreeMap<usize, Vec<&Value>> = BTreeMap::new();
    for event in array_at(root, "actions") {
        if !event_is_active(event) {
            continue;
        }
        by_floor.entry(event_floor(event)).or_default().push(event);
    }

    let mut is_ccw = false;
    let mut speed: f64 = 1.0;
    let mut planets = 2;
    let mut radius_carry_extra = 0.0;

    for index in 0..floors.len() {
        floors[index].extra_beats = 0.0;
        let events = by_floor
            .get(&floors[index].seq_id)
            .cloned()
            .unwrap_or_default();
        let set_speed_events: Vec<&Value> = events
            .iter()
            .copied()
            .filter(|event| event_type(event) == Some("SetSpeed"))
            .collect();

        for event in &events {
            match event_type(event) {
                Some("Twirl") => {
                    is_ccw = !is_ccw;
                }
                Some("Pause") => {
                    if index + 1 < floors.len() && !floors[index].mid_spin {
                        floors[index].extra_beats += value_as_f64(event.get("duration"), 0.0);
                        floors[index].countdown_ticks =
                            value_as_f64(event.get("countdownTicks"), 0.0) as i32;
                    }
                }
                Some("FreeRoam") => {
                    let duration = value_as_f64(event.get("duration"), 0.0);
                    if index + 1 < floors.len() && duration >= 2.0 {
                        let angle_beats = single_floor_angle_beats(
                            &floors[index],
                            is_ccw,
                            planets,
                            floors
                                .get(index.wrapping_sub(1))
                                .map(|floor| floor.mid_spin)
                                .unwrap_or(false),
                        );
                        floors[index].extra_beats += angle_beats.max(duration - angle_beats);
                        floors[index].countdown_ticks =
                            value_as_f64(event.get("countdownTicks"), 0.0) as i32;
                        floors[index].freeroam_sound_on = event
                            .get("hitsoundOnBeats")
                            .and_then(Value::as_str)
                            .map(str::to_string);
                        floors[index].freeroam_sound_off = event
                            .get("hitsoundOffBeats")
                            .and_then(Value::as_str)
                            .map(str::to_string);
                    }
                }
                Some("Hold") => {
                    let duration = value_as_f64(event.get("duration"), -1.0) as i32;
                    floors[index].hold_length = if index + 1 < floors.len() && duration >= 0 {
                        duration
                    } else {
                        -1
                    };
                }
                Some("MultiPlanet") => {
                    planets = planet_count(event.get("planets"), planets);
                    if index > 0 && floors[index - 1].mid_spin {
                        floors[index - 1].num_planets = planets;
                    }
                }
                Some("Multitap") => {
                    floors[index].taps_needed = value_as_f64(event.get("taps"), 1.0).round() as i32;
                }
                Some("SetHitsound") => {
                    floors[index].set_hitsound = Some(SoundChange {
                        game_sound: value_as_string(event.get("gameSound"), "Hitsound"),
                        hitsound: value_as_string(event.get("hitsound"), "Kick"),
                        volume: (value_as_f64(event.get("hitsoundVolume"), 100.0) / 100.0) as f32,
                    });
                }
                Some("SetHoldSound") => {
                    let mid_delay_beats = value_as_f64(event.get("holdMidSoundDelay"), 0.5);
                    let mid_delay_sec = mid_delay_beats * (60.0 / base_bpm) / speed.max(0.001);
                    floors[index].set_hold_sound = Some(HoldSoundChange {
                        start: value_as_string(event.get("holdStartSound"), "Fuse"),
                        loop_sound: value_as_string(event.get("holdLoopSound"), "Fuse"),
                        end: value_as_string(event.get("holdEndSound"), "Fuse"),
                        mid: value_as_string(event.get("holdMidSound"), "None"),
                        mid_type: value_as_string(event.get("holdMidSoundType"), "Once"),
                        mid_delay_sec,
                        mid_timing: value_as_string(
                            event.get("holdMidSoundTimingRelativeTo"),
                            "End",
                        ),
                        volume: (value_as_f64(event.get("holdSoundVolume"), 100.0) / 100.0) as f32,
                    });
                }
                Some("ScaleRadius") => {
                    radius_carry_extra += 0.0;
                }
                _ => {}
            }
        }

        let angle_deg =
            angle_moved(floors[index].entry_angle, floors[index].exit_angle, !is_ccw).to_degrees();
        let (floor_speed, next_speed) =
            speed_from_events(&set_speed_events, base_bpm, speed, angle_deg);
        floors[index].speed = floor_speed;
        speed = next_speed;
        floors[index].is_ccw = is_ccw;
        floors[index].num_planets = planets;
        floors[index].extra_beats += radius_carry_extra;
    }
}

fn planet_count(value: Option<&Value>, fallback: i32) -> i32 {
    match value {
        Some(Value::Number(number)) => number
            .as_f64()
            .map(|value| value.round().clamp(2.0, 3.0) as i32)
            .unwrap_or(fallback),
        Some(Value::String(text)) => {
            let normalized = text.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "twoplanets" | "two" | "2" => 2,
                "threeplanets" | "three" | "3" => 3,
                _ => fallback,
            }
        }
        _ => fallback,
    }
}

fn speed_from_events(
    events: &[&Value],
    base_bpm: f64,
    current_speed: f64,
    angle_deg: f64,
) -> (f64, f64) {
    if events.is_empty() {
        return (current_speed, current_speed);
    }

    let mut sorted = events.to_vec();
    sorted.sort_by(|a, b| {
        value_as_f64(a.get("angleOffset"), 0.0)
            .partial_cmp(&value_as_f64(b.get("angleOffset"), 0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let has_mid_floor_speed = sorted
        .iter()
        .any(|event| value_as_f64(event.get("angleOffset"), 0.0) > 0.0);

    let mut new_speed = current_speed;
    if !has_mid_floor_speed {
        for event in sorted {
            new_speed = speed_after_event(event, base_bpm, new_speed);
        }
        return (new_speed, new_speed);
    }

    let total_angle = if angle_deg.abs() <= 0.001 {
        360.0
    } else {
        angle_deg
    };
    let mut elapsed = 0.0;
    let mut previous_offset = 0.0;
    let mut segment_speed = current_speed;

    for event in sorted {
        let offset = value_as_f64(event.get("angleOffset"), 0.0).clamp(0.0, total_angle);
        elapsed += ((offset - previous_offset) / 180.0) * (60.0 / (base_bpm * segment_speed));
        segment_speed = speed_after_event(event, base_bpm, segment_speed);
        previous_offset = offset;
    }

    elapsed += ((total_angle - previous_offset) / 180.0) * (60.0 / (base_bpm * segment_speed));
    let effective_speed = if elapsed > 0.0 {
        60.0 / elapsed * (total_angle / 180.0) / base_bpm
    } else {
        segment_speed
    };

    (effective_speed, segment_speed)
}

fn speed_after_event(event: &Value, base_bpm: f64, current_speed: f64) -> f64 {
    match event
        .get("speedType")
        .and_then(Value::as_str)
        .unwrap_or("Multiplier")
    {
        "Bpm" => value_as_f64(event.get("beatsPerMinute"), base_bpm) / base_bpm,
        _ => current_speed * value_as_f64(event.get("bpmMultiplier"), 1.0),
    }
}

fn calculate_entry_times(
    floors: &mut [Floor],
    bpm: f64,
    pitch: f64,
    adjusted_countdown_ticks: f64,
) {
    if floors.len() <= 1 {
        return;
    }
    let crotchet = 60.0 / bpm;
    let mut time = crotchet * (adjusted_countdown_ticks - 1.0)
        + time_between_angles(
            floors[0].entry_angle,
            floors[0].exit_angle,
            floors[0].speed,
            bpm,
            !floors[0].is_ccw,
        );
    floors[0].entry_time = 0.0;
    floors[1].entry_time = time;
    floors[1].entry_time_pitch_adj = time / pitch;

    for i in 1..floors.len().saturating_sub(1) {
        let previous_mid = floors[i - 1].mid_spin;
        let inverse = inverse_angle_per_beat(floors[i].num_planets as f64)
            * if !floors[i].is_ccw { 1.0 } else { -1.0 };
        let mut angle_offset = if floors[i].mid_spin { 0.0 } else { inverse };
        if previous_mid && floors[i].num_planets > 2 {
            angle_offset -= (TAU + inverse_angle_per_beat(floors[i].num_planets as f64))
                * if !floors[i].is_ccw { 1.0 } else { -1.0 };
        }
        let mut segment = time_between_angles(
            floors[i].entry_angle + angle_offset,
            floors[i].exit_angle
                + if floors[i].mid_spin {
                    angle_offset
                } else {
                    0.0
                },
            floors[i].speed,
            bpm,
            !floors[i].is_ccw,
        );
        let full_spin =
            segment <= 0.000001 || segment >= (2.0 * crotchet / floors[i].speed) - 0.000001;
        if full_spin {
            segment = if floors[i].mid_spin {
                0.0
            } else {
                2.0 * time_between_angles(0.0, PI, floors[i].speed, bpm, false)
            };
        }
        time += segment;
        if floors[i].hold_length > 0 {
            time += floors[i].hold_length as f64
                * 2.0
                * time_between_angles(0.0, PI, floors[i].speed, bpm, false);
        }
        let mut extra_beats = floors[i].extra_beats;
        if extra_beats > 0.0 && full_spin {
            extra_beats -= 1.0;
        }
        time += extra_beats * time_between_angles(0.0, PI, floors[i].speed, bpm, false);
        floors[i + 1].entry_time = time;
        floors[i + 1].entry_time_pitch_adj = time / pitch;
    }
}

fn append_hit_events(
    timeline: &mut AudioTimeline,
    root: &Value,
    level_path: &Path,
    floors: &[Floor],
    bpm: f64,
    pitch: f64,
    song_start_delay: f64,
    countdown_speed_multiplier: f64,
    default_hitsound: String,
    default_volume: f32,
) {
    let mut hit_sound = default_hitsound;
    let mut midspin_sound = hit_sound.clone();
    let mut use_midspin_sound = false;
    let mut has_set_midspin_sound = false;
    let mut volume = default_volume;
    let mut hold_change = HoldSoundChange {
        start: "Fuse".to_string(),
        loop_sound: "Fuse".to_string(),
        end: "Fuse".to_string(),
        mid: "None".to_string(),
        mid_type: "Once".to_string(),
        mid_delay_sec: 0.5 * (60.0 / bpm),
        mid_timing: "End".to_string(),
        volume: 1.0,
    };

    for k in 1..floors.len() {
        let floor = &floors[k];
        let floor_entry_time = event_time(
            floor.entry_time_pitch_adj,
            timeline.song_offset_ms,
            pitch,
            song_start_delay,
        );
        if let Some(change) = &floor.set_hitsound {
            if change.game_sound == "Midspin" {
                use_midspin_sound = true;
                midspin_sound = change.hitsound.clone();
            } else {
                hit_sound = change.hitsound.clone();
                if !has_set_midspin_sound {
                    has_set_midspin_sound = true;
                    midspin_sound = change.hitsound.clone();
                }
            }
            volume = change.volume;
        }
        if let Some(change) = &floor.set_hold_sound {
            hold_change = change.clone();
        }

        let previous = &floors[k - 1];
        let mut floor_hit_time = None;
        if floor.hold_length <= -1
            && previous.hold_length <= -1
            && !(previous.mid_spin && k >= 2 && floors[k - 2].hold_length > -1)
        {
            let sound = if previous.mid_spin && use_midspin_sound {
                midspin_sound.clone()
            } else if floor.taps_needed > 1 {
                midspin_sound.clone()
            } else {
                hit_sound.clone()
            };
            if sound != "None" && !floor.mid_spin {
                let time_sec = (floor_entry_time - hit_sound_offset(&sound)).max(0.0);
                floor_hit_time = Some(time_sec);
                timeline.hit_events.push(HitEvent {
                    time_sec,
                    end_time_sec: None,
                    sound_name: format!("snd{sound}"),
                    volume,
                    pitch: 1.0,
                    source_floor: floor.seq_id,
                    kind: "hitsound".to_string(),
                });
            }
        }

        if floor.num_planets != previous.num_planets {
            timeline.hit_events.push(HitEvent {
                time_sec: floor_hit_time.unwrap_or(floor_entry_time),
                end_time_sec: None,
                sound_name: if floor.num_planets > previous.num_planets {
                    "sndVehiclePositive".to_string()
                } else {
                    "sndVehicleNegative".to_string()
                },
                volume: volume * 0.6,
                pitch: 1.0,
                source_floor: floor.seq_id,
                kind: "vehicle".to_string(),
            });
        }

        if floor.hold_length > -1 && !floor.mid_spin && k + 1 < floors.len() {
            append_hold_events(timeline, floors, k, &hold_change, pitch, song_start_delay);
        }

        if floor.freeroam_sound_on.is_some() || floor.freeroam_sound_off.is_some() {
            append_freeroam_events(
                timeline,
                floor,
                bpm,
                pitch,
                volume,
                song_start_delay,
                countdown_speed_multiplier,
            );
        }
    }

    append_play_sound_events(timeline, root, level_path, floors, pitch, song_start_delay);
}

fn append_hold_events(
    timeline: &mut AudioTimeline,
    floors: &[Floor],
    index: usize,
    hold_change: &HoldSoundChange,
    pitch: f64,
    song_start_delay: f64,
) {
    let floor = &floors[index];
    let next = &floors[index + 1];
    let start = event_time(
        floor.entry_time_pitch_adj,
        timeline.song_offset_ms,
        pitch,
        song_start_delay,
    );
    let end = event_time(
        next.entry_time_pitch_adj,
        timeline.song_offset_ms,
        pitch,
        song_start_delay,
    );
    let mid_offset = hold_mid_offset(&hold_change.mid);
    let end_offset = hold_end_offset(&hold_change.end);

    if hold_change.start != "None" {
        timeline.hold_sound_events.push(HitEvent {
            time_sec: start,
            end_time_sec: None,
            sound_name: format!("sndHeldbeatStart{}", hold_change.start),
            volume: hold_change.volume,
            pitch: 1.0,
            source_floor: floor.seq_id,
            kind: "hold-start".to_string(),
        });
    }
    if hold_change.loop_sound != "None" {
        timeline.hold_sound_events.push(HitEvent {
            time_sec: start,
            end_time_sec: Some(end),
            sound_name: format!("sndHeldbeatLoop{}", hold_change.loop_sound),
            volume: hold_change.volume,
            pitch: 1.0,
            source_floor: floor.seq_id,
            kind: "hold-loop".to_string(),
        });
    }
    if hold_change.mid != "None" {
        let delay = hold_change.mid_delay_sec / pitch;
        let mid_start = (start - mid_offset).max(0.0);
        let mid_end = (end - mid_offset).max(0.0);
        if hold_change.mid_type == "Repeat" && delay > 0.0 {
            let mut t = if hold_change.mid_timing == "Start" {
                mid_start + delay
            } else {
                mid_end - delay
            };
            while t > mid_start && t < mid_end {
                timeline.hold_sound_events.push(HitEvent {
                    time_sec: t,
                    end_time_sec: None,
                    sound_name: format!("sndHeldbeatMid{}", hold_change.mid),
                    volume: hold_change.volume,
                    pitch: 1.0,
                    source_floor: floor.seq_id,
                    kind: "hold-mid".to_string(),
                });
                if hold_change.mid_timing == "Start" {
                    t += delay;
                } else {
                    t -= delay;
                }
            }
        } else {
            let t = if hold_change.mid_timing == "Start" {
                mid_start + delay
            } else {
                mid_end - delay
            };
            if t > mid_start && t < mid_end {
                timeline.hold_sound_events.push(HitEvent {
                    time_sec: t,
                    end_time_sec: None,
                    sound_name: format!("sndHeldbeatMid{}", hold_change.mid),
                    volume: hold_change.volume,
                    pitch: 1.0,
                    source_floor: floor.seq_id,
                    kind: "hold-mid".to_string(),
                });
            }
        }
    }
    if hold_change.end != "None" {
        timeline.hold_sound_events.push(HitEvent {
            time_sec: (end - end_offset).max(0.0),
            end_time_sec: None,
            sound_name: format!("sndHeldbeatEnd{}", hold_change.end),
            volume: hold_change.volume,
            pitch: 1.0,
            source_floor: floor.seq_id,
            kind: "hold-end".to_string(),
        });
    }
}

fn append_freeroam_events(
    timeline: &mut AudioTimeline,
    floor: &Floor,
    bpm: f64,
    pitch: f64,
    volume: f32,
    song_start_delay: f64,
    countdown_speed_multiplier: f64,
) {
    let mut index = 0;
    let mut beat = 0.0;
    while beat < floor.extra_beats - floor.countdown_ticks as f64 / countdown_speed_multiplier + 1.0
    {
        let sound = if index % 2 == 0 {
            floor.freeroam_sound_on.as_ref()
        } else {
            floor.freeroam_sound_off.as_ref()
        };
        if let Some(sound) = sound {
            if sound != "None" {
                let time = event_time(
                    floor.entry_time_pitch_adj,
                    timeline.song_offset_ms,
                    pitch,
                    song_start_delay,
                ) + beat * (60.0 / bpm) / floor.speed / pitch
                    - hit_sound_offset(sound);
                timeline.hit_events.push(HitEvent {
                    time_sec: time.max(0.0),
                    end_time_sec: None,
                    sound_name: format!("snd{sound}"),
                    volume,
                    pitch: 1.0,
                    source_floor: floor.seq_id,
                    kind: "freeroam".to_string(),
                });
            }
        }
        index += 1;
        beat += 0.5;
    }
}

fn append_play_sound_events(
    timeline: &mut AudioTimeline,
    root: &Value,
    level_path: &Path,
    floors: &[Floor],
    pitch: f64,
    song_start_delay: f64,
) {
    let parent = level_path.parent().unwrap_or_else(|| Path::new(""));
    for event in array_at(root, "actions") {
        if !event_is_active(event) {
            continue;
        }
        if event_type(event) != Some("PlaySound") {
            continue;
        }
        let floor = event_floor(event);
        let floor_time = floors
            .get(floor)
            .map(|floor| {
                event_time(
                    floor.entry_time_pitch_adj,
                    timeline.song_offset_ms,
                    pitch,
                    song_start_delay,
                )
            })
            .unwrap_or_default();
        let sound = value_as_string(event.get("hitsound"), "Kick");
        let offset = value_as_f64(event.get("offset"), 0.0) / 1000.0;
        let play_pitch = (value_as_f64(event.get("pitch"), 100.0) / 100.0) as f32;
        let volume = (value_as_f64(event.get("hitsoundVolume"), 100.0) / 100.0) as f32;
        let duration = value_as_f64(event.get("playDuration"), 0.0) / 1000.0 / pitch;
        let sound_offset = if is_builtin_hitsound(&sound) {
            hit_sound_offset(&sound)
        } else {
            0.0
        };
        let time_sec = (floor_time - offset / pitch - sound_offset).max(0.0);
        let sound_name = if is_builtin_hitsound(&sound) {
            format!("snd{sound}")
        } else {
            parent.join(&sound).to_string_lossy().to_string()
        };
        timeline.play_sound_events.push(HitEvent {
            time_sec,
            end_time_sec: if duration > 0.0 {
                Some(time_sec + duration)
            } else {
                None
            },
            sound_name,
            volume,
            pitch: play_pitch,
            source_floor: floor,
            kind: "play-sound".to_string(),
        });
    }
}

fn is_builtin_hitsound(sound: &str) -> bool {
    matches!(
        sound,
        "Hat"
            | "Kick"
            | "Shaker"
            | "Sizzle"
            | "Chuck"
            | "ShakerLoud"
            | "Hammer"
            | "KickChroma"
            | "SnareAcoustic2"
            | "Sidestick"
            | "Stick"
            | "ReverbClack"
            | "Squareshot"
            | "PowerDown"
            | "PowerUp"
            | "KickHouse"
            | "KickRupture"
            | "HatHouse"
            | "SnareHouse"
            | "SnareVapor"
            | "ClapHit"
            | "ClapHitEcho"
            | "ReverbClap"
            | "FireTile"
            | "IceTile"
            | "VehiclePositive"
            | "VehicleNegative"
    )
}

fn hit_sound_offset(sound: &str) -> f64 {
    match sound {
        "Shaker" => 0.015,
        "ShakerLoud" => 0.015,
        "Hammer" => 0.03,
        "Stick" => 0.021,
        "Squareshot" => 0.05,
        "ClapHit" => 0.036,
        "ClapHitEcho" => 0.116,
        "ReverbClap" => 0.026,
        _ => 0.0,
    }
}

fn hold_mid_offset(sound: &str) -> f64 {
    match sound {
        "Fuse" => 0.107,
        "SingSing" => 0.212,
        _ => 0.0,
    }
}

fn hold_end_offset(sound: &str) -> f64 {
    match sound {
        "Fuse" => 0.099,
        _ => 0.0,
    }
}

fn event_time(entry_time_pitch_adj: f64, offset_ms: f64, pitch: f64, song_start_delay: f64) -> f64 {
    (entry_time_pitch_adj + offset_ms / 1000.0 / pitch - song_start_delay).max(0.0)
}

fn time_between_angles(
    entry_angle: f64,
    exit_angle: f64,
    speed: f64,
    bpm: f64,
    is_cw: bool,
) -> f64 {
    let direction = if is_cw { 1.0 } else { -1.0 };
    modulo((exit_angle - entry_angle) * direction, TAU) / PI * ((60.0 / bpm) / speed.max(0.001))
}

fn angle_moved(entry_angle: f64, exit_angle: f64, is_cw: bool) -> f64 {
    let direction = if is_cw { 1.0 } else { -1.0 };
    modulo((exit_angle - entry_angle) * direction, TAU)
}

fn inverse_angle_per_beat(planets: f64) -> f64 {
    PI * (planets - 2.0) / planets
}

fn single_floor_angle_beats(
    floor: &Floor,
    is_ccw: bool,
    num_planets: i32,
    previous_mid_spin: bool,
) -> f64 {
    let direction = if !is_ccw { 1.0 } else { -1.0 };
    let mut inverse = inverse_angle_per_beat(num_planets as f64) * direction;
    if floor.mid_spin {
        inverse = 0.0;
    }
    if previous_mid_spin && num_planets > 2 {
        inverse -= (TAU + inverse_angle_per_beat(num_planets as f64)) * direction;
    }
    let angle = angle_moved(
        floor.entry_angle + inverse,
        floor.exit_angle + if floor.mid_spin { inverse } else { 0.0 },
        !is_ccw,
    );
    if angle <= 0.000001 || angle >= TAU - 0.000001 {
        if floor.mid_spin {
            0.0
        } else {
            2.0
        }
    } else {
        angle / PI
    }
}

fn modulo(value: f64, modulus: f64) -> f64 {
    ((value % modulus) + modulus) % modulus
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.0001
}

fn path_data_to_angles(path_data: &str, warnings: &mut Vec<String>) -> Vec<f64> {
    let mut previous = 0.0;
    let mut unknown = BTreeSet::new();
    let mut angles = Vec::with_capacity(path_data.chars().count());

    for ch in path_data.chars() {
        let angle = char_to_angle(ch).unwrap_or_else(|| match ch {
            '5' => previous + 72.0,
            '6' => previous - 72.0,
            '7' => previous + 52.0,
            '8' => previous - 52.0,
            '9' => previous - 30.0,
            'h' => previous + 120.0,
            'j' => previous - 120.0,
            't' => previous + 60.0,
            'y' => previous + 300.0,
            _ => {
                unknown.insert(ch);
                previous
            }
        });
        angles.push(angle);
        previous = angle;
    }

    if !unknown.is_empty() {
        let chars = unknown
            .into_iter()
            .map(|ch| ch.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        warnings.push(format!(
            "pathData 包含未识别路径字符：{chars}，已按官方兼容逻辑沿用上一角度"
        ));
    }

    angles
}

fn char_to_angle(ch: char) -> Option<f64> {
    match ch {
        'R' => Some(0.0),
        'E' => Some(45.0),
        'U' => Some(90.0),
        'Q' => Some(135.0),
        'L' => Some(180.0),
        'Z' => Some(225.0),
        'D' => Some(270.0),
        'C' => Some(315.0),
        'B' => Some(300.0),
        'T' => Some(60.0),
        'G' => Some(120.0),
        'F' => Some(240.0),
        'J' => Some(30.0),
        'H' => Some(150.0),
        'N' => Some(210.0),
        'M' => Some(330.0),
        'p' => Some(15.0),
        'o' => Some(75.0),
        'q' => Some(105.0),
        'W' => Some(165.0),
        'x' => Some(195.0),
        'V' => Some(255.0),
        'Y' => Some(285.0),
        'A' => Some(345.0),
        '!' => Some(999.0),
        _ => None,
    }
}

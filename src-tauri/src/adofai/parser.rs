use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ParsedLevel {
    pub root: Value,
    pub parse_mode: String,
    pub warnings: Vec<String>,
}

pub fn parse_level_file(path: &Path, lenient: bool) -> Result<ParsedLevel, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("读取谱面失败: {err}"))?;
    parse_level_text(&raw, lenient)
}

pub fn parse_level_text(raw: &str, lenient: bool) -> Result<ParsedLevel, String> {
    let text = raw.trim_start_matches('\u{feff}');
    match serde_json::from_str::<Value>(text) {
        Ok(root) => Ok(ParsedLevel {
            root,
            parse_mode: "strict-json".to_string(),
            warnings: Vec::new(),
        }),
        Err(strict_err) if lenient => {
            let cleaned = repair_lenient_json_text(text);
            json5::from_str::<Value>(&cleaned)
                .map(|root| ParsedLevel {
                    root,
                    parse_mode: "lenient-json5".to_string(),
                    warnings: {
                        let _ = strict_err;
                        vec!["已自动兼容这个谱面的特殊格式".to_string()]
                    },
                })
                .map_err(|_lenient_err| "谱面文件格式异常，暂时无法读取".to_string())
        }
        Err(_err) => Err("谱面文件格式异常，暂时无法读取".to_string()),
    }
}

fn repair_lenient_json_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
                out.push(ch);
                continue;
            }

            if ch == '\\' {
                escaped = true;
                out.push(ch);
                continue;
            }
            if ch == '"' {
                in_string = false;
                out.push(ch);
                continue;
            }
            if ch == '\n' {
                out.push_str("\\n");
                continue;
            }
            if ch == '\r' {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                out.push_str("\\n");
                continue;
            }
            if ch == '\t' {
                out.push_str("\\t");
                continue;
            }
            if ch.is_control() {
                out.push_str(&format!("\\u{:04x}", ch as u32));
                continue;
            }
            out.push(ch);
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }

        if ch == '}' || ch == ']' {
            out.push(ch);
            let mut lookahead = chars.clone();
            while matches!(lookahead.peek(), Some(next) if next.is_whitespace()) {
                lookahead.next();
            }
            if matches!(lookahead.peek(), Some('{') | Some('"')) {
                out.push(',');
            }
            continue;
        }

        if ch == ',' {
            let mut lookahead = chars.clone();
            while matches!(lookahead.peek(), Some(next) if next.is_whitespace()) {
                lookahead.next();
            }
            if matches!(lookahead.peek(), Some(',') | Some('}') | Some(']')) {
                continue;
            }
        }

        out.push(ch);
    }

    out
}

pub fn object_at<'a>(root: &'a Value, key: &str) -> Option<&'a Map<String, Value>> {
    root.get(key)?.as_object()
}

pub fn array_at<'a>(root: &'a Value, key: &str) -> Vec<&'a Value> {
    root.get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

pub fn settings_map(root: &Value) -> BTreeMap<String, Value> {
    object_at(root, "settings")
        .map(|settings| {
            settings
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn string_setting(settings: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    settings
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn clean_display_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            let mut found_end = false;
            while let Some(next) = chars.next() {
                if next == '>' {
                    found_end = true;
                    break;
                }
                if next == '\n' || next == '\r' {
                    break;
                }
                tag.push(next);
            }

            if found_end && is_rich_text_tag(&tag) {
                continue;
            }

            output.push('<');
            output.push_str(&tag);
            if found_end {
                output.push('>');
            }
            continue;
        }

        output.push(ch);
    }

    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_rich_text_tag(tag: &str) -> bool {
    let trimmed = tag.trim();
    let name = trimmed
        .trim_start_matches('/')
        .split(|ch| ch == '=' || ch == ' ')
        .next()
        .unwrap_or_default()
        .trim();

    matches!(
        name.to_ascii_lowercase().as_str(),
        "align"
            | "alpha"
            | "b"
            | "br"
            | "color"
            | "font"
            | "i"
            | "indent"
            | "line-height"
            | "line-indent"
            | "link"
            | "lowercase"
            | "margin"
            | "mark"
            | "mspace"
            | "nobr"
            | "noparse"
            | "pos"
            | "quad"
            | "rotate"
            | "s"
            | "size"
            | "smallcaps"
            | "space"
            | "sprite"
            | "style"
            | "sub"
            | "sup"
            | "u"
            | "uppercase"
            | "voffset"
            | "width"
    )
}

pub fn number_setting(settings: &BTreeMap<String, Value>, key: &str, fallback: f64) -> f64 {
    settings
        .get(key)
        .and_then(|value| match value {
            Value::Number(number) => number.as_f64(),
            Value::String(text) => text.parse::<f64>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
}

pub fn resolve_sibling(base_file: &Path, filename: Option<&str>) -> Option<PathBuf> {
    let filename = filename?.trim();
    if filename.is_empty() {
        return None;
    }
    let path = Path::new(filename);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    Some(base_file.parent()?.join(path))
}

pub fn event_type(event: &Value) -> Option<&str> {
    event.get("eventType").and_then(Value::as_str)
}

pub fn event_is_active(event: &Value) -> bool {
    event
        .get("active")
        .and_then(|value| match value {
            Value::Bool(value) => Some(*value),
            Value::String(text) => text.parse::<bool>().ok(),
            _ => None,
        })
        .unwrap_or(true)
}

pub fn event_floor(event: &Value) -> usize {
    event
        .get("floor")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(0)
}

pub fn value_as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value
        .and_then(|value| match value {
            Value::Number(number) => number.as_f64(),
            Value::String(text) => text.parse::<f64>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
}

pub fn value_as_string(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

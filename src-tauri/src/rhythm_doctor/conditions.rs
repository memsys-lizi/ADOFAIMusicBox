use super::parser::{array_at, event_is_active, value_as_bool, value_as_i64};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ConditionContext {
    condition_values: HashMap<String, bool>,
    two_player_mode: bool,
}

impl ConditionContext {
    pub fn single_player(root: &Value) -> Self {
        Self::for_player_mode(root, false)
    }

    fn for_player_mode(root: &Value, two_player_mode: bool) -> Self {
        let mut context = Self {
            condition_values: HashMap::new(),
            two_player_mode,
        };

        for conditional in array_at(root, "conditionals") {
            if let Some(value) = context.static_condition_value(conditional) {
                context.insert_condition_aliases(conditional, value);
            }
        }

        context
    }

    pub fn event_runs(&self, event: &Value) -> bool {
        if !event_is_active(event) {
            return false;
        }
        if self.has_event_tag(event) && !value_as_bool(event.get("runTag"), false) {
            return false;
        }
        self.event_condition(event).is_none_or(|condition| {
            self.evaluate_condition_expression(condition)
                .unwrap_or(false)
        })
    }

    pub fn event_has_filter(event: &Value) -> bool {
        event
            .get("if")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
            || event
                .get("tag")
                .and_then(Value::as_str)
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
    }

    fn event_condition<'a>(&self, event: &'a Value) -> Option<&'a str> {
        event
            .get("if")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    fn has_event_tag(&self, event: &Value) -> bool {
        event
            .get("tag")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    }

    fn static_condition_value(&self, conditional: &Value) -> Option<bool> {
        match conditional.get("type").and_then(Value::as_str) {
            Some("PlayerMode") => {
                let expects_two_player = conditional
                    .get("twoPlayerMode")
                    .or_else(|| conditional.get("playerModeIsTwoPlayer"))
                    .map(|value| value_as_bool(Some(value), true))
                    .unwrap_or(true);
                Some(expects_two_player == self.two_player_mode)
            }
            Some("Custom") => conditional
                .get("expression")
                .and_then(Value::as_str)
                .and_then(|expression| self.static_custom_expression_value(expression)),
            _ => None,
        }
    }

    fn static_custom_expression_value(&self, expression: &str) -> Option<bool> {
        let normalized = expression
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect::<String>();
        match normalized.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            "1pmode" | "!2pmode" => Some(!self.two_player_mode),
            "2pmode" | "!1pmode" => Some(self.two_player_mode),
            _ => None,
        }
    }

    fn insert_condition_aliases(&mut self, conditional: &Value, value: bool) {
        if conditional.get("id").is_some() {
            let id = value_as_i64(conditional.get("id"), -1);
            if id >= 0 {
                self.condition_values.insert(id.to_string(), value);
            }
            if let Some(id_text) = conditional.get("id").and_then(Value::as_str) {
                self.insert_alias(id_text, value);
            }
        }
        for key in ["tag", "name"] {
            if let Some(alias) = conditional.get(key).and_then(Value::as_str) {
                self.insert_alias(alias, value);
            }
        }
    }

    fn insert_alias(&mut self, alias: &str, value: bool) {
        let alias = alias.trim();
        if !alias.is_empty() {
            self.condition_values.insert(alias.to_string(), value);
        }
    }

    fn evaluate_condition_expression(&self, expression: &str) -> Option<bool> {
        self.evaluate_or(expression.trim())
    }

    fn evaluate_or(&self, expression: &str) -> Option<bool> {
        let parts = split_top_level(expression, '|');
        if parts.len() <= 1 {
            return self.evaluate_and(expression);
        }

        let mut saw_unknown = false;
        for part in parts {
            match self.evaluate_and(part) {
                Some(true) => return Some(true),
                Some(false) => {}
                None => saw_unknown = true,
            }
        }
        if saw_unknown {
            None
        } else {
            Some(false)
        }
    }

    fn evaluate_and(&self, expression: &str) -> Option<bool> {
        let parts = split_top_level(expression, '&');
        if parts.len() <= 1 {
            return self.evaluate_unary(expression);
        }

        let mut saw_unknown = false;
        for part in parts {
            match self.evaluate_unary(part) {
                Some(true) => {}
                Some(false) => return Some(false),
                None => saw_unknown = true,
            }
        }
        if saw_unknown {
            None
        } else {
            Some(true)
        }
    }

    fn evaluate_unary(&self, expression: &str) -> Option<bool> {
        let mut expression = expression.trim();
        let mut negate = false;
        while let Some(rest) = expression
            .strip_prefix('~')
            .or_else(|| expression.strip_prefix('!'))
        {
            negate = !negate;
            expression = rest.trim();
        }

        let value = self.evaluate_atom(expression)?;
        Some(if negate { !value } else { value })
    }

    fn evaluate_atom(&self, expression: &str) -> Option<bool> {
        let expression = trim_wrapping_parentheses(expression.trim());
        let key = strip_delay_suffix(expression);
        let normalized = key
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect::<String>();
        match normalized.as_str() {
            "true" => return Some(true),
            "false" => return Some(false),
            "1pmode" => return Some(!self.two_player_mode),
            "2pmode" => return Some(self.two_player_mode),
            _ => {}
        }
        self.condition_values.get(key).copied()
    }
}

fn split_top_level(expression: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (index, ch) in expression.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if ch == separator && depth == 0 => {
                parts.push(expression[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(expression[start..].trim());
    parts
}

fn trim_wrapping_parentheses(expression: &str) -> &str {
    let mut expression = expression.trim();
    loop {
        if !expression.starts_with('(') || !expression.ends_with(')') {
            return expression;
        }
        let mut depth = 0_i32;
        let mut wraps = true;
        for (index, ch) in expression.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && index != expression.len() - ch.len_utf8() {
                        wraps = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if wraps {
            expression = expression[1..expression.len() - 1].trim();
        } else {
            return expression;
        }
    }
}

fn strip_delay_suffix(token: &str) -> &str {
    let token = token.trim();
    let bytes = token.as_bytes();
    let mut index = bytes.len();
    while index > 0 && bytes[index - 1].is_ascii_digit() {
        index -= 1;
    }
    if index < bytes.len() && index > 1 && bytes[index - 1] == b'd' {
        token[..index - 1].trim()
    } else {
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn player_mode_condition_uses_single_player_branch() {
        let root = json!({
            "conditionals": [
                { "type": "PlayerMode", "id": 1, "tag": "1", "twoPlayerMode": true }
            ]
        });
        let context = ConditionContext::single_player(&root);
        assert_eq!(context.evaluate_condition_expression("1d0"), Some(false));
        assert_eq!(context.evaluate_condition_expression("~1d0"), Some(true));
    }

    #[test]
    fn tagged_event_requires_run_tag() {
        let context = ConditionContext::single_player(&json!({}));
        assert!(!context.event_runs(&json!({ "type": "PlaySound", "tag": "Later" })));
        assert!(context.event_runs(&json!({ "type": "PlaySound", "tag": "Later", "runTag": true })));
    }

    #[test]
    fn unknown_conditions_do_not_run_static_timeline() {
        let context = ConditionContext::single_player(&json!({}));
        assert!(!context.event_runs(&json!({ "type": "PlaySound", "if": "9d0" })));
        assert!(!context.event_runs(&json!({ "type": "PlaySound", "if": "~9d0" })));
    }
}

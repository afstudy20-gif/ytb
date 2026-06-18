//! Small, well-tested helpers for traversing InnerTube's loosely-typed JSON.
//!
//! InnerTube responses are deeply-nested, polymorphic, and use different
//! render keys for the same conceptual thing across endpoints. Centralising
//! the traversal here keeps the parsing modules readable and means a future
//! InnerTube shape change can often be fixed in one place.
//!
//! Some helpers in this module are kept around even when not currently used
//! by a parser — they form the public-ish toolbox future parsers and
//! downstream code are likely to need.

#![allow(dead_code)]

use serde_json::Value;

/// Borrow a child object by key, or `None`.
#[must_use]
pub(crate) fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key)
}

/// Borrow a nested child via a dotted path. `path(&v, "a.b.c")` is sugar for
/// `v.get("a")?.get("b")?.get("c")`. Returns `None` if any segment is absent
/// or non-object.
#[must_use]
pub(crate) fn path<'a>(v: &'a Value, dotted: &str) -> Option<&'a Value> {
    let mut cur = v;
    for seg in dotted.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// Borrow an element from a JSON array at the given index, or `None`.
#[must_use]
pub(crate) fn at<'a>(v: &'a Value, idx: usize) -> Option<&'a Value> {
    v.get(idx)
}

/// Borrow a string child by key, or `None`.
#[must_use]
pub(crate) fn get_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str())
}

/// Borrow a string child by dotted path, or `None`.
#[must_use]
pub(crate) fn path_str<'a>(v: &'a Value, dotted: &str) -> Option<&'a str> {
    path(v, dotted).and_then(|x| x.as_str())
}

/// Borrow a u64 child by dotted path, or `None`. Accepts JSON numbers or
/// numeric strings.
#[must_use]
pub(crate) fn path_u64(v: &Value, dotted: &str) -> Option<u64> {
    let v = path(v, dotted)?;
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// Borrow a string array child by dotted path, or empty.
#[must_use]
pub(crate) fn path_str_array<'a>(v: &'a Value, dotted: &str) -> Vec<&'a str> {
    match path(v, dotted) {
        Some(Value::Array(arr)) => arr.iter().filter_map(|x| x.as_str()).collect(),
        _ => Vec::new(),
    }
}

/// Iterate over the InnerTube "renderer" objects inside a list-shaped `Value`.
///
/// InnerTube tends to wrap every conceptual item in `{"<Kind>Renderer": {...}}`.
/// This helper yields the *inner* renderer object for each such wrapper, so
/// callers can pattern-match on the inner key directly.
#[must_use]
pub(crate) fn renderers(v: &Value) -> Vec<&Value> {
    match v {
        Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                if let Value::Object(obj) = item {
                    let inner_renderer = obj.iter().find(|(k, _)| k.ends_with("Renderer"));
                    inner_renderer.map(|(_, v)| v).or(Some(item))
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Walk the JSON tree (BFS) and yield the first sub-object that contains the
/// given key. Useful for finding `continuationItemRenderer` etc. regardless
/// of where in the response InnerTube buried it this week.
#[must_use]
pub(crate) fn find_first_with_key<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    let mut stack: Vec<&Value> = vec![v];
    while let Some(node) = stack.pop() {
        if let Value::Object(map) = node {
            if let Some(found) = map.get(key) {
                return Some(found);
            }
            stack.extend(map.values());
        } else if let Value::Array(arr) = node {
            stack.extend(arr.iter());
        }
    }
    None
}

/// Walk the JSON tree (BFS) and yield every object that *is* a key-bearing
/// renderer with the requested name. Used by parsers that need every match
/// (e.g. all `compactVideoRenderer`s inside a search response).
#[must_use]
pub fn find_all_with_key<'a>(v: &'a Value, key: &str) -> Vec<&'a Value> {
    let mut out = Vec::new();
    let mut stack: Vec<&Value> = vec![v];
    while let Some(node) = stack.pop() {
        match node {
            Value::Object(map) => {
                if let Some(found) = map.get(key) {
                    out.push(found);
                }
                stack.extend(map.values());
            }
            Value::Array(arr) => stack.extend(arr.iter()),
            _ => {}
        }
    }
    out
}

/// Concatenate every `simpleText` or `runs[].text` under `v`. InnerTube
/// represents human-readable strings in two ways; this helper unifies them.
#[must_use]
pub(crate) fn collect_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(map) => {
            if let Some(simple) = map.get("simpleText").and_then(|x| x.as_str()) {
                return Some(simple.to_string());
            }
            if let Some(runs) = map.get("runs").and_then(|x| x.as_array()) {
                let joined: String = runs
                    .iter()
                    .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                    .collect();
                if !joined.is_empty() {
                    return Some(joined);
                }
            }
            None
        }
        _ => None,
    }
}

/// Same as [`collect_text`] but tolerant of `None`-ness; returns empty string.
#[must_use]
pub(crate) fn collect_text_or_empty(v: &Value) -> String {
    collect_text(v).unwrap_or_default()
}

/// Take the first thumbnail URL from a `thumbnails` array, if any.
#[must_use]
pub(crate) fn first_thumbnail(v: &Value) -> Option<String> {
    let arr = v.get("thumbnails").and_then(|x| x.as_array())?;
    let first = arr.first()?;
    first.get("url").and_then(|x| x.as_str()).map(String::from)
}

/// Parse a view/subscriber/video count from InnerTube's human-readable text
/// such as `"1,234,567 views"` or `"1.2M subscribers"`.
///
/// Handles comma separators and K/M/B suffixes. Returns `None` on parse
/// failure (e.g. `"No views"`).
#[must_use]
pub(crate) fn parse_count(text: &str) -> Option<u64> {
    // Strip non-ASCII digits and any prefix, retain suffix letter.
    let mut digits = String::new();
    let mut suffix: f64 = 1.0;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if ch == '.' || ch == ',' {
            // keep dots only if surrounded by digits (decimal separator)
            if ch == '.' && !digits.is_empty() {
                digits.push(ch);
            }
        } else if suffix == 1.0 {
            // first non-digit, non-separator char we see marks the suffix
            match ch.to_ascii_uppercase() {
                'K' => suffix = 1_000.0,
                'M' => suffix = 1_000_000.0,
                'B' => suffix = 1_000_000_000.0,
                _ => break,
            }
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    // Remove trailing dots left from decimal-style counts like "1.2".
    while digits.ends_with('.') {
        digits.pop();
    }
    let parsed: f64 = digits.parse().ok()?;
    let result = (parsed * suffix).round() as u64;
    Some(result)
}

/// Convert a length string like `"M:SS"`, `"H:MM:SS"`, or `"MM:SS"` to
/// seconds. Returns `None` if the input doesn't match the expected shape.
#[must_use]
pub(crate) fn length_text_to_seconds(s: &str) -> Option<u64> {
    let parts: Vec<u64> = s
        .split(':')
        .map(|p| p.trim().parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    let total = match parts.len() {
        2 => parts[0] * 60 + parts[1],
        3 => parts[0] * 3600 + parts[1] * 60 + parts[2],
        _ => return None,
    };
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_count_handles_examples() {
        assert_eq!(parse_count("1,234,567 views"), Some(1_234_567));
        assert_eq!(parse_count("1.2M subscribers"), Some(1_200_000));
        assert_eq!(parse_count("3K views"), Some(3_000));
        assert_eq!(parse_count("5B views"), Some(5_000_000_000));
        assert_eq!(parse_count("No views"), None);
    }

    #[test]
    fn length_text_to_seconds_examples() {
        assert_eq!(length_text_to_seconds("4:33"), Some(273));
        assert_eq!(length_text_to_seconds("1:02:03"), Some(3723));
        assert_eq!(length_text_to_seconds("0:08"), Some(8));
        assert_eq!(length_text_to_seconds("nonsense"), None);
    }

    #[test]
    fn collect_text_unifies_simple_and_runs() {
        let simple = serde_json::json!({"simpleText": "hello"});
        let runs = serde_json::json!({"runs": [{"text": "a"}, {"text": "b"}, {"text": "c"}]});
        assert_eq!(collect_text(&simple).as_deref(), Some("hello"));
        assert_eq!(collect_text(&runs).as_deref(), Some("abc"));
    }

    #[test]
    fn find_first_with_key_finds_anywhere() {
        let v = serde_json::json!({
            "a": {
                "b": [
                    {"videoRenderer": {"videoId": "x"}},
                    {"foo": "bar"}
                ]
            }
        });
        let found = find_first_with_key(&v, "videoRenderer");
        assert_eq!(found.and_then(|x| x.get("videoId")).and_then(|x| x.as_str()), Some("x"));
    }
}

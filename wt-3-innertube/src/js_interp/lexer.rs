//! Source-level parsers used by both the cipher extractor and the n-sig
//! interpreter.
//!
//! These functions deliberately do *not* build an AST — they return slices
//! or lists of slices of the original source string, which keeps allocation
//! cheap and lets the higher layers reach into the JS as needed. They are
//! all `&str`-based and string-literal-aware.

#![allow(dead_code)]

use crate::error::{Error, Result};

/// Split a JS source into top-level statements delimited by `;`, respecting
/// braces, brackets, parentheses, and string literals.
pub(crate) fn split_top_level_statements(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth_paren = 0i32;
    let mut depth_brace = 0i32;
    let mut depth_bracket = 0i32;
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {
                buf.push(c);
                let quote = c;
                while let Some(inner) = chars.next() {
                    buf.push(inner);
                    if inner == '\\' {
                        if let Some(esc) = chars.next() {
                            buf.push(esc);
                        }
                        continue;
                    }
                    if inner == quote {
                        break;
                    }
                }
            }
            '(' => {
                depth_paren += 1;
                buf.push(c);
            }
            ')' => {
                depth_paren -= 1;
                buf.push(c);
            }
            '{' => {
                depth_brace += 1;
                buf.push(c);
            }
            '}' => {
                depth_brace -= 1;
                buf.push(c);
                // A top-level `{...}` block that just closed is the end of
                // a statement (e.g. a `function ... { ... }` declaration,
                // which has no trailing `;`). Flush so the next declaration
                // is parsed independently.
                if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 {
                    if !buf.trim().is_empty() {
                        out.push(std::mem::take(&mut buf));
                    } else {
                        buf.clear();
                    }
                }
            }
            '[' => {
                depth_bracket += 1;
                buf.push(c);
            }
            ']' => {
                depth_bracket -= 1;
                buf.push(c);
            }
            ';' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => {
                if !buf.trim().is_empty() {
                    out.push(std::mem::take(&mut buf));
                } else {
                    buf.clear();
                }
            }
            _ => buf.push(c),
        }
    }
    if !buf.trim().is_empty() {
        out.push(buf);
    }
    out
}

/// Split on top-level commas (respecting brackets/parens/braces/strings).
pub(crate) fn split_top_level_commas(src: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            '"' | '\'' => {
                let quote = c;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == '\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == quote {
                        break;
                    }
                    i += 1;
                }
            }
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&src[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if start <= src.len() {
        out.push(&src[start..]);
    }
    out
}

/// Parse `name(params) { body }` from the fragment *after* `function `.
pub(crate) fn parse_named_function(rest: &str) -> Result<Option<(String, Vec<String>, String)>> {
    let rest = rest.trim();
    let paren = match rest.find('(') {
        Some(p) => p,
        None => return Ok(None),
    };
    let name = rest[..paren].trim().to_string();
    let close = match rest[paren..].find(')') {
        Some(c) => paren + c,
        None => return Ok(None),
    };
    let params: Vec<String> = rest[paren + 1..close]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let brace = match rest[close..].find('{') {
        Some(b) => close + b,
        None => return Ok(None),
    };
    let body = extract_brace_body(&rest[brace..])?;
    Ok(Some((name, params, body)))
}

/// Parse `(params) { body }` from the fragment *after* `function` (i.e. an
/// anonymous function literal).
pub(crate) fn parse_anon_function(rest: &str) -> Result<Option<((), Vec<String>, String)>> {
    let rest = rest.trim_start();
    let paren = match rest.find('(') {
        Some(p) => p,
        None => return Ok(None),
    };
    let close = match rest[paren..].find(')') {
        Some(c) => paren + c,
        None => return Ok(None),
    };
    let params: Vec<String> = rest[paren + 1..close]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let brace = match rest[close..].find('{') {
        Some(b) => close + b,
        None => return Ok(None),
    };
    let body = extract_brace_body(&rest[brace..])?;
    Ok(Some(((), params, body)))
}

/// Given a string that begins with `{`, return its contents (between matching
/// braces).
pub(crate) fn extract_brace_body(src: &str) -> Result<String> {
    if !src.starts_with('{') {
        return Err(Error::cipher("expected '{'"));
    }
    let mut depth = 0i32;
    let mut chars = src.chars().peekable();
    let mut consumed = String::new();
    while let Some(c) = chars.next() {
        consumed.push(c);
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let inner = &consumed[1..consumed.len() - 1];
                    return Ok(inner.to_string());
                }
            }
            '"' | '\'' => {
                let quote = c;
                while let Some(inner) = chars.next() {
                    consumed.push(inner);
                    if inner == '\\' {
                        if let Some(esc) = chars.next() {
                            consumed.push(esc);
                        }
                        continue;
                    }
                    if inner == quote {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    Err(Error::cipher("unterminated block"))
}

/// True if `s` is a bare JS identifier.
pub(crate) fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && s
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// Decode standard JS string escapes.
pub(crate) fn unescape_js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(esc) = chars.next() {
                match esc {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    '\\' => out.push('\\'),
                    '"' => out.push('"'),
                    '\'' => out.push('\''),
                    '0' => out.push('\0'),
                    'x' => {
                        let hex: String = chars.by_ref().take(2).collect();
                        if let Ok(n) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(n) {
                                out.push(ch);
                            }
                        }
                    }
                    'u' => {
                        let hex: String = chars.by_ref().take(4).collect();
                        if let Ok(n) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(n) {
                                out.push(ch);
                            }
                        }
                    }
                    other => out.push(other),
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parse a simple JS literal — number, string, or boolean. Returns `None`
/// for things that aren't literals.
pub(crate) fn parse_simple_literal(src: &str) -> Option<crate::js_interp::JsValue> {
    use crate::js_interp::JsValue;
    let src = src.trim().trim_end_matches(';').trim();
    if src == "undefined" {
        return Some(JsValue::Undef);
    }
    if src == "true" {
        return Some(JsValue::Bool(true));
    }
    if src == "false" {
        return Some(JsValue::Bool(false));
    }
    if (src.starts_with('"') && src.ends_with('"') && src.len() >= 2)
        || (src.starts_with('\'') && src.ends_with('\'') && src.len() >= 2)
    {
        let unquoted = &src[1..src.len() - 1];
        return Some(JsValue::Str(unescape_js_string(unquoted)));
    }
    if let Ok(n) = src.parse::<f64>() {
        return Some(JsValue::Num(n));
    }
    None
}

/// Parse a JS array literal of literals: `["a", "b", 3]`.
pub(crate) fn parse_array_literal(src: &str) -> Option<Vec<crate::js_interp::JsValue>> {
    let src = src.trim().trim_end_matches(';').trim();
    if !src.starts_with('[') || !src.ends_with(']') {
        return None;
    }
    let inner = &src[1..src.len() - 1];
    let parts = split_top_level_commas(inner);
    let mut out = Vec::with_capacity(parts.len());
    for p in parts {
        let v = parse_simple_literal(p.trim())?;
        out.push(v);
    }
    Some(out)
}

/// Locate the index of `=` for a plain assignment, skipping `==`, `===`,
/// `!=`, `<=`, `>=`, `+=`, and friends.
pub(crate) fn find_assignment(stmt: &str) -> Option<usize> {
    let bytes: Vec<char> = stmt.chars().collect();
    let mut depth = 0i32;
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            } else if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '=' if depth == 0 => {
                let prev = if i > 0 { bytes[i - 1] } else { ' ' };
                let next = bytes.get(i + 1).copied();
                if next == Some('=') {
                    i += 2;
                    continue;
                }
                if matches!(prev, '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^') {
                    i += 1;
                    continue;
                }
                return Some(i);
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Find a compound assignment (`+=`, `-=`, `^=`, ...) and return
/// `(lhs, op, rhs)` where `op` is the underlying binary operator.
pub(crate) fn find_compound_assignment(stmt: &str) -> Option<(&str, &str, &str)> {
    for &(sym, _) in &[
        ("+=", "+"), ("-=", "-"), ("*=", "*"), ("/=", "/"), ("%=", "%"),
        ("&=", "&"), ("|=", "|"), ("^=", "^"),
    ] {
        if let Some(idx) = stmt.find(sym) {
            let lhs = stmt[..idx].trim();
            let rhs = stmt[idx + sym.len()..].trim();
            if !lhs.is_empty() && !rhs.is_empty() {
                let op = &sym[..sym.len() - 1];
                return Some((lhs, op, rhs));
            }
        }
    }
    None
}

/// Locate the top-level `.` that separates a member access, returning its
/// byte index.
pub(crate) fn find_member_dot(expr: &str) -> Option<usize> {
    let bytes: Vec<char> = expr.chars().collect();
    let mut depth = 0i32;
    let mut last_dot = None;
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            } else if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '.' if depth == 0 => {
                let prev = if i > 0 { bytes[i - 1] } else { ' ' };
                let next = bytes.get(i + 1).copied();
                if prev.is_ascii_digit() && next.is_some_and(|c| c.is_ascii_digit()) {
                    // part of a float
                } else {
                    last_dot = Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    last_dot
}

/// Find the index of the first top-level `[` whose matching `]` ends `expr`.
pub(crate) fn find_top_level_open_bracket(expr: &str) -> Option<usize> {
    let bytes: Vec<char> = expr.chars().collect();
    let mut depth = 0i32;
    let mut candidate = None;
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            } else if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '[' => {
                if depth == 0 {
                    candidate = Some(i);
                }
                depth += 1;
            }
            ']' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    candidate
}

/// Split an expression at the lowest-precedence top-level binary operator
/// present, returning `(lhs, op, rhs)`.
pub(crate) fn split_binary(expr: &str) -> Option<(&str, &str, &str)> {
    let candidates: &[&str] = &[
        "||", "&&", "|", "^", "&", "===", "!==", "==", "!=", ">>>", "<<", ">>",
        "<=", ">=", "<", ">", "+", "-", "*", "/", "%",
    ];
    let bytes: Vec<char> = expr.chars().collect();
    for op in candidates {
        if let Some(split) = split_at_top_level_custom(expr, op, &bytes) {
            return Some(split);
        }
    }
    None
}

fn split_at_top_level_custom<'a>(
    expr: &'a str,
    op: &'a str,
    bytes: &[char],
) -> Option<(&'a str, &'a str, &'a str)> {
    let mut depth = 0i32;
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            } else if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {
                if depth == 0 && i > 0 {
                    let end = (i + op.len()).min(bytes.len());
                    let slice: String = bytes[i..end].iter().collect();
                    if slice == *op {
                        // Skip backward over whitespace to find the real
                        // "previous significant character".
                        let mut p = i;
                        while p > 0 && bytes[p - 1].is_whitespace() {
                            p -= 1;
                        }
                        if p == 0 {
                            // Operator at the very start: must be unary.
                            i += 1;
                            continue;
                        }
                        let prev = bytes[p - 1];
                        // Avoid splitting unary minus / sign after operator.
                        if prev.is_ascii_digit()
                            || prev == ')'
                            || prev == ']'
                            || prev == '\''
                            || prev == '"'
                            || prev.is_alphabetic()
                            || prev == '_'
                            || prev == '$'
                        {
                            let lhs = expr[..i].trim();
                            let rhs = expr[i + op.len()..].trim();
                            if !lhs.is_empty() && !rhs.is_empty() {
                                return Some((lhs, op, rhs));
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Split at `||` / `&&` / `??`.
pub(crate) fn split_logical(expr: &str) -> Option<(&str, &str, &str)> {
    for op in &["||", "&&", "??"] {
        if let Some((l, op_found, r)) = split_at_top_level_simple(expr, op) {
            return Some((l, op_found, r));
        }
    }
    None
}

fn split_at_top_level_simple<'a>(
    expr: &'a str,
    op: &'a str,
) -> Option<(&'a str, &'a str, &'a str)> {
    let bytes: Vec<char> = expr.chars().collect();
    let mut depth = 0i32;
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            } else if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {
                if depth == 0 && i > 0 {
                    let end = (i + op.len()).min(bytes.len());
                    let slice: String = bytes[i..end].iter().collect();
                    if slice == *op {
                        let mut p = i;
                        while p > 0 && bytes[p - 1].is_whitespace() {
                            p -= 1;
                        }
                        if p == 0 {
                            i += 1;
                            continue;
                        }
                        let prev = bytes[p - 1];
                        if prev.is_alphanumeric()
                            || prev == ')'
                            || prev == ']'
                            || prev == '\''
                            || prev == '"'
                        {
                            return Some((expr[..i].trim(), op, expr[i + op.len()..].trim()));
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Split a ternary `cond ? a : b`.
pub(crate) fn split_ternary(expr: &str) -> Option<(&str, &str, &str)> {
    let bytes: Vec<char> = expr.chars().collect();
    let mut depth = 0i32;
    let mut i = 0;
    let mut in_str: Option<char> = None;
    let mut question_idx = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            } else if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '?' if depth == 0 => {
                question_idx = Some(i);
                break;
            }
            _ => {}
        }
        i += 1;
    }
    let q = question_idx?;
    // Find the matching `:` at depth 0 after the `?`.
    let mut d = 0i32;
    let mut j = q + 1;
    let mut in_str2: Option<char> = None;
    while j < bytes.len() {
        let c = bytes[j];
        if let Some(s) = in_str2 {
            if c == s {
                in_str2 = None;
            } else if c == '\\' && j + 1 < bytes.len() {
                j += 2;
                continue;
            }
            j += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str2 = Some(c),
            '(' | '[' | '{' => d += 1,
            ')' | ']' | '}' => d -= 1,
            ':' if d == 0 => {
                let cond = expr[..q].trim();
                let a = expr[q + 1..j].trim();
                let b = expr[j + 1..].trim();
                return Some((cond, a, b));
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// Parse `callee(arg1, arg2, ...)` returning `(callee, args)`. Handles
/// nested parentheses inside arguments.
pub(crate) fn parse_call(expr: &str) -> Option<(String, Vec<&str>)> {
    let open = expr.find('(')?;
    let callee = expr[..open].trim().to_string();
    if callee.is_empty() {
        return None;
    }
    let bytes: Vec<char> = expr[open..].chars().collect();
    let mut depth = 0i32;
    let mut close_local = None;
    let mut i = 0;
    let mut in_str: Option<char> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            } else if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_local = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let close = close_local?;
    let close_abs = open + close;
    let inner = &expr[open + 1..close_abs];
    let args = split_top_level_commas(inner);
    let args: Vec<&str> = args.into_iter().filter(|a| !a.trim().is_empty()).collect();
    Some((callee, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_top_level_keeps_strings() {
        let src = r#"var x = "a;b"; var y = "c";"#;
        let stmts = split_top_level_statements(src);
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].contains("\"a;b\""));
    }

    #[test]
    fn unescape_works() {
        assert_eq!(unescape_js_string(r#"a\nb"#), "a\nb");
        assert_eq!(unescape_js_string(r#"\u0041"#), "A");
    }

    #[test]
    fn parse_call_simple() {
        let (callee, args) = parse_call("foo(1, 2, 3)").expect("call");
        assert_eq!(callee, "foo");
        // Args are raw slices — callers trim when evaluating.
        assert_eq!(args.iter().map(|a| a.trim()).collect::<Vec<_>>(), vec!["1", "2", "3"]);
    }

    #[test]
    fn parse_call_nested() {
        let (callee, args) = parse_call("bar(a, f(1, 2), c)").expect("call");
        assert_eq!(callee, "bar");
        assert_eq!(args.len(), 3);
        assert_eq!(args[1].trim(), "f(1, 2)");
    }

    #[test]
    fn find_assignment_skips_equality() {
        assert!(find_assignment("if (a == b) foo()").is_none());
        assert!(find_assignment("x = 1").is_some());
    }

    #[test]
    fn find_compound_assignment_recognises_xor() {
        let (lhs, op, rhs) = find_compound_assignment("b[c] ^= b[d]").expect("matched");
        assert_eq!(lhs, "b[c]");
        assert_eq!(op, "^");
        assert_eq!(rhs, "b[d]");
    }
}

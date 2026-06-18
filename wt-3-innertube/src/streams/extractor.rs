//! Regex-driven extraction of the cipher program and n-sig function from a
//! `base.js` source string.

use std::collections::HashMap;

use once_cell::sync::OnceCell;
use regex::Regex;

use crate::client::InnerTubeClient;
use crate::error::{Error, Result};
use crate::js_interp::{build_cipher_program, CipherProgram};

/// Captured n-sig function: its declared name and body. Held inside an
/// `Arc` inside the player cache.
#[derive(Debug, Clone)]
pub struct NsigFn {
    pub name: String,
    pub body: String,
}

/// Fetch `https://www.youtube.com/iframe_api`, parse out the player URL it
/// redirects to, then return the resolved `base.js` URL.
pub async fn discover_player_js_url(http: &InnerTubeClient) -> Result<String> {
    let api_src = http
        .get_text("https://www.youtube.com/iframe_api")
        .await?;
    static PLAYER_RE: OnceCell<Regex> = OnceCell::new();
    let re = PLAYER_RE.get_or_init(|| {
        Regex::new(r#"(/s/player/[0-9a-f]{8,}/[a-zA-Z0-9_]+/base\.js)"#)
            .expect("player js regex")
    });
    if let Some(caps) = re.captures(&api_src) {
        return Ok(format!("https://www.youtube.com{}", &caps[1]));
    }
    Err(Error::cipher("could not discover player.js URL from iframe_api"))
}

/// Extract the cipher program from a `base.js` source string.
///
/// The decipher function is matched by any of:
///   - `<name>:function(a){a=a.split("")`
///   - `function <name>(a){a=a.split("")`
///   - `<name>=function(a){a=a.split("")`
///
/// We run two regexes (one for the `name:function`/`name=function` shapes,
/// one for the `function name` shape) because Rust's regex crate does not
/// support optional groups with distinct capture semantics.
pub fn extract_cipher_program(source: &str) -> Result<CipherProgram> {
    // Try `name:function(a){a=a.split("")` and `name=function(a){...}`.
    static DECIPHER_NAMED_RE: OnceCell<Regex> = OnceCell::new();
    let named_re = DECIPHER_NAMED_RE.get_or_init(|| {
        Regex::new(
            r#"(?m)([a-zA-Z_$][\w$]*)\s*[:=]\s*function\s*\([a-zA-Z_$][\w$]*\)\s*\{\s*[a-zA-Z_$][\w$]*\s*=\s*[a-zA-Z_$][\w$]*\.split\(""\)"#,
        )
        .expect("decipher named regex")
    });
    // Try `function name(a){a=a.split("")`.
    static DECIPHER_STANDALONE_RE: OnceCell<Regex> = OnceCell::new();
    let standalone_re = DECIPHER_STANDALONE_RE.get_or_init(|| {
        Regex::new(
            r#"(?m)function\s+([a-zA-Z_$][\w$]*)\s*\([a-zA-Z_$][\w$]*\)\s*\{\s*[a-zA-Z_$][\w$]*\s*=\s*[a-zA-Z_$][\w$]*\.split\(""\)"#,
        )
        .expect("decipher standalone regex")
    });
    let fn_name = if let Some(caps) = named_re.captures(source) {
        caps[1].to_string()
    } else if let Some(caps) = standalone_re.captures(source) {
        caps[1].to_string()
    } else {
        return Err(Error::cipher("could not locate decipher function name"));
    };

    // Find the function body. We search for a body-opening `{` whose
    // preceding context is one of the supported declaration shapes, then
    // scan to the matching `}`. Whitespace between `)` and `{` is allowed.
    //
    // We do *not* cache this regex in a `OnceCell` because the pattern
    // contains the function name, which varies per player build.
    let escaped = regex_escape(&fn_name);
    let body_pat = format!(
        r#"(?:{n}:function\s*\(|{n}=function\s*\(|function\s+{n}\s*\()[a-zA-Z_$][\w$]*\)\s*\{{"#,
        n = escaped,
    );
    let locator = Regex::new(&body_pat)
        .map_err(|e| Error::cipher(format!("decipher body regex: {e}")))?;
    let body_open_idx = locator
        .find(source)
        .ok_or_else(|| Error::cipher("could not locate decipher function body"))?
        .range()
        .end
        .checked_sub(1)
        .ok_or_else(|| Error::cipher("decipher body range underflow"))?;
    let body = brace_delimited(&source[body_open_idx..])?;

    let helpers = extract_helper_bodies(source)?;
    build_cipher_program(&body, &helpers).map_err(|e| {
        Error::cipher(format!("building cipher program for '{fn_name}': {e}"))
    })
}

/// Escape `s` for safe interpolation into a [`regex::Regex`].
fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if !c.is_alphanumeric() && c != '_' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Walk the player source looking for helper functions whose body
/// involves splice/reverse/swap. Recognises two declaration shapes:
///   - `function name(a, b) { ... }`
///   - `name:function(a, b) { ... }` or `name=function(a, b) { ... }`
///     (object-literal methods and var-assigned anonymous functions).
pub fn extract_helper_bodies(
    source: &str,
) -> Result<HashMap<String, (Vec<String>, String)>> {
    let mut out = HashMap::new();

    // Form 1: `function name(params) {`
    static HELPER_NAMED_RE: OnceCell<Regex> = OnceCell::new();
    let named_re = HELPER_NAMED_RE.get_or_init(|| {
        Regex::new(r#"(?m)function\s+([a-zA-Z_$][\w$]*)\s*\(([^)]*)\)\s*\{"#)
            .expect("helper named regex")
    });
    for caps in named_re.captures_iter(source) {
        let name = caps[1].to_string();
        let params: Vec<String> = caps[2]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let match_end = caps.get(0).expect("match 0").end();
        let open_brace = match_end - 1;
        if let Ok(body) = brace_delimited(&source[open_brace..]) {
            let stripped: String = body.split_whitespace().collect();
            if is_cipher_helper(&stripped) {
                out.insert(name, (params, body));
            }
        }
    }

    // Form 2: `name:function(params) {` or `name=function(params) {`.
    static HELPER_METHOD_RE: OnceCell<Regex> = OnceCell::new();
    let method_re = HELPER_METHOD_RE.get_or_init(|| {
        Regex::new(r#"(?m)([a-zA-Z_$][\w$]*)\s*[:=]\s*function\s*\(([^)]*)\)\s*\{"#)
            .expect("helper method regex")
    });
    for caps in method_re.captures_iter(source) {
        let name = caps[1].to_string();
        let params: Vec<String> = caps[2]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let match_end = caps.get(0).expect("match 0").end();
        let open_brace = match_end - 1;
        if let Ok(body) = brace_delimited(&source[open_brace..]) {
            let stripped: String = body.split_whitespace().collect();
            if is_cipher_helper(&stripped) {
                out.entry(name).or_insert((params, body));
            }
        }
    }

    Ok(out)
}

/// Heuristic: does this minified body look like a cipher helper?
fn is_cipher_helper(stripped: &str) -> bool {
    stripped.contains(".splice(0,")
        || stripped.contains(".reverse()")
        || (stripped.contains("[0]=a[") && stripped.contains("=c"))
        || (stripped.contains("[0]=a.length") && stripped.contains("=c"))
}

/// Extract the n-sig function name and body from a `base.js` source string.
pub fn extract_nsig_fn(source: &str) -> Result<NsigFn> {
    static NSIG_NAME_RE: OnceCell<Regex> = OnceCell::new();
    let name_re = NSIG_NAME_RE.get_or_init(|| {
        // Whitespace-tolerant: matches `&& (b = name(arg))`.
        Regex::new(
            r#"\.get\(\s*"n"\s*\)\s*\)\s*&&\s*\(\s*b\s*=\s*([a-zA-Z_$][\w$]*)\s*\(\s*([a-zA-Z_$][\w$]*)\s*\)"#,
        )
        .expect("nsig name regex")
    });
    let caps = name_re
        .captures(source)
        .ok_or_else(|| Error::cipher("could not locate n-sig function name"))?;
    let fn_name = caps[1].to_string();
    let param = caps[2].to_string();

    // The function declaration is one of:
    //   `function name(param) { ... }`            (standalone)
    //   `name:function(param) { ... }`            (object method)
    //   `name=function(param) { ... }`            (assignment)
    //   `var name=function(param) { ... }`        (var assignment)
    //
    // We use a regex with `\s*` between `)` and `{` so whitespace-tolerant
    // matching works regardless of how the fixture/player formats the
    // source.
    let escaped = regex_escape(&fn_name);
    let escaped_p = regex_escape(&param);
    let pat = format!(
        r#"(?:{n}:function\s*\(|{n}=function\s*\(|function\s+{n}\s*\(){p}\)\s*\{{"#,
        n = escaped,
        p = escaped_p,
    );
    let locator = Regex::new(&pat).map_err(|e| Error::cipher(format!("nsig body regex: {e}")))?;
    let body_open_idx = locator
        .find(source)
        .ok_or_else(|| Error::cipher("could not locate n-sig function body"))?
        .range()
        .end
        .checked_sub(1)
        .ok_or_else(|| Error::cipher("n-sig body range underflow"))?;
    let body = brace_delimited(&source[body_open_idx..])?;
    Ok(NsigFn {
        name: fn_name,
        body,
    })
}

/// Pull the contents of a `{...}` block, returning the inside (without the
/// outer braces). Respects string literals.
pub(crate) fn brace_delimited(src: &str) -> Result<String> {
    if !src.starts_with('{') {
        return Err(Error::cipher("brace_delimited: src must start with '{'"));
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
                    return Ok(consumed[1..consumed.len() - 1].to_string());
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
    Err(Error::cipher("unterminated brace block"))
}

// Re-export so other modules can hold an Arc<CipherProgram>.
#[allow(unused_imports)]
pub(crate) use std::sync::Arc as _ArcMarker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_delimited_basic() {
        let body = brace_delimited("{a; b;}").expect("body");
        assert_eq!(body, "a; b;");
    }

    #[test]
    fn brace_delimited_nested() {
        let body = brace_delimited("{a {b} c}").expect("body");
        assert_eq!(body, "a {b} c");
    }

    #[test]
    fn brace_delimited_with_strings() {
        let body = brace_delimited(r#"{a = "}"; return a}"#).expect("body");
        assert_eq!(body, r#"a = "}"; return a"#);
    }
}

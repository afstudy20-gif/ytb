//! Query-string helpers and `signatureCipher` parsing.

use crate::error::Result;

/// Decode a `signatureCipher` query string into its three components:
/// `(s, sp, url)`.
///
/// Format: `s=<ciphered>&sp=<param name>&url=<unciphered URL>`.
pub(crate) fn parse_signature_cipher(sc: &str) -> Option<(String, String, String)> {
    let mut s = None;
    let mut sp = None;
    let mut url = None;
    for part in sc.split('&') {
        if let Some(val) = part.strip_prefix("s=") {
            s = Some(val.to_string());
        } else if let Some(val) = part.strip_prefix("sp=") {
            sp = Some(val.to_string());
        } else if let Some(val) = part.strip_prefix("url=") {
            url = Some(val.to_string());
        }
    }
    Some((s?, sp.unwrap_or_else(|| "signature".to_string()), url?))
}

/// Extract the first value of a query parameter from a URL.
pub(crate) fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let q = url.split_once('?').map(|(_, q)| q).unwrap_or(url);
    for pair in q.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(percent_decode(v));
            }
        }
    }
    None
}

/// Replace (or append) the value of a query parameter.
pub(crate) fn replace_query_param(url: &str, key: &str, value: &str) -> String {
    let (base, query) = url.split_once('?').unwrap_or((url, ""));
    let mut kept: Vec<String> = Vec::new();
    let mut found = false;
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        if let Some((k, _)) = pair.split_once('=') {
            if k == key {
                kept.push(format!("{key}={value}"));
                found = true;
                continue;
            }
        }
        kept.push(pair.to_string());
    }
    if !found {
        kept.push(format!("{key}={value}"));
    }
    if kept.is_empty() {
        base.to_string()
    } else {
        format!("{}?{}", base, kept.join("&"))
    }
}

/// Apply n-sig rewriting to a resolved URL, in place. If the URL has no `n`
/// param the original is returned unchanged.
pub(crate) async fn rewrite_n_param(
    url: &str,
    resolver: &crate::streams::PlayerJsResolver,
) -> Result<String> {
    let Some(n) = extract_query_param(url, "n") else {
        return Ok(url.to_string());
    };
    let transformed = resolver.transform_n(&n).await?;
    Ok(replace_query_param(url, "n", &transformed))
}

/// Minimal percent-decoding of a URL parameter value.
fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                out.push(byte as char);
            } else {
                out.push('%');
                out.push_str(&hex);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_signature_cipher_basic() {
        // Real signature cipher: `&` separates fields, `s` is the
        // percent-encoded value. We don't percent-decode `s` here because
        // the decipher step needs the raw chars.
        let sc = "s=AB%3Dx&sp=signature&url=https://example.com/v";
        let (s, sp, url) = parse_signature_cipher(sc).expect("parsed");
        assert_eq!(s, "AB%3Dx");
        assert_eq!(sp, "signature");
        assert_eq!(url, "https://example.com/v");
    }

    #[test]
    fn extract_query_param_basic() {
        let url = "https://x/y?a=1&n=ABC&b=2";
        assert_eq!(extract_query_param(url, "n"), Some("ABC".to_string()));
        assert_eq!(extract_query_param(url, "z"), None);
    }

    #[test]
    fn replace_query_param_replaces() {
        let url = "https://x/y?a=1&n=OLD&b=2";
        let out = replace_query_param(url, "n", "NEW");
        assert!(out.contains("n=NEW"));
        assert!(!out.contains("n=OLD"));
    }

    #[test]
    fn replace_query_param_appends() {
        let url = "https://x/y?a=1";
        let out = replace_query_param(url, "n", "NEW");
        assert!(out.contains("n=NEW"));
    }

    #[test]
    fn percent_decode_works() {
        // Direct call via the public API to keep coverage tight.
        let url = "https://x/y?a=hello%20world";
        assert_eq!(extract_query_param(url, "a"), Some("hello world".to_string()));
    }
}

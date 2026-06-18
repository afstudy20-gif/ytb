//! Cipher extractor + n-sig extractor tests against captured player JS
//! fixtures. These exercise the regex extractor and the resulting cipher
//! program / n-sig function execution end-to-end against realistic
//! (synthetic) player JS.

#![cfg(test)]

use std::collections::HashMap;
use std::fs;

use innertube::js_interp::{build_cipher_program, Interp, JsValue};
use innertube::streams::extractor::{extract_cipher_program, extract_helper_bodies, extract_nsig_fn};

/// Load a fixture as a string.
fn fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{name}");
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read fixture {path}: {e}"))
}

/// Helper: load a fixture, extract its cipher program, and apply it to an
/// input string. Returns the deciphered string.
fn decipher_with_fixture(fixture_name: &str, input: &str) -> String {
    let src = fixture(fixture_name);
    let program = extract_cipher_program(&src).expect("cipher program extracts");
    program.apply(input.to_string())
}

#[test]
fn fixture_a_cipher_program_extracts_and_runs() {
    // The fixture's decipher body is:
    //   a.split(""); qza(a, 3); Opa(a, 17); rca(a); Opa(a, 2); join
    // i.e. splice(3), swap(17 % len), reverse, swap(2 % len).
    let src = fixture("base_js_sample_a.js");
    let program = extract_cipher_program(&src).expect("extracts");
    assert_eq!(program.len(), 4, "expected 4 ops");

    // Manual reference computation against the same fixture.
    let helpers = extract_helper_bodies(&src).expect("helpers");
    // The sample fixture must have classified at least swap/splice/reverse.
    assert!(helpers.contains_key("Opa"));
    assert!(helpers.contains_key("qza"));
    assert!(helpers.contains_key("rca"));
    // Make sure build_cipher_program also produces a non-empty program for
    // the same helpers.
    let body = r#"a=a.split("");Xva.qza(a,3);Xva.Opa(a,17);Xva.rca(a);Xva.Opa(a,2);return a.join("")"#;
    let _p = build_cipher_program(body, &helpers).expect("builds");

    // End-to-end: apply to a known signature and check determinism.
    let out1 = decipher_with_fixture("base_js_sample_a.js", "0123456789abcdef");
    let out2 = decipher_with_fixture("base_js_sample_a.js", "0123456789abcdef");
    assert_eq!(out1, out2, "decipher must be deterministic");
    assert_ne!(out1, "0123456789abcdef", "decipher must change the input");
}

#[test]
fn fixture_b_cipher_program_extracts_and_runs() {
    let src = fixture("base_js_sample_b.js");
    let program = extract_cipher_program(&src).expect("extracts");
    assert_eq!(program.len(), 4, "expected 4 ops");
    let out = program.apply("abcdefghijklmnop".to_string());
    assert_ne!(out, "abcdefghijklmnop");
}

#[test]
fn fixture_a_nsig_function_extracts_and_runs() {
    let src = fixture("base_js_sample_a.js");
    let nsig = extract_nsig_fn(&src).expect("n-sig extracts");
    assert_eq!(nsig.name, "mwa");

    let mut interp = Interp::new();
    interp.register_function(&nsig.name, vec!["a".to_string()], nsig.body.clone());
    // Run twice on the same input — must be deterministic.
    let a1 = interp
        .call("mwa", &[JsValue::Str("ABCDEFGH".to_string())])
        .expect("call 1")
        .into_string()
        .expect("string");
    let a2 = interp
        .call("mwa", &[JsValue::Str("ABCDEFGH".to_string())])
        .expect("call 2")
        .into_string()
        .expect("string");
    assert_eq!(a1, a2);
    // Body: swap first and last chars, then bump the new last by 1.
    // Input "ABCDEFGH": first='A'(65), last='H'(72).
    // After swap: b[0]='H', b[7]='A'.
    // After +1: b[7]='B'.
    // Result: "HBCDEFGB"
    assert_eq!(a1, "HBCDEFGB");
}

#[test]
fn fixture_b_nsig_function_extracts_and_runs() {
    let src = fixture("base_js_sample_b.js");
    let nsig = extract_nsig_fn(&src).expect("n-sig extracts");
    assert_eq!(nsig.name, "nsi");

    let mut interp = Interp::new();
    interp.register_function(&nsig.name, vec!["a".to_string()], nsig.body.clone());
    // Body: split, take first char's code, XOR with 1, rejoin.
    // Input "abc": first char 'a' = 97 = 0b1100001, XOR 1 = 0b1100000 = 96 = '`'.
    // Result: "`bc"
    let out = interp
        .call("nsi", &[JsValue::Str("abc".to_string())])
        .expect("call")
        .into_string()
        .expect("string");
    assert_eq!(out, "`bc");
}

#[test]
fn extract_helper_bodies_skips_non_cipher_helpers() {
    // Synthesize a source with one cipher-y helper and one unrelated
    // helper. Only the cipher one should be retained.
    let src = r#"
        function unrelated(a, b) {
            return a + b;
        }
        function cipher_swap(a, b) {
            var c = a[0]; a[0] = a[b]; a[b] = c;
        }
    "#;
    let helpers: HashMap<String, (Vec<String>, String)> = extract_helper_bodies(src).expect("ok");
    assert!(!helpers.contains_key("unrelated"));
    assert!(helpers.contains_key("cipher_swap"));
}

#[test]
fn cipher_program_empty_input_does_not_panic() {
    let src = fixture("base_js_sample_b.js");
    let program = extract_cipher_program(&src).expect("extracts");
    let out = program.apply(String::new());
    assert_eq!(out, "");
}

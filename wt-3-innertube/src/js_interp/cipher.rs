//! The high-level cipher program: recognising reverse/swap/splice ops in
//! the YouTube decipher function body without re-running JS.
//!
//! YouTube's signature cipher is always a sequence of three op kinds applied
//! to the characters of the signature string:
//!
//! 1. `reverse()` — reverse the working array.
//! 2. `splice(0, n)` — drop the first `n` elements.
//! 3. `swap(n)` — swap element 0 with element `n`.
//!
//! The ops are individually wrapped in tiny helper functions whose names
//! change every player build; their *shapes* are stable. We classify each
//! helper by inspecting its body and then walk the main decipher function
//! in source order, mapping each helper call into a [`CipherOp`].

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::js_interp::interp::Interp;
use crate::js_interp::lexer::{parse_call, split_top_level_statements};

/// One operation in a decipher program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherOp {
    /// Reverse the working array.
    Reverse,
    /// Remove the first `n` elements.
    Splice(u32),
    /// Swap the element at index 0 with the element at index `n`.
    Swap(u32),
}

/// A decipher program: the ordered sequence of ops that, when applied to the
/// characters of the cipher input, produces the deciphered signature.
#[derive(Debug, Clone)]
pub struct CipherProgram {
    ops: Vec<CipherOp>,
}

impl CipherProgram {
    /// Construct an empty program.
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Append an op.
    pub fn push(&mut self, op: CipherOp) {
        self.ops.push(op);
    }

    /// Number of ops.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Whether the program is empty.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Apply this program to a signature string, returning the deciphered
    /// signature.
    pub fn apply(&self, input: String) -> String {
        let mut chars: Vec<char> = input.chars().collect();
        for op in &self.ops {
            match *op {
                CipherOp::Reverse => chars.reverse(),
                CipherOp::Splice(n) => {
                    let n = (n as usize).min(chars.len());
                    chars.drain(0..n);
                }
                CipherOp::Swap(n) => {
                    // Match the JS cipher helper: `a[b % a.length]`. With
                    // an empty array there's nothing to swap.
                    if chars.is_empty() {
                        continue;
                    }
                    let n = (n as usize) % chars.len();
                    if n > 0 {
                        chars.swap(0, n);
                    }
                }
            }
        }
        chars.into_iter().collect()
    }
}

impl Default for CipherProgram {
    fn default() -> Self {
        Self::new()
    }
}

/// Inspect a single cipher helper function body and classify it as a
/// [`CipherOp`] template. `n` is set to 0 here; the real value comes from
/// the call site in the decipher function (see [`build_cipher_program`]).
fn classify_helper(body: &str) -> Result<CipherOp> {
    let stripped: String = body.split_whitespace().collect();
    if stripped.contains(".reverse()") {
        return Ok(CipherOp::Reverse);
    }
    if stripped.contains(".splice(0,") {
        return Ok(CipherOp::Splice(0));
    }
    // Swap helpers come in a few shapes. The defining feature is an
    // assignment that swaps a[0] with some other index, e.g.
    // `var c=a[0];a[0]=a[b];a[b]=c;`.
    if (stripped.contains("[0]=a[") && stripped.contains("=c"))
        || (stripped.contains("[0]=a.length") || stripped.contains("[b]=a[0]"))
        || (stripped.contains("[a.length-") && stripped.contains("[0]="))
    {
        return Ok(CipherOp::Swap(0));
    }
    Err(Error::cipher(format!(
        "unrecognised cipher helper body: '{body}'"
    )))
}

/// Given the cipher function body and a map of helper-name -> helper-source,
/// produce a [`CipherProgram`].
///
/// `cipher_fn_body` is the body of the cipher function (between `{` and `}`),
/// e.g.:
/// ```text
/// var a = a.split("");
/// HFa(a, 5);
/// qza(a, 87);
/// a.reverse();
/// return a.join("")
/// ```
/// `helpers` maps each helper name to `(params, body)`.
pub fn build_cipher_program(
    cipher_fn_body: &str,
    helpers: &HashMap<String, (Vec<String>, String)>,
) -> Result<CipherProgram> {
    let mut program = CipherProgram::new();
    let mut interpreter = Interp::new();
    for raw_stmt in split_top_level_statements(cipher_fn_body) {
        let stmt = raw_stmt.trim().trim_end_matches(';').trim();
        if stmt.is_empty() || stmt.starts_with("return ") {
            continue;
        }
        // Helper call: `HFa(a, 5)` or `Obj.HFa(a, 5)` — callee (or its
        // last dotted segment) matches a known helper name.
        if let Some((callee, args)) = parse_call(stmt) {
            let bare_callee = callee.rsplit('.').next().unwrap_or(callee.as_str());
            let lookup = helpers
                .get(&callee)
                .or_else(|| helpers.get(bare_callee));
            if let Some((params, body)) = lookup {
                let template = classify_helper(body)?;
                let final_op = resolve_op_arg(template, &args, params, &mut interpreter, stmt)?;
                program.push(final_op);
                continue;
            }
            // Direct `a.reverse()` or `Obj.a.reverse()` inside the body.
            if callee.ends_with(".reverse") {
                program.push(CipherOp::Reverse);
                continue;
            }
        }
        // Direct `a.reverse();` shape without a helper.
        if stmt.replace(' ', "") == "a.reverse()" {
            program.push(CipherOp::Reverse);
            continue;
        }
        // Direct `a.splice(0, n);` shape.
        if stmt.replace(' ', "").starts_with("a.splice(") {
            if let Some((_, args)) = parse_call(stmt) {
                if let Some(n_str) = args.get(1) {
                    if let Ok(n) = n_str.trim().parse::<u32>() {
                        program.push(CipherOp::Splice(n));
                        continue;
                    }
                }
            }
        }
        // Ignore anything we don't recognise rather than failing; the cipher
        // function occasionally has side-effect-free lines we don't care
        // about.
        tracing::trace!(stmt = stmt, "skipping unrecognised cipher stmt");
    }
    if program.is_empty() {
        return Err(Error::cipher("cipher program is empty"));
    }
    Ok(program)
}

/// Resolve the numeric argument for a splice/swap op, evaluating it via the
/// interpreter if it isn't a plain literal.
fn resolve_op_arg(
    template: CipherOp,
    call_args: &[&str],
    _params: &[String],
    interp: &mut Interp,
    stmt_for_error: &str,
) -> Result<CipherOp> {
    match template {
        CipherOp::Reverse => Ok(CipherOp::Reverse),
        CipherOp::Splice(_) | CipherOp::Swap(_) => {
            let n_str = call_args.get(1).ok_or_else(|| {
                Error::cipher(format!(
                    "cipher call missing numeric arg in '{stmt_for_error}'"
                ))
            })?;
            let n_str = n_str.trim();
            if let Ok(n) = n_str.parse::<u32>() {
                return Ok(match template {
                    CipherOp::Splice(_) => CipherOp::Splice(n),
                    CipherOp::Swap(_) => CipherOp::Swap(n),
                    other => other,
                });
            }
            // Evaluate a complex expression like `c.length - 1`.
            let value = interp.run_expr(n_str, &mut HashMap::new())?;
            let n = value
                .as_int()
                .map(|i| i.max(0) as u32)
                .map_err(|e| Error::cipher(format!("cipher arg eval failed: {e}")))?;
            Ok(match template {
                CipherOp::Splice(_) => CipherOp::Splice(n),
                CipherOp::Swap(_) => CipherOp::Swap(n),
                other => other,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cipher_program_reverse_swap_splice() {
        let mut p = CipherProgram::new();
        p.push(CipherOp::Splice(2));
        p.push(CipherOp::Swap(1));
        p.push(CipherOp::Reverse);
        // "abcdef"
        //   -> Splice(2) drops first 2 chars: "cdef"  (chars: c,d,e,f)
        //   -> Swap(1) swaps index 0 with index 1: "dcef" (chars: d,c,e,f)
        //   -> Reverse: "fecd"
        assert_eq!(p.apply("abcdef".to_string()), "fecd");
    }

    #[test]
    fn cipher_program_handles_out_of_range_args() {
        let mut p = CipherProgram::new();
        // Swap(100) on "abc": real JS cipher does a[b % a.length] so
        // 100 % 3 = 1, swap(0, 1) gives "bac".
        p.push(CipherOp::Swap(100));
        assert_eq!(p.apply("abc".to_string()), "bac");
    }

    #[test]
    fn build_cipher_program_with_helpers() {
        let mut helpers = HashMap::new();
        helpers.insert(
            "HFa".to_string(),
            (
                vec!["a".to_string(), "b".to_string()],
                "var c=a[0];a[0]=a[b%a.length];a[b%a.length]=c".to_string(),
            ),
        );
        helpers.insert(
            "qza".to_string(),
            (
                vec!["a".to_string(), "b".to_string()],
                "a.splice(0,b)".to_string(),
            ),
        );
        let body = r#"
            var a = a.split("");
            qza(a, 2);
            HFa(a, 5);
            a.reverse();
            return a.join("")
        "#;
        let program = build_cipher_program(body, &helpers).expect("program");
        assert_eq!(program.len(), 3);
        // Trace on "abcdef":
        //   qza(a, 2)  -> Splice(2)        : "cdef"      (chars: c,d,e,f)
        //   HFa(a, 5)  -> Swap(5 % 4 = 1)  : swap(0,1)   : "dcef"      (chars: d,c,e,f)
        //   a.reverse() -> Reverse         : "fecd"
        assert_eq!(program.apply("abcdef".to_string()), "fecd");
    }

    #[test]
    fn classify_helper_reverse() {
        let body = "a.reverse()";
        assert_eq!(classify_helper(body).unwrap(), CipherOp::Reverse);
    }

    #[test]
    fn classify_helper_splice() {
        let body = "a.splice(0, b)";
        assert_eq!(classify_helper(body).unwrap(), CipherOp::Splice(0));
    }

    #[test]
    fn classify_helper_swap() {
        let body = "var c=a[0];a[0]=a[b];a[b]=c";
        assert_eq!(classify_helper(body).unwrap(), CipherOp::Swap(0));
    }
}

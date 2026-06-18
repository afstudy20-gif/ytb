//! The n-sig interpreter: executes the JS functions YouTube uses for the `n`
//! parameter transformation.
//!
//! Unlike the cipher program, the n-sig function varies heavily between
//! player builds and is obfuscated, so we can't pattern-match it. Instead we
//! execute it with a small mutation-aware interpreter that supports the
//! subset described in the module-level docs.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::js_interp::lexer::{
    find_assignment, find_compound_assignment, find_member_dot,
    find_top_level_open_bracket, is_identifier, parse_anon_function, parse_array_literal,
    parse_call, parse_named_function, parse_simple_literal, split_logical,
    split_top_level_statements, split_ternary,
};
use crate::js_interp::ops::{
    apply_binary, eval_math, eval_method_on, eval_string_from_char_code, get_index, get_member,
};
use crate::js_interp::value::{is_truthy, JsValue};

/// A first-class function in the interpreter: parameter list + raw source
/// body. Re-interpreted on every call (cheap; the functions are tiny).
#[derive(Debug, Clone)]
struct JsFunction {
    params: Vec<String>,
    body: String,
}

/// The interpreter. Holds global functions and helper-table globals.
pub struct Interp {
    functions: HashMap<String, JsFunction>,
    globals: HashMap<String, JsValue>,
}

impl std::fmt::Debug for Interp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interp")
            .field("fns", &self.functions.keys().collect::<Vec<_>>())
            .field("globals", &self.globals.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for Interp {
    fn default() -> Self {
        Self::new()
    }
}

impl Interp {
    /// Construct an empty interpreter.
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            globals: HashMap::new(),
        }
    }

    /// Load top-level definitions from a JS source fragment. Recognised
    /// forms: `function name(...) { ... }`, `var name = function(...) {
    /// ... }`, `var name = [ ... ];`, `var name = <literal>;`. Anything
    /// else is ignored.
    #[allow(dead_code)]
    pub fn load(&mut self, src: &str) -> Result<()> {
        for stmt in split_top_level_statements(src) {
            let trimmed = stmt.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("function ") {
                if let Some((name, params, body)) = parse_named_function(rest)? {
                    self.functions.insert(name, JsFunction { params, body });
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("var ") {
                if let Some(eq) = rest.find('=') {
                    let name = rest[..eq].trim().to_string();
                    let rhs = rest[eq + 1..].trim();
                    if let Some(body_src) = rhs.strip_prefix("function").map(str::trim_start) {
                        if let Some((_, params, body)) = parse_anon_function(body_src)? {
                            self.functions.insert(name.clone(), JsFunction { params, body });
                            continue;
                        }
                    }
                    if let Some(arr) = parse_array_literal(rhs) {
                        self.globals.insert(name, JsValue::arr(arr));
                        continue;
                    }
                    if let Some(v) = parse_simple_literal(rhs) {
                        self.globals.insert(name, v);
                        continue;
                    }
                }
            }
            // Ignore anything else.
        }
        Ok(())
    }

    /// Register a function directly. Used by `streams.rs` to install the
    /// n-sig function once its declaration has been sliced out of the
    /// player source.
    pub fn register_function(&mut self, name: &str, params: Vec<String>, body: String) {
        self.functions.insert(name.to_string(), JsFunction { params, body });
    }

    /// Look up a function by name.
    #[allow(dead_code)]
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Call a named function with positional arguments.
    pub fn call(&mut self, name: &str, args: &[JsValue]) -> Result<JsValue> {
        let func = self
            .functions
            .get(name)
            .ok_or_else(|| Error::cipher(format!("unknown function '{name}'")))?
            .clone();
        self.call_function(&func, args)
    }

    fn call_function(&mut self, func: &JsFunction, args: &[JsValue]) -> Result<JsValue> {
        if args.len() != func.params.len() {
            return Err(Error::cipher(format!(
                "function expected {} args, got {}",
                func.params.len(),
                args.len()
            )));
        }
        let mut scope: HashMap<String, JsValue> = HashMap::new();
        for (param, arg) in func.params.iter().zip(args.iter()) {
            scope.insert(param.clone(), arg.clone());
        }
        self.run_block(&func.body, &mut scope)
    }

    /// Execute a block of statements. Returns the value of the last
    /// evaluated statement or the explicit `return`.
    fn run_block(
        &mut self,
        body: &str,
        scope: &mut HashMap<String, JsValue>,
    ) -> Result<JsValue> {
        let mut last = JsValue::Undef;
        for raw in split_top_level_statements(body) {
            let stmt = raw.trim();
            if stmt.is_empty() {
                continue;
            }
            if let Some(expr) = stmt.strip_prefix("return ") {
                let value = self.run_expr(expr.trim().trim_end_matches(';'), scope)?;
                return Ok(value);
            }
            if stmt == "return;" || stmt == "return" {
                return Ok(JsValue::Undef);
            }
            last = self.run_stmt(stmt, scope)?;
        }
        Ok(last)
    }

    /// Execute a single statement (assignment, declaration, or expression).
    fn run_stmt(
        &mut self,
        stmt: &str,
        scope: &mut HashMap<String, JsValue>,
    ) -> Result<JsValue> {
        let stmt = stmt.trim().trim_end_matches(';').trim();
        if stmt.is_empty() {
            return Ok(JsValue::Undef);
        }
        if let Some(rest) = stmt.strip_prefix("var ") {
            if let Some(eq) = rest.find('=') {
                let name = rest[..eq].trim().to_string();
                let expr_src = rest[eq + 1..].trim();
                let value = self.run_expr(expr_src, scope)?;
                scope.insert(name, value);
                return Ok(JsValue::Undef);
            }
            let name = rest.trim();
            scope.insert(name.to_string(), JsValue::Undef);
            return Ok(JsValue::Undef);
        }
        if let Some(eq) = find_assignment(stmt) {
            let lhs = stmt[..eq].trim();
            let rhs = stmt[eq + 1..].trim();
            let value = self.run_expr(rhs, scope)?;
            self.assign(lhs, value, scope)?;
            return Ok(JsValue::Undef);
        }
        if let Some((lhs, op, rhs)) = find_compound_assignment(stmt) {
            let current = self.run_expr(lhs, scope)?;
            let other = self.run_expr(rhs, scope)?;
            let combined = apply_binary(op, current, other)?;
            self.assign(lhs, combined, scope)?;
            return Ok(JsValue::Undef);
        }
        let v = self.run_expr(stmt, scope)?;
        Ok(v)
    }

    fn assign(
        &mut self,
        lhs: &str,
        value: JsValue,
        scope: &mut HashMap<String, JsValue>,
    ) -> Result<()> {
        if is_identifier(lhs) {
            scope.insert(lhs.to_string(), value);
            return Ok(());
        }
        if lhs.ends_with(']') {
            if let Some(open) = find_top_level_open_bracket(lhs) {
                let obj_name = lhs[..open].trim();
                let idx_src = &lhs[open + 1..lhs.len() - 1];
                let idx = self.run_expr(idx_src, scope)?;
                let obj = scope
                    .get_mut(obj_name)
                    .ok_or_else(|| Error::cipher(format!("assign to missing var '{obj_name}'")))?;
                let i = idx.as_int()? as usize;
                if let JsValue::Arr(a) = obj {
                    let mut borrowed = a.borrow_mut();
                    if i < borrowed.len() {
                        borrowed[i] = value;
                    }
                    return Ok(());
                }
                return Err(Error::cipher("cannot index-assign into non-array"));
            }
        }
        Err(Error::cipher(format!("unsupported lhs '{lhs}'")))
    }

    /// Evaluate an expression.
    pub(crate) fn run_expr(
        &mut self,
        expr: &str,
        scope: &mut HashMap<String, JsValue>,
    ) -> Result<JsValue> {
        let expr = expr.trim();
        if expr.is_empty() {
            return Ok(JsValue::Undef);
        }
        if let Some(v) = parse_simple_literal(expr) {
            return Ok(v);
        }
        if let Some(v) = parse_array_literal(expr) {
            return Ok(JsValue::arr(v));
        }
        if is_identifier(expr) {
            return self.lookup(expr, scope);
        }
        if let Some((lhs, op, rhs)) = crate::js_interp::lexer::split_binary(expr) {
            let lhs_v = self.run_expr(lhs, scope)?;
            let rhs_v = self.run_expr(rhs, scope)?;
            return Ok(apply_binary(op, lhs_v, rhs_v)?);
        }
        if let Some((lhs, op, rhs)) = split_logical(expr) {
            let lhs_v = self.run_expr(lhs, scope)?;
            return Ok(self.apply_logical(op, lhs_v, rhs, scope)?);
        }
        if let Some((cond, a, b)) = split_ternary(expr) {
            let cond_v = self.run_expr(cond, scope)?;
            let truthy = is_truthy(&cond_v);
            return self.run_expr(if truthy { a } else { b }, scope);
        }
        if let Some((callee, args)) = parse_call(expr) {
            return self.eval_call(&callee, &args, scope);
        }
        if let Some(v) = self.eval_member(expr, scope)? {
            return Ok(v);
        }
        Err(Error::cipher(format!("cannot parse expression: '{expr}'")))
    }

    fn apply_logical(
        &mut self,
        op: &str,
        lhs: JsValue,
        rhs_src: &str,
        scope: &mut HashMap<String, JsValue>,
    ) -> Result<JsValue> {
        match op {
            "&&" => {
                if is_truthy(&lhs) {
                    self.run_expr(rhs_src, scope)
                } else {
                    Ok(lhs)
                }
            }
            "||" => {
                if is_truthy(&lhs) {
                    Ok(lhs)
                } else {
                    self.run_expr(rhs_src, scope)
                }
            }
            "??" => {
                if matches!(lhs, JsValue::Undef) {
                    self.run_expr(rhs_src, scope)
                } else {
                    Ok(lhs)
                }
            }
            _ => Err(Error::cipher(format!("unsupported logical '{op}'"))),
        }
    }

    fn lookup(&self, name: &str, scope: &HashMap<String, JsValue>) -> Result<JsValue> {
        if let Some(v) = scope.get(name) {
            return Ok(v.clone());
        }
        if let Some(v) = self.globals.get(name) {
            return Ok(v.clone());
        }
        Err(Error::cipher(format!("undefined variable '{name}'")))
    }

    fn eval_call(
        &mut self,
        callee: &str,
        args: &[&str],
        scope: &mut HashMap<String, JsValue>,
    ) -> Result<JsValue> {
        if self.functions.contains_key(callee) && !callee.contains('.') {
            let arg_vals: Vec<JsValue> = args
                .iter()
                .map(|a| self.run_expr(a, scope))
                .collect::<Result<Vec<_>>>()?;
            return self.call(callee, &arg_vals);
        }
        if let Some((obj, method)) = callee.rsplit_once('.') {
            if obj == "Math" {
                let argv: Vec<JsValue> = args
                    .iter()
                    .map(|a| self.run_expr(a, scope))
                    .collect::<Result<Vec<_>>>()?;
                return eval_math(method, &argv);
            }
            if obj == "String" && method == "fromCharCode" {
                let argv: Vec<JsValue> = args
                    .iter()
                    .map(|a| self.run_expr(a, scope))
                    .collect::<Result<Vec<_>>>()?;
                return eval_string_from_char_code(&argv);
            }
            return self.eval_method_call(obj, method, args, scope);
        }
        Err(Error::cipher(format!("unknown call '{callee}'")))
    }

    /// Evaluate a method call `receiver.method(args)`. Mutating methods
    /// (`reverse`, `splice`, `push`, `shift`, `sort`) write back to the
    /// scope variable when the receiver is a bare identifier.
    fn eval_method_call(
        &mut self,
        recv_src: &str,
        method: &str,
        args: &[&str],
        scope: &mut HashMap<String, JsValue>,
    ) -> Result<JsValue> {
        let argv: Vec<JsValue> = args
            .iter()
            .map(|a| self.run_expr(a, scope))
            .collect::<Result<Vec<_>>>()?;
        if is_identifier(recv_src) {
            let recv = self.lookup(recv_src, scope)?;
            let (result, new_recv) = eval_method_on(method, recv, &argv, true)?;
            if let Some(new_recv) = new_recv {
                scope.insert(recv_src.to_string(), new_recv);
            }
            Ok(result)
        } else {
            let recv = self.run_expr(recv_src, scope)?;
            let (result, _) = eval_method_on(method, recv, &argv, false)?;
            Ok(result)
        }
    }

    fn eval_member(
        &mut self,
        expr: &str,
        scope: &mut HashMap<String, JsValue>,
    ) -> Result<Option<JsValue>> {
        if let Some(dot) = find_member_dot(expr) {
            let obj_src = &expr[..dot];
            let prop = &expr[dot + 1..];
            if is_identifier(prop) {
                let obj = self.run_expr(obj_src, scope)?;
                return Ok(Some(get_member(&obj, prop)?));
            }
        }
        if expr.ends_with(']') {
            if let Some(open) = find_top_level_open_bracket(expr) {
                let obj_src = &expr[..open];
                let idx_src = &expr[open + 1..expr.len() - 1];
                let obj = self.run_expr(obj_src, scope)?;
                let idx = self.run_expr(idx_src, scope)?;
                return Ok(Some(get_index(&obj, &idx)?));
            }
        }
        Ok(None)
    }
}

/// Convenience constructor used in tests to install a function straight from
/// its declaration source (skipping the `load()` step).
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn interp_from_function_decl(src: &str) -> Result<Interp> {
    let mut interp = Interp::new();
    interp.load(src)?;
    Ok(interp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js_interp::lexer::extract_brace_body;

    #[test]
    fn interp_runs_reverse_via_method_call() {
        let mut interp = Interp::new();
        interp
            .load(
                r#"
                function decode(s) {
                    var a = s.split("");
                    a.reverse();
                    return a.join("");
                }
                "#,
            )
            .expect("load");
        let out = interp
            .call("decode", &[JsValue::Str("abc".to_string())])
            .expect("call");
        assert_eq!(out.into_string().unwrap(), "cba");
    }

    #[test]
    fn interp_runs_splice_mutation() {
        let mut interp = Interp::new();
        interp
            .load(
                r#"
                function decode(s) {
                    var a = s.split("");
                    a.splice(0, 2);
                    return a.join("");
                }
                "#,
            )
            .expect("load");
        let out = interp
            .call("decode", &[JsValue::Str("hello".to_string())])
            .expect("call");
        assert_eq!(out.into_string().unwrap(), "llo");
    }

    #[test]
    fn interp_runs_swap_via_helper() {
        let mut interp = Interp::new();
        interp
            .load(
                r#"
                function swap(a, b) {
                    var c = a[0];
                    a[0] = a[b];
                    a[b] = c;
                }
                function decode(s) {
                    var a = s.split("");
                    swap(a, 1);
                    return a.join("");
                }
                "#,
            )
            .expect("load");
        let out = interp
            .call("decode", &[JsValue::Str("abcdef".to_string())])
            .expect("call");
        assert_eq!(out.into_string().unwrap(), "bacdef");
    }

    #[test]
    fn interp_runs_bitwise_xor() {
        let mut interp = Interp::new();
        interp
            .load(
                r#"
                function f(a, b) { return a ^ b; }
                "#,
            )
            .expect("load");
        let out = interp
            .call("f", &[JsValue::Num(5.0), JsValue::Num(3.0)])
            .expect("call");
        assert_eq!(out.as_int().unwrap(), 6);
    }

    #[test]
    fn interp_runs_simple_n_sig() {
        let mut interp = Interp::new();
        interp
            .load(
                r#"
                function ntransform(s) {
                    var b = s.split("");
                    var c = b.length;
                    var d = b[0].charCodeAt(0);
                    b[0] = String.fromCharCode(d + 1);
                    return b.join("");
                }
                "#,
            )
            .expect("load");
        let out = interp
            .call("ntransform", &[JsValue::Str("hello".to_string())])
            .expect("call");
        // 'h' is 104 -> 105 -> 'i', so result is "iello".
        assert_eq!(out.into_string().unwrap(), "iello");
    }

    #[test]
    fn interp_runs_compound_xor_assign() {
        let mut interp = Interp::new();
        interp
            .load(
                r#"
                function f() {
                    var b = [1, 2, 3];
                    b[0] ^= b[1];
                    return b.join(",");
                }
                "#,
            )
            .expect("load");
        let out = interp.call("f", &[]).expect("call");
        // 1 ^ 2 = 3, so result is "3,2,3".
        assert_eq!(out.into_string().unwrap(), "3,2,3");
    }

    #[test]
    fn interp_runs_array_index_get() {
        let mut interp = Interp::new();
        interp
            .load(
                r#"
                function f(s) {
                    var b = s.split("");
                    return b[1];
                }
                "#,
            )
            .expect("load");
        let out = interp
            .call("f", &[JsValue::Str("hello".to_string())])
            .expect("call");
        assert_eq!(out.into_string().unwrap(), "e");
    }

    #[test]
    fn extract_brace_body_basic() {
        let body = extract_brace_body("{a; b;}").expect("body");
        assert_eq!(body, "a; b;");
    }

    #[test]
    fn extract_brace_body_nested() {
        let body = extract_brace_body("{a {b} c}").expect("body");
        assert_eq!(body, "a {b} c");
    }

    #[test]
    fn extract_brace_body_with_strings() {
        let body = extract_brace_body(r#"{a = "}"; return a}"#).expect("body");
        assert_eq!(body, r#"a = "}"; return a"#);
    }
}

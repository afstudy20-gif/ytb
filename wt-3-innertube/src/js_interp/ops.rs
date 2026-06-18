//! Operator and method implementations for the JS-subset interpreter.

use std::rc::Rc;

use crate::error::{Error, Result};
use crate::js_interp::value::JsValue;

/// Apply a JS binary operator. Handles `+`, `-`, `*`, `/`, `%` and the JS
/// bitwise ops `&`, `|`, `^`, `<<`, `>>`, `>>>` (which operate on int32).
pub(crate) fn apply_binary(op: &str, lhs: JsValue, rhs: JsValue) -> Result<JsValue> {
    match op {
        "+" => {
            if matches!(lhs, JsValue::Str(_)) || matches!(rhs, JsValue::Str(_)) {
                let l = lhs.into_string()?;
                let r = rhs.into_string()?;
                Ok(JsValue::Str(l + &r))
            } else {
                Ok(JsValue::Num(lhs.as_num()? + rhs.as_num()?))
            }
        }
        "-" => Ok(JsValue::Num(lhs.as_num()? - rhs.as_num()?)),
        "*" => Ok(JsValue::Num(lhs.as_num()? * rhs.as_num()?)),
        "/" => Ok(JsValue::Num(lhs.as_num()? / rhs.as_num()?)),
        "%" => Ok(JsValue::Num(lhs.as_num()? % rhs.as_num()?)),
        "&" => Ok(JsValue::Num((lhs.as_int()? & rhs.as_int()?) as f64)),
        "|" => Ok(JsValue::Num((lhs.as_int()? | rhs.as_int()?) as f64)),
        "^" => Ok(JsValue::Num((lhs.as_int()? ^ rhs.as_int()?) as f64)),
        "<<" => {
            let l = lhs.as_u32()?;
            let r = (rhs.as_int()? & 31) as u32;
            Ok(JsValue::Num((l << r) as i64 as f64))
        }
        ">>" => {
            let l = lhs.as_int()?;
            let r = rhs.as_int()? & 31;
            Ok(JsValue::Num((l >> r) as f64))
        }
        ">>>" => {
            let l = lhs.as_u32()?;
            let r = (rhs.as_int()? & 31) as u32;
            Ok(JsValue::Num((l >> r) as f64))
        }
        "==" => Ok(JsValue::Bool(loose_eq(&lhs, &rhs))),
        "===" => Ok(JsValue::Bool(strict_eq(&lhs, &rhs))),
        "!=" => Ok(JsValue::Bool(!loose_eq(&lhs, &rhs))),
        "!==" => Ok(JsValue::Bool(!strict_eq(&lhs, &rhs))),
        "<" => Ok(JsValue::Bool(lhs.as_num()? < rhs.as_num()?)),
        "<=" => Ok(JsValue::Bool(lhs.as_num()? <= rhs.as_num()?)),
        ">" => Ok(JsValue::Bool(lhs.as_num()? > rhs.as_num()?)),
        ">=" => Ok(JsValue::Bool(lhs.as_num()? >= rhs.as_num()?)),
        _ => Err(Error::cipher(format!("unsupported operator '{op}'"))),
    }
}

fn loose_eq(a: &JsValue, b: &JsValue) -> bool {
    match (a, b) {
        (JsValue::Num(x), JsValue::Num(y)) => x == y,
        (JsValue::Str(x), JsValue::Str(y)) => x == y,
        (JsValue::Bool(x), JsValue::Bool(y)) => x == y,
        _ => {
            let x = a.clone().into_string().unwrap_or_default();
            let y = b.clone().into_string().unwrap_or_default();
            x == y
        }
    }
}

fn strict_eq(a: &JsValue, b: &JsValue) -> bool {
    match (a, b) {
        (JsValue::Num(x), JsValue::Num(y)) => x == y,
        (JsValue::Str(x), JsValue::Str(y)) => x == y,
        (JsValue::Bool(x), JsValue::Bool(y)) => x == y,
        (JsValue::Undef, JsValue::Undef) => true,
        (JsValue::Arr(x), JsValue::Arr(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

/// Evaluate a String/Array method. Returns `(return_value, optional new_receiver)`.
/// When `new_receiver` is `Some`, the caller should write it back to the
/// scope variable (mirroring JS mutation semantics for `reverse`/`splice`/
/// `push`/`shift`/`sort`). With shared-storage arrays this is mostly a
/// no-op; the in-place mutation has already happened.
pub(crate) fn eval_method_on(
    method: &str,
    recv: JsValue,
    args: &[JsValue],
    _allow_mutation: bool,
) -> Result<(JsValue, Option<JsValue>)> {
    match method {
        "split" => {
            let s = recv.as_str()?.to_string();
            let sep = args.first().and_then(|a| a.as_str().ok()).unwrap_or("");
            let parts: Vec<JsValue> = if sep.is_empty() {
                s.chars().map(|c| JsValue::Str(c.to_string())).collect()
            } else {
                s.split(sep).map(|p| JsValue::Str(p.to_string())).collect()
            };
            Ok((JsValue::arr(parts), None))
        }
        "join" => {
            let sep = args
                .first()
                .map(|v| v.clone().into_string().unwrap_or_default())
                .unwrap_or_default();
            let joined = recv.with_arr(|a| {
                let parts: Vec<String> = a
                    .iter()
                    .map(|v| match v {
                        JsValue::Undef | JsValue::Arr(_) => String::new(),
                        other => other.clone().into_string().unwrap_or_default(),
                    })
                    .collect();
                parts.join(&sep)
            })?;
            Ok((JsValue::Str(joined), None))
        }
        "reverse" => {
            recv.with_arr_mut(|a| a.reverse())?;
            Ok((recv.clone(), None))
        }
        "slice" => {
            let sliced = recv.with_arr(|a| {
                let len = a.len() as i64;
                let start = args.first().map(|v| v.as_int().unwrap_or(0)).unwrap_or(0);
                let end = args.get(1).map(|v| v.as_int().unwrap_or(len));
                let s = if start < 0 { (len + start).max(0) } else { start.min(len) };
                let e = match end {
                    Some(e) if e < 0 => (len + e).max(0),
                    Some(e) => e.min(len),
                    None => len,
                };
                if s < e {
                    a[s as usize..e as usize].to_vec()
                } else {
                    Vec::new()
                }
            })?;
            Ok((JsValue::arr(sliced), None))
        }
        "splice" => eval_splice(recv, args),
        "push" => {
            let new_len = recv.with_arr_mut(|a| {
                for v in args {
                    a.push(v.clone());
                }
                a.len()
            })?;
            Ok((JsValue::Num(new_len as f64), None))
        }
        "pop" => {
            let v = recv.with_arr_mut(|a| a.pop().unwrap_or(JsValue::Undef))?;
            Ok((v, None))
        }
        "shift" => {
            let v = recv.with_arr_mut(|a| {
                if a.is_empty() {
                    JsValue::Undef
                } else {
                    a.remove(0)
                }
            })?;
            Ok((v, None))
        }
        "unshift" => {
            let len = recv.with_arr_mut(|a| {
                for (i, v) in args.iter().enumerate() {
                    a.insert(i, v.clone());
                }
                a.len()
            })?;
            Ok((JsValue::Num(len as f64), None))
        }
        "sort" => {
            recv.with_arr_mut(|a| {
                // Default JS sort is lexicographic on the string form.
                a.sort_by(|x, y| {
                    let xs = x.clone().into_string().unwrap_or_default();
                    let ys = y.clone().into_string().unwrap_or_default();
                    xs.cmp(&ys)
                });
            })?;
            Ok((recv.clone(), None))
        }
        "charCodeAt" => {
            let s = recv.as_str()?;
            let idx = args.first().map(|v| v.as_int().unwrap_or(0)).unwrap_or(0) as usize;
            let ch = s
                .chars()
                .nth(idx)
                .ok_or_else(|| Error::cipher("charCodeAt out of range"))?;
            Ok((JsValue::Num(ch as u32 as f64), None))
        }
        "charAt" => {
            let s = recv.as_str()?;
            let idx = args.first().map(|v| v.as_int().unwrap_or(0)).unwrap_or(0) as usize;
            Ok((
                JsValue::Str(
                    s.chars()
                        .nth(idx)
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                ),
                None,
            ))
        }
        "substring" | "substr" => {
            let s = recv.as_str()?;
            let start = args.first().map(|v| v.as_int().unwrap_or(0)).unwrap_or(0);
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as i64;
            let s_idx = start.clamp(0, len).max(0) as usize;
            let end = args
                .get(1)
                .map(|v| v.as_int().unwrap_or(len))
                .unwrap_or(len)
                .clamp(0, len) as usize;
            let sliced: String = if s_idx <= end {
                chars[s_idx..end].iter().collect()
            } else {
                chars[end..s_idx].iter().collect()
            };
            Ok((JsValue::Str(sliced), None))
        }
        "toString" => {
            let s = recv.into_string()?;
            Ok((JsValue::Str(s), None))
        }
        _ => Err(Error::cipher(format!("unsupported method '.{method}'"))),
    }
}

fn eval_splice(recv: JsValue, args: &[JsValue]) -> Result<(JsValue, Option<JsValue>)> {
    let removed = recv.with_arr_mut(|a| {
        let start_raw = args.first().map(|v| v.as_int().unwrap_or(0)).unwrap_or(0);
        let len = a.len() as i64;
        let start = if start_raw < 0 {
            (len + start_raw).max(0) as usize
        } else {
            (start_raw as usize).min(a.len())
        };
        let delete = args
            .get(1)
            .map(|v| v.as_int().unwrap_or(len) as usize)
            .unwrap_or_else(|| a.len() - start)
            .min(a.len() - start);
        let removed: Vec<JsValue> = a.drain(start..start + delete).collect();
        for (i, item) in args.iter().skip(2).enumerate() {
            a.insert(start + i, item.clone());
        }
        removed
    })?;
    Ok((JsValue::arr(removed), None))
}

/// Evaluate a `Math.*` builtin.
pub(crate) fn eval_math(method: &str, args: &[JsValue]) -> Result<JsValue> {
    let val = args
        .first()
        .ok_or_else(|| Error::cipher(format!("Math.{method} needs an argument")))?
        .as_num()?;
    let result = match method {
        "floor" => val.floor(),
        "ceil" => val.ceil(),
        "round" => val.round(),
        "abs" => val.abs(),
        "sign" => val.signum(),
        "trunc" => val.trunc(),
        _ => return Err(Error::cipher(format!("unsupported Math.{method}"))),
    };
    Ok(JsValue::Num(result))
}

/// Evaluate `String.fromCharCode(...)`.
pub(crate) fn eval_string_from_char_code(args: &[JsValue]) -> Result<JsValue> {
    let codes: Vec<u32> = args.iter().map(|v| v.as_u32()).collect::<Result<Vec<_>>>()?;
    let s: String = codes.iter().filter_map(|&c| char::from_u32(c)).collect();
    Ok(JsValue::Str(s))
}

/// Get a property by name (only `length` is supported).
pub(crate) fn get_member(obj: &JsValue, prop: &str) -> Result<JsValue> {
    match prop {
        "length" => match obj {
            JsValue::Str(s) => Ok(JsValue::Num(s.chars().count() as f64)),
            JsValue::Arr(a) => Ok(JsValue::Num(a.borrow().len() as f64)),
            _ => Err(Error::cipher(format!("no length on {obj:?}"))),
        },
        _ => Err(Error::cipher(format!("unsupported member '.{prop}'"))),
    }
}

/// Get an index from a value (string char or array element).
pub(crate) fn get_index(obj: &JsValue, idx: &JsValue) -> Result<JsValue> {
    let i = idx.as_int()?;
    match obj {
        JsValue::Str(s) => {
            let chars: Vec<char> = s.chars().collect();
            let i = if i < 0 {
                (chars.len() as i64 + i).max(0)
            } else {
                i
            } as usize;
            chars
                .get(i)
                .map(|c| JsValue::Str(c.to_string()))
                .ok_or_else(|| Error::cipher(format!("string index {i} out of range")))
        }
        JsValue::Arr(a) => {
            let borrowed = a.borrow();
            if i < 0 || i as usize >= borrowed.len() {
                Ok(JsValue::Undef)
            } else {
                Ok(borrowed[i as usize].clone())
            }
        }
        _ => Err(Error::cipher(format!("cannot index {obj:?}"))),
    }
}

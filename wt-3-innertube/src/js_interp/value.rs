//! [`JsValue`] and the coercion helpers used throughout the interpreter.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{Error, Result};

/// A live JS-subset value. Strings, numbers, arrays of values, undefined,
/// and boolean are all this interpreter needs.
///
/// Arrays use [`Rc<RefCell<Vec<JsValue>>>`] so that JS's pass-by-reference
/// semantics for arrays (and therefore for the cipher helper functions,
/// which mutate their `a` argument in place) work correctly. Cloning a
/// [`JsValue::Arr`] is cheap and shares the underlying storage.
#[derive(Debug, Clone)]
pub enum JsValue {
    /// UTF-8 string. Cipher functions operate on strings-as-character-
    /// arrays.
    Str(String),
    /// A floating-point number. Used as an index or as the return value of
    /// arithmetic like `length - 1`.
    Num(f64),
    /// An array of `JsValue`s. Shared via `Rc<RefCell<...>>` so JS-style
    /// pass-by-reference works for the cipher helper functions.
    Arr(Rc<RefCell<Vec<JsValue>>>),
    /// `undefined`. Returned by `splice` ops etc.
    Undef,
    /// Boolean (rarely used by the cipher functions).
    Bool(bool),
}

impl JsValue {
    /// String view, erroring if the value isn't a string.
    pub fn as_str(&self) -> Result<&str> {
        match self {
            JsValue::Str(s) => Ok(s.as_str()),
            other => Err(Error::cipher(format!("expected string, got {other:?}"))),
        }
    }

    /// Borrow the array's contents as a slice. Errors if the value isn't
    /// an array.
    pub fn with_arr<R>(&self, f: impl FnOnce(&[JsValue]) -> R) -> Result<R> {
        match self {
            JsValue::Arr(a) => {
                let borrowed = a.borrow();
                // Polonius would let us pass &borrowed directly; for now
                // we copy what we need via a closure.
                Ok(f(&borrowed))
            }
            other => Err(Error::cipher(format!("expected array, got {other:?}"))),
        }
    }

    /// Apply a mutating closure to the array's contents.
    pub fn with_arr_mut<R>(
        &self,
        f: impl FnOnce(&mut Vec<JsValue>) -> R,
    ) -> Result<R> {
        match self {
            JsValue::Arr(a) => Ok(f(&mut a.borrow_mut())),
            other => Err(Error::cipher(format!("expected array, got {other:?}"))),
        }
    }

    /// Construct an owned array value from a `Vec<JsValue>`.
    pub fn arr(values: Vec<JsValue>) -> Self {
        JsValue::Arr(Rc::new(RefCell::new(values)))
    }

    /// Coerce to f64 like JS would.
    pub fn as_num(&self) -> Result<f64> {
        match self {
            JsValue::Num(n) => Ok(*n),
            JsValue::Str(s) => s
                .trim()
                .parse::<f64>()
                .map_err(|e| Error::cipher(format!("string '{s}' not numeric: {e}"))),
            JsValue::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            JsValue::Undef => Ok(f64::NAN),
            JsValue::Arr(_) => Err(Error::cipher("cannot coerce array to number")),
        }
    }

    /// Coerce to an integer (truncating), JS-style.
    pub fn as_int(&self) -> Result<i64> {
        Ok(self.as_num()? as i64)
    }

    /// Coerce to a u32 (for bitwise ops), JS-style (`ToUint32`).
    pub fn as_u32(&self) -> Result<u32> {
        let n = self.as_num()?;
        if !n.is_finite() {
            return Ok(0);
        }
        let positive = n.abs();
        let truncated = positive.floor() as u64;
        Ok((truncated & 0xFFFF_FFFF) as u32)
    }

    /// Stringify like JS (`+ ""`).
    pub fn into_string(self) -> Result<String> {
        match self {
            JsValue::Str(s) => Ok(s),
            JsValue::Num(n) => Ok(format_js_number(n)),
            JsValue::Bool(b) => Ok(b.to_string()),
            JsValue::Undef => Ok("undefined".to_string()),
            JsValue::Arr(a) => {
                // JS stringifies arrays by joining their string forms
                // with commas.
                let borrowed = a.borrow();
                let parts: Vec<String> = borrowed
                    .iter()
                    .map(|v| match v {
                        JsValue::Undef | JsValue::Arr(_) => String::new(),
                        other => other.clone().into_string().unwrap_or_default(),
                    })
                    .collect();
                Ok(parts.join(","))
            }
        }
    }
}

impl std::fmt::Display for JsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsValue::Str(s) => write!(f, "{s:?}"),
            JsValue::Num(n) => write!(f, "{n}"),
            JsValue::Arr(a) => write!(f, "[{} els]", a.borrow().len()),
            JsValue::Undef => write!(f, "undefined"),
            JsValue::Bool(b) => write!(f, "{b}"),
        }
    }
}

/// Format a JS number the way JS would (`5.0` -> `"5"`).
pub(crate) fn format_js_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e21 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Truthiness test matching JS semantics (`ToBoolean`).
pub(crate) fn is_truthy(v: &JsValue) -> bool {
    match v {
        JsValue::Undef => false,
        JsValue::Bool(b) => *b,
        JsValue::Num(n) => *n != 0.0 && !n.is_nan(),
        JsValue::Str(s) => !s.is_empty(),
        JsValue::Arr(a) => !a.borrow().is_empty(),
    }
}

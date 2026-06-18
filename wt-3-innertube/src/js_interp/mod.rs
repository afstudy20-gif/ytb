//! JavaScript-subset execution for YouTube player code.
//!
//! ## Why hand-roll instead of `boa_engine`?
//!
//! The YouTube player exercises only a tiny corner of JavaScript when
//! deciphering stream signatures (`s` param) and transforming the `n`
//! param: array/string indexing, slice/splice/reverse/push/shift, `split`,
//! `join`, `length`, `var` declarations, function calls, integer and
//! bitwise arithmetic, and `charCodeAt`/`fromCharCode`. `boa_engine` is a
//! complete JS implementation that pulls in several MB of code, a GC, and
//! spec-compliant quirks we don't need. A hand-rolled interpreter for this
//! subset is ~1.5k lines but no API churn and keeps the dependency tree
//! lean. If YouTube ever evolves the player beyond this subset, the right
//! move is to extend this module rather than swap in a full JS engine —
//! the surface that touches the deciphering functions is small.
//!
//! The module is split into several files to keep each under 400 lines:
//!
//! - [`value`]: `JsValue` and the coercion helpers.
//! - [`ops`]: binary and method-call implementations.
//! - [`cipher`]: the high-level [`cipher::CipherProgram`] that decodes the
//!   classic signature cipher (reverse/swap/splice) without re-running JS.
//! - [`interp`]: the [`interp::Interp`] struct that executes n-sig-style
//!   JS functions with proper mutation semantics.
//! - [`lexer`]: the source-level parsers (`split_top_level_statements`,
//!   `parse_call`, etc.) used by both `cipher` and `interp`.

pub mod cipher;
pub mod interp;
pub mod lexer;
pub mod ops;
pub mod value;

pub use cipher::{build_cipher_program, CipherProgram};
pub use interp::Interp;
pub use value::JsValue;

//! The unit language: a purpose-built interpreter for a Python-shaped
//! language, deterministic by construction. The spec is
//! `docs/01-language.md` and its parts; this crate is what that doc is built
//! from, and nothing here is inherited from the host — every evaluation
//! order, rounding, limit and failure is the doc's.
//!
//! **Read CLAUDE.md's determinism rules before changing anything here.** No
//! floats, no hash-order iteration, no wall clock: `crates/sim/tests/no_floats.rs`
//! scans this crate too.
//!
//! Layout, in the order a program passes through it:
//!
//! - [`num`] — the one number type, an i128 scaled by 10¹².
//! - [`lexer`] — tokens with significant indentation.
//! - [`ast`] and [`parser`] — the boundary `docs/01-language/syntax.md` draws.
//! - [`compile`] — the AST to a small bytecode, so a program can be paused
//!   between any two operations and resumed on a later tick.
//! - [`value`] — what a program computes with.
//! - [`vm`] — the metered evaluator: frames, budgets, boundaries, faults.
//! - [`dispatch`] — operations that run user code: the dunder set, `key=`,
//!   the comparisons a sort makes; resumable, so a budget can pause them.
//! - [`data`] — the cost and limit tables, loaded from `data/language/`.
//!
//! T10 built the lexer, the procedural core, metering, costs and limits;
//! T11 added `class`, T12 `match` and T13 `import`, which completes the
//! boundary `docs/01-language/syntax.md` draws. The interrupt kinds beyond
//! `fault` land with the sim that raises them (T7).

pub mod ast;
pub mod builtins;
pub mod compile;
pub mod data;
pub mod dispatch;
pub mod errors;
pub mod lexer;
pub mod num;
pub mod parser;
pub mod value;
pub mod vm;

pub use data::{Costs, DataError, Limits};
pub use errors::{ExcClass, Exception};
pub use num::Num;
pub use value::Value;
pub use vm::{Host, HostCall, Machine, Program, Slice};

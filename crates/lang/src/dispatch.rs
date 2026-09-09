//! Operations that may run user code: the closed dunder set of
//! `docs/01-language/syntax.md`'s Classes section, and every builtin whose
//! procedure calls back into the program — `sorted` comparing with `__lt__`,
//! `key=` functions, `in` asking `__eq__` of each element.
//!
//! A machine can be paused between any two operations by its tick budget,
//! and a call is a sequence of operations, so nothing here may *run* a call:
//! each [`Native`] is a small state machine that asks the VM for a call,
//! is suspended in its frame while the callee's frames execute, and is
//! stepped again with the result. A pair of values with no instance in it
//! is decided synchronously by `value.rs`'s rules, which keeps the plain
//! cases on one code path.
//!
//! The order of calls is spec: `syntax.md` fixes the sequence of
//! comparisons a sort makes and which operand is the receiver, and this
//! module is where those sentences become code.

use crate::ast::{BinOp, CmpOp};
use crate::builtins::{self, Outcome};
use crate::errors::{ExcClass, Exception};
use crate::value::{Class, Instance, R, Value, dict_find, has_instance, quoted};
use crate::vm::Machine;
use std::cell::RefCell;
use std::rc::Rc;

/// What a native needs next from the VM.
pub enum Need {
    /// Finished: push the value on the frame that started the native, or
    /// deliver it to the native beneath. `None` means the native has
    /// arranged the frame's stack itself.
    Done(Option<Value>),
    /// Run a nested native first and deliver its value back here.
    Sub(Native),
    /// Call user code and deliver what it returns. `self_uncharged` is a
    /// dunder dispatch: `costs.md` charges the call with `self` free.
    Call {
        func: Value,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
        self_uncharged: bool,
    },
    /// Call a game builtin and deliver what the host returns.
    Host {
        name: String,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
    },
}

/// What a completed [`Native::Materialise`] does with its list.
pub enum Then {
    /// Put the frame's operands back with the list in the middle and run
    /// the instruction again — for `Unpack` and a starred call argument.
    Restore {
        below: Vec<Value>,
        above: Vec<Value>,
    },
    /// Re-enter a builtin with the argument replaced.
    Builtin {
        name: String,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
        slot: usize,
    },
    /// Re-enter a method of a built-in type with the argument replaced.
    Method {
        recv: Value,
        name: &'static str,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
        slot: usize,
    },
}

/// A merge sort's work item, on an explicit stack so it can pause.
pub enum SortTask {
    Split {
        lo: usize,
        hi: usize,
    },
    Merge {
        lo: usize,
        mid: usize,
        hi: usize,
        left: Vec<usize>,
        right: Vec<usize>,
        i: usize,
        j: usize,
        k: usize,
        started: bool,
    },
}

pub enum FindMode {
    Contains,
    Index,
    Count,
    Remove(Rc<crate::value::ListObj>),
}

pub enum Native {
    /// `a == b` (`negate` for `!=`).
    Eq {
        a: Value,
        b: Value,
        negate: bool,
        stage: u8,
    },
    /// Elementwise `==` of two sequences holding instances.
    SeqEq {
        xs: Vec<Value>,
        ys: Vec<Value>,
        i: usize,
    },
    /// `==` of two dicts whose values hold instances: the matched value
    /// pairs, in the left dict's order.
    PairsEq {
        pairs: Vec<(Value, Value)>,
        i: usize,
    },
    /// `a < b`.
    Lt {
        a: Value,
        b: Value,
        stage: u8,
    },
    /// Lexicographic `<` of two sequences holding instances.
    SeqLt {
        xs: Vec<Value>,
        ys: Vec<Value>,
        i: usize,
        stage: u8,
    },
    /// The whole comparison operator, with the derived forms.
    Cmp {
        op: CmpOp,
        a: Value,
        b: Value,
        stage: u8,
    },
    /// `item in container` (`negate` for `not in`).
    Contains {
        container: Value,
        item: Value,
        negate: bool,
        stage: u8,
    },
    /// A walk of a sequence comparing each element (on the left) to a
    /// target: `in`, `index`, `count`, `remove`.
    Find {
        items: Vec<Value>,
        target: Value,
        i: usize,
        count: i128,
        mode: FindMode,
    },
    /// Truth of a value: `__len__` when defined.
    Truthy {
        v: Value,
        stage: u8,
    },
    /// `len(x)`.
    Len {
        v: Value,
        stage: u8,
    },
    /// `str(x)`, `__str__` included, for a value holding instances.
    Str {
        v: Value,
        stage: u8,
    },
    /// `str` of a container holding instances: literal pieces and values
    /// still to render.
    StrWalk {
        pieces: Vec<Piece>,
        i: usize,
        out: String,
    },
    /// An f-string field with a spec, on an instance.
    Format {
        v: Value,
        spec: Option<String>,
        stage: u8,
    },
    /// `sorted`, `list.sort`.
    Sort {
        items: Vec<Value>,
        key: Option<Value>,
        keys: Vec<Value>,
        reverse: bool,
        idx: Vec<usize>,
        tasks: Vec<SortTask>,
        target: Option<Rc<crate::value::ListObj>>,
        stage: u8,
    },
    /// `min`, `max`.
    MinMax {
        items: Vec<Value>,
        key: Option<Value>,
        keys: Vec<Value>,
        is_min: bool,
        best: usize,
        i: usize,
        stage: u8,
    },
    /// `any`, `all`.
    AnyAll {
        items: Vec<Value>,
        i: usize,
        is_any: bool,
    },
    /// `C(…)`: allocate, then `__init__`.
    Construct {
        class: Rc<Class>,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
        inst: Option<Rc<Instance>>,
        then_raise: bool,
    },
    /// The items of an instance, `x[0]` … `x[len(x) - 1]`, gathered for an
    /// operation that needs them all.
    Materialise {
        obj: Rc<Instance>,
        n: Option<usize>,
        i: usize,
        out: Vec<Value>,
        then: Option<Then>,
        stage: u8,
    },
    /// `len(x)` once, at `for` entry over an instance.
    IterStart {
        obj: Rc<Instance>,
    },
    /// `x[i]` as a `for` reaches it.
    GetItem {
        obj: Rc<Instance>,
        idx: Value,
    },
    /// `x[k] = v` on an instance.
    SetItem {
        obj: Rc<Instance>,
        idx: Value,
        v: Value,
        stage: u8,
    },
    /// `+ - *` with an instance on the left, and unary `-`.
    BinOp {
        op: BinOp,
        a: Rc<Instance>,
        b: Value,
    },
    Neg {
        a: Rc<Instance>,
    },
    /// `not x` on a value whose truth needs a call.
    Not {
        v: Value,
        stage: u8,
    },
    /// A jump on truth: `stage` 0 asks, 1 has the answer and the VM acts.
    JumpTruth {
        v: Value,
        stage: u8,
    },
    /// `log(*values)`: `str` of each value, then the host.
    LogArgs {
        name: String,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
        i: usize,
        out: Vec<Value>,
    },
}

/// A piece of a container's string form.
pub enum Piece {
    Lit(String),
    /// A value to render as `str` would inside a container.
    Val(Value),
}

fn type_error() -> Exception {
    Exception::type_error()
}

fn as_bool(v: &Value) -> bool {
    matches!(v, Value::Bool(true))
}

/// `__len__`'s result, checked: an integral, non-negative `num`.
fn len_result(v: &Value) -> R<usize> {
    let n = v.expect_integral()?;
    usize::try_from(n).map_err(|_| Exception::value_error())
}

fn dunder_call(inst: &Rc<Instance>, name: &str, mut args: Vec<Value>) -> Option<Need> {
    let f = inst.dunder(name)?;
    args.insert(0, Value::Inst(inst.clone()));
    Some(Need::Call {
        func: Value::Func(f),
        args,
        kwargs: vec![],
        self_uncharged: true,
    })
}

/// The pieces of a container's string form, instances left for the walk.
fn pieces_of(v: &Value, depth: u32, out: &mut Vec<Piece>) -> R<()> {
    if depth == 0 {
        return Err(Exception::new(ExcClass::LimitError, vec![]));
    }
    let d = depth.wrapping_sub(1);
    let seq =
        |out: &mut Vec<Piece>, items: &[Value], open: &str, close: &str, trailing: bool| -> R<()> {
            out.push(Piece::Lit(open.to_string()));
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(Piece::Lit(", ".to_string()));
                }
                if has_instance(x, d) {
                    pieces_of(x, d, out)?;
                } else {
                    out.push(Piece::Lit(quoted(x)));
                }
            }
            if trailing {
                out.push(Piece::Lit(",".to_string()));
            }
            out.push(Piece::Lit(close.to_string()));
            Ok(())
        };
    match v {
        Value::List(l) => seq(out, &l.items.borrow(), "[", "]", false),
        Value::Tuple(t) => seq(out, t, "(", ")", t.len() == 1),
        Value::Set(s) => seq(out, &s.items.borrow(), "{", "}", false),
        Value::Dict(dict) => {
            out.push(Piece::Lit("{".to_string()));
            for (i, (k, x)) in dict.items.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(Piece::Lit(", ".to_string()));
                }
                if has_instance(k, d) {
                    pieces_of(k, d, out)?;
                } else {
                    out.push(Piece::Lit(quoted(k)));
                }
                out.push(Piece::Lit(": ".to_string()));
                if has_instance(x, d) {
                    pieces_of(x, d, out)?;
                } else {
                    out.push(Piece::Lit(quoted(x)));
                }
            }
            out.push(Piece::Lit("}".to_string()));
            Ok(())
        }
        other => {
            out.push(Piece::Val(other.clone()));
            Ok(())
        }
    }
}

impl Native {
    pub fn eq(a: Value, b: Value, negate: bool) -> Native {
        Native::Eq {
            a,
            b,
            negate,
            stage: 0,
        }
    }

    pub fn lt(a: Value, b: Value) -> Native {
        Native::Lt { a, b, stage: 0 }
    }

    pub fn truthy(v: Value) -> Native {
        Native::Truthy { v, stage: 0 }
    }

    pub fn str_of(v: Value) -> Native {
        Native::Str { v, stage: 0 }
    }

    pub fn sort(
        items: Vec<Value>,
        key: Option<Value>,
        reverse: bool,
        target: Option<Rc<crate::value::ListObj>>,
    ) -> Native {
        Native::Sort {
            items,
            key,
            keys: vec![],
            reverse,
            idx: vec![],
            tasks: vec![],
            target,
            stage: 0,
        }
    }

    pub fn min_max(items: Vec<Value>, key: Option<Value>, is_min: bool) -> Native {
        Native::MinMax {
            items,
            key,
            keys: vec![],
            is_min,
            best: 0,
            i: 0,
            stage: 0,
        }
    }

    pub fn materialise(obj: Rc<Instance>, then: Then) -> Native {
        Native::Materialise {
            obj,
            n: None,
            i: 0,
            out: vec![],
            then: Some(then),
            stage: 0,
        }
    }

    /// Advance with the value the last request produced (`None` on the
    /// first step).
    pub fn step(&mut self, m: &mut Machine, input: Option<Value>) -> R<Need> {
        let depth = m.limits().depth_nesting as u32;
        let c = m.costs().clone();
        match self {
            Native::Eq {
                a,
                b,
                negate,
                stage,
            } => match *stage {
                0 => {
                    if let Value::Inst(i) = a {
                        if let Some(need) = dunder_call(i, "__eq__", vec![b.clone()]) {
                            *stage = 1;
                            return Ok(need);
                        }
                        return Ok(Need::Done(Some(Value::Bool(
                            a.eq_value(b, depth)? ^ *negate,
                        ))));
                    }
                    if matches!(b, Value::Inst(_)) {
                        return Ok(Need::Done(Some(Value::Bool(*negate))));
                    }
                    if !has_instance(a, depth) || !has_instance(b, depth) {
                        return Ok(Need::Done(Some(Value::Bool(
                            a.eq_value(b, depth)? ^ *negate,
                        ))));
                    }
                    *stage = 2;
                    match (&*a, &*b) {
                        (Value::List(x), Value::List(y)) => Ok(Need::Sub(Native::SeqEq {
                            xs: x.items.borrow().clone(),
                            ys: y.items.borrow().clone(),
                            i: 0,
                        })),
                        (Value::Tuple(x), Value::Tuple(y)) => Ok(Need::Sub(Native::SeqEq {
                            xs: x.to_vec(),
                            ys: y.to_vec(),
                            i: 0,
                        })),
                        (Value::Dict(x), Value::Dict(y)) => {
                            let (xs, ys) = (x.items.borrow(), y.items.borrow());
                            if xs.len() != ys.len() {
                                return Ok(Need::Done(Some(Value::Bool(*negate))));
                            }
                            let mut pairs = Vec::with_capacity(xs.len());
                            for (k, v) in xs.iter() {
                                match dict_find(&ys, k, depth)? {
                                    Some(j) => pairs.push((v.clone(), ys[j].1.clone())),
                                    None => return Ok(Need::Done(Some(Value::Bool(*negate)))),
                                }
                            }
                            m.charge_each(xs.len().saturating_add(ys.len()), c.factor_traverse);
                            Ok(Need::Sub(Native::PairsEq { pairs, i: 0 }))
                        }
                        // Sets hold keys, compared by identity: no call.
                        _ => Ok(Need::Done(Some(Value::Bool(
                            a.eq_value(b, depth)? ^ *negate,
                        )))),
                    }
                }
                1 => {
                    *stage = 2;
                    Ok(Need::Sub(Native::truthy(input.unwrap_or(Value::None))))
                }
                _ => Ok(Need::Done(Some(Value::Bool(
                    as_bool(&input.unwrap_or(Value::None)) ^ *negate,
                )))),
            },
            Native::SeqEq { xs, ys, i } => {
                if xs.len() != ys.len() {
                    return Ok(Need::Done(Some(Value::Bool(false))));
                }
                if let Some(v) = input {
                    if !as_bool(&v) {
                        return Ok(Need::Done(Some(Value::Bool(false))));
                    }
                    *i = i.wrapping_add(1);
                }
                if *i >= xs.len() {
                    return Ok(Need::Done(Some(Value::Bool(true))));
                }
                m.charge(c.factor_traverse);
                Ok(Need::Sub(Native::eq(xs[*i].clone(), ys[*i].clone(), false)))
            }
            Native::PairsEq { pairs, i } => {
                if let Some(v) = input {
                    if !as_bool(&v) {
                        return Ok(Need::Done(Some(Value::Bool(false))));
                    }
                    *i = i.wrapping_add(1);
                }
                if *i >= pairs.len() {
                    return Ok(Need::Done(Some(Value::Bool(true))));
                }
                let (x, y) = pairs[*i].clone();
                Ok(Need::Sub(Native::eq(x, y, false)))
            }
            Native::Lt { a, b, stage } => match *stage {
                0 => {
                    if let Value::Inst(i) = a {
                        return match dunder_call(i, "__lt__", vec![b.clone()]) {
                            Some(need) => {
                                *stage = 1;
                                Ok(need)
                            }
                            None => Err(type_error()),
                        };
                    }
                    if matches!(b, Value::Inst(_)) {
                        return Err(type_error());
                    }
                    if !has_instance(a, depth) || !has_instance(b, depth) {
                        return Ok(Need::Done(Some(Value::Bool(a.lt_value(b, depth)?))));
                    }
                    *stage = 2;
                    match (&*a, &*b) {
                        (Value::List(x), Value::List(y)) => Ok(Need::Sub(Native::SeqLt {
                            xs: x.items.borrow().clone(),
                            ys: y.items.borrow().clone(),
                            i: 0,
                            stage: 0,
                        })),
                        (Value::Tuple(x), Value::Tuple(y)) => Ok(Need::Sub(Native::SeqLt {
                            xs: x.to_vec(),
                            ys: y.to_vec(),
                            i: 0,
                            stage: 0,
                        })),
                        _ => Err(type_error()),
                    }
                }
                1 => {
                    *stage = 2;
                    Ok(Need::Sub(Native::truthy(input.unwrap_or(Value::None))))
                }
                _ => Ok(Need::Done(Some(Value::Bool(as_bool(
                    &input.unwrap_or(Value::None),
                ))))),
            },
            Native::SeqLt { xs, ys, i, stage } => {
                // stage 0: ask `x == y`; stage 1: got equality; stage 2: got `x < y`.
                match (*stage, input) {
                    (1, Some(v)) => {
                        if as_bool(&v) {
                            *i = i.wrapping_add(1);
                            *stage = 0;
                        } else {
                            *stage = 2;
                            return Ok(Need::Sub(Native::lt(xs[*i].clone(), ys[*i].clone())));
                        }
                    }
                    (2, Some(v)) => return Ok(Need::Done(Some(v))),
                    _ => {}
                }
                if *i >= xs.len() || *i >= ys.len() {
                    return Ok(Need::Done(Some(Value::Bool(xs.len() < ys.len()))));
                }
                m.charge(c.factor_traverse);
                *stage = 1;
                Ok(Need::Sub(Native::eq(xs[*i].clone(), ys[*i].clone(), false)))
            }
            Native::Cmp { op, a, b, stage } => match (*op, *stage) {
                // Stage 9: one sub-native decides; stage 1 and 2: the two
                // halves of a derived `<=` / `>=`.
                (_, 9) | (CmpOp::Le | CmpOp::Ge, 2) => Ok(Need::Done(input)),
                (CmpOp::Eq, _) => {
                    *stage = 9;
                    Ok(Need::Sub(Native::eq(a.clone(), b.clone(), false)))
                }
                (CmpOp::Ne, _) => {
                    *stage = 9;
                    Ok(Need::Sub(Native::eq(a.clone(), b.clone(), true)))
                }
                (CmpOp::Lt, _) => {
                    *stage = 9;
                    Ok(Need::Sub(Native::lt(a.clone(), b.clone())))
                }
                (CmpOp::Gt, _) => {
                    *stage = 9;
                    Ok(Need::Sub(Native::lt(b.clone(), a.clone())))
                }
                (CmpOp::Le | CmpOp::Ge, 0) => {
                    *stage = 1;
                    let (x, y) = if *op == CmpOp::Le {
                        (a.clone(), b.clone())
                    } else {
                        (b.clone(), a.clone())
                    };
                    Ok(Need::Sub(Native::lt(x, y)))
                }
                (CmpOp::Le | CmpOp::Ge, _) => {
                    if as_bool(&input.unwrap_or(Value::None)) {
                        return Ok(Need::Done(Some(Value::Bool(true))));
                    }
                    *stage = 2;
                    Ok(Need::Sub(Native::eq(a.clone(), b.clone(), false)))
                }
                (CmpOp::In | CmpOp::NotIn, _) => {
                    *stage = 9;
                    Ok(Need::Sub(Native::Contains {
                        container: b.clone(),
                        item: a.clone(),
                        negate: *op == CmpOp::NotIn,
                        stage: 0,
                    }))
                }
                (CmpOp::Is, _) => Ok(Need::Done(Some(Value::Bool(matches!(a, Value::None))))),
                (CmpOp::IsNot, _) => Ok(Need::Done(Some(Value::Bool(!matches!(a, Value::None))))),
            },
            Native::Contains {
                container,
                item,
                negate,
                stage,
            } => match *stage {
                0 => match container {
                    Value::Inst(i) => match dunder_call(i, "__contains__", vec![item.clone()]) {
                        Some(need) => {
                            *stage = 1;
                            Ok(need)
                        }
                        None => Err(type_error()),
                    },
                    Value::List(l) => {
                        *stage = 2;
                        Ok(Need::Sub(Native::Find {
                            items: l.items.borrow().clone(),
                            target: item.clone(),
                            i: 0,
                            count: 0,
                            mode: FindMode::Contains,
                        }))
                    }
                    Value::Tuple(t) => {
                        *stage = 2;
                        Ok(Need::Sub(Native::Find {
                            items: t.to_vec(),
                            target: item.clone(),
                            i: 0,
                            count: 0,
                            mode: FindMode::Contains,
                        }))
                    }
                    other => {
                        let found = builtins::contains(m, other, item)?;
                        Ok(Need::Done(Some(Value::Bool(found ^ *negate))))
                    }
                },
                1 => {
                    *stage = 2;
                    Ok(Need::Sub(Native::truthy(input.unwrap_or(Value::None))))
                }
                _ => Ok(Need::Done(Some(Value::Bool(
                    as_bool(&input.unwrap_or(Value::None)) ^ *negate,
                )))),
            },
            Native::Find {
                items,
                target,
                i,
                count,
                mode,
            } => {
                if let Some(v) = input {
                    let hit = as_bool(&v);
                    match mode {
                        FindMode::Contains if hit => {
                            return Ok(Need::Done(Some(Value::Bool(true))));
                        }
                        FindMode::Index if hit => {
                            return Ok(Need::Done(Some(Value::int(*i as i128))));
                        }
                        FindMode::Remove(list) if hit => {
                            let mut xs = list.items.borrow_mut();
                            if *i < xs.len() {
                                xs.remove(*i);
                            }
                            return Ok(Need::Done(Some(Value::None)));
                        }
                        FindMode::Count if hit => *count = count.wrapping_add(1),
                        _ => {}
                    }
                    *i = i.wrapping_add(1);
                }
                if *i >= items.len() {
                    return match mode {
                        FindMode::Contains => Ok(Need::Done(Some(Value::Bool(false)))),
                        FindMode::Count => Ok(Need::Done(Some(Value::int(*count)))),
                        FindMode::Index | FindMode::Remove(_) => Err(Exception::value_error()),
                    };
                }
                m.charge(c.factor_traverse);
                // The element is the receiver: `x in xs` compares each
                // element on the left.
                Ok(Need::Sub(Native::eq(
                    items[*i].clone(),
                    target.clone(),
                    false,
                )))
            }
            Native::Truthy { v, stage } => match *stage {
                0 => {
                    if let Value::Inst(i) = v
                        && let Some(need) = dunder_call(i, "__len__", vec![])
                    {
                        *stage = 1;
                        return Ok(need);
                    }
                    Ok(Need::Done(Some(Value::Bool(v.truthy()))))
                }
                _ => {
                    let n = len_result(&input.unwrap_or(Value::None))?;
                    Ok(Need::Done(Some(Value::Bool(n != 0))))
                }
            },
            Native::Len { v, stage } => match *stage {
                0 => {
                    if let Value::Inst(i) = v {
                        return match dunder_call(i, "__len__", vec![]) {
                            Some(need) => {
                                *stage = 1;
                                Ok(need)
                            }
                            None => Err(type_error()),
                        };
                    }
                    Ok(Need::Done(Some(Value::int(builtins::length(v)? as i128))))
                }
                _ => {
                    let n = len_result(&input.unwrap_or(Value::None))?;
                    Ok(Need::Done(Some(Value::int(n as i128))))
                }
            },
            Native::Str { v, stage } => match *stage {
                0 => {
                    if let Value::Inst(i) = v {
                        if let Some(need) = dunder_call(i, "__str__", vec![]) {
                            *stage = 1;
                            return Ok(need);
                        }
                        return Ok(Need::Done(Some(Value::str(&v.to_str_value()))));
                    }
                    if !has_instance(v, depth) {
                        return Ok(Need::Done(Some(Value::str(&v.to_str_value()))));
                    }
                    *stage = 2;
                    let mut pieces = Vec::new();
                    pieces_of(v, depth, &mut pieces)?;
                    Ok(Need::Sub(Native::StrWalk {
                        pieces,
                        i: 0,
                        out: String::new(),
                    }))
                }
                1 => match input {
                    Some(s @ Value::Str(_)) => Ok(Need::Done(Some(s))),
                    _ => Err(type_error()),
                },
                _ => Ok(Need::Done(input)),
            },
            Native::StrWalk { pieces, i, out } => {
                if let Some(v) = input {
                    out.push_str(&v.to_str_value());
                    *i = i.wrapping_add(1);
                }
                while *i < pieces.len() {
                    match &pieces[*i] {
                        Piece::Lit(s) => {
                            out.push_str(s);
                            *i = i.wrapping_add(1);
                        }
                        Piece::Val(v) => return Ok(Need::Sub(Native::str_of(v.clone()))),
                    }
                }
                m.check_size(out.chars().count())?;
                Ok(Need::Done(Some(Value::str(out))))
            }
            Native::Format { v, spec, stage } => match *stage {
                0 => {
                    *stage = 1;
                    Ok(Need::Sub(Native::str_of(v.clone())))
                }
                _ => {
                    let s = input.unwrap_or(Value::None);
                    let out = builtins::format_value(&s, spec.as_deref())?;
                    Ok(Need::Done(Some(Value::str(&out))))
                }
            },
            Native::Sort {
                items,
                key,
                keys,
                reverse,
                idx,
                tasks,
                target,
                stage,
            } => {
                // Stage 0: key calls, one per element in order. Stage 1: the
                // merge sort, one comparison per step.
                if *stage == 0 {
                    if let Some(v) = input {
                        keys.push(v);
                    }
                    if let Some(k) = key {
                        if keys.len() < items.len() {
                            return Ok(Need::Call {
                                func: k.clone(),
                                args: vec![items[keys.len()].clone()],
                                kwargs: vec![],
                                self_uncharged: false,
                            });
                        }
                    } else {
                        *keys = items.clone();
                    }
                    *stage = 1;
                    *idx = (0..items.len()).collect();
                    tasks.push(SortTask::Split {
                        lo: 0,
                        hi: items.len(),
                    });
                    return self.step(m, None);
                }
                let mut input = input;
                loop {
                    let Some(task) = tasks.pop() else {
                        let sorted: Vec<Value> = idx.iter().map(|&i| items[i].clone()).collect();
                        return match target {
                            Some(list) => {
                                *list.items.borrow_mut() = sorted;
                                Ok(Need::Done(Some(Value::None)))
                            }
                            None => Ok(Need::Done(Some(m.new_list(sorted)))),
                        };
                    };
                    match task {
                        SortTask::Split { lo, hi } => {
                            if hi.wrapping_sub(lo) <= 1 {
                                continue;
                            }
                            let mid = lo.wrapping_add(hi.wrapping_sub(lo).wrapping_div(2));
                            tasks.push(SortTask::Merge {
                                lo,
                                mid,
                                hi,
                                left: vec![],
                                right: vec![],
                                i: 0,
                                j: 0,
                                k: lo,
                                started: false,
                            });
                            tasks.push(SortTask::Split { lo: mid, hi });
                            tasks.push(SortTask::Split { lo, hi: mid });
                        }
                        SortTask::Merge {
                            lo,
                            mid,
                            hi,
                            mut left,
                            mut right,
                            mut i,
                            mut j,
                            mut k,
                            started,
                        } => {
                            if !started {
                                left = idx[lo..mid].to_vec();
                                right = idx[mid..hi].to_vec();
                            } else if let Some(v) = input.take() {
                                // The answer to the comparison asked last time.
                                if as_bool(&v) {
                                    idx[k] = right[j];
                                    j = j.wrapping_add(1);
                                } else {
                                    idx[k] = left[i];
                                    i = i.wrapping_add(1);
                                }
                                k = k.wrapping_add(1);
                            }
                            if i < left.len() && j < right.len() {
                                let (l, r) = (keys[left[i]].clone(), keys[right[j]].clone());
                                tasks.push(SortTask::Merge {
                                    lo,
                                    mid,
                                    hi,
                                    left,
                                    right,
                                    i,
                                    j,
                                    k,
                                    started: true,
                                });
                                // Take `right` only when `right < left`;
                                // reversed, only when `left < right`.
                                return Ok(Need::Sub(if *reverse {
                                    Native::lt(l, r)
                                } else {
                                    Native::lt(r, l)
                                }));
                            }
                            while i < left.len() {
                                idx[k] = left[i];
                                i = i.wrapping_add(1);
                                k = k.wrapping_add(1);
                            }
                            while j < right.len() {
                                idx[k] = right[j];
                                j = j.wrapping_add(1);
                                k = k.wrapping_add(1);
                            }
                        }
                    }
                }
            }
            Native::MinMax {
                items,
                key,
                keys,
                is_min,
                best,
                i,
                stage,
            } => {
                if *stage == 0 {
                    if let Some(v) = input {
                        keys.push(v);
                    }
                    if let Some(k) = key {
                        if keys.len() < items.len() {
                            return Ok(Need::Call {
                                func: k.clone(),
                                args: vec![items[keys.len()].clone()],
                                kwargs: vec![],
                                self_uncharged: false,
                            });
                        }
                    } else {
                        *keys = items.clone();
                    }
                    *stage = 1;
                    *i = 1;
                    return self.step(m, None);
                }
                if let Some(v) = input {
                    if as_bool(&v) {
                        *best = *i;
                    }
                    *i = i.wrapping_add(1);
                }
                if *i >= items.len() {
                    return Ok(Need::Done(Some(items[*best].clone())));
                }
                let (x, b) = (keys[*i].clone(), keys[*best].clone());
                Ok(Need::Sub(if *is_min {
                    Native::lt(x, b)
                } else {
                    Native::lt(b, x)
                }))
            }
            Native::AnyAll { items, i, is_any } => {
                if let Some(v) = input {
                    let t = as_bool(&v);
                    if *is_any && t {
                        return Ok(Need::Done(Some(Value::Bool(true))));
                    }
                    if !*is_any && !t {
                        return Ok(Need::Done(Some(Value::Bool(false))));
                    }
                    *i = i.wrapping_add(1);
                }
                if *i >= items.len() {
                    return Ok(Need::Done(Some(Value::Bool(!*is_any))));
                }
                m.charge(c.factor_traverse);
                Ok(Need::Sub(Native::truthy(items[*i].clone())))
            }
            Native::Construct {
                class,
                args,
                kwargs,
                inst,
                then_raise,
            } => {
                let Some(obj) = inst.clone() else {
                    let obj = Rc::new(Instance {
                        id: m.alloc_id(),
                        class: class.clone(),
                        attrs: RefCell::new(vec![]),
                    });
                    if class.exc_base.is_some() {
                        obj.set("args", Value::tuple(args.clone()));
                    }
                    *inst = Some(obj.clone());
                    match obj.dunder("__init__") {
                        Some(f) => {
                            let mut full = vec![Value::Inst(obj.clone())];
                            full.append(args);
                            return Ok(Need::Call {
                                func: Value::Func(f),
                                args: full,
                                kwargs: std::mem::take(kwargs),
                                self_uncharged: true,
                            });
                        }
                        None => {
                            // Nothing to bind the arguments to, unless the
                            // class is an exception, whose `args` took them.
                            if class.exc_base.is_none() && (!args.is_empty() || !kwargs.is_empty())
                            {
                                return Err(type_error());
                            }
                            return finish_construct(obj, *then_raise);
                        }
                    }
                };
                if !matches!(input, Some(Value::None)) {
                    return Err(type_error());
                }
                finish_construct(obj, *then_raise)
            }
            Native::Materialise {
                obj,
                n,
                i,
                out,
                then,
                stage,
            } => {
                match *stage {
                    0 => {
                        // `len(x)` first, then `x[0]` … in order.
                        *stage = 1;
                        return dunder_call(obj, "__len__", vec![]).ok_or_else(type_error);
                    }
                    1 => {
                        *n = Some(len_result(&input.unwrap_or(Value::None))?);
                        m.check_size(n.unwrap_or(0))?;
                        *stage = 2;
                    }
                    2 => {
                        out.push(input.unwrap_or(Value::None));
                        *i = i.wrapping_add(1);
                    }
                    _ => return Ok(Need::Done(input)),
                }
                if *i < n.unwrap_or(0) {
                    m.charge(c.op_iteration);
                    return dunder_call(obj, "__getitem__", vec![Value::int(*i as i128)])
                        .ok_or_else(type_error);
                }
                let list = std::mem::take(out);
                *stage = 3;
                match then.take() {
                    Some(Then::Restore { below, above }) => {
                        let mid = m.new_list(list);
                        m.restore_and_rewind(below, mid, above);
                        Ok(Need::Done(None))
                    }
                    Some(Then::Builtin {
                        name,
                        mut args,
                        kwargs,
                        slot,
                    }) => {
                        args[slot] = m.new_list(list);
                        match builtins::builtin_entry(m, &name, &args, &kwargs)? {
                            Outcome::Value(v) => Ok(Need::Done(Some(v))),
                            Outcome::Native(n) => Ok(Need::Sub(n)),
                            // Only language builtins iterate an argument.
                            Outcome::NotOurs => Err(type_error()),
                        }
                    }
                    Some(Then::Method {
                        recv,
                        name,
                        mut args,
                        kwargs,
                        slot,
                    }) => {
                        args[slot] = m.new_list(list);
                        match builtins::method_entry(m, &recv, name, &args, &kwargs)? {
                            Outcome::Value(v) => Ok(Need::Done(Some(v))),
                            Outcome::Native(n) => Ok(Need::Sub(n)),
                            Outcome::NotOurs => Err(type_error()),
                        }
                    }
                    None => Ok(Need::Done(Some(m.new_list(list)))),
                }
            }
            Native::IterStart { obj } => match input {
                None => {
                    if obj.dunder("__getitem__").is_none() {
                        return Err(type_error());
                    }
                    dunder_call(obj, "__len__", vec![]).ok_or_else(type_error)
                }
                Some(v) => {
                    let n = len_result(&v)?;
                    Ok(Need::Done(Some(Value::Iter(Rc::new(
                        crate::value::IterObj {
                            items: vec![],
                            pos: std::cell::Cell::new(0),
                            inst: Some((obj.clone(), n)),
                        },
                    )))))
                }
            },
            Native::GetItem { obj, idx } => match input {
                None => dunder_call(obj, "__getitem__", vec![idx.clone()]).ok_or_else(type_error),
                Some(v) => Ok(Need::Done(Some(v))),
            },
            Native::SetItem { obj, idx, v, stage } => match *stage {
                0 => {
                    *stage = 1;
                    dunder_call(obj, "__setitem__", vec![idx.clone(), v.clone()])
                        .ok_or_else(type_error)
                }
                _ => Ok(Need::Done(None)),
            },
            Native::BinOp { op, a, b } => match input {
                None => {
                    let name = match op {
                        BinOp::Add => "__add__",
                        BinOp::Sub => "__sub__",
                        BinOp::Mul => "__mul__",
                        _ => return Err(type_error()),
                    };
                    dunder_call(a, name, vec![b.clone()]).ok_or_else(type_error)
                }
                Some(v) => Ok(Need::Done(Some(v))),
            },
            Native::Neg { a } => match input {
                None => dunder_call(a, "__neg__", vec![]).ok_or_else(type_error),
                Some(v) => Ok(Need::Done(Some(v))),
            },
            Native::Not { v, stage } => match *stage {
                0 => {
                    *stage = 1;
                    Ok(Need::Sub(Native::truthy(v.clone())))
                }
                _ => Ok(Need::Done(Some(Value::Bool(!as_bool(
                    &input.unwrap_or(Value::None),
                ))))),
            },
            Native::JumpTruth { v, stage } => match *stage {
                0 => {
                    *stage = 1;
                    Ok(Need::Sub(Native::truthy(v.clone())))
                }
                _ => Ok(Need::Done(Some(input.unwrap_or(Value::None)))),
            },
            Native::LogArgs {
                name,
                args,
                kwargs,
                i,
                out,
            } => {
                if *i > args.len() {
                    // The host answered.
                    return Ok(Need::Done(Some(input.unwrap_or(Value::None))));
                }
                if let Some(v) = input {
                    out.push(v);
                }
                if *i < args.len() {
                    let v = args[*i].clone();
                    *i = i.wrapping_add(1);
                    return Ok(Need::Sub(Native::str_of(v)));
                }
                *i = i.wrapping_add(1);
                Ok(Need::Host {
                    name: name.clone(),
                    args: std::mem::take(out),
                    kwargs: std::mem::take(kwargs),
                })
            }
        }
    }
}

fn finish_construct(obj: Rc<Instance>, then_raise: bool) -> R<Need> {
    if then_raise {
        return Err(Exception::from_instance(obj).ok_or_else(type_error)?);
    }
    Ok(Need::Done(Some(Value::Inst(obj))))
}

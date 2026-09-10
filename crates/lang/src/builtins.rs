//! The language's own builtins and the methods of the built-in types —
//! exactly the tables in `docs/01-language/syntax.md`, each priced by its
//! row in `costs.md`. Every one is a determinism obligation: nothing here
//! consults the host for anything but memory.

use crate::ast::BinOp;
use crate::dispatch::{FindMode, Native, Then};
use crate::errors::{ExcClass, Exception};
use crate::num::{Num, PLACES};
use crate::value::{R, Record, Value, check_key, dict_find, has_instance, quoted, set_find};
use crate::vm::{Machine, method};
use std::rc::Rc;

/// How a builtin or method call ends: with a value, with a native that
/// still has user code to run (`dispatch.rs`), or not here at all — a game
/// builtin, which is the host's.
pub enum Outcome {
    Value(Value),
    Native(Native),
    NotOurs,
}

/// The argument positions a builtin iterates, so an instance there is
/// gathered (`__len__`, then `__getitem__` in order) before the builtin
/// runs, as Python does.
fn iterable_slots(name: &str, argc: usize) -> &'static [usize] {
    match name {
        "list" | "set" | "dict" | "sorted" | "sum" | "enumerate" | "any" | "all" => &[0],
        "min" | "max" if argc == 1 => &[0],
        "zip" => &[0, 1, 2, 3, 4, 5, 6, 7],
        _ => &[],
    }
}

/// The entry the VM calls: gathers instance iterables, routes what needs
/// user code to a native, and runs the rest synchronously.
pub fn builtin_entry(
    m: &mut Machine,
    name: &str,
    args: &[Value],
    kwargs: &[(String, Value)],
) -> R<Outcome> {
    for &slot in iterable_slots(name, args.len()) {
        if let Some(Value::Inst(inst)) = args.get(slot) {
            let then = Then::Builtin {
                name: name.to_string(),
                args: args.to_vec(),
                kwargs: kwargs.to_vec(),
                slot,
            };
            return Ok(Outcome::Native(Native::materialise(inst.clone(), then)));
        }
    }
    let c = m.costs().clone();
    let depth = m.limits().depth_nesting as u32;
    let no_kw = |kwargs: &[(String, Value)]| -> R<()> {
        if kwargs.is_empty() {
            Ok(())
        } else {
            Err(Exception::type_error())
        }
    };
    match name {
        "len" if matches!(args, [Value::Inst(_)]) => {
            no_kw(kwargs)?;
            m.charge(c.builtin_len);
            Ok(Outcome::Native(Native::Len {
                v: args[0].clone(),
                stage: 0,
            }))
        }
        "str" if args.len() == 1 && has_instance(&args[0], depth) => {
            no_kw(kwargs)?;
            m.charge(c.builtin_convert);
            Ok(Outcome::Native(Native::str_of(args[0].clone())))
        }
        "bool" if matches!(args, [Value::Inst(_)]) => {
            no_kw(kwargs)?;
            m.charge(c.builtin_convert);
            Ok(Outcome::Native(Native::truthy(args[0].clone())))
        }
        "sorted" => {
            let key = kwarg(kwargs, "key", &["key", "reverse"])?;
            let reverse =
                kwarg(kwargs, "reverse", &["key", "reverse"])?.is_some_and(|v| v.truthy());
            m.charge(c.builtin_sorted);
            let items = m.snapshot(one(args)?)?;
            m.charge_each(items.len(), c.factor_sort);
            Ok(Outcome::Native(Native::sort(items, key, reverse, None)))
        }
        "min" | "max" => {
            let key = kwarg(kwargs, "key", &["key"])?;
            m.charge(c.builtin_minmax);
            let items: Vec<Value> = match args {
                [single] if !matches!(single, Value::Num(_) | Value::Bool(_)) => {
                    m.snapshot(single)?
                }
                [] => return Err(Exception::type_error()),
                many => many.to_vec(),
            };
            if items.is_empty() {
                return Err(Exception::value_error());
            }
            m.charge_each(items.len(), c.factor_traverse);
            Ok(Outcome::Native(Native::min_max(items, key, name == "min")))
        }
        "any" | "all" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_anyall);
            let items = m.snapshot(one(args)?)?;
            Ok(Outcome::Native(Native::AnyAll {
                items,
                i: 0,
                is_any: name == "any",
            }))
        }
        _ => Ok(match call_builtin(m, name, args, kwargs)? {
            Some(v) => Outcome::Value(v),
            None => Outcome::NotOurs,
        }),
    }
}

/// The method entry the VM calls, with the same routing as
/// [`builtin_entry`].
pub fn method_entry(
    m: &mut Machine,
    recv: &Value,
    name: &'static str,
    args: &[Value],
    kwargs: &[(String, Value)],
) -> R<Outcome> {
    let gathers = matches!(
        (recv, name),
        (Value::List(_), "extend") | (Value::Str(_), "join")
    );
    if gathers && let Some(Value::Inst(inst)) = args.first() {
        let then = Then::Method {
            recv: recv.clone(),
            name,
            args: args.to_vec(),
            kwargs: kwargs.to_vec(),
            slot: 0,
        };
        return Ok(Outcome::Native(Native::materialise(inst.clone(), then)));
    }
    let c = m.costs().clone();
    match (recv, name) {
        (Value::List(l), "sort") => {
            let key = kwarg(kwargs, "key", &["key", "reverse"])?;
            let reverse =
                kwarg(kwargs, "reverse", &["key", "reverse"])?.is_some_and(|v| v.truthy());
            if !args.is_empty() {
                return Err(Exception::type_error());
            }
            m.charge(c.method_list_sort);
            let items = l.items.borrow().clone();
            m.charge_each(items.len(), c.factor_sort);
            Ok(Outcome::Native(Native::sort(
                items,
                key,
                reverse,
                Some(l.clone()),
            )))
        }
        (Value::List(l), "remove" | "index" | "count") => {
            if !kwargs.is_empty() {
                return Err(Exception::type_error());
            }
            m.charge(c.method_list_search);
            let target = one(args)?.clone();
            let mode = match name {
                "remove" => FindMode::Remove(l.clone()),
                "index" => FindMode::Index,
                _ => FindMode::Count,
            };
            Ok(Outcome::Native(Native::Find {
                items: l.items.borrow().clone(),
                target,
                i: 0,
                count: 0,
                mode,
            }))
        }
        (Value::Tuple(t), "index" | "count") => {
            if !kwargs.is_empty() {
                return Err(Exception::type_error());
            }
            m.charge(c.method_list_search);
            let target = one(args)?.clone();
            let mode = if name == "index" {
                FindMode::Index
            } else {
                FindMode::Count
            };
            Ok(Outcome::Native(Native::Find {
                items: t.to_vec(),
                target,
                i: 0,
                count: 0,
                mode,
            }))
        }
        _ => call_method(m, recv, name, args, kwargs).map(Outcome::Value),
    }
}

const BUILTINS: &[&str] = &[
    "len",
    "range",
    "min",
    "max",
    "sum",
    "abs",
    "sorted",
    "enumerate",
    "zip",
    "any",
    "all",
    "int",
    "num",
    "str",
    "bool",
    "list",
    "dict",
    "set",
    "isinstance",
    "round",
];

/// The types `isinstance` and a class pattern may name.
const TYPE_NAMES: &[&str] = &["num", "str", "bool", "list", "tuple", "dict", "set"];

/// A builtin or exception class by name, or `None`.
pub fn lookup(name: &str) -> Option<Value> {
    if BUILTINS.contains(&name) || name == "tuple" {
        return Some(Value::Builtin(Rc::from(name)));
    }
    ExcClass::from_name(name).map(Value::ExcClass)
}

fn one(args: &[Value]) -> R<&Value> {
    match args {
        [a] => Ok(a),
        _ => Err(Exception::type_error()),
    }
}

fn integral_index(v: &Value) -> R<i128> {
    v.expect_integral()
}

/// Call a language builtin; `Ok(None)` if the name is the game's.
pub fn call_builtin(
    m: &mut Machine,
    name: &str,
    args: &[Value],
    kwargs: &[(String, Value)],
) -> R<Option<Value>> {
    let c = m.costs().clone();
    let no_kw = |kwargs: &[(String, Value)]| -> R<()> {
        if kwargs.is_empty() {
            Ok(())
        } else {
            Err(Exception::type_error())
        }
    };
    let v = match name {
        "len" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_len);
            Value::int(length(one(args)?)? as i128)
        }
        "range" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_range);
            let (start, stop, step) = match args {
                [stop] => (0, integral_index(stop)?, 1),
                [start, stop] => (integral_index(start)?, integral_index(stop)?, 1),
                [start, stop, step] => (
                    integral_index(start)?,
                    integral_index(stop)?,
                    integral_index(step)?,
                ),
                _ => return Err(Exception::type_error()),
            };
            if step == 0 {
                return Err(Exception::value_error());
            }
            let mut items = Vec::new();
            let mut i = start;
            while (step > 0 && i < stop) || (step < 0 && i > stop) {
                m.check_size(items.len().wrapping_add(1))?;
                items.push(Value::int(i));
                i = i
                    .checked_add(step)
                    .ok_or_else(|| Exception::new(ExcClass::OverflowError, vec![]))?;
            }
            m.charge_each(items.len(), c.factor_copy);
            m.new_list(items)
        }
        "sum" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_sum);
            let (iterable, start) = match args {
                [it] => (it, Num::ZERO),
                [it, s] => (it, s.expect_num()?),
                _ => return Err(Exception::type_error()),
            };
            let items = m.snapshot(iterable)?;
            m.charge_each(items.len(), c.factor_traverse);
            let mut acc = start;
            for v in items {
                acc = acc.checked_add(v.expect_num()?)?;
            }
            Value::Num(acc)
        }
        "abs" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_abs);
            Value::Num(one(args)?.expect_num()?.checked_abs()?)
        }
        "enumerate" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_enumerate);
            let (iterable, start) = match args {
                [it] => (it, 0),
                [it, s] => (it, integral_index(s)?),
                _ => return Err(Exception::type_error()),
            };
            let items = m.snapshot(iterable)?;
            m.charge_each(items.len(), c.factor_copy);
            let mut out = Vec::with_capacity(items.len());
            for (i, v) in items.into_iter().enumerate() {
                let idx = start
                    .checked_add(i as i128)
                    .ok_or_else(|| Exception::new(ExcClass::OverflowError, vec![]))?;
                out.push(Value::tuple(vec![Value::int(idx), v]));
            }
            m.new_list(out)
        }
        "zip" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_zip);
            let lists: Vec<Vec<Value>> = args.iter().map(|a| m.snapshot(a)).collect::<R<_>>()?;
            let total: usize = lists.iter().map(Vec::len).sum();
            m.charge_each(total, c.factor_copy);
            let n = lists.iter().map(Vec::len).min().unwrap_or(0);
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                out.push(Value::tuple(lists.iter().map(|l| l[i].clone()).collect()));
            }
            m.new_list(out)
        }
        "int" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_convert);
            match one(args)? {
                Value::Str(s) => {
                    m.charge_each(s.chars().count(), c.factor_char);
                    Value::Num(Num::parse_int_str(s)?)
                }
                v => Value::Num(v.expect_num()?.trunc()),
            }
        }
        "num" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_convert);
            match one(args)? {
                Value::Str(s) => {
                    m.charge_each(s.chars().count(), c.factor_char);
                    Value::Num(Num::parse_num_str(s)?)
                }
                v => Value::Num(v.expect_num()?),
            }
        }
        "str" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_convert);
            let s = one(args)?.to_str_value();
            m.charge_each(s.chars().count(), c.factor_char);
            Value::str(&s)
        }
        "bool" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_convert);
            Value::Bool(one(args)?.truthy())
        }
        "list" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_construct);
            let items = match args {
                [] => vec![],
                [it] => m.snapshot(it)?,
                _ => return Err(Exception::type_error()),
            };
            m.charge_each(items.len(), c.factor_copy);
            m.new_list(items)
        }
        "dict" => {
            m.charge(c.builtin_construct);
            let mut items: Vec<(Value, Value)> = match args {
                [] => vec![],
                [Value::Dict(d)] => d.items.borrow().clone(),
                [it] => {
                    let pairs = m.snapshot(it)?;
                    let mut out = Vec::with_capacity(pairs.len());
                    for p in pairs {
                        let Value::Tuple(t) = &p else {
                            return Err(Exception::type_error());
                        };
                        let [k, v] = &t[..] else {
                            return Err(Exception::type_error());
                        };
                        check_key(k)?;
                        out.push((k.clone(), v.clone()));
                    }
                    out
                }
                _ => return Err(Exception::type_error()),
            };
            for (k, v) in kwargs {
                items.push((Value::str(k), v.clone()));
            }
            m.charge_each(items.len(), c.factor_copy);
            let deduped = dedup_pairs(m, items)?;
            m.new_dict(deduped)
        }
        "set" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_construct);
            let items = match args {
                [] => vec![],
                [it] => m.snapshot(it)?,
                _ => return Err(Exception::type_error()),
            };
            m.charge_each(items.len(), c.factor_copy);
            let deduped = dedup_values(m, items)?;
            m.new_set(deduped)
        }
        "tuple" => return Err(Exception::type_error()),
        "isinstance" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_isinstance);
            let [v, t] = args else {
                return Err(Exception::type_error());
            };
            Value::Bool(is_instance(v, t)?)
        }
        "round" => {
            no_kw(kwargs)?;
            m.charge(c.builtin_round);
            match args {
                [x] => Value::Num(x.expect_num()?.round_half_even(0)),
                [x, n] => {
                    let places = n.expect_integral()?;
                    if !(0..=PLACES as i128).contains(&places) {
                        return Err(Exception::value_error());
                    }
                    Value::Num(x.expect_num()?.round_half_even(places as u32))
                }
                _ => return Err(Exception::type_error()),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(v))
}

fn kwarg(kwargs: &[(String, Value)], name: &str, allowed: &[&str]) -> R<Option<Value>> {
    for (k, _) in kwargs {
        if !allowed.contains(&k.as_str()) {
            return Err(Exception::type_error());
        }
    }
    Ok(kwargs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone()))
}

pub fn is_instance(v: &Value, t: &Value) -> R<bool> {
    match t {
        Value::Tuple(ts) => {
            for t in ts.iter() {
                if is_instance(v, t)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Value::Builtin(name) if TYPE_NAMES.contains(&&**name) => Ok(match &**name {
            "num" => matches!(v, Value::Num(_) | Value::Bool(_)),
            "bool" => matches!(v, Value::Bool(_)),
            "str" => matches!(v, Value::Str(_)),
            "list" => matches!(v, Value::List(_)),
            "tuple" => matches!(v, Value::Tuple(_)),
            "dict" => matches!(v, Value::Dict(_)),
            "set" => matches!(v, Value::Set(_)),
            _ => false,
        }),
        Value::ExcClass(class) => Ok(match v {
            Value::Exc(e) => e.class.is_a(*class),
            Value::Inst(i) => i.class.is_exception(*class),
            _ => false,
        }),
        Value::Class(c) => Ok(matches!(v, Value::Inst(i) if i.class.is_subclass_of(c))),
        _ => Err(Exception::type_error()),
    }
}

pub fn length(v: &Value) -> R<usize> {
    Ok(match v {
        Value::Str(s) => s.chars().count(),
        Value::List(l) => l.items.borrow().len(),
        Value::Tuple(t) => t.len(),
        Value::Dict(d) => d.items.borrow().len(),
        Value::Set(s) => s.items.borrow().len(),
        _ => return Err(Exception::type_error()),
    })
}

fn dedup_pairs(m: &Machine, items: Vec<(Value, Value)>) -> R<Vec<(Value, Value)>> {
    let depth = m.limits().depth_nesting as u32;
    let mut out: Vec<(Value, Value)> = Vec::with_capacity(items.len());
    for (k, v) in items {
        check_key(&k)?;
        match dict_find(&out, &k, depth)? {
            Some(i) => out[i].1 = v,
            None => out.push((k, v)),
        }
    }
    m.check_size(out.len())?;
    Ok(out)
}

fn dedup_values(m: &Machine, items: Vec<Value>) -> R<Vec<Value>> {
    let depth = m.limits().depth_nesting as u32;
    let mut out: Vec<Value> = Vec::with_capacity(items.len());
    for v in items {
        check_key(&v)?;
        if set_find(&out, &v, depth)?.is_none() {
            out.push(v);
        }
    }
    m.check_size(out.len())?;
    Ok(out)
}

// -------------------------------------------------------------- access ---

/// `x.name`: a method of a built-in type, a record field, or an exception
/// attribute.
pub fn attribute(obj: &Value, name: &str) -> R<Value> {
    let bound = |n: &'static str| Ok(method(obj.clone(), n));
    match obj {
        Value::List(_) => match name {
            "append" => bound("append"),
            "extend" => bound("extend"),
            "insert" => bound("insert"),
            "pop" => bound("pop"),
            "remove" => bound("remove"),
            "index" => bound("index"),
            "count" => bound("count"),
            "sort" => bound("sort"),
            "reverse" => bound("reverse"),
            "clear" => bound("clear"),
            "copy" => bound("copy"),
            _ => Err(Exception::attribute_error(name)),
        },
        Value::Dict(_) => match name {
            "get" => bound("get"),
            "keys" => bound("keys"),
            "values" => bound("values"),
            "items" => bound("items"),
            "pop" => bound("pop"),
            "setdefault" => bound("setdefault"),
            "update" => bound("update"),
            "clear" => bound("clear"),
            "copy" => bound("copy"),
            _ => Err(Exception::attribute_error(name)),
        },
        Value::Set(_) => match name {
            "add" => bound("add"),
            "remove" => bound("remove"),
            "discard" => bound("discard"),
            "pop" => bound("pop"),
            "clear" => bound("clear"),
            "copy" => bound("copy"),
            _ => Err(Exception::attribute_error(name)),
        },
        Value::Str(_) => match name {
            "split" => bound("split"),
            "join" => bound("join"),
            "strip" => bound("strip"),
            "lstrip" => bound("lstrip"),
            "rstrip" => bound("rstrip"),
            "startswith" => bound("startswith"),
            "endswith" => bound("endswith"),
            "find" => bound("find"),
            "replace" => bound("replace"),
            "upper" => bound("upper"),
            "lower" => bound("lower"),
            "isdigit" => bound("isdigit"),
            "isalpha" => bound("isalpha"),
            "zfill" => bound("zfill"),
            _ => Err(Exception::attribute_error(name)),
        },
        Value::Tuple(_) => match name {
            "index" => bound("index"),
            "count" => bound("count"),
            _ => Err(Exception::attribute_error(name)),
        },
        Value::Exc(e) => match name {
            "args" => Ok(Value::tuple(e.args.clone())),
            "file" => Ok(e.file.as_deref().map(Value::str).unwrap_or(Value::None)),
            "line" => Ok(Value::int(e.line as i128)),
            "tick" => Ok(Value::int(e.tick as i128)),
            _ => Err(Exception::attribute_error(name)),
        },
        Value::Record(r) => r
            .fields
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| Exception::attribute_error(name)),
        Value::Inst(i) => i.get(name).ok_or_else(|| Exception::attribute_error(name)),
        Value::Class(c) => c
            .lookup(name)
            .ok_or_else(|| Exception::attribute_error(name)),
        // A module's attributes are its globals (`syntax.md`, `import`).
        Value::Module(m) => m
            .globals
            .borrow()
            .get(name)
            .cloned()
            .ok_or_else(|| Exception::attribute_error(name)),
        _ => Err(Exception::attribute_error(name)),
    }
}

fn is_ascii_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c' | '\x0b')
}

/// Call a method of a built-in type.
pub fn call_method(
    m: &mut Machine,
    recv: &Value,
    name: &str,
    args: &[Value],
    kwargs: &[(String, Value)],
) -> R<Value> {
    let c = m.costs().clone();
    let depth = m.limits().depth_nesting as u32;
    match recv {
        Value::List(l) => {
            if !kwargs.is_empty() && name != "sort" {
                return Err(Exception::type_error());
            }
            match name {
                "append" => {
                    m.charge(c.method_list_append);
                    let v = one(args)?.clone();
                    let mut items = l.items.borrow_mut();
                    m.check_size(items.len().wrapping_add(1))?;
                    items.push(v);
                    Ok(Value::None)
                }
                "extend" => {
                    m.charge(c.method_list_extend);
                    let add = m.snapshot(one(args)?)?;
                    m.charge_each(add.len(), c.factor_copy);
                    let mut items = l.items.borrow_mut();
                    m.check_size(items.len().saturating_add(add.len()))?;
                    items.extend(add);
                    Ok(Value::None)
                }
                "insert" => {
                    m.charge(c.method_list_insert);
                    let [i, v] = args else {
                        return Err(Exception::type_error());
                    };
                    let mut items = l.items.borrow_mut();
                    m.check_size(items.len().wrapping_add(1))?;
                    let at = clamp_index(integral_index(i)?, items.len());
                    m.charge_each(items.len().wrapping_sub(at), c.factor_traverse);
                    items.insert(at, v.clone());
                    Ok(Value::None)
                }
                "pop" => {
                    m.charge(c.method_list_pop);
                    let mut items = l.items.borrow_mut();
                    let at = match args {
                        [] => items.len().checked_sub(1),
                        [i] => normalize_index(integral_index(i)?, items.len()),
                        _ => return Err(Exception::type_error()),
                    }
                    .ok_or_else(|| Exception::new(ExcClass::IndexError, vec![]))?;
                    m.charge_each(items.len().wrapping_sub(at), c.factor_traverse);
                    Ok(items.remove(at))
                }
                "reverse" => {
                    m.charge(c.method_list_reverse);
                    let mut items = l.items.borrow_mut();
                    m.charge_each(items.len(), c.factor_traverse);
                    items.reverse();
                    Ok(Value::None)
                }
                "clear" => {
                    m.charge(c.method_list_clear);
                    l.items.borrow_mut().clear();
                    Ok(Value::None)
                }
                "copy" => {
                    m.charge(c.method_list_copy);
                    let items = l.items.borrow().clone();
                    m.charge_each(items.len(), c.factor_copy);
                    Ok(m.new_list(items))
                }
                _ => Err(Exception::attribute_error(name)),
            }
        }
        Value::Dict(d) => {
            if !kwargs.is_empty() && name != "update" {
                return Err(Exception::type_error());
            }
            match name {
                "get" | "pop" | "setdefault" => {
                    m.charge(c.method_dict_lookup);
                    let (k, default) = match args {
                        [k] => (k, None),
                        [k, d] => (k, Some(d.clone())),
                        _ => return Err(Exception::type_error()),
                    };
                    let mut items = d.items.borrow_mut();
                    let pos = dict_find(&items, k, depth)?;
                    match (name, pos) {
                        ("get", Some(i)) => Ok(items[i].1.clone()),
                        ("get", None) => Ok(default.unwrap_or(Value::None)),
                        ("pop", Some(i)) => Ok(items.remove(i).1),
                        ("pop", None) => default
                            .ok_or_else(|| Exception::new(ExcClass::KeyError, vec![k.clone()])),
                        ("setdefault", Some(i)) => Ok(items[i].1.clone()),
                        (_, None) => {
                            check_key(k)?;
                            let v = default.unwrap_or(Value::None);
                            m.check_size(items.len().wrapping_add(1))?;
                            items.push((k.clone(), v.clone()));
                            Ok(v)
                        }
                        _ => Err(Exception::type_error()),
                    }
                }
                "keys" | "values" | "items" => {
                    m.charge(c.method_dict_view);
                    let items = d.items.borrow();
                    m.charge_each(items.len(), c.factor_copy);
                    let out: Vec<Value> = match name {
                        "keys" => items.iter().map(|(k, _)| k.clone()).collect(),
                        "values" => items.iter().map(|(_, v)| v.clone()).collect(),
                        _ => items
                            .iter()
                            .map(|(k, v)| Value::tuple(vec![k.clone(), v.clone()]))
                            .collect(),
                    };
                    drop(items);
                    Ok(m.new_list(out))
                }
                "update" => {
                    m.charge(c.method_dict_update);
                    let mut add: Vec<(Value, Value)> = match args {
                        [] => vec![],
                        [Value::Dict(o)] => o.items.borrow().clone(),
                        _ => return Err(Exception::type_error()),
                    };
                    for (k, v) in kwargs {
                        add.push((Value::str(k), v.clone()));
                    }
                    m.charge_each(add.len(), c.factor_copy);
                    let mut items = d.items.borrow_mut();
                    for (k, v) in add {
                        check_key(&k)?;
                        match dict_find(&items, &k, depth)? {
                            Some(i) => items[i].1 = v,
                            None => {
                                m.check_size(items.len().wrapping_add(1))?;
                                items.push((k, v));
                            }
                        }
                    }
                    Ok(Value::None)
                }
                "clear" => {
                    m.charge(c.method_dict_clear);
                    d.items.borrow_mut().clear();
                    Ok(Value::None)
                }
                "copy" => {
                    m.charge(c.method_dict_copy);
                    let items = d.items.borrow().clone();
                    m.charge_each(items.len(), c.factor_copy);
                    Ok(m.new_dict(items))
                }
                _ => Err(Exception::attribute_error(name)),
            }
        }
        Value::Set(s) => {
            if !kwargs.is_empty() {
                return Err(Exception::type_error());
            }
            match name {
                "add" | "remove" | "discard" => {
                    m.charge(c.method_set_single);
                    let v = one(args)?;
                    let mut items = s.items.borrow_mut();
                    let pos = set_find(&items, v, depth)?;
                    match (name, pos) {
                        ("add", None) => {
                            check_key(v)?;
                            m.check_size(items.len().wrapping_add(1))?;
                            items.push(v.clone());
                        }
                        ("add", Some(_)) => {}
                        (_, Some(i)) => {
                            items.remove(i);
                        }
                        ("remove", None) => {
                            return Err(Exception::new(ExcClass::KeyError, vec![v.clone()]));
                        }
                        _ => {}
                    }
                    Ok(Value::None)
                }
                "pop" => {
                    m.charge(c.method_set_single);
                    let mut items = s.items.borrow_mut();
                    if items.is_empty() {
                        return Err(Exception::new(ExcClass::KeyError, vec![]));
                    }
                    Ok(items.remove(0))
                }
                "clear" => {
                    m.charge(c.method_set_clear);
                    s.items.borrow_mut().clear();
                    Ok(Value::None)
                }
                "copy" => {
                    m.charge(c.method_set_copy);
                    let items = s.items.borrow().clone();
                    m.charge_each(items.len(), c.factor_copy);
                    Ok(m.new_set(items))
                }
                _ => Err(Exception::attribute_error(name)),
            }
        }
        Value::Str(s) => {
            if !kwargs.is_empty() {
                return Err(Exception::type_error());
            }
            let n = s.chars().count();
            match name {
                "split" => {
                    m.charge(c.method_str_split);
                    m.charge_each(n, c.factor_char);
                    let parts: Vec<Value> = match args {
                        [] => s
                            .split(is_ascii_ws)
                            .filter(|p| !p.is_empty())
                            .map(Value::str)
                            .collect(),
                        [sep] => {
                            let sep = sep.expect_str()?;
                            if sep.is_empty() {
                                return Err(Exception::value_error());
                            }
                            s.split(&*sep).map(Value::str).collect()
                        }
                        _ => return Err(Exception::type_error()),
                    };
                    Ok(m.new_list(parts))
                }
                "join" => {
                    m.charge(c.method_str_join);
                    let items = m.snapshot(one(args)?)?;
                    let mut out = String::new();
                    for (i, v) in items.iter().enumerate() {
                        if i > 0 {
                            out.push_str(s);
                        }
                        out.push_str(&v.expect_str()?);
                    }
                    m.charge_each(out.chars().count(), c.factor_char);
                    m.check_size(out.chars().count())?;
                    Ok(Value::str(&out))
                }
                "strip" | "lstrip" | "rstrip" => {
                    m.charge(c.method_str_strip);
                    m.charge_each(n, c.factor_char);
                    let out = match (name, args) {
                        ("strip", []) => s.trim_matches(is_ascii_ws).to_string(),
                        ("lstrip", []) => s.trim_start_matches(is_ascii_ws).to_string(),
                        ("rstrip", []) => s.trim_end_matches(is_ascii_ws).to_string(),
                        (_, [chars]) => {
                            let set: Vec<char> = chars.expect_str()?.chars().collect();
                            let f = |c: char| set.contains(&c);
                            match name {
                                "strip" => s.trim_matches(f).to_string(),
                                "lstrip" => s.trim_start_matches(f).to_string(),
                                _ => s.trim_end_matches(f).to_string(),
                            }
                        }
                        _ => return Err(Exception::type_error()),
                    };
                    Ok(Value::str(&out))
                }
                "startswith" | "endswith" => {
                    m.charge(c.method_str_affix);
                    let affix = one(args)?.expect_str()?;
                    m.charge_each(affix.chars().count(), c.factor_char);
                    Ok(Value::Bool(if name == "startswith" {
                        s.starts_with(&*affix)
                    } else {
                        s.ends_with(&*affix)
                    }))
                }
                "find" => {
                    m.charge(c.method_str_scan);
                    m.charge_each(n, c.factor_char);
                    let needle = one(args)?.expect_str()?;
                    Ok(match s.find(&*needle) {
                        Some(byte) => Value::int(s[..byte].chars().count() as i128),
                        None => Value::int(-1),
                    })
                }
                "replace" => {
                    m.charge(c.method_str_scan);
                    m.charge_each(n, c.factor_char);
                    let [from, to] = args else {
                        return Err(Exception::type_error());
                    };
                    let (from, to) = (from.expect_str()?, to.expect_str()?);
                    if from.is_empty() {
                        return Err(Exception::value_error());
                    }
                    let out = s.replace(&*from, &to);
                    m.check_size(out.chars().count())?;
                    Ok(Value::str(&out))
                }
                "upper" | "lower" => {
                    m.charge(c.method_str_map);
                    m.charge_each(n, c.factor_char);
                    Ok(Value::str(&if name == "upper" {
                        s.to_ascii_uppercase()
                    } else {
                        s.to_ascii_lowercase()
                    }))
                }
                "zfill" => {
                    m.charge(c.method_str_map);
                    let width = integral_index(one(args)?)?;
                    let width = usize::try_from(width).unwrap_or(0);
                    let (sign, body) = match s.strip_prefix('-') {
                        Some(rest) => ("-", rest),
                        None => ("", &**s),
                    };
                    let pad = width.saturating_sub(n);
                    let out = format!("{sign}{}{body}", "0".repeat(pad));
                    m.charge_each(out.chars().count(), c.factor_char);
                    m.check_size(out.chars().count())?;
                    Ok(Value::str(&out))
                }
                "isdigit" | "isalpha" => {
                    m.charge(c.method_str_test);
                    m.charge_each(n, c.factor_char);
                    let ok = !s.is_empty()
                        && s.chars().all(|ch| {
                            if name == "isdigit" {
                                ch.is_ascii_digit()
                            } else {
                                ch.is_ascii_alphabetic()
                            }
                        });
                    Ok(Value::Bool(ok))
                }
                _ => Err(Exception::attribute_error(name)),
            }
        }
        _ => Err(Exception::attribute_error(name)),
    }
}

/// A negative index counts from the end; out of range is `None`.
fn normalize_index(i: i128, len: usize) -> Option<usize> {
    let len_i = len as i128;
    let i = if i < 0 { i.checked_add(len_i)? } else { i };
    if i < 0 || i >= len_i {
        None
    } else {
        usize::try_from(i).ok()
    }
}

/// `insert`'s clamping: past the end appends, before the start prepends.
fn clamp_index(i: i128, len: usize) -> usize {
    let len_i = len as i128;
    let i = if i < 0 {
        i.saturating_add(len_i).max(0)
    } else {
        i.min(len_i)
    };
    usize::try_from(i).unwrap_or(0)
}

/// Slice bounds as Python computes them.
fn slice_bounds(lo: &Value, hi: &Value, step: &Value, len: usize) -> R<(i128, i128, i128)> {
    let step = match step {
        Value::None => 1,
        v => integral_index(v)?,
    };
    if step == 0 {
        return Err(Exception::value_error());
    }
    let len_i = len as i128;
    let clamp = |v: &Value, default: i128| -> R<i128> {
        Ok(match v {
            Value::None => default,
            v => {
                let i = integral_index(v)?;
                let i = if i < 0 { i.saturating_add(len_i) } else { i };
                if step > 0 {
                    i.clamp(0, len_i)
                } else {
                    i.clamp(-1, len_i.saturating_sub(1))
                }
            }
        })
    };
    let (dlo, dhi) = if step > 0 {
        (0, len_i)
    } else {
        (len_i.saturating_sub(1), -1)
    };
    Ok((clamp(lo, dlo)?, clamp(hi, dhi)?, step))
}

fn slice_indices(lo: i128, hi: i128, step: i128) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = lo;
    while (step > 0 && i < hi) || (step < 0 && i > hi) {
        if let Ok(u) = usize::try_from(i) {
            out.push(u);
        }
        i = i.saturating_add(step);
    }
    out
}

/// `obj[idx]`.
pub fn subscript(m: &mut Machine, obj: &Value, idx: &Value) -> R<Value> {
    let c = m.costs().clone();
    if let Value::Record(r) = idx
        && r.type_name == "slice"
    {
        let (lo, hi, step) = (&r.fields[0].1, &r.fields[1].1, &r.fields[2].1);
        return match obj {
            Value::List(l) => {
                let items = l.items.borrow();
                let (a, b, s) = slice_bounds(lo, hi, step, items.len())?;
                let out: Vec<Value> = slice_indices(a, b, s)
                    .into_iter()
                    .filter_map(|i| items.get(i).cloned())
                    .collect();
                m.charge_each(out.len(), c.factor_copy);
                drop(items);
                Ok(m.new_list(out))
            }
            Value::Tuple(t) => {
                let (a, b, s) = slice_bounds(lo, hi, step, t.len())?;
                let out: Vec<Value> = slice_indices(a, b, s)
                    .into_iter()
                    .filter_map(|i| t.get(i).cloned())
                    .collect();
                m.charge_each(out.len(), c.factor_copy);
                Ok(Value::tuple(out))
            }
            Value::Str(st) => {
                let chars: Vec<char> = st.chars().collect();
                let (a, b, s) = slice_bounds(lo, hi, step, chars.len())?;
                let out: String = slice_indices(a, b, s)
                    .into_iter()
                    .filter_map(|i| chars.get(i))
                    .collect();
                m.charge_each(out.chars().count(), c.factor_copy);
                Ok(Value::str(&out))
            }
            _ => Err(Exception::type_error()),
        };
    }
    match obj {
        Value::List(l) => {
            let items = l.items.borrow();
            let i = normalize_index(integral_index(idx)?, items.len())
                .ok_or_else(|| Exception::new(ExcClass::IndexError, vec![idx.clone()]))?;
            Ok(items[i].clone())
        }
        Value::Tuple(t) => {
            let i = normalize_index(integral_index(idx)?, t.len())
                .ok_or_else(|| Exception::new(ExcClass::IndexError, vec![idx.clone()]))?;
            Ok(t[i].clone())
        }
        Value::Str(s) => {
            let n = s.chars().count();
            let i = normalize_index(integral_index(idx)?, n)
                .ok_or_else(|| Exception::new(ExcClass::IndexError, vec![idx.clone()]))?;
            Ok(Value::str(
                &s.chars().nth(i).map(|c| c.to_string()).unwrap_or_default(),
            ))
        }
        Value::Dict(d) => {
            let depth = m.limits().depth_nesting as u32;
            if let Value::Tuple(t) = idx {
                m.charge_each(t.len(), c.factor_traverse);
            }
            let items = d.items.borrow();
            match dict_find(&items, idx, depth)? {
                Some(i) => Ok(items[i].1.clone()),
                None => Err(Exception::new(ExcClass::KeyError, vec![idx.clone()])),
            }
        }
        _ => Err(Exception::type_error()),
    }
}

/// `obj[idx] = v`.
pub fn store_subscript(m: &mut Machine, obj: &Value, idx: Value, v: Value) -> R<()> {
    match obj {
        Value::List(l) => {
            let mut items = l.items.borrow_mut();
            let i = normalize_index(integral_index(&idx)?, items.len())
                .ok_or_else(|| Exception::new(ExcClass::IndexError, vec![idx.clone()]))?;
            items[i] = v;
            Ok(())
        }
        Value::Dict(d) => {
            check_key(&idx)?;
            let depth = m.limits().depth_nesting as u32;
            let mut items = d.items.borrow_mut();
            match dict_find(&items, &idx, depth)? {
                Some(i) => items[i].1 = v,
                None => {
                    m.check_size(items.len().wrapping_add(1))?;
                    items.push((idx, v));
                }
            }
            Ok(())
        }
        _ => Err(Exception::type_error()),
    }
}

/// `x in container`.
pub fn contains(m: &mut Machine, container: &Value, x: &Value) -> R<bool> {
    let c = m.costs().clone();
    let depth = m.limits().depth_nesting as u32;
    match container {
        Value::Str(s) => {
            let needle = x.expect_str()?;
            m.charge_each(s.chars().count(), c.factor_char);
            Ok(s.contains(&*needle))
        }
        // Lists and tuples with no instance inside: the plain walk. With
        // one, `dispatch.rs` walks instead, asking `__eq__`.
        Value::List(l) => {
            let items = l.items.borrow();
            let mut examined = 0usize;
            for v in items.iter() {
                examined = examined.wrapping_add(1);
                if v.eq_value(x, depth)? {
                    m.charge_each(examined, c.factor_traverse);
                    return Ok(true);
                }
            }
            m.charge_each(examined, c.factor_traverse);
            Ok(false)
        }
        Value::Tuple(t) => {
            let mut examined = 0usize;
            for v in t.iter() {
                examined = examined.wrapping_add(1);
                if v.eq_value(x, depth)? {
                    m.charge_each(examined, c.factor_traverse);
                    return Ok(true);
                }
            }
            m.charge_each(examined, c.factor_traverse);
            Ok(false)
        }
        Value::Dict(d) => Ok(dict_find(&d.items.borrow(), x, depth)?.is_some()),
        Value::Set(s) => Ok(set_find(&s.items.borrow(), x, depth)?.is_some()),
        _ => Err(Exception::type_error()),
    }
}

/// `+`, `*` and the set operators on collections.
pub fn binop_collections(m: &mut Machine, op: BinOp, l: Value, r: Value) -> R<Value> {
    let c = m.costs().clone();
    let depth = m.limits().depth_nesting as u32;
    match (op, &l, &r) {
        (BinOp::Add, Value::Str(a), Value::Str(b)) => {
            let out = format!("{a}{b}");
            m.charge_each(out.chars().count(), c.factor_char);
            m.check_size(out.chars().count())?;
            Ok(Value::str(&out))
        }
        (BinOp::Add, Value::List(a), Value::List(b)) => {
            let mut items = a.items.borrow().clone();
            items.extend(b.items.borrow().iter().cloned());
            m.charge_each(items.len(), c.factor_copy);
            m.check_size(items.len())?;
            Ok(m.new_list(items))
        }
        (BinOp::Add, Value::Tuple(a), Value::Tuple(b)) => {
            let mut items = a.to_vec();
            items.extend(b.iter().cloned());
            m.charge_each(items.len(), c.factor_copy);
            m.check_size(items.len())?;
            Ok(Value::tuple(items))
        }
        (
            BinOp::Mul,
            Value::Str(_) | Value::List(_) | Value::Tuple(_),
            Value::Num(_) | Value::Bool(_),
        )
        | (
            BinOp::Mul,
            Value::Num(_) | Value::Bool(_),
            Value::Str(_) | Value::List(_) | Value::Tuple(_),
        ) => {
            let (seq, n) = if matches!(l, Value::Num(_) | Value::Bool(_)) {
                (&r, &l)
            } else {
                (&l, &r)
            };
            let n = usize::try_from(integral_index(n)?.max(0)).unwrap_or(0);
            match seq {
                Value::Str(s) => {
                    let out = s.repeat(n);
                    m.charge_each(out.chars().count(), c.factor_char);
                    m.check_size(out.chars().count())?;
                    Ok(Value::str(&out))
                }
                Value::List(a) => {
                    let base = a.items.borrow().clone();
                    let total = base.len().saturating_mul(n);
                    m.check_size(total)?;
                    m.charge_each(total, c.factor_copy);
                    let items: Vec<Value> = (0..n).flat_map(|_| base.iter().cloned()).collect();
                    Ok(m.new_list(items))
                }
                Value::Tuple(t) => {
                    let total = t.len().saturating_mul(n);
                    m.check_size(total)?;
                    m.charge_each(total, c.factor_copy);
                    Ok(Value::tuple(
                        (0..n).flat_map(|_| t.iter().cloned()).collect(),
                    ))
                }
                _ => Err(Exception::type_error()),
            }
        }
        (
            BinOp::BitOr | BinOp::BitAnd | BinOp::Sub | BinOp::BitXor,
            Value::Set(a),
            Value::Set(b),
        ) => {
            let (x, y) = (a.items.borrow().clone(), b.items.borrow().clone());
            m.charge_each(x.len().saturating_add(y.len()), c.factor_traverse);
            let out: Vec<Value> = match op {
                BinOp::BitOr => {
                    let mut out = x.clone();
                    for v in y {
                        if set_find(&out, &v, depth)?.is_none() {
                            out.push(v);
                        }
                    }
                    out
                }
                BinOp::BitAnd => {
                    let mut out = Vec::new();
                    for v in x {
                        if set_find(&y, &v, depth)?.is_some() {
                            out.push(v);
                        }
                    }
                    out
                }
                BinOp::Sub => {
                    let mut out = Vec::new();
                    for v in x {
                        if set_find(&y, &v, depth)?.is_none() {
                            out.push(v);
                        }
                    }
                    out
                }
                _ => {
                    let mut out = Vec::new();
                    for v in &x {
                        if set_find(&y, v, depth)?.is_none() {
                            out.push(v.clone());
                        }
                    }
                    for v in y {
                        if set_find(&x, &v, depth)?.is_none() {
                            out.push(v);
                        }
                    }
                    out
                }
            };
            m.check_size(out.len())?;
            Ok(m.new_set(out))
        }
        _ => Err(Exception::type_error()),
    }
}

/// An f-string field: `str` of the value, or the numeric and alignment specs
/// `numbers.md` and `syntax.md` allow.
pub fn format_value(v: &Value, spec: Option<&str>) -> R<String> {
    let Some(spec) = spec else {
        return Ok(v.to_str_value());
    };
    // Alignment prefix: `>W`, `<W`, `^W`, then an optional numeric spec.
    let (align, rest) = match spec.chars().next() {
        Some(a @ ('>' | '<' | '^')) => {
            let digits: String = spec[1..].chars().take_while(char::is_ascii_digit).collect();
            let width: usize = digits.parse().map_err(|_| Exception::value_error())?;
            (Some((a, width)), &spec[1usize.wrapping_add(digits.len())..])
        }
        _ => (None, spec),
    };
    let body = if rest.is_empty() {
        v.to_str_value()
    } else {
        let n = v.expect_num()?;
        if let Some(places) = rest.strip_prefix('.').and_then(|r| r.strip_suffix('f')) {
            let p: u32 = places.parse().map_err(|_| Exception::value_error())?;
            if p > PLACES {
                return Err(Exception::value_error());
            }
            n.to_fixed(p)
        } else if rest == "d" {
            if !n.is_integral() {
                return Err(Exception::type_error());
            }
            n.to_decimal()
        } else if rest == "," {
            let s = n.to_decimal();
            let (neg, digits) = match s.strip_prefix('-') {
                Some(r) => (true, r.to_string()),
                None => (false, s),
            };
            let (int_part, frac) = match digits.find('.') {
                Some(i) => (&digits[..i], &digits[i..]),
                None => (&digits[..], ""),
            };
            let mut grouped = String::new();
            let chars: Vec<char> = int_part.chars().collect();
            for (i, ch) in chars.iter().enumerate() {
                if i > 0 && (chars.len().wrapping_sub(i)).wrapping_rem(3) == 0 {
                    grouped.push(',');
                }
                grouped.push(*ch);
            }
            format!("{}{grouped}{frac}", if neg { "-" } else { "" })
        } else if let Some(w) = rest.strip_prefix('0').and_then(|r| r.strip_suffix('d')) {
            if !n.is_integral() {
                return Err(Exception::type_error());
            }
            let width: usize = w.parse().map_err(|_| Exception::value_error())?;
            let s = n.to_decimal();
            let (sign, body) = match s.strip_prefix('-') {
                Some(r) => ("-", r.to_string()),
                None => ("", s),
            };
            let pad = width.saturating_sub(body.len()).saturating_sub(sign.len());
            format!("{sign}{}{body}", "0".repeat(pad))
        } else {
            return Err(Exception::value_error());
        }
    };
    Ok(match align {
        None => body,
        Some((a, width)) => {
            let n = body.chars().count();
            let pad = width.saturating_sub(n);
            match a {
                '>' => format!("{}{body}", " ".repeat(pad)),
                '<' => format!("{body}{}", " ".repeat(pad)),
                _ => {
                    let left = pad.wrapping_div(2);
                    format!(
                        "{}{body}{}",
                        " ".repeat(left),
                        " ".repeat(pad.wrapping_sub(left))
                    )
                }
            }
        }
    })
}

/// A record value, for hosts.
pub fn record(type_name: &str, fields: Vec<(&str, Value)>) -> Value {
    Value::Record(Rc::new(Record {
        type_name: type_name.to_string(),
        fields: fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    }))
}

/// Quoted form, re-exported for the VM's diagnostics.
pub fn quote(v: &Value) -> String {
    quoted(v)
}

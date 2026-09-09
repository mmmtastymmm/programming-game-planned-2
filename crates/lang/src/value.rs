//! What a program computes with: the seven named types, functions, exception
//! values and records. Every rule about equality, ordering, truth and
//! printing in `docs/01-language/syntax.md` lives here.
//!
//! Lists, dicts and sets are **objects**: shared by reference, keyed by
//! identity. Strings and tuples are **values**. Identity is an allocation
//! counter the VM assigns, so it is deterministic and never observable as a
//! number.

use crate::errors::{ExcClass, Exception};
use crate::num::Num;
use std::cell::RefCell;
use std::rc::Rc;

pub type R<T> = Result<T, Exception>;

#[derive(Clone, Debug)]
pub enum Value {
    None,
    Bool(bool),
    Num(Num),
    Str(Rc<str>),
    List(Rc<ListObj>),
    Tuple(Rc<[Value]>),
    Dict(Rc<DictObj>),
    Set(Rc<SetObj>),
    Func(Rc<Func>),
    /// A builtin of the language (`len`, `sorted`, …) or of the game, by name.
    Builtin(Rc<str>),
    /// A method of a built-in type bound to its receiver: `xs.append`.
    Method(Rc<Method>),
    /// An exception class, callable to make an exception value.
    ExcClass(ExcClass),
    Exc(Rc<Exception>),
    /// A read-only attribute record the game returns (`me()`, a sighting, a
    /// tile). Fields are read as attributes; nothing else is possible.
    Record(Rc<Record>),
    /// A `for` loop's snapshot, on the VM stack only; never visible to a
    /// program.
    Iter(Rc<IterObj>),
}

#[derive(Debug)]
pub struct IterObj {
    pub items: Vec<Value>,
    pub pos: std::cell::Cell<usize>,
}

#[derive(Debug)]
pub struct ListObj {
    pub id: u64,
    pub items: RefCell<Vec<Value>>,
}

#[derive(Debug)]
pub struct DictObj {
    pub id: u64,
    /// Insertion-ordered pairs; keys compared by [`Value::key_eq`].
    pub items: RefCell<Vec<(Value, Value)>>,
}

#[derive(Debug)]
pub struct SetObj {
    pub id: u64,
    pub items: RefCell<Vec<Value>>,
}

#[derive(Debug)]
pub struct Func {
    pub code: Rc<crate::compile::Code>,
    pub defaults: Vec<Value>,
}

#[derive(Debug)]
pub struct Method {
    pub receiver: Value,
    pub name: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub type_name: String,
    pub fields: Vec<(String, Value)>,
}

impl PartialEq for Value {
    /// Structural equality for testing and hashing purposes; the language's
    /// `==` is [`Value::eq_value`], which can fail.
    fn eq(&self, other: &Value) -> bool {
        self.eq_value(other, 64).unwrap_or(false)
    }
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(Rc::from(s))
    }

    pub fn num(n: Num) -> Value {
        Value::Num(n)
    }

    pub fn int(i: i128) -> Value {
        Value::Num(Num::from_int(i).unwrap_or(Num::ZERO))
    }

    pub fn tuple(items: Vec<Value>) -> Value {
        Value::Tuple(Rc::from(items))
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::None => "NoneType",
            Value::Bool(_) => "bool",
            Value::Num(_) => "num",
            Value::Str(_) => "str",
            Value::List(_) => "list",
            Value::Tuple(_) => "tuple",
            Value::Dict(_) => "dict",
            Value::Set(_) => "set",
            Value::Func(_) | Value::Builtin(_) | Value::Method(_) => "function",
            Value::ExcClass(_) => "type",
            Value::Exc(_) => "exception",
            Value::Record(_) => "record",
            Value::Iter(_) => "iterator",
        }
    }

    /// `bool` is a subclass of `num`: a number where one is wanted.
    pub fn as_num(&self) -> Option<Num> {
        match self {
            Value::Num(n) => Some(*n),
            Value::Bool(b) => Some(Num::from_bool(*b)),
            _ => None,
        }
    }

    pub fn expect_num(&self) -> R<Num> {
        self.as_num().ok_or_else(Exception::type_error)
    }

    /// An integral context (`numbers.md`): an integral `num`, as an `i128`.
    pub fn expect_integral(&self) -> R<i128> {
        self.expect_num()?
            .as_integral()
            .ok_or_else(Exception::type_error)
    }

    pub fn expect_str(&self) -> R<Rc<str>> {
        match self {
            Value::Str(s) => Ok(s.clone()),
            _ => Err(Exception::type_error()),
        }
    }

    /// Truth: zero, empty and `None` are false.
    pub fn truthy(&self) -> bool {
        match self {
            Value::None => false,
            Value::Bool(b) => *b,
            Value::Num(n) => !n.is_zero(),
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.items.borrow().is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Dict(d) => !d.items.borrow().is_empty(),
            Value::Set(s) => !s.items.borrow().is_empty(),
            _ => true,
        }
    }

    /// The identity of an object, if this is one.
    pub fn object_id(&self) -> Option<u64> {
        match self {
            Value::List(l) => Some(l.id),
            Value::Dict(d) => Some(d.id),
            Value::Set(s) => Some(s.id),
            _ => None,
        }
    }

    /// `==`. Numbers by value, strings and tuples elementwise, objects by
    /// identity unless both are the same kind of container, in which case
    /// elementwise; different types are unequal. `depth` is the nesting
    /// depth still allowed (`execution.md`, Limits): a walk that would
    /// descend past it raises `LimitError`.
    pub fn eq_value(&self, other: &Value, depth: u32) -> R<bool> {
        if depth == 0 {
            return Err(Exception::new(ExcClass::LimitError, vec![]));
        }
        Ok(match (self, other) {
            (Value::None, Value::None) => true,
            (Value::Num(_) | Value::Bool(_), Value::Num(_) | Value::Bool(_)) => {
                self.as_num() == other.as_num()
            }
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => seq_eq(a, b, depth)?,
            (Value::List(a), Value::List(b)) => {
                let (x, y) = (a.items.borrow(), b.items.borrow());
                seq_eq(&x, &y, depth)?
            }
            (Value::Dict(a), Value::Dict(b)) => {
                if Rc::ptr_eq(a, b) {
                    return Ok(true);
                }
                let (x, y) = (a.items.borrow(), b.items.borrow());
                if x.len() != y.len() {
                    return Ok(false);
                }
                for (k, v) in x.iter() {
                    match dict_find(&y, k, depth.wrapping_sub(1))? {
                        Some(i) => {
                            if !v.eq_value(&y[i].1, depth.wrapping_sub(1))? {
                                return Ok(false);
                            }
                        }
                        None => return Ok(false),
                    }
                }
                true
            }
            (Value::Set(a), Value::Set(b)) => {
                let (x, y) = (a.items.borrow(), b.items.borrow());
                if x.len() != y.len() {
                    return Ok(false);
                }
                for v in x.iter() {
                    if set_find(&y, v, depth.wrapping_sub(1))?.is_none() {
                        return Ok(false);
                    }
                }
                true
            }
            (Value::Func(a), Value::Func(b)) => Rc::ptr_eq(a, b),
            (Value::Builtin(a), Value::Builtin(b)) => a == b,
            (Value::Method(a), Value::Method(b)) => Rc::ptr_eq(a, b),
            (Value::ExcClass(a), Value::ExcClass(b)) => a == b,
            (Value::Exc(a), Value::Exc(b)) => Rc::ptr_eq(a, b),
            (Value::Record(a), Value::Record(b)) => a == b,
            _ => false,
        })
    }

    /// Equality as a dict or set key: `1` and `True` are one key; objects by
    /// identity; strings and tuples by value.
    pub fn key_eq(&self, other: &Value, depth: u32) -> R<bool> {
        match (self.object_id(), other.object_id()) {
            (Some(a), Some(b)) => Ok(a == b),
            (Some(_), None) | (None, Some(_)) => Ok(false),
            (None, None) => self.eq_value(other, depth),
        }
    }

    /// `<`: numbers by value, strings by scalar values, tuples and lists
    /// lexicographically; anything else is a `TypeError`.
    pub fn lt_value(&self, other: &Value, depth: u32) -> R<bool> {
        if depth == 0 {
            return Err(Exception::new(ExcClass::LimitError, vec![]));
        }
        match (self, other) {
            (Value::Num(_) | Value::Bool(_), Value::Num(_) | Value::Bool(_)) => {
                Ok(self.as_num() < other.as_num())
            }
            (Value::Str(a), Value::Str(b)) => Ok(a.chars().lt(b.chars())),
            (Value::Tuple(a), Value::Tuple(b)) => seq_lt(a, b, depth),
            (Value::List(a), Value::List(b)) => {
                let (x, y) = (a.items.borrow(), b.items.borrow());
                seq_lt(&x, &y, depth)
            }
            _ => Err(Exception::type_error()),
        }
    }

    /// `str(x)`, per the "str of everything" table.
    pub fn to_str_value(&self) -> String {
        match self {
            Value::None => "None".to_string(),
            Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            Value::Num(n) => n.to_decimal(),
            Value::Str(s) => s.to_string(),
            Value::List(l) => format!("[{}]", join_quoted(&l.items.borrow())),
            Value::Tuple(t) => {
                if t.len() == 1 {
                    format!("({},)", quoted(&t[0]))
                } else {
                    format!("({})", join_quoted(t))
                }
            }
            Value::Dict(d) => {
                let items: Vec<String> = d
                    .items
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", quoted(k), quoted(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Set(s) => {
                let items = s.items.borrow();
                if items.is_empty() {
                    "set()".to_string()
                } else {
                    format!("{{{}}}", join_quoted(&items))
                }
            }
            Value::Func(f) => format!("<function {}>", f.code.name),
            Value::Builtin(name) => format!("<function {name}>"),
            Value::Method(m) => format!("<function {}>", m.name),
            Value::ExcClass(c) => format!("<class {}>", c.name()),
            Value::Exc(e) => e.display(),
            Value::Record(r) => format!("<{}>", r.type_name),
            Value::Iter(_) => "<iterator>".to_string(),
        }
    }
}

fn seq_eq(a: &[Value], b: &[Value], depth: u32) -> R<bool> {
    if a.len() != b.len() {
        return Ok(false);
    }
    for (x, y) in a.iter().zip(b.iter()) {
        if !x.eq_value(y, depth.wrapping_sub(1))? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn seq_lt(a: &[Value], b: &[Value], depth: u32) -> R<bool> {
    for (x, y) in a.iter().zip(b.iter()) {
        if x.lt_value(y, depth.wrapping_sub(1))? {
            return Ok(true);
        }
        if y.lt_value(x, depth.wrapping_sub(1))? {
            return Ok(false);
        }
    }
    Ok(a.len() < b.len())
}

/// The index of `key` in a dict's pairs, by key equality.
pub fn dict_find(items: &[(Value, Value)], key: &Value, depth: u32) -> R<Option<usize>> {
    for (i, (k, _)) in items.iter().enumerate() {
        if k.key_eq(key, depth)? {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// The index of `v` in a set's elements, by key equality.
pub fn set_find(items: &[Value], v: &Value, depth: u32) -> R<Option<usize>> {
    for (i, k) in items.iter().enumerate() {
        if k.key_eq(v, depth)? {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// A value as it appears inside a container: strings quoted and escaped.
pub fn quoted(v: &Value) -> String {
    match v {
        Value::Str(s) => {
            let mut out = String::from("'");
            for c in s.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '\'' => out.push_str("\\'"),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\r' => out.push_str("\\r"),
                    c if (' '..='~').contains(&c) => out.push(c),
                    c if (c as u32) < 0x100 => out.push_str(&format!("\\x{:02x}", c as u32)),
                    c if (c as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push_str(&format!("\\U{:08x}", c as u32)),
                }
            }
            out.push('\'');
            out
        }
        other => other.to_str_value(),
    }
}

fn join_quoted(items: &[Value]) -> String {
    items.iter().map(quoted).collect::<Vec<_>>().join(", ")
}

/// A bounded check that a key is a legal dict/set key: a number, a string,
/// a bool, `None`, a tuple of keys, or an object (by identity).
pub fn check_key(v: &Value) -> R<()> {
    match v {
        Value::None | Value::Bool(_) | Value::Num(_) | Value::Str(_) => Ok(()),
        Value::Tuple(t) => t.iter().try_for_each(check_key),
        // Instances are keys by identity (T11); everything else — a list,
        // dict, set, function, record — is a `TypeError`.
        _ => Err(Exception::type_error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_of_everything() {
        let s = Value::str("a'b\n");
        assert_eq!(s.to_str_value(), "a'b\n");
        let t = Value::tuple(vec![Value::int(1), s.clone()]);
        assert_eq!(t.to_str_value(), "(1, 'a\\'b\\n')");
        assert_eq!(Value::tuple(vec![Value::int(1)]).to_str_value(), "(1,)");
        assert_eq!(Value::Bool(true).to_str_value(), "True");
        assert_eq!(Value::None.to_str_value(), "None");
        assert_eq!(quoted(&Value::str("é")), "'\\xe9'");
    }

    #[test]
    fn equality_and_order() {
        assert!(Value::int(1).eq_value(&Value::Bool(true), 8).unwrap());
        assert!(Value::int(1).key_eq(&Value::Bool(true), 8).unwrap());
        assert!(!Value::int(1).eq_value(&Value::str("1"), 8).unwrap());
        assert!(Value::str("a").lt_value(&Value::str("b"), 8).unwrap());
        assert!(Value::int(1).lt_value(&Value::str("a"), 8).is_err());
        let a = Value::tuple(vec![Value::int(1), Value::int(2)]);
        let b = Value::tuple(vec![Value::int(1), Value::int(3)]);
        assert!(a.lt_value(&b, 8).unwrap());
    }
}

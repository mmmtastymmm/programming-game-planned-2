//! Exceptions: the closed built-in set (`docs/01-language/syntax.md`,
//! Exceptions) and the record a fault leaves. An interpreter-raised exception
//! carries no prose — its `args` is the offending value where there is one,
//! and empty otherwise — so two peers can never disagree on a message.

use crate::value::{Instance, Value, exception_form};
use std::fmt;
use std::rc::Rc;

/// The built-in exception classes, a closed set with single inheritance.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum ExcClass {
    Exception,
    ValueError,
    TypeError,
    IndexError,
    KeyError,
    AttributeError,
    NameError,
    UnboundLocalError,
    ZeroDivisionError,
    OverflowError,
    RecursionError,
    LimitError,
}

impl ExcClass {
    pub const ALL: [ExcClass; 12] = [
        ExcClass::Exception,
        ExcClass::ValueError,
        ExcClass::TypeError,
        ExcClass::IndexError,
        ExcClass::KeyError,
        ExcClass::AttributeError,
        ExcClass::NameError,
        ExcClass::UnboundLocalError,
        ExcClass::ZeroDivisionError,
        ExcClass::OverflowError,
        ExcClass::RecursionError,
        ExcClass::LimitError,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ExcClass::Exception => "Exception",
            ExcClass::ValueError => "ValueError",
            ExcClass::TypeError => "TypeError",
            ExcClass::IndexError => "IndexError",
            ExcClass::KeyError => "KeyError",
            ExcClass::AttributeError => "AttributeError",
            ExcClass::NameError => "NameError",
            ExcClass::UnboundLocalError => "UnboundLocalError",
            ExcClass::ZeroDivisionError => "ZeroDivisionError",
            ExcClass::OverflowError => "OverflowError",
            ExcClass::RecursionError => "RecursionError",
            ExcClass::LimitError => "LimitError",
        }
    }

    pub fn from_name(name: &str) -> Option<ExcClass> {
        ExcClass::ALL.iter().copied().find(|c| c.name() == name)
    }

    /// The parent class, for `except` matching: everything is under
    /// `Exception`, and `UnboundLocalError` is under `NameError`.
    pub fn parent(self) -> Option<ExcClass> {
        match self {
            ExcClass::Exception => None,
            ExcClass::UnboundLocalError => Some(ExcClass::NameError),
            _ => Some(ExcClass::Exception),
        }
    }

    /// `self` is `target` or derives from it.
    pub fn is_a(self, target: ExcClass) -> bool {
        let mut c = Some(self);
        while let Some(k) = c {
            if k == target {
                return true;
            }
            c = k.parent();
        }
        false
    }
}

/// An exception value: its class, its `args`, and where it was raised. The
/// location is filled in by the VM at the raising operation. A raised
/// instance of a user class deriving from a built-in exception travels as
/// `user`, with `class` the built-in ancestor `except` clauses match by.
#[derive(Clone, Debug)]
pub struct Exception {
    pub class: ExcClass,
    pub args: Vec<Value>,
    pub file: Option<String>,
    pub line: u32,
    pub tick: u64,
    pub user: Option<Rc<Instance>>,
}

impl PartialEq for Exception {
    fn eq(&self, other: &Exception) -> bool {
        self.class == other.class
            && self.args == other.args
            && self.file == other.file
            && self.line == other.line
            && self.tick == other.tick
            && match (&self.user, &other.user) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

impl Exception {
    pub fn new(class: ExcClass, args: Vec<Value>) -> Exception {
        Exception {
            class,
            args,
            file: None,
            line: 0,
            tick: 0,
            user: None,
        }
    }

    /// The exception a raised instance of a user class becomes; `None` if
    /// its class derives from no built-in exception.
    pub fn from_instance(inst: Rc<Instance>) -> Option<Exception> {
        let class = inst.class.exc_base?;
        Some(Exception {
            class,
            args: inst.exc_args(),
            file: None,
            line: 0,
            tick: 0,
            user: Some(inst),
        })
    }

    /// The class name a program sees: the user class's, if there is one.
    pub fn class_name(&self) -> String {
        match &self.user {
            Some(i) => i.class.name.clone(),
            None => self.class.name().to_string(),
        }
    }

    /// The value `except … as e` binds: the instance itself for a user
    /// class, so its attributes are reachable.
    pub fn as_value(&self) -> Value {
        match &self.user {
            Some(i) => Value::Inst(i.clone()),
            None => Value::Exc(Rc::new(self.clone())),
        }
    }

    pub fn type_error() -> Exception {
        Exception::new(ExcClass::TypeError, vec![])
    }

    pub fn value_error() -> Exception {
        Exception::new(ExcClass::ValueError, vec![])
    }

    pub fn name_error(name: &str) -> Exception {
        Exception::new(ExcClass::NameError, vec![Value::str(name)])
    }

    pub fn attribute_error(name: &str) -> Exception {
        Exception::new(ExcClass::AttributeError, vec![Value::str(name)])
    }

    pub fn located(mut self, file: &str, line: u32, tick: u64) -> Exception {
        if self.file.is_none() {
            self.file = Some(file.to_string());
            self.line = line;
            self.tick = tick;
            // A user instance carries its location as attributes, so
            // `e.line` reads the same way on every exception.
            if let Some(i) = &self.user {
                i.set("file", Value::str(file));
                i.set("line", Value::int(i128::from(line)));
                i.set("tick", Value::int(i128::from(tick)));
            }
        }
        self
    }

    /// `str(e)`: the class name, then `: ` and the arguments joined by `, `.
    pub fn display(&self) -> String {
        exception_form(&self.class_name(), &self.args)
    }
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}

/// A load-time refusal: a parse error, a scope error, a bundle that does not
/// meet the rules. Never reaches a running program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadError {
    pub file: String,
    pub line: u32,
    pub message: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

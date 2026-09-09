//! The metered evaluator (`docs/01-language/execution.md`). A [`Machine`]
//! holds one program's whole execution state as data — frames, stacks,
//! globals, the deficit — so a tick can stop it at any boundary and the next
//! tick can continue it. Every instruction is charged from the cost table
//! before it runs; the budget is checked at every boundary.
//!
//! This is T10's slice of the interrupt model: main flow, metering, the
//! restart at the end of `main.py`, and a `fault` that writes its record and
//! restarts. Hooks, `dying`, `death` and `redeploy` are raised by the sim
//! (T7) and land there.

use crate::ast::{BinOp, CmpOp, UnaryOp};
use crate::builtins;
use crate::compile::{Code, Instr, compile_module};
use crate::data::{Costs, Limits};
use crate::errors::{ExcClass, Exception, LoadError};
use crate::num::Num;
use crate::parser::parse_file;
use crate::value::{
    DictObj, Func, IterObj, ListObj, Method, R, SetObj, Value, check_key, dict_find, set_find,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

/// A loaded bundle: today `main.py` alone (T13 brings `import`).
#[derive(Debug)]
pub struct Program {
    pub main: Rc<Code>,
    /// The bundle's version: FNV-1a over each file's length-prefixed name and
    /// bytes in sorted name order (`syntax.md`, The bundle).
    pub version: u64,
}

impl Program {
    /// Load a bundle from `(name, source)` pairs. Refuses what the doc
    /// refuses: a missing `main.py`, a bad file name, too many or too large
    /// files, and anything the parser or the compiler rejects.
    pub fn load(files: &[(&str, &str)], limits: &Limits) -> Result<Program, LoadError> {
        let bad = |file: &str, message: String| LoadError {
            file: file.to_string(),
            line: 0,
            message,
        };
        if files.len() as u64 > limits.bundle_files {
            return Err(bad("<bundle>", "too many files in the bundle".into()));
        }
        let mut sorted: Vec<(&str, &str)> = files.to_vec();
        sorted.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        let mut version = Fnv(0xcbf29ce484222325);
        let mut main = None;
        for (name, src) in &sorted {
            if !valid_file_name(name) {
                return Err(bad(name, "a file name is `[a-z_][a-z0-9_]*.py`".into()));
            }
            if src.len() as u64 > limits.bundle_file_bytes {
                return Err(bad(name, "the file is over the size limit".into()));
            }
            version.write_len_prefixed(name.as_bytes());
            version.write_len_prefixed(src.as_bytes());
            if *name == "main.py" {
                main = Some(*src);
            } else {
                return Err(bad(name, "`import` is not in this slice of the language yet (T13); a bundle is `main.py` alone".into()));
            }
        }
        let Some(src) = main else {
            return Err(bad("<bundle>", "a bundle must contain `main.py`".into()));
        };
        let stmts = parse_file("main.py", src)?;
        let main = compile_module("main.py", &stmts)?;
        Ok(Program {
            main,
            version: version.0,
        })
    }
}

fn valid_file_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".py") else {
        return false;
    };
    let mut chars = stem.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

struct Fnv(u64);

impl Fnv {
    fn write_len_prefixed(&mut self, bytes: &[u8]) {
        for b in (bytes.len() as u64)
            .to_le_bytes()
            .iter()
            .chain(bytes.iter())
        {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}

/// What the game supplies: its builtins, by name.
pub trait Host {
    fn call(&mut self, name: &str, args: Vec<Value>, kwargs: Vec<(String, Value)>) -> R<HostCall>;
}

pub enum HostCall {
    /// The builtin returned a value.
    Value(Value),
    /// The builtin began a waiting action; the program yields until the host
    /// resumes it, and the call then returns what the host passes.
    Wait,
    /// No game builtin of that name.
    Unknown,
}

/// A host with no builtins, for tests.
pub struct NoHost;

impl Host for NoHost {
    fn call(&mut self, _: &str, _: Vec<Value>, _: Vec<(String, Value)>) -> R<HostCall> {
        Ok(HostCall::Unknown)
    }
}

/// How a slice ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Slice {
    /// The tick budget is spent; resume next tick.
    Yield,
    /// A game builtin began a waiting action; call `resume` when it is done.
    Wait,
    /// Main flow reached its end and started again from the top.
    Restarted,
    /// An exception escaped main flow: the record is written and main flow
    /// restarts. The sim charges the damage and runs `on_fault` (T7).
    Fault(Exception),
}

#[derive(Clone, Copy)]
enum BlockKind {
    Except(usize),
    Finally(usize),
}

struct Block {
    kind: BlockKind,
    stack_len: usize,
    handling_len: usize,
}

struct Frame {
    code: Rc<Code>,
    pc: usize,
    stack: Vec<Value>,
    locals: Vec<Option<Value>>,
    blocks: Vec<Block>,
    /// Exceptions being handled, innermost last, for a bare `raise`.
    handling: Vec<Rc<Exception>>,
    /// An exception a `finally` will re-raise when it ends.
    pending: Option<Exception>,
}

impl Frame {
    fn new(code: Rc<Code>) -> Frame {
        let n = code.local_names.len();
        Frame {
            code,
            pc: 0,
            stack: Vec::new(),
            locals: vec![None; n],
            blocks: Vec::new(),
            handling: Vec::new(),
            pending: None,
        }
    }
}

/// One machine's interpreter: the whole execution state of one program.
pub struct Machine {
    program: Rc<Code>,
    costs: Rc<Costs>,
    limits: Rc<Limits>,
    frames: Vec<Frame>,
    globals: BTreeMap<String, Value>,
    /// Cost units spent this tick.
    spent: u64,
    /// Cost units the next tick starts in the red by.
    deficit: u64,
    /// Whether main flow has already restarted this tick.
    restarted_this_tick: bool,
    next_object_id: u64,
    /// Set while a host call is waiting; `resume` pushes its result.
    waiting: bool,
    /// The last fault, as `execution.md` defines the record.
    pub fault_record: Option<Exception>,
    pub tick: u64,
}

impl Machine {
    pub fn new(program: &Program, costs: Rc<Costs>, limits: Rc<Limits>) -> Machine {
        let mut m = Machine {
            program: program.main.clone(),
            costs,
            limits,
            frames: Vec::new(),
            globals: BTreeMap::new(),
            spent: 0,
            deficit: 0,
            restarted_this_tick: false,
            next_object_id: 1,
            waiting: false,
            fault_record: None,
            tick: 0,
        };
        m.start_main();
        m
    }

    /// Every global cleared, `main.py` at its first statement.
    fn start_main(&mut self) {
        self.globals.clear();
        self.frames.clear();
        self.frames.push(Frame::new(self.program.clone()));
        self.waiting = false;
    }

    /// Replace the program (a `redeploy`'s swap) and start it from the top.
    pub fn swap_program(&mut self, program: &Program) {
        self.program = program.main.clone();
        self.start_main();
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_object_id;
        self.next_object_id = self.next_object_id.wrapping_add(1);
        id
    }

    pub fn new_list(&mut self, items: Vec<Value>) -> Value {
        Value::List(Rc::new(ListObj {
            id: self.alloc_id(),
            items: RefCell::new(items),
        }))
    }

    pub fn new_dict(&mut self, items: Vec<(Value, Value)>) -> Value {
        Value::Dict(Rc::new(DictObj {
            id: self.alloc_id(),
            items: RefCell::new(items),
        }))
    }

    pub fn new_set(&mut self, items: Vec<Value>) -> Value {
        Value::Set(Rc::new(SetObj {
            id: self.alloc_id(),
            items: RefCell::new(items),
        }))
    }

    pub fn costs(&self) -> &Costs {
        &self.costs
    }

    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Spend cost units.
    pub fn charge(&mut self, units: u64) {
        self.spent = self.spent.saturating_add(units);
    }

    /// Spend a per-element part: `n × factor`.
    pub fn charge_each(&mut self, n: usize, factor: u64) {
        self.charge((n as u64).saturating_mul(factor));
    }

    /// The collection-size limit, checked before a write.
    pub fn check_size(&self, n: usize) -> R<()> {
        if n as u64 > self.limits.size_collection {
            Err(Exception::new(ExcClass::LimitError, vec![]))
        } else {
            Ok(())
        }
    }

    /// Resume a waiting host call with its result.
    pub fn resume(&mut self, value: Value) {
        if self.waiting {
            self.waiting = false;
            if let Some(f) = self.frames.last_mut() {
                f.stack.push(value);
            }
        }
    }

    /// The value of a global, for tests and the snapshot.
    pub fn global(&self, name: &str) -> Option<&Value> {
        self.globals.get(name)
    }

    /// Run one tick's slice: until the budget is spent, a host call waits,
    /// main flow restarts, or a fault escapes.
    pub fn run_slice(&mut self, host: &mut dyn Host) -> Slice {
        self.spent = 0;
        self.restarted_this_tick = false;
        let budget = self.limits.budget_tick.saturating_sub(self.deficit);
        let result = self.run(host, budget);
        self.deficit = self.spent.saturating_sub(budget);
        result
    }

    fn run(&mut self, host: &mut dyn Host, budget: u64) -> Slice {
        loop {
            if self.waiting {
                return Slice::Wait;
            }
            if self.spent >= budget {
                return Slice::Yield;
            }
            match self.step(host) {
                Ok(Step::Continue) => {}
                Ok(Step::Wait) => {
                    self.waiting = true;
                    return Slice::Wait;
                }
                Ok(Step::MainEnded) => {
                    self.start_main();
                    if self.restarted_this_tick {
                        return Slice::Yield;
                    }
                    self.restarted_this_tick = true;
                    return Slice::Restarted;
                }
                Err(exc) => {
                    let exc = self.locate(exc);
                    match self.unwind(exc) {
                        None => {}
                        Some(escaped) => {
                            self.fault_record = Some(escaped.clone());
                            self.start_main();
                            return Slice::Fault(escaped);
                        }
                    }
                }
            }
        }
    }

    fn locate(&self, exc: Exception) -> Exception {
        match self.frames.last() {
            Some(f) => {
                let line = f
                    .code
                    .lines
                    .get(f.pc.saturating_sub(1))
                    .copied()
                    .unwrap_or(0);
                exc.located(&f.code.file, line, self.tick)
            }
            None => exc,
        }
    }

    /// Unwind to the nearest handler; `Some` if the exception escaped.
    fn unwind(&mut self, exc: Exception) -> Option<Exception> {
        loop {
            let Some(frame) = self.frames.last_mut() else {
                return Some(exc);
            };
            if let Some(block) = frame.blocks.pop() {
                frame.stack.truncate(block.stack_len);
                frame.handling.truncate(block.handling_len);
                match block.kind {
                    BlockKind::Except(handler) => {
                        frame.handling.push(Rc::new(exc.clone()));
                        frame.stack.push(Value::Exc(Rc::new(exc)));
                        frame.pc = handler;
                    }
                    BlockKind::Finally(target) => {
                        frame.pending = Some(exc);
                        frame.pc = target;
                    }
                }
                return None;
            }
            // No handler in this frame: leave it, charging the unwind.
            self.frames.pop();
            self.charge(self.costs.op_raise);
            if self.frames.is_empty() {
                return Some(exc);
            }
        }
    }

    fn frame(&mut self) -> &mut Frame {
        // Every step runs with at least the module frame present.
        self.frames.last_mut().expect("a frame")
    }

    fn pop(&mut self) -> R<Value> {
        self.frame()
            .stack
            .pop()
            .ok_or_else(|| Exception::new(ExcClass::Exception, vec![Value::str("stack")]))
    }

    fn push(&mut self, v: Value) {
        self.frame().stack.push(v);
    }

    fn step(&mut self, host: &mut dyn Host) -> R<Step> {
        let (instr, code) = {
            let f = self.frame();
            let code = f.code.clone();
            let Some(instr) = code.code.get(f.pc).cloned() else {
                return Err(Exception::new(ExcClass::Exception, vec![Value::str("pc")]));
            };
            f.pc = f.pc.wrapping_add(1);
            (instr, code)
        };
        let c = self.costs.clone();
        match instr {
            Instr::Stmt => self.charge(c.op_statement),
            Instr::Const(i) => {
                self.charge(c.op_literal);
                let v = code.consts.get(i).cloned().unwrap_or(Value::None);
                self.push(v);
            }
            Instr::LoadLocal(slot) => {
                self.charge(c.op_name);
                let v = self.frame().locals.get(slot).cloned().flatten();
                match v {
                    Some(v) => self.push(v),
                    None => {
                        let name = code.local_names.get(slot).cloned().unwrap_or_default();
                        return Err(Exception::new(
                            ExcClass::UnboundLocalError,
                            vec![Value::str(&name)],
                        ));
                    }
                }
            }
            Instr::StoreLocal(slot) => {
                let v = self.pop()?;
                if let Some(s) = self.frame().locals.get_mut(slot) {
                    *s = Some(v);
                }
            }
            Instr::LoadGlobal(i) => {
                self.charge(c.op_name);
                let name = code.names.get(i).cloned().unwrap_or_default();
                // A global, a language builtin, or a game builtin — the
                // last known by its row in the cost table (`costs.md`), so
                // a host that lacks one the table names is the host's bug,
                // not a `NameError` a program can catch.
                let v = match self.globals.get(&name) {
                    Some(v) => v.clone(),
                    None => match builtins::lookup(&name) {
                        Some(v) => v,
                        None if self.costs.game_cost(&name).is_some() => {
                            Value::Builtin(Rc::from(name.as_str()))
                        }
                        None => return Err(Exception::name_error(&name)),
                    },
                };
                self.push(v);
            }
            Instr::StoreGlobal(i) => {
                let v = self.pop()?;
                let name = code.names.get(i).cloned().unwrap_or_default();
                self.globals.insert(name, v);
            }
            Instr::Pop => {
                self.pop()?;
            }
            Instr::Dup => {
                let v = self.pop()?;
                self.push(v.clone());
                self.push(v);
            }
            Instr::Dup2 => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(a.clone());
                self.push(b.clone());
                self.push(a);
                self.push(b);
            }
            Instr::Swap => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(b);
                self.push(a);
            }
            Instr::Rot3 => {
                let c = self.pop()?;
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(c);
                self.push(a);
                self.push(b);
            }
            Instr::BinOp(op) => {
                let r = self.pop()?;
                let l = self.pop()?;
                let v = self.binop(op, l, r)?;
                self.push(v);
            }
            Instr::UnaryOp(op) => {
                self.charge(c.op_operator);
                let v = self.pop()?;
                let out = match op {
                    UnaryOp::Not => Value::Bool(!v.truthy()),
                    UnaryOp::Neg => Value::Num(v.expect_num()?.checked_neg()?),
                    UnaryOp::Pos => Value::Num(v.expect_num()?),
                };
                self.push(out);
            }
            Instr::Compare(op) => {
                let r = self.pop()?;
                let l = self.pop()?;
                let v = self.compare(op, &l, &r)?;
                self.push(Value::Bool(v));
            }
            Instr::Jump(t) => self.frame().pc = t,
            Instr::JumpIfFalsePop(t) => {
                if !self.pop()?.truthy() {
                    self.frame().pc = t;
                }
            }
            Instr::JumpIfTruePop(t) => {
                if self.pop()?.truthy() {
                    self.frame().pc = t;
                }
            }
            Instr::JumpIfFalseKeep(t) => {
                let keep = !self.frame().stack.last().is_some_and(Value::truthy);
                if keep {
                    self.frame().pc = t;
                }
            }
            Instr::JumpIfTrueKeep(t) => {
                let keep = self.frame().stack.last().is_some_and(Value::truthy);
                if keep {
                    self.frame().pc = t;
                }
            }
            Instr::BuildList(n) => {
                self.charge(c.op_display);
                self.charge_each(n, c.factor_copy);
                let items = self.pop_n(n)?;
                self.check_size(items.len())?;
                let v = self.new_list(items);
                self.push(v);
            }
            Instr::BuildTuple(n) => {
                if n == usize::MAX {
                    // Convert the list built for a starred display.
                    let v = self.pop()?;
                    let items = match v {
                        Value::List(l) => l.items.borrow().clone(),
                        _ => vec![],
                    };
                    self.push(Value::tuple(items));
                } else {
                    self.charge(c.op_display);
                    self.charge_each(n, c.factor_copy);
                    let items = self.pop_n(n)?;
                    self.check_size(items.len())?;
                    self.push(Value::tuple(items));
                }
            }
            Instr::BuildDict(n) => {
                self.charge(c.op_display);
                self.charge_each(n, c.factor_copy);
                let flat = self.pop_n(n.wrapping_mul(2))?;
                let mut items: Vec<(Value, Value)> = Vec::with_capacity(n);
                let mut it = flat.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    check_key(&k)?;
                    match dict_find(&items, &k, self.limits.depth_nesting as u32)? {
                        Some(i) => items[i].1 = v,
                        None => items.push((k, v)),
                    }
                }
                let v = self.new_dict(items);
                self.push(v);
            }
            Instr::BuildSet(n) => {
                self.charge(c.op_display);
                self.charge_each(n, c.factor_copy);
                let flat = self.pop_n(n)?;
                let mut items: Vec<Value> = Vec::with_capacity(n);
                for v in flat {
                    check_key(&v)?;
                    if set_find(&items, &v, self.limits.depth_nesting as u32)?.is_none() {
                        items.push(v);
                    }
                }
                let v = self.new_set(items);
                self.push(v);
            }
            Instr::CompAppend(slot) => {
                let v = self.pop()?;
                let target = if slot == usize::MAX {
                    self.frame().stack.last().cloned()
                } else {
                    self.frame().locals.get(slot).cloned().flatten()
                };
                if let Some(Value::List(l)) = target {
                    let mut items = l.items.borrow_mut();
                    self.check_size(items.len().wrapping_add(1))?;
                    items.push(v);
                }
            }
            Instr::CompAdd(slot) => {
                let v = self.pop()?;
                check_key(&v)?;
                if let Some(Value::Set(s)) = self.frame().locals.get(slot).cloned().flatten() {
                    let mut items = s.items.borrow_mut();
                    if set_find(&items, &v, self.limits.depth_nesting as u32)?.is_none() {
                        self.check_size(items.len().wrapping_add(1))?;
                        items.push(v);
                    }
                }
            }
            Instr::CompSet(slot) => {
                let v = self.pop()?;
                let k = self.pop()?;
                check_key(&k)?;
                if let Some(Value::Dict(d)) = self.frame().locals.get(slot).cloned().flatten() {
                    let mut items = d.items.borrow_mut();
                    match dict_find(&items, &k, self.limits.depth_nesting as u32)? {
                        Some(i) => items[i].1 = v,
                        None => {
                            self.check_size(items.len().wrapping_add(1))?;
                            items.push((k, v));
                        }
                    }
                }
            }
            Instr::ListExtend => {
                let iterable = self.pop()?;
                let items = self.snapshot(&iterable)?;
                if let Some(Value::List(l)) = self.frame().stack.last().cloned() {
                    let mut dst = l.items.borrow_mut();
                    self.check_size(dst.len().saturating_add(items.len()))?;
                    self.charge_each(items.len(), c.factor_copy);
                    dst.extend(items);
                }
            }
            Instr::Subscript => {
                self.charge(c.op_subscript);
                let idx = self.pop()?;
                let obj = self.pop()?;
                let v = builtins::subscript(self, &obj, &idx)?;
                self.push(v);
            }
            Instr::StoreSubscript => {
                self.charge(c.op_subscript);
                let idx = self.pop()?;
                let obj = self.pop()?;
                let v = self.pop()?;
                builtins::store_subscript(self, &obj, idx, v)?;
            }
            Instr::BuildSlice => {
                let step = self.pop()?;
                let hi = self.pop()?;
                let lo = self.pop()?;
                self.push(Value::Record(Rc::new(crate::value::Record {
                    type_name: "slice".into(),
                    fields: vec![("lo".into(), lo), ("hi".into(), hi), ("step".into(), step)],
                })));
            }
            Instr::LoadAttr(i) => {
                self.charge(c.op_attribute);
                let obj = self.pop()?;
                let name = code.names.get(i).cloned().unwrap_or_default();
                let v = builtins::attribute(&obj, &name)?;
                self.push(v);
            }
            Instr::StoreAttr(_) => {
                return Err(Exception::type_error());
            }
            Instr::Call {
                argc,
                kwnames,
                star,
                dstar,
            } => {
                let dstar_v = if dstar { Some(self.pop()?) } else { None };
                let kwnames_v: Vec<String> = if kwnames == usize::MAX {
                    vec![]
                } else {
                    match code.consts.get(kwnames) {
                        Some(Value::Tuple(t)) => t.iter().map(Value::to_str_value).collect(),
                        _ => vec![],
                    }
                };
                let kwvals = self.pop_n(kwnames_v.len())?;
                let star_v = if star { Some(self.pop()?) } else { None };
                let mut args = self.pop_n(argc)?;
                let func = self.pop()?;
                if let Some(s) = star_v {
                    let extra = self.snapshot(&s)?;
                    self.charge(c.op_operator);
                    self.charge_each(extra.len(), c.factor_copy);
                    args.extend(extra);
                }
                let mut kwargs: Vec<(String, Value)> = kwnames_v.into_iter().zip(kwvals).collect();
                if let Some(d) = dstar_v {
                    let Value::Dict(d) = d else {
                        return Err(Exception::type_error());
                    };
                    self.charge(c.op_operator);
                    let items = d.items.borrow();
                    self.charge_each(items.len(), c.factor_copy);
                    for (k, v) in items.iter() {
                        let Value::Str(k) = k else {
                            return Err(Exception::type_error());
                        };
                        if kwargs.iter().any(|(n, _)| **n == **k) {
                            return Err(Exception::type_error());
                        }
                        kwargs.push((k.to_string(), v.clone()));
                    }
                }
                return self.call(host, func, args, kwargs);
            }
            Instr::MakeFunction { child, ndefaults } => {
                self.charge(c.op_literal);
                let defaults = self.pop_n(ndefaults)?;
                let code = code
                    .children
                    .get(child)
                    .cloned()
                    .ok_or_else(Exception::type_error)?;
                self.push(Value::Func(Rc::new(Func { code, defaults })));
            }
            Instr::Return => {
                let v = self.pop()?;
                let done = self.frames.pop().is_some_and(|f| f.code.is_module);
                if done || self.frames.is_empty() {
                    return Ok(Step::MainEnded);
                }
                self.push(v);
            }
            Instr::GetIter => {
                let v = self.pop()?;
                let items = self.snapshot(&v)?;
                if matches!(v, Value::List(_) | Value::Dict(_) | Value::Set(_)) {
                    self.charge_each(items.len(), c.factor_copy);
                }
                self.push(Value::Iter(Rc::new(IterObj {
                    items,
                    pos: Cell::new(0),
                })));
            }
            Instr::ForIter(exit) => {
                let Some(Value::Iter(it)) = self.frame().stack.last().cloned() else {
                    return Err(Exception::type_error());
                };
                let pos = it.pos.get();
                match it.items.get(pos) {
                    Some(v) => {
                        self.charge(c.op_iteration);
                        it.pos.set(pos.wrapping_add(1));
                        let v = v.clone();
                        self.push(v);
                    }
                    None => {
                        self.pop()?;
                        self.frame().pc = exit;
                    }
                }
            }
            Instr::Unpack { n, star } => {
                self.charge(c.op_operator);
                let v = self.pop()?;
                let items = self.snapshot(&v)?;
                self.charge_each(items.len(), c.factor_copy);
                let pieces: Vec<Value> = match star {
                    None => {
                        if items.len() != n {
                            return Err(Exception::value_error());
                        }
                        items
                    }
                    Some(s) => {
                        let fixed = n.wrapping_sub(1);
                        if items.len() < fixed {
                            return Err(Exception::value_error());
                        }
                        let after = fixed.wrapping_sub(s);
                        let mid_end = items.len().wrapping_sub(after);
                        let mut out: Vec<Value> = items[..s].to_vec();
                        let middle = items[s..mid_end].to_vec();
                        out.push(self.new_list(middle));
                        out.extend_from_slice(&items[mid_end..]);
                        out
                    }
                };
                for v in pieces.into_iter().rev() {
                    self.push(v);
                }
            }
            Instr::SetupExcept(handler) => {
                let f = self.frame();
                let (stack_len, handling_len) = (f.stack.len(), f.handling.len());
                f.blocks.push(Block {
                    kind: BlockKind::Except(handler),
                    stack_len,
                    handling_len,
                });
            }
            Instr::SetupFinally(target) => {
                let f = self.frame();
                let (stack_len, handling_len) = (f.stack.len(), f.handling.len());
                f.blocks.push(Block {
                    kind: BlockKind::Finally(target),
                    stack_len,
                    handling_len,
                });
            }
            Instr::PopBlock => {
                self.frame().blocks.pop();
            }
            Instr::EndFinally => {
                if let Some(exc) = self.frame().pending.take() {
                    return Err(exc);
                }
            }
            Instr::PopExcept => {
                self.frame().handling.pop();
            }
            Instr::Raise(has_value) => {
                self.charge(c.op_raise);
                if has_value {
                    let v = self.pop()?;
                    let exc = match v {
                        Value::Exc(e) => (*e).clone(),
                        Value::ExcClass(class) => Exception::new(class, vec![]),
                        _ => return Err(Exception::type_error()),
                    };
                    return Err(exc);
                }
                match self.frame().handling.last() {
                    Some(e) => return Err((**e).clone()),
                    None => return Err(Exception::new(ExcClass::Exception, vec![])),
                }
            }
            Instr::ExcMatch => {
                self.charge(c.op_pattern);
                let types = self.pop()?;
                let exc = self.pop()?;
                let Value::Exc(e) = &exc else {
                    return Err(Exception::type_error());
                };
                let matched = exc_matches(e.class, &types)?;
                self.push(Value::Bool(matched));
            }
            Instr::FormatValue(spec) => {
                let v = self.pop()?;
                let spec = if spec == usize::MAX {
                    None
                } else {
                    code.consts.get(spec).map(Value::to_str_value)
                };
                let s = builtins::format_value(&v, spec.as_deref())?;
                self.charge_each(s.chars().count(), c.factor_char);
                self.push(Value::str(&s));
            }
            Instr::BuildString(n) => {
                self.charge(c.op_operator);
                let parts = self.pop_n(n)?;
                let mut out = String::new();
                for p in parts {
                    out.push_str(&p.to_str_value());
                }
                self.check_size(out.chars().count())?;
                self.push(Value::str(&out));
            }
        }
        Ok(Step::Continue)
    }

    fn pop_n(&mut self, n: usize) -> R<Vec<Value>> {
        let f = self.frame();
        let at = f
            .stack
            .len()
            .checked_sub(n)
            .ok_or_else(Exception::type_error)?;
        Ok(f.stack.split_off(at))
    }

    /// The items of an iterable, as a snapshot (`syntax.md`, `for`).
    pub fn snapshot(&mut self, v: &Value) -> R<Vec<Value>> {
        Ok(match v {
            Value::List(l) => l.items.borrow().clone(),
            Value::Tuple(t) => t.to_vec(),
            Value::Str(s) => s.chars().map(|c| Value::str(&c.to_string())).collect(),
            Value::Dict(d) => d.items.borrow().iter().map(|(k, _)| k.clone()).collect(),
            Value::Set(s) => s.items.borrow().clone(),
            Value::Iter(it) => it.items[it.pos.get()..].to_vec(),
            _ => return Err(Exception::type_error()),
        })
    }

    fn binop(&mut self, op: BinOp, l: Value, r: Value) -> R<Value> {
        let c = self.costs.clone();
        self.charge(c.op_operator);
        // Numbers first: the common case.
        if let (Some(a), Some(b)) = (l.as_num(), r.as_num()) {
            return Ok(Value::Num(match op {
                BinOp::Add => a.checked_add(b)?,
                BinOp::Sub => a.checked_sub(b)?,
                BinOp::Mul => a.checked_mul(b)?,
                BinOp::Div => a.checked_div(b)?,
                BinOp::FloorDiv => a.checked_floordiv(b)?,
                BinOp::Mod => a.checked_rem(b)?,
                BinOp::Pow => {
                    let (v, steps) = a.checked_pow(b)?;
                    self.charge_each(steps as usize, c.op_operator);
                    v
                }
                BinOp::BitOr | BinOp::BitAnd | BinOp::BitXor => return Err(Exception::type_error()),
            }));
        }
        builtins::binop_collections(self, op, l, r)
    }

    fn compare(&mut self, op: CmpOp, l: &Value, r: &Value) -> R<bool> {
        let c = self.costs.clone();
        self.charge(c.op_operator);
        let depth = self.limits.depth_nesting as u32;
        Ok(match op {
            CmpOp::Eq => {
                self.charge_compare(l, r);
                l.eq_value(r, depth)?
            }
            CmpOp::Ne => {
                self.charge_compare(l, r);
                !l.eq_value(r, depth)?
            }
            CmpOp::Lt => {
                self.charge_compare(l, r);
                l.lt_value(r, depth)?
            }
            CmpOp::Gt => {
                self.charge_compare(l, r);
                r.lt_value(l, depth)?
            }
            CmpOp::Le => {
                self.charge_compare(l, r);
                l.lt_value(r, depth)? || l.eq_value(r, depth)?
            }
            CmpOp::Ge => {
                self.charge_compare(l, r);
                r.lt_value(l, depth)? || l.eq_value(r, depth)?
            }
            CmpOp::In => builtins::contains(self, r, l)?,
            CmpOp::NotIn => !builtins::contains(self, r, l)?,
            CmpOp::Is => matches!(l, Value::None),
            CmpOp::IsNot => !matches!(l, Value::None),
        })
    }

    /// The per-element part of comparing two sequences.
    fn charge_compare(&mut self, l: &Value, r: &Value) {
        let n = match (l, r) {
            (Value::Str(a), Value::Str(b)) => a.chars().count().min(b.chars().count()),
            (Value::Tuple(a), Value::Tuple(b)) => a.len().min(b.len()),
            (Value::List(a), Value::List(b)) => a.items.borrow().len().min(b.items.borrow().len()),
            (Value::Dict(a), Value::Dict(b)) => a
                .items
                .borrow()
                .len()
                .saturating_add(b.items.borrow().len()),
            (Value::Set(a), Value::Set(b)) => a
                .items
                .borrow()
                .len()
                .saturating_add(b.items.borrow().len()),
            _ => 0,
        };
        let f = self.costs.factor_traverse;
        self.charge_each(n, f);
    }

    fn call(
        &mut self,
        host: &mut dyn Host,
        func: Value,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
    ) -> R<Step> {
        match func {
            Value::Func(f) => {
                let c = self.costs.clone();
                self.charge(c.op_call);
                self.charge_each(args.len().saturating_add(kwargs.len()), c.op_argument);
                if self.frames.len() as u64 >= self.limits.depth_call {
                    return Err(Exception::new(ExcClass::RecursionError, vec![]));
                }
                let frame = bind_arguments(&f, args, kwargs, self)?;
                self.frames.push(frame);
                Ok(Step::Continue)
            }
            Value::Builtin(name) => {
                if let Some(v) = builtins::call_builtin(self, &name, &args, &kwargs)? {
                    self.push(v);
                    return Ok(Step::Continue);
                }
                match self.costs.game_cost(&name) {
                    Some(cost) => self.charge(cost),
                    None => return Err(Exception::name_error(&name)),
                }
                match host.call(&name, args, kwargs)? {
                    HostCall::Value(v) => {
                        self.push(v);
                        Ok(Step::Continue)
                    }
                    HostCall::Wait => Ok(Step::Wait),
                    HostCall::Unknown => Err(Exception::name_error(&name)),
                }
            }
            Value::Method(m) => {
                let v = builtins::call_method(self, &m.receiver, m.name, &args, &kwargs)?;
                self.push(v);
                Ok(Step::Continue)
            }
            Value::ExcClass(class) => {
                let c = self.costs.clone();
                self.charge(c.op_construct);
                if !kwargs.is_empty() {
                    return Err(Exception::type_error());
                }
                self.push(Value::Exc(Rc::new(Exception::new(class, args))));
                Ok(Step::Continue)
            }
            _ => Err(Exception::type_error()),
        }
    }
}

enum Step {
    Continue,
    Wait,
    MainEnded,
}

fn exc_matches(class: ExcClass, types: &Value) -> R<bool> {
    match types {
        Value::ExcClass(t) => Ok(class.is_a(*t)),
        Value::Tuple(ts) => {
            for t in ts.iter() {
                if exc_matches(class, t)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Err(Exception::type_error()),
    }
}

/// Bind a call's arguments to a new frame's locals, as Python does.
fn bind_arguments(
    f: &Func,
    args: Vec<Value>,
    kwargs: Vec<(String, Value)>,
    m: &mut Machine,
) -> R<Frame> {
    let code = f.code.clone();
    let mut frame = Frame::new(code.clone());
    let n = code.nparams;
    let mut extra = Vec::new();
    for (i, a) in args.into_iter().enumerate() {
        if i < n {
            frame.locals[i] = Some(a);
        } else {
            extra.push(a);
        }
    }
    match code.star {
        Some(slot) => frame.locals[slot] = Some(Value::tuple(extra)),
        None => {
            if !extra.is_empty() {
                return Err(Exception::type_error());
            }
        }
    }
    let mut extra_kw: Vec<(Value, Value)> = Vec::new();
    for (k, v) in kwargs {
        match code.local_names[..n].iter().position(|p| *p == k) {
            Some(slot) => {
                if frame.locals[slot].is_some() {
                    return Err(Exception::type_error());
                }
                frame.locals[slot] = Some(v);
            }
            None => {
                if code.dstar.is_none() {
                    return Err(Exception::type_error());
                }
                extra_kw.push((Value::str(&k), v));
            }
        }
    }
    if let Some(slot) = code.dstar {
        frame.locals[slot] = Some(m.new_dict(extra_kw));
    }
    // Defaults fill from the right.
    let first_default = n.saturating_sub(f.defaults.len());
    for i in 0..n {
        if frame.locals[i].is_none() {
            match i.checked_sub(first_default).and_then(|d| f.defaults.get(d)) {
                Some(d) => frame.locals[i] = Some(d.clone()),
                None => return Err(Exception::type_error()),
            }
        }
    }
    Ok(frame)
}

/// Helper for tests and the host: an integral `num`.
pub fn int(i: i128) -> Value {
    Value::Num(Num::from_int(i).unwrap_or(Num::ZERO))
}

/// A bound method value, for the builtins module.
pub fn method(receiver: Value, name: &'static str) -> Value {
    Value::Method(Rc::new(Method { receiver, name }))
}

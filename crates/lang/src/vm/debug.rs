//! What a debugger reads of a machine and the one thing it writes
//! (`docs/06`, Forks: Stopping and Editing). Reading changes nothing and
//! charges nothing. An edit is a fork's alone: the match's sim holds no
//! call to it.

use super::{Machine, Mode, Program};
use crate::ast::{Expr, ExprKind, StmtKind, Target, UnaryOp};
use crate::compile::Code;
use crate::errors::Exception;
use crate::parser::parse_file_bounded;
use crate::value::{R, Value, check_key, dict_find, set_find};
use crate::vm::Interrupt;
use std::collections::BTreeSet;

/// One frame of a stopped machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameView {
    pub file: String,
    /// The line the frame stands on: for the frame on top, the statement
    /// it is in or about to begin; for one beneath, its call.
    pub line: u32,
    pub function: String,
    /// The frame's locals in the order the function declares them, with
    /// what each holds as text, or `None` where nothing is bound yet.
    pub locals: Vec<(String, Option<String>)>,
}

/// The globals of one module the machine has run; `main` is `main.py`'s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleView {
    pub name: String,
    pub globals: Vec<(String, String)>,
}

/// The handler a machine is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandlerView {
    pub kind: Interrupt,
    pub hook_spent: u64,
    pub hook_budget: u64,
}

/// What a debugger is shown of a machine at a position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inspection {
    /// Bottom of the stack first: main flow's frame is `0`.
    pub frames: Vec<FrameView>,
    /// `main` first, then every other module run, by name.
    pub modules: Vec<ModuleView>,
    /// The tick budget, before any deficit.
    pub budget: u64,
    /// Cost units spent this tick so far.
    pub spent: u64,
    /// Cost units the tick's slice opened in the red by or, once the
    /// slice is over, the next tick's will.
    pub deficit: u64,
    /// Pending interrupts, lowest priority first.
    pub pending: Vec<Interrupt>,
    /// `None` in main flow.
    pub handler: Option<HandlerView>,
    /// A host call is waiting on an action.
    pub waiting: bool,
    pub dead: bool,
}

/// What an edit sets (`docs/06`, Editing).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EditTarget {
    /// A local of the frame `depth` up from the bottom of the stack.
    Local { depth: usize, name: String },
    /// A global of a module the machine has run; `main` is `main.py`'s.
    Global { module: String, name: String },
}

/// A value as text, for an inspector: as `str` writes it inside a
/// container, an instance and a record with their attributes, cut off at
/// the nesting limit and at `left` characters.
struct Shown {
    out: String,
    left: usize,
}

impl Shown {
    fn text(&mut self, s: &str) {
        for c in s.chars() {
            if self.left == 0 {
                return;
            }
            self.left = self.left.saturating_sub(1);
            self.out.push(c);
        }
    }

    fn seq<'a>(&mut self, items: impl Iterator<Item = &'a Value>, depth: u32) {
        for (i, v) in items.enumerate() {
            if self.left == 0 {
                return;
            }
            if i > 0 {
                self.text(", ");
            }
            self.value(v, depth);
        }
    }

    fn attrs(&mut self, attrs: &[(String, Value)], depth: u32) {
        for (i, (k, v)) in attrs.iter().enumerate() {
            if self.left == 0 {
                return;
            }
            if i > 0 {
                self.text(", ");
            }
            self.text(k);
            self.text("=");
            self.value(v, depth);
        }
    }

    fn value(&mut self, v: &Value, depth: u32) {
        if self.left == 0 {
            return;
        }
        let Some(below) = depth.checked_sub(1) else {
            self.text("...");
            return;
        };
        match v {
            Value::List(l) => {
                self.text("[");
                self.seq(l.items.borrow().iter(), below);
                self.text("]");
            }
            Value::Tuple(t) => {
                self.text("(");
                self.seq(t.iter(), below);
                self.text(if t.len() == 1 { ",)" } else { ")" });
            }
            Value::Set(s) => {
                let items = s.items.borrow();
                if items.is_empty() {
                    self.text("set()");
                } else {
                    self.text("{");
                    self.seq(items.iter(), below);
                    self.text("}");
                }
            }
            Value::Dict(d) => {
                self.text("{");
                for (i, (k, x)) in d.items.borrow().iter().enumerate() {
                    if self.left == 0 {
                        return;
                    }
                    if i > 0 {
                        self.text(", ");
                    }
                    self.value(k, below);
                    self.text(": ");
                    self.value(x, below);
                }
                self.text("}");
            }
            Value::Inst(i) => {
                self.text(&i.class.name);
                self.text("(");
                self.attrs(&i.attrs.borrow(), below);
                self.text(")");
            }
            Value::Record(r) => {
                self.text(&r.type_name);
                self.text("(");
                self.attrs(&r.fields, below);
                self.text(")");
            }
            // Everything else holds nothing to descend into.
            other => self.text(&crate::value::quoted(other)),
        }
    }
}

fn show(v: &Value, depth: u32, chars: usize) -> String {
    let mut s = Shown {
        out: String::new(),
        left: chars,
    };
    s.value(v, depth.saturating_add(1));
    if s.left == 0 {
        s.out.push('…');
    }
    s.out
}

/// A hidden slot of the compiler's — a comprehension's or a `match`'s —
/// is no local of the program's.
fn hidden(name: &str) -> bool {
    name.starts_with('<')
}

impl Machine {
    /// The machine as a debugger shows it, each value as text of at most
    /// `value_chars` characters (`docs/06`, Stopping).
    pub fn inspect(&self, value_chars: usize) -> Inspection {
        let depth = self.limits.depth_nesting as u32;
        let top = self.frames.len().saturating_sub(1);
        let frames = self
            .frames
            .iter()
            .enumerate()
            .map(|(i, f)| {
                // The frame on top stands before the instruction at its
                // `pc` unless it is inside one — waiting on a host call or
                // running a native. A frame beneath is inside its call.
                let inside = i < top || self.waiting || !f.natives.is_empty();
                let at = if inside || f.pc >= f.code.lines.len() {
                    f.pc.saturating_sub(1)
                } else {
                    f.pc
                };
                FrameView {
                    file: f.code.file.clone(),
                    line: f.code.lines.get(at).copied().unwrap_or(0),
                    function: f.code.name.clone(),
                    locals: f
                        .code
                        .local_names
                        .iter()
                        .zip(&f.locals)
                        .filter(|(name, _)| !hidden(name))
                        .map(|(name, v)| {
                            (
                                name.clone(),
                                v.as_ref().map(|v| show(v, depth, value_chars)),
                            )
                        })
                        .collect(),
                }
            })
            .collect();
        let view = |name: &str, g: &crate::value::Globals| ModuleView {
            name: name.to_string(),
            globals: g
                .borrow()
                .iter()
                .map(|(k, v)| (k.clone(), show(v, depth, value_chars)))
                .collect(),
        };
        let mut modules = vec![view("main", &self.globals)];
        modules.extend(self.modules_run.iter().map(|(n, m)| view(n, &m.globals)));
        Inspection {
            frames,
            modules,
            budget: self.limits.budget_tick,
            spent: self.spent,
            deficit: self.deficit,
            pending: self.pending.iter().copied().collect(),
            handler: match self.mode {
                Mode::Main => None,
                Mode::Handler {
                    kind,
                    hook_spent,
                    hook_budget,
                    ..
                } => Some(HandlerView {
                    kind,
                    hook_spent,
                    hook_budget,
                }),
            },
            waiting: self.waiting,
            dead: self.dead,
        }
    }

    /// Frames on the stack; an *over* step compares it between boundaries.
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// How many boundaries this tick's slice has passed before the one
    /// the machine stands at: the `n` of a position (`docs/06`, Positions).
    pub fn boundary(&self) -> u64 {
        self.passed.saturating_sub(1)
    }

    /// The counters a step compares between two boundaries.
    pub fn trace(&self) -> super::Trace {
        self.trace
    }

    /// The interrupt the boundary the machine stands at delivers, if it
    /// delivers one: the next thing its slice does. `None` where the slice
    /// ends first, at a restart's boundary, and for a dead machine.
    pub fn delivering(&self) -> Option<Interrupt> {
        if self.dead || self.slice_end.is_some() {
            return None;
        }
        self.deliverable()
    }

    /// Whether a slice of this machine is stopped at a boundary.
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// The bundle version the machine runs.
    pub fn version(&self) -> u64 {
        self.program.version
    }

    /// The machine's program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Set a local or a global to a value written as a literal or a
    /// display of literals (`docs/06`, Editing). The value is held to the
    /// limits an assignment of the program's own is: its own sizes, and
    /// the machine's live values counted with it in place of the old. A
    /// refusal names its reason and leaves the machine as it was.
    pub fn edit(&mut self, target: &EditTarget, literal: &str) -> Result<(), String> {
        if self.dead {
            return Err("the machine is dead".into());
        }
        let nesting = self.limits.depth_nesting as u32;
        let stmts = parse_file_bounded("<edit>", &format!("edit = {literal}\n"), nesting)
            .map_err(|e| e.message)?;
        let expr = match stmts.as_slice() {
            [one] => match &one.kind {
                StmtKind::Assign { targets, value }
                    if matches!(targets.as_slice(), [Target::Name(_)]) =>
                {
                    value
                }
                _ => return Err("an edit is one value".into()),
            },
            _ => return Err("an edit is one value".into()),
        };
        // A refused edit leaves nothing behind: not an identity taken,
        // nor the live-values measure moved.
        let saved = (self.next_object_id, self.live_measured, self.live_pressure);
        let refuse = |m: &mut Machine, why: String| {
            (m.next_object_id, m.live_measured, m.live_pressure) = saved;
            Err(why)
        };
        let value = match self.literal(expr) {
            Ok(v) => v,
            Err(why) => return refuse(self, why),
        };
        let old = match self.swap_slot(target, Some(value)) {
            Ok(old) => old,
            Err(why) => return refuse(self, why),
        };
        let limit = usize::try_from(self.limits.size_live_values).unwrap_or(usize::MAX);
        let count = self.live_count(limit.saturating_add(1));
        if count > limit {
            // The slot was just set, so putting the old value back cannot
            // fail.
            let _ = self.swap_slot(target, old);
            return refuse(self, "LimitError: the machine's live values".into());
        }
        self.live_measured = count;
        self.live_pressure = 0;
        Ok(())
    }

    /// Put `value` in the slot `target` names and hand back what it held;
    /// `None` is a local with nothing bound.
    fn swap_slot(
        &mut self,
        target: &EditTarget,
        value: Option<Value>,
    ) -> Result<Option<Value>, String> {
        match target {
            EditTarget::Local { depth, name } => {
                let f = self
                    .frames
                    .get_mut(*depth)
                    .ok_or_else(|| format!("no frame at depth {depth}"))?;
                let slot = f
                    .code
                    .local_names
                    .iter()
                    .position(|n| n == name && !hidden(n))
                    .ok_or_else(|| format!("`{}` has no local `{name}`", f.code.name))?;
                let cell = f
                    .locals
                    .get_mut(slot)
                    .ok_or_else(|| format!("`{}` has no local `{name}`", f.code.name))?;
                Ok(std::mem::replace(cell, value))
            }
            EditTarget::Global { module, name } => {
                let globals = if module == "main" {
                    self.globals.clone()
                } else {
                    self.modules_run
                        .get(module)
                        .ok_or_else(|| format!("the machine has not run `{module}`"))?
                        .globals
                        .clone()
                };
                let mut g = globals.borrow_mut();
                if !g.contains_key(name) {
                    return Err(format!("`{module}` has no global `{name}`"));
                }
                Ok(match value {
                    Some(v) => g.insert(name.clone(), v),
                    None => g.remove(name),
                })
            }
        }
    }

    /// The value a literal or a display of literals writes.
    fn literal(&mut self, e: &Expr) -> Result<Value, String> {
        let named = |r: R<()>| r.map_err(|e: Exception| e.class_name());
        let nesting = self.limits.depth_nesting as u32;
        Ok(match &e.kind {
            ExprKind::Num(n) => Value::Num(*n),
            ExprKind::Str(s) => {
                named(self.check_size(s.chars().count()))
                    .map_err(|c| format!("{c}: the size of a str"))?;
                self.note_alloc(1);
                Value::str(s)
            }
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::None => Value::None,
            ExprKind::UnaryOp(UnaryOp::Neg, inner) => match &inner.kind {
                ExprKind::Num(n) => Value::Num(n.checked_neg().map_err(|e| e.class_name())?),
                _ => return Err("an edit is a literal or a display of literals".into()),
            },
            ExprKind::List(items) | ExprKind::Tuple(items) | ExprKind::Set(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.literal(item)?);
                }
                named(self.check_size(values.len()))
                    .map_err(|c| format!("{c}: the size of a collection"))?;
                match &e.kind {
                    ExprKind::List(_) => self.new_list(values),
                    ExprKind::Tuple(_) => {
                        self.note_values(&values);
                        Value::tuple(values)
                    }
                    _ => {
                        let mut unique: Vec<Value> = Vec::with_capacity(values.len());
                        for v in values {
                            named(check_key(&v))?;
                            let found =
                                set_find(&unique, &v, nesting).map_err(|e| e.class_name())?;
                            if found.is_none() {
                                unique.push(v);
                            }
                        }
                        self.new_set(unique)
                    }
                }
            }
            ExprKind::Dict(pairs) => {
                named(self.check_size(pairs.len()))
                    .map_err(|c| format!("{c}: the size of a collection"))?;
                let mut items: Vec<(Value, Value)> = Vec::with_capacity(pairs.len());
                for (k, v) in pairs {
                    let k = self.literal(k)?;
                    let v = self.literal(v)?;
                    named(check_key(&k))?;
                    match dict_find(&items, &k, nesting).map_err(|e| e.class_name())? {
                        Some(i) => {
                            if let Some(slot) = items.get_mut(i) {
                                slot.1 = v;
                            }
                        }
                        None => items.push((k, v)),
                    }
                }
                self.new_dict(items)
            }
            _ => return Err("an edit is a literal or a display of literals".into()),
        })
    }
}

fn begin_lines_of(code: &Code, out: &mut BTreeSet<u32>) {
    for (line, begins) in code.lines.iter().zip(&code.begins) {
        if *begins {
            out.insert(*line);
        }
    }
    for child in &code.children {
        begin_lines_of(child, out);
    }
}

impl Program {
    /// The lines of a file of the bundle on which a statement can begin
    /// (`docs/06`, Statements): where a breakpoint may sit. Empty for a
    /// file the bundle does not hold.
    pub fn begin_lines(&self, file: &str) -> BTreeSet<u32> {
        let mut out = BTreeSet::new();
        if let Some(code) = self.modules.get(file.trim_end_matches(".py")) {
            begin_lines_of(code, &mut out);
        }
        out
    }
}

//! A machine's deep copy, for a saved state and a fork (`docs/06`, Forks).
//!
//! The copy shares nothing mutable with its source: every list, dict, set,
//! class, instance and module's globals is its own, with the source's
//! identity number, and whatever the source shares between two slots the
//! copy shares between the same two. What cannot change — compiled code,
//! strings, the cost and limit tables, the program — is shared.
//!
//! Sharing is found by address: the memo maps a source object's address to
//! its copy. An address is only ever looked up, never ordered on and never
//! written anywhere, so it decides nothing a peer could see differently
//! (CLAUDE.md, rule 3).

use super::{After, Block, FaultRecord, Frame, Machine};
use crate::dispatch::{FindMode, Native, Piece, SortTask, Then};
use crate::errors::Exception;
use crate::value::{
    Bound, Class, DictObj, Func, Globals, Instance, IterObj, ListObj, Method, Module, Record,
    SetObj, Value,
};
use crate::vm::Event;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

/// A source object's address, as a memo key.
fn key<T: ?Sized>(rc: &Rc<T>) -> usize {
    Rc::as_ptr(rc).cast::<()>() as usize
}

#[derive(Default)]
struct Memo {
    lists: BTreeMap<usize, Rc<ListObj>>,
    dicts: BTreeMap<usize, Rc<DictObj>>,
    sets: BTreeMap<usize, Rc<SetObj>>,
    tuples: BTreeMap<usize, Rc<[Value]>>,
    classes: BTreeMap<usize, Rc<Class>>,
    insts: BTreeMap<usize, Rc<Instance>>,
    funcs: BTreeMap<usize, Rc<Func>>,
    bounds: BTreeMap<usize, Rc<Bound>>,
    methods: BTreeMap<usize, Rc<Method>>,
    modules: BTreeMap<usize, Rc<Module>>,
    excs: BTreeMap<usize, Rc<Exception>>,
    records: BTreeMap<usize, Rc<Record>>,
    iters: BTreeMap<usize, Rc<IterObj>>,
    globals: BTreeMap<usize, Globals>,
}

/// What can be copied through a memo.
trait Copied {
    fn copied(&self, m: &mut Memo) -> Self;
}

impl<T: Copied> Copied for Vec<T> {
    fn copied(&self, m: &mut Memo) -> Self {
        self.iter().map(|x| x.copied(m)).collect()
    }
}

impl<T: Copied> Copied for Option<T> {
    fn copied(&self, m: &mut Memo) -> Self {
        self.as_ref().map(|x| x.copied(m))
    }
}

impl<A: Copied, B: Copied> Copied for (A, B) {
    fn copied(&self, m: &mut Memo) -> Self {
        (self.0.copied(m), self.1.copied(m))
    }
}

impl Copied for String {
    fn copied(&self, _: &mut Memo) -> Self {
        self.clone()
    }
}

impl Copied for Value {
    fn copied(&self, m: &mut Memo) -> Self {
        match self {
            Value::None
            | Value::Bool(_)
            | Value::Num(_)
            | Value::Str(_)
            | Value::Builtin(_)
            | Value::ExcClass(_) => self.clone(),
            Value::List(l) => Value::List(m.list(l)),
            Value::Tuple(t) => Value::Tuple(m.tuple(t)),
            Value::Dict(d) => Value::Dict(m.dict(d)),
            Value::Set(s) => Value::Set(m.set(s)),
            Value::Func(f) => Value::Func(m.func(f)),
            Value::Method(x) => Value::Method(m.method(x)),
            Value::Exc(e) => Value::Exc(m.exc(e)),
            Value::Record(r) => Value::Record(m.record(r)),
            Value::Iter(it) => Value::Iter(m.iter(it)),
            Value::Class(c) => Value::Class(m.class(c)),
            Value::Inst(i) => Value::Inst(m.inst(i)),
            Value::Bound(b) => Value::Bound(m.bound(b)),
            Value::Module(x) => Value::Module(m.module(x)),
        }
    }
}

impl Copied for Exception {
    fn copied(&self, m: &mut Memo) -> Self {
        Exception {
            class: self.class,
            args: self.args.copied(m),
            file: self.file.clone(),
            line: self.line,
            tick: self.tick,
            user: self.user.as_ref().map(|i| m.inst(i)),
        }
    }
}

// An object that can be reached from itself is made empty, entered in the
// memo, and only then filled: the walk that fills it finds the entry.
// One that cannot change copies what it holds first and looks again
// before it is made, since that walk may have come back round to it.
impl Memo {
    fn list(&mut self, l: &Rc<ListObj>) -> Rc<ListObj> {
        if let Some(c) = self.lists.get(&key(l)) {
            return c.clone();
        }
        let c = Rc::new(ListObj {
            id: l.id,
            items: RefCell::new(Vec::new()),
        });
        self.lists.insert(key(l), c.clone());
        let items = l.items.borrow().copied(self);
        *c.items.borrow_mut() = items;
        c
    }

    fn dict(&mut self, d: &Rc<DictObj>) -> Rc<DictObj> {
        if let Some(c) = self.dicts.get(&key(d)) {
            return c.clone();
        }
        let c = Rc::new(DictObj {
            id: d.id,
            items: RefCell::new(Vec::new()),
        });
        self.dicts.insert(key(d), c.clone());
        let items = d.items.borrow().copied(self);
        *c.items.borrow_mut() = items;
        c
    }

    fn set(&mut self, s: &Rc<SetObj>) -> Rc<SetObj> {
        if let Some(c) = self.sets.get(&key(s)) {
            return c.clone();
        }
        let c = Rc::new(SetObj {
            id: s.id,
            items: RefCell::new(Vec::new()),
        });
        self.sets.insert(key(s), c.clone());
        let items = s.items.borrow().copied(self);
        *c.items.borrow_mut() = items;
        c
    }

    fn tuple(&mut self, t: &Rc<[Value]>) -> Rc<[Value]> {
        if let Some(c) = self.tuples.get(&key(t)) {
            return c.clone();
        }
        let items: Vec<Value> = t.iter().map(|v| v.copied(self)).collect();
        if let Some(c) = self.tuples.get(&key(t)) {
            return c.clone();
        }
        let c: Rc<[Value]> = Rc::from(items);
        self.tuples.insert(key(t), c.clone());
        c
    }

    fn class(&mut self, c: &Rc<Class>) -> Rc<Class> {
        if let Some(x) = self.classes.get(&key(c)) {
            return x.clone();
        }
        let base = c.base.as_ref().map(|b| self.class(b));
        if let Some(x) = self.classes.get(&key(c)) {
            return x.clone();
        }
        let x = Rc::new(Class {
            id: c.id,
            name: c.name.clone(),
            base,
            exc_base: c.exc_base,
            attrs: RefCell::new(Vec::new()),
        });
        self.classes.insert(key(c), x.clone());
        let attrs = c.attrs.borrow().copied(self);
        *x.attrs.borrow_mut() = attrs;
        x
    }

    fn inst(&mut self, i: &Rc<Instance>) -> Rc<Instance> {
        if let Some(x) = self.insts.get(&key(i)) {
            return x.clone();
        }
        let class = self.class(&i.class);
        if let Some(x) = self.insts.get(&key(i)) {
            return x.clone();
        }
        let x = Rc::new(Instance {
            id: i.id,
            class,
            attrs: RefCell::new(Vec::new()),
        });
        self.insts.insert(key(i), x.clone());
        let attrs = i.attrs.borrow().copied(self);
        *x.attrs.borrow_mut() = attrs;
        x
    }

    fn globals(&mut self, g: &Globals) -> Globals {
        if let Some(x) = self.globals.get(&key(g)) {
            return x.clone();
        }
        let x: Globals = Rc::new(RefCell::new(BTreeMap::new()));
        self.globals.insert(key(g), x.clone());
        let filled: BTreeMap<String, Value> = g
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.copied(self)))
            .collect();
        *x.borrow_mut() = filled;
        x
    }

    fn func(&mut self, f: &Rc<Func>) -> Rc<Func> {
        if let Some(x) = self.funcs.get(&key(f)) {
            return x.clone();
        }
        let defaults = f.defaults.copied(self);
        let globals = self.globals(&f.globals);
        if let Some(x) = self.funcs.get(&key(f)) {
            return x.clone();
        }
        let x = Rc::new(Func {
            code: f.code.clone(),
            defaults,
            globals,
        });
        self.funcs.insert(key(f), x.clone());
        x
    }

    fn bound(&mut self, b: &Rc<Bound>) -> Rc<Bound> {
        if let Some(x) = self.bounds.get(&key(b)) {
            return x.clone();
        }
        let func = self.func(&b.func);
        let receiver = b.receiver.copied(self);
        if let Some(x) = self.bounds.get(&key(b)) {
            return x.clone();
        }
        let x = Rc::new(Bound { func, receiver });
        self.bounds.insert(key(b), x.clone());
        x
    }

    fn method(&mut self, b: &Rc<Method>) -> Rc<Method> {
        if let Some(x) = self.methods.get(&key(b)) {
            return x.clone();
        }
        let receiver = b.receiver.copied(self);
        if let Some(x) = self.methods.get(&key(b)) {
            return x.clone();
        }
        let x = Rc::new(Method {
            receiver,
            name: b.name,
        });
        self.methods.insert(key(b), x.clone());
        x
    }

    fn module(&mut self, b: &Rc<Module>) -> Rc<Module> {
        if let Some(x) = self.modules.get(&key(b)) {
            return x.clone();
        }
        let globals = self.globals(&b.globals);
        if let Some(x) = self.modules.get(&key(b)) {
            return x.clone();
        }
        let x = Rc::new(Module {
            name: b.name.clone(),
            globals,
        });
        self.modules.insert(key(b), x.clone());
        x
    }

    fn exc(&mut self, e: &Rc<Exception>) -> Rc<Exception> {
        if let Some(x) = self.excs.get(&key(e)) {
            return x.clone();
        }
        let copy = e.as_ref().copied(self);
        if let Some(x) = self.excs.get(&key(e)) {
            return x.clone();
        }
        let x = Rc::new(copy);
        self.excs.insert(key(e), x.clone());
        x
    }

    fn record(&mut self, r: &Rc<Record>) -> Rc<Record> {
        if let Some(x) = self.records.get(&key(r)) {
            return x.clone();
        }
        let fields = r.fields.copied(self);
        if let Some(x) = self.records.get(&key(r)) {
            return x.clone();
        }
        let x = Rc::new(Record {
            type_name: r.type_name.clone(),
            fields,
        });
        self.records.insert(key(r), x.clone());
        x
    }

    fn iter(&mut self, it: &Rc<IterObj>) -> Rc<IterObj> {
        if let Some(x) = self.iters.get(&key(it)) {
            return x.clone();
        }
        let items = it.items.copied(self);
        let inst = it.inst.as_ref().map(|(i, n)| (self.inst(i), *n));
        if let Some(x) = self.iters.get(&key(it)) {
            return x.clone();
        }
        let x = Rc::new(IterObj {
            items,
            pos: Cell::new(it.pos.get()),
            inst,
        });
        self.iters.insert(key(it), x.clone());
        x
    }
}

impl Copied for SortTask {
    fn copied(&self, _: &mut Memo) -> Self {
        match self {
            SortTask::Split { lo, hi } => SortTask::Split { lo: *lo, hi: *hi },
            SortTask::Merge {
                lo,
                mid,
                hi,
                left,
                right,
                i,
                j,
                k,
                started,
            } => SortTask::Merge {
                lo: *lo,
                mid: *mid,
                hi: *hi,
                left: left.clone(),
                right: right.clone(),
                i: *i,
                j: *j,
                k: *k,
                started: *started,
            },
        }
    }
}

impl Copied for Piece {
    fn copied(&self, m: &mut Memo) -> Self {
        match self {
            Piece::Lit(s) => Piece::Lit(s.clone()),
            Piece::Val(v) => Piece::Val(v.copied(m)),
        }
    }
}

impl Copied for Then {
    fn copied(&self, m: &mut Memo) -> Self {
        match self {
            Then::Restore { below, above } => Then::Restore {
                below: below.copied(m),
                above: above.copied(m),
            },
            Then::Builtin {
                name,
                args,
                kwargs,
                slot,
            } => Then::Builtin {
                name: name.clone(),
                args: args.copied(m),
                kwargs: kwargs.copied(m),
                slot: *slot,
            },
            Then::Method {
                recv,
                name,
                args,
                kwargs,
                slot,
            } => Then::Method {
                recv: recv.copied(m),
                name,
                args: args.copied(m),
                kwargs: kwargs.copied(m),
                slot: *slot,
            },
        }
    }
}

impl Copied for Native {
    fn copied(&self, m: &mut Memo) -> Self {
        match self {
            Native::Eq {
                a,
                b,
                negate,
                stage,
            } => Native::Eq {
                a: a.copied(m),
                b: b.copied(m),
                negate: *negate,
                stage: *stage,
            },
            Native::SeqEq { xs, ys, i } => Native::SeqEq {
                xs: xs.copied(m),
                ys: ys.copied(m),
                i: *i,
            },
            Native::PairsEq { pairs, i } => Native::PairsEq {
                pairs: pairs.copied(m),
                i: *i,
            },
            Native::Lt { a, b, stage } => Native::Lt {
                a: a.copied(m),
                b: b.copied(m),
                stage: *stage,
            },
            Native::SeqLt { xs, ys, i, stage } => Native::SeqLt {
                xs: xs.copied(m),
                ys: ys.copied(m),
                i: *i,
                stage: *stage,
            },
            Native::Cmp { op, a, b, stage } => Native::Cmp {
                op: *op,
                a: a.copied(m),
                b: b.copied(m),
                stage: *stage,
            },
            Native::Contains {
                container,
                item,
                negate,
                stage,
            } => Native::Contains {
                container: container.copied(m),
                item: item.copied(m),
                negate: *negate,
                stage: *stage,
            },
            Native::Find {
                items,
                target,
                i,
                count,
                mode,
            } => Native::Find {
                items: items.copied(m),
                target: target.copied(m),
                i: *i,
                count: *count,
                mode: match mode {
                    FindMode::Contains => FindMode::Contains,
                    FindMode::Index => FindMode::Index,
                    FindMode::Count => FindMode::Count,
                    FindMode::Remove(l) => FindMode::Remove(m.list(l)),
                },
            },
            Native::Truthy { v, stage } => Native::Truthy {
                v: v.copied(m),
                stage: *stage,
            },
            Native::Len { v, stage } => Native::Len {
                v: v.copied(m),
                stage: *stage,
            },
            Native::Str { v, stage } => Native::Str {
                v: v.copied(m),
                stage: *stage,
            },
            Native::StrWalk { pieces, i, out } => Native::StrWalk {
                pieces: pieces.copied(m),
                i: *i,
                out: out.clone(),
            },
            Native::Format { v, spec, stage } => Native::Format {
                v: v.copied(m),
                spec: spec.clone(),
                stage: *stage,
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
            } => Native::Sort {
                items: items.copied(m),
                key: key.copied(m),
                keys: keys.copied(m),
                reverse: *reverse,
                idx: idx.clone(),
                tasks: tasks.copied(m),
                target: target.as_ref().map(|l| m.list(l)),
                stage: *stage,
            },
            Native::MinMax {
                items,
                key,
                keys,
                is_min,
                best,
                i,
                stage,
            } => Native::MinMax {
                items: items.copied(m),
                key: key.copied(m),
                keys: keys.copied(m),
                is_min: *is_min,
                best: *best,
                i: *i,
                stage: *stage,
            },
            Native::AnyAll { items, i, is_any } => Native::AnyAll {
                items: items.copied(m),
                i: *i,
                is_any: *is_any,
            },
            Native::Construct {
                class,
                args,
                kwargs,
                inst,
                then_raise,
            } => Native::Construct {
                class: m.class(class),
                args: args.copied(m),
                kwargs: kwargs.copied(m),
                inst: inst.as_ref().map(|i| m.inst(i)),
                then_raise: *then_raise,
            },
            Native::Materialise {
                obj,
                n,
                i,
                out,
                then,
                stage,
            } => Native::Materialise {
                obj: m.inst(obj),
                n: *n,
                i: *i,
                out: out.copied(m),
                then: then.copied(m),
                stage: *stage,
            },
            Native::IterStart { obj } => Native::IterStart { obj: m.inst(obj) },
            Native::GetItem { obj, idx } => Native::GetItem {
                obj: m.inst(obj),
                idx: idx.copied(m),
            },
            Native::SetItem { obj, idx, v, stage } => Native::SetItem {
                obj: m.inst(obj),
                idx: idx.copied(m),
                v: v.copied(m),
                stage: *stage,
            },
            Native::BinOp { op, a, b } => Native::BinOp {
                op: *op,
                a: m.inst(a),
                b: b.copied(m),
            },
            Native::Neg { a } => Native::Neg { a: m.inst(a) },
            Native::Not { v, stage } => Native::Not {
                v: v.copied(m),
                stage: *stage,
            },
            Native::JumpTruth { v, stage } => Native::JumpTruth {
                v: v.copied(m),
                stage: *stage,
            },
            Native::LogArgs {
                name,
                args,
                kwargs,
                i,
                out,
            } => Native::LogArgs {
                name: name.clone(),
                args: args.copied(m),
                kwargs: kwargs.copied(m),
                i: *i,
                out: out.copied(m),
            },
        }
    }
}

impl Copied for Frame {
    fn copied(&self, m: &mut Memo) -> Self {
        Frame {
            code: self.code.clone(),
            pc: self.pc,
            stack: self.stack.copied(m),
            locals: self.locals.copied(m),
            blocks: self
                .blocks
                .iter()
                .map(|b| Block {
                    kind: b.kind,
                    stack_len: b.stack_len,
                    handling_len: b.handling_len,
                })
                .collect(),
            handling: self.handling.iter().map(|e| m.exc(e)).collect(),
            pending: self.pending.copied(m),
            natives: self.natives.copied(m),
            after: self.after.as_ref().map(|a| After {
                target: a.target,
                jump_if: a.jump_if,
                keep: a.keep.copied(m),
                pop: a.pop,
            }),
            class_base: self.class_base.copied(m),
            globals: m.globals(&self.globals),
            import_of: self.import_of.as_ref().map(|x| m.module(x)),
            discard_result: self.discard_result,
        }
    }
}

impl Copied for Event {
    fn copied(&self, m: &mut Memo) -> Self {
        match self {
            Event::Fault(e) => Event::Fault(e.copied(m)),
            Event::Abandoned(e) => Event::Abandoned(e.copied(m)),
            Event::Escalated { from, to } => Event::Escalated {
                from: *from,
                to: *to,
            },
            Event::Dying => Event::Dying,
            Event::Death => Event::Death,
            Event::Redeploy { swapped } => Event::Redeploy { swapped: *swapped },
        }
    }
}

impl Machine {
    /// A copy that shares nothing the program can change with this
    /// machine, standing exactly where it stands — mid-slice included —
    /// and running from there exactly as it would (`docs/06`, Forks).
    pub fn deep_copy(&self) -> Machine {
        let mut m = Memo::default();
        Machine {
            program: self.program.clone(),
            costs: self.costs.clone(),
            limits: self.limits.clone(),
            frames: self.frames.copied(&mut m),
            globals: m.globals(&self.globals),
            modules_run: self
                .modules_run
                .iter()
                .map(|(k, v)| (k.clone(), m.module(v)))
                .collect(),
            spent: self.spent,
            deficit: self.deficit,
            restarted_this_tick: self.restarted_this_tick,
            next_object_id: self.next_object_id,
            waiting: self.waiting,
            native_waiting: self.native_waiting,
            native_input: self.native_input.copied(&mut m),
            live_measured: self.live_measured,
            live_pressure: self.live_pressure,
            fault_record: self.fault_record.as_ref().map(|r| FaultRecord {
                exception: r.exception.copied(&mut m),
                exhausted: r.exhausted,
                file: r.file.clone(),
                line: r.line,
                tick: r.tick,
                version: r.version,
            }),
            tick: self.tick,
            mode: self.mode,
            pending: self.pending.clone(),
            events: self.events.copied(&mut m),
            dead: self.dead,
            slice_budget: self.slice_budget,
            slice_end: self.slice_end,
            passed: self.passed,
            stopped: self.stopped,
            trace: self.trace,
        }
    }
}

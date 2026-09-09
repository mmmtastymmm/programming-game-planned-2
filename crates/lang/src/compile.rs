//! The AST to bytecode. A program has to pause between any two operations
//! and resume on a later tick with every variable intact, which a tree
//! walker cannot do without unwinding the host stack; a small stack machine
//! can, because its whole state is data. Every instruction is one operation
//! of `docs/01-language/costs.md`, or one bookkeeping step that costs
//! nothing.
//!
//! Scope is resolved here (`syntax.md`, Names and scope): a name assigned
//! anywhere in a `def` body is a local slot throughout it, a name only read
//! is a global; a comprehension target is a private slot; a `lambda` body
//! that names an enclosing local is a load error.

use crate::ast::*;
use crate::errors::LoadError;
use crate::value::Value;
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Instr {
    /// A statement begins: charge `op.statement`.
    Stmt,
    Const(usize),
    LoadLocal(usize),
    StoreLocal(usize),
    LoadGlobal(usize),
    StoreGlobal(usize),
    Pop,
    Dup,
    /// Duplicate the top two.
    Dup2,
    Swap,
    /// `a b c` → `c a b`: moves the top under the two below it, so a value
    /// computed after its target's pieces can be stored by the same
    /// instruction plain assignment uses.
    Rot3,
    BinOp(BinOp),
    UnaryOp(UnaryOp),
    Compare(CmpOp),
    Jump(usize),
    JumpIfFalsePop(usize),
    JumpIfTruePop(usize),
    JumpIfFalseKeep(usize),
    JumpIfTrueKeep(usize),
    BuildList(usize),
    BuildTuple(usize),
    BuildDict(usize),
    BuildSet(usize),
    /// Append the top to the list in the given slot (a comprehension).
    CompAppend(usize),
    CompAdd(usize),
    /// Pop value then key, set into the dict in the slot.
    CompSet(usize),
    Subscript,
    StoreSubscript,
    /// Pop step, hi, lo (`None` for absent) and push a slice.
    BuildSlice,
    LoadAttr(usize),
    StoreAttr(usize),
    Call {
        argc: usize,
        /// Index of a const tuple of keyword names, or `usize::MAX`.
        kwnames: usize,
        star: bool,
        dstar: bool,
    },
    /// Run a class body (a child code object) as a frame whose locals become
    /// the class's attributes; the base, if `has_base`, is on the stack.
    BuildClass {
        child: usize,
        has_base: bool,
    },
    /// The last instruction of a class body: pack the frame's locals into a
    /// class and hand it to the caller.
    ReturnClass,
    MakeFunction {
        child: usize,
        ndefaults: usize,
    },
    Return,
    /// Snapshot the iterable on top into an iterator.
    GetIter,
    /// Push the next item, or pop the iterator and jump.
    ForIter(usize),
    /// Pop an iterable; push its items so that `n` stores pop them in order.
    Unpack {
        n: usize,
        star: Option<usize>,
    },
    SetupExcept(usize),
    SetupFinally(usize),
    PopBlock,
    /// Leave a `finally` body: re-raise a pending exception, if any.
    EndFinally,
    /// Pop the exception being handled (end of an `except` body).
    PopExcept,
    /// `raise` with a value (`true`) or bare (`false`).
    Raise(bool),
    /// Pop the types; keep the exception; push whether it matches.
    ExcMatch,
    /// Pop a value, push its `str` with a format spec const (or MAX).
    FormatValue(usize),
    /// Pop `n` strings and push their concatenation.
    BuildString(usize),
    /// Extend the list on top with the iterable below-top (a starred display).
    ListExtend,
}

#[derive(Debug)]
pub struct Code {
    pub name: String,
    pub file: String,
    pub nparams: usize,
    pub star: Option<usize>,
    pub dstar: Option<usize>,
    pub local_names: Vec<String>,
    pub consts: Vec<Value>,
    pub names: Vec<String>,
    pub code: Vec<Instr>,
    pub lines: Vec<u32>,
    pub children: Vec<Rc<Code>>,
    pub is_module: bool,
}

type C<T> = Result<T, LoadError>;

enum BlockCtx {
    Except,
    Finally(Vec<Stmt>),
}

struct LoopCtx {
    start: usize,
    breaks: Vec<usize>,
    blocks_at_entry: usize,
}

struct Ctx<'a> {
    file: &'a str,
    name: String,
    is_module: bool,
    locals: Vec<String>,
    /// Names that are locals of this scope (assigned anywhere in it).
    local_set: BTreeSet<String>,
    /// Names the enclosing `def` holds as locals — forbidden in a `lambda`.
    forbidden: BTreeSet<String>,
    /// Comprehension targets in effect: name → private slot.
    overrides: Vec<(String, usize)>,
    consts: Vec<Value>,
    names: Vec<String>,
    code: Vec<Instr>,
    lines: Vec<u32>,
    children: Vec<Rc<Code>>,
    loops: Vec<LoopCtx>,
    blocks: Vec<BlockCtx>,
    line: u32,
    /// Module-level names the module assigns, so a `def` body can be told
    /// which names are globals (for error messages only).
    globals_hint: BTreeSet<String>,
}

/// Compile a parsed file into its module code object.
pub fn compile_module(file: &str, stmts: &[Stmt]) -> C<Rc<Code>> {
    let mut ctx = Ctx::new(file, "<module>", true);
    ctx.globals_hint = assigned_names(stmts);
    ctx.compile_body(stmts)?;
    {
        let k = ctx.const_index(Value::None);
        ctx.emit(Instr::Const(k));
    }
    ctx.emit(Instr::Return);
    Ok(Rc::new(ctx.finish(0, None, None)))
}

impl<'a> Ctx<'a> {
    fn new(file: &'a str, name: &str, is_module: bool) -> Ctx<'a> {
        Ctx {
            file,
            name: name.to_string(),
            is_module,
            locals: vec![],
            local_set: BTreeSet::new(),
            forbidden: BTreeSet::new(),
            overrides: vec![],
            consts: vec![],
            names: vec![],
            code: vec![],
            lines: vec![],
            children: vec![],
            loops: vec![],
            blocks: vec![],
            line: 0,
            globals_hint: BTreeSet::new(),
        }
    }

    fn err(&self, message: impl Into<String>) -> LoadError {
        LoadError {
            file: self.file.to_string(),
            line: self.line,
            message: message.into(),
        }
    }

    fn finish(self, nparams: usize, star: Option<usize>, dstar: Option<usize>) -> Code {
        Code {
            name: self.name,
            file: self.file.to_string(),
            nparams,
            star,
            dstar,
            local_names: self.locals,
            consts: self.consts,
            names: self.names,
            code: self.code,
            lines: self.lines,
            children: self.children,
            is_module: self.is_module,
        }
    }

    fn emit(&mut self, i: Instr) -> usize {
        self.code.push(i);
        self.lines.push(self.line);
        self.code.len().wrapping_sub(1)
    }

    fn here(&self) -> usize {
        self.code.len()
    }

    fn patch(&mut self, at: usize, target: usize) {
        if let Some(
            Instr::Jump(t)
            | Instr::JumpIfFalsePop(t)
            | Instr::JumpIfTruePop(t)
            | Instr::JumpIfFalseKeep(t)
            | Instr::JumpIfTrueKeep(t)
            | Instr::ForIter(t)
            | Instr::SetupExcept(t)
            | Instr::SetupFinally(t),
        ) = self.code.get_mut(at)
        {
            *t = target;
        }
    }

    fn const_index(&mut self, v: Value) -> usize {
        // Constants are deduplicated by structural equality where that is
        // well defined (numbers, strings, None, bools); tuples of names too.
        for (i, c) in self.consts.iter().enumerate() {
            let same = match (c, &v) {
                (Value::None, Value::None) => true,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                (Value::Num(a), Value::Num(b)) => a == b,
                (Value::Str(a), Value::Str(b)) => a == b,
                _ => false,
            };
            if same {
                return i;
            }
        }
        self.consts.push(v);
        self.consts.len().wrapping_sub(1)
    }

    fn name_index(&mut self, name: &str) -> usize {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            return i;
        }
        self.names.push(name.to_string());
        self.names.len().wrapping_sub(1)
    }

    fn local_slot(&mut self, name: &str) -> usize {
        if let Some(i) = self.locals.iter().position(|n| n == name) {
            return i;
        }
        self.locals.push(name.to_string());
        self.locals.len().wrapping_sub(1)
    }

    fn hidden_slot(&mut self, tag: &str) -> usize {
        let name = format!("<{tag}{}>", self.locals.len());
        self.local_slot(&name)
    }

    // -------------------------------------------------------- names ---

    fn load_name(&mut self, name: &str) -> C<()> {
        if let Some((_, slot)) = self.overrides.iter().rev().find(|(n, _)| n == name) {
            let slot = *slot;
            self.emit(Instr::LoadLocal(slot));
            return Ok(());
        }
        if self.forbidden.contains(name) {
            return Err(self.err(format!(
                "a `lambda` body cannot name `{name}`, a local of the enclosing function — pass it through a parameter default"
            )));
        }
        if !self.is_module && self.local_set.contains(name) {
            let slot = self.local_slot(name);
            self.emit(Instr::LoadLocal(slot));
        } else {
            let idx = self.name_index(name);
            self.emit(Instr::LoadGlobal(idx));
        }
        Ok(())
    }

    fn store_name(&mut self, name: &str) -> C<()> {
        if let Some((_, slot)) = self.overrides.iter().rev().find(|(n, _)| n == name) {
            let slot = *slot;
            self.emit(Instr::StoreLocal(slot));
            return Ok(());
        }
        if self.is_module {
            let idx = self.name_index(name);
            self.emit(Instr::StoreGlobal(idx));
        } else {
            let slot = self.local_slot(name);
            self.emit(Instr::StoreLocal(slot));
        }
        Ok(())
    }

    // --------------------------------------------------- statements ---

    fn compile_body(&mut self, stmts: &[Stmt]) -> C<()> {
        for s in stmts {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> C<()> {
        self.line = s.line;
        self.emit(Instr::Stmt);
        match &s.kind {
            StmtKind::Expr(e) => {
                self.expr(e)?;
                self.emit(Instr::Pop);
            }
            StmtKind::Assign { targets, value } => {
                self.expr(value)?;
                for (i, t) in targets.iter().enumerate() {
                    if i.wrapping_add(1) < targets.len() {
                        self.emit(Instr::Dup);
                    }
                    self.store_target(t)?;
                }
            }
            StmtKind::AugAssign { target, op, value } => match target {
                Target::Name(n) => {
                    self.load_name(n)?;
                    self.expr(value)?;
                    self.emit(Instr::BinOp(*op));
                    self.store_name(n)?;
                }
                Target::Subscript(obj, idx) => {
                    self.expr(obj)?;
                    self.expr(idx)?;
                    self.emit(Instr::Dup2);
                    self.emit(Instr::Subscript);
                    self.expr(value)?;
                    self.emit(Instr::BinOp(*op));
                    self.emit(Instr::Rot3);
                    self.emit(Instr::StoreSubscript);
                }
                Target::Attr(obj, name) => {
                    self.expr(obj)?;
                    self.emit(Instr::Dup);
                    let idx = self.name_index(name);
                    self.emit(Instr::LoadAttr(idx));
                    self.expr(value)?;
                    self.emit(Instr::BinOp(*op));
                    self.emit(Instr::Swap);
                    self.emit(Instr::StoreAttr(idx));
                }
                Target::Tuple(..) => {
                    return Err(self.err("augmented assignment needs a single target"));
                }
            },
            StmtKind::If { cond, body, orelse } => {
                self.expr(cond)?;
                let j = self.emit(Instr::JumpIfFalsePop(0));
                self.compile_body(body)?;
                if orelse.is_empty() {
                    let end = self.here();
                    self.patch(j, end);
                } else {
                    let jend = self.emit(Instr::Jump(0));
                    let els = self.here();
                    self.patch(j, els);
                    self.compile_body(orelse)?;
                    let end = self.here();
                    self.patch(jend, end);
                }
            }
            StmtKind::While { cond, body, orelse } => {
                let start = self.here();
                self.expr(cond)?;
                let jexit = self.emit(Instr::JumpIfFalsePop(0));
                self.loops.push(LoopCtx {
                    start,
                    breaks: vec![],
                    blocks_at_entry: self.blocks.len(),
                });
                self.compile_body(body)?;
                self.emit(Instr::Jump(start));
                let lp = self.loops.pop().ok_or_else(|| self.err("loop stack"))?;
                let exit = self.here();
                self.patch(jexit, exit);
                self.compile_body(orelse)?;
                let end = self.here();
                for b in lp.breaks {
                    self.patch(b, end);
                }
            }
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => {
                self.expr(iter)?;
                self.emit(Instr::GetIter);
                let start = self.here();
                let jexit = self.emit(Instr::ForIter(0));
                self.store_target(target)?;
                self.loops.push(LoopCtx {
                    start,
                    breaks: vec![],
                    blocks_at_entry: self.blocks.len(),
                });
                self.compile_body(body)?;
                self.emit(Instr::Jump(start));
                let lp = self.loops.pop().ok_or_else(|| self.err("loop stack"))?;
                let exit = self.here();
                self.patch(jexit, exit);
                self.compile_body(orelse)?;
                let end = self.here();
                for b in lp.breaks {
                    self.patch(b, end);
                }
            }
            StmtKind::Break => self.compile_break()?,
            StmtKind::Continue => self.compile_continue()?,
            StmtKind::Pass => {}
            StmtKind::Def { name, params, body } => {
                self.compile_function(name, params, body)?;
                self.store_name(name)?;
            }
            StmtKind::Class { name, base, body } => {
                if let Some(b) = base {
                    self.expr(b)?;
                }
                // The body is its own scope for the names it assigns
                // (`syntax.md`, Names and scope); everything else it reads
                // is a global. Methods are ordinary `def`s compiled inside
                // it, which see globals, never the body's names.
                let mut child = Ctx::new(self.file, name, false);
                child.line = self.line;
                child.globals_hint = self.globals_hint.clone();
                for n in assigned_names(body) {
                    child.local_set.insert(n);
                }
                child.compile_body(body)?;
                child.emit(Instr::ReturnClass);
                let code = child.finish(0, None, None);
                self.children.push(Rc::new(code));
                let idx = self.children.len().wrapping_sub(1);
                self.emit(Instr::BuildClass {
                    child: idx,
                    has_base: base.is_some(),
                });
                self.store_name(name)?;
            }
            StmtKind::Return(value) => {
                match value {
                    Some(e) => self.expr(e)?,
                    None => {
                        let c = self.const_index(Value::None);
                        self.emit(Instr::Const(c));
                    }
                }
                self.unwind_blocks(0)?;
                self.emit(Instr::Return);
            }
            StmtKind::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => self.compile_try(body, handlers, orelse, finalbody)?,
            StmtKind::Raise(value) => match value {
                Some(e) => {
                    self.expr(e)?;
                    self.emit(Instr::Raise(true));
                }
                None => {
                    self.emit(Instr::Raise(false));
                }
            },
        }
        Ok(())
    }

    /// Leave every block opened since `keep` blocks, inlining `finally`
    /// bodies, for `return`, `break` and `continue`.
    fn unwind_blocks(&mut self, keep: usize) -> C<()> {
        let mut i = self.blocks.len();
        while i > keep {
            i = i.wrapping_sub(1);
            self.emit(Instr::PopBlock);
            if let BlockCtx::Finally(body) = &self.blocks[i] {
                let body = body.clone();
                let saved: Vec<BlockCtx> = self.blocks.drain(i..).collect();
                self.compile_body(&body)?;
                self.blocks.extend(saved);
            }
        }
        Ok(())
    }

    fn compile_break(&mut self) -> C<()> {
        let Some(lp) = self.loops.last() else {
            return Err(self.err("`break` outside a loop"));
        };
        let keep = lp.blocks_at_entry;
        let is_for = matches!(self.code.get(lp.start), Some(Instr::ForIter(_)));
        self.unwind_blocks(keep)?;
        if is_for {
            self.emit(Instr::Pop);
        }
        let j = self.emit(Instr::Jump(0));
        if let Some(lp) = self.loops.last_mut() {
            lp.breaks.push(j);
        }
        Ok(())
    }

    fn compile_continue(&mut self) -> C<()> {
        let Some(lp) = self.loops.last() else {
            return Err(self.err("`continue` outside a loop"));
        };
        let (keep, start) = (lp.blocks_at_entry, lp.start);
        self.unwind_blocks(keep)?;
        self.emit(Instr::Jump(start));
        Ok(())
    }

    fn compile_try(
        &mut self,
        body: &[Stmt],
        handlers: &[Handler],
        orelse: &[Stmt],
        finalbody: &[Stmt],
    ) -> C<()> {
        if !finalbody.is_empty() {
            let setup = self.emit(Instr::SetupFinally(0));
            self.blocks.push(BlockCtx::Finally(finalbody.to_vec()));
            if handlers.is_empty() {
                self.compile_body(body)?;
            } else {
                self.compile_try(body, handlers, orelse, &[])?;
            }
            self.blocks.pop();
            self.emit(Instr::PopBlock);
            // Normal path: run the finally body, skip the exceptional copy.
            self.compile_body(finalbody)?;
            let jafter = self.emit(Instr::Jump(0));
            let target = self.here();
            self.patch(setup, target);
            self.compile_body(finalbody)?;
            self.emit(Instr::EndFinally);
            let after = self.here();
            self.patch(jafter, after);
            return Ok(());
        }
        let setup = self.emit(Instr::SetupExcept(0));
        self.blocks.push(BlockCtx::Except);
        self.compile_body(body)?;
        self.blocks.pop();
        self.emit(Instr::PopBlock);
        self.compile_body(orelse)?;
        let jend = self.emit(Instr::Jump(0));
        let handler_start = self.here();
        self.patch(setup, handler_start);
        // The exception is on the stack.
        let mut ends = vec![jend];
        for h in handlers {
            self.line = h.line;
            let next = match &h.types {
                Some(t) => {
                    self.emit(Instr::Dup);
                    self.expr(t)?;
                    self.emit(Instr::ExcMatch);
                    Some(self.emit(Instr::JumpIfFalsePop(0)))
                }
                None => None,
            };
            if let Some(name) = &h.name {
                self.emit(Instr::Dup);
                self.store_name(name)?;
            }
            self.emit(Instr::Pop);
            self.blocks.push(BlockCtx::Except);
            self.compile_body(&h.body)?;
            self.blocks.pop();
            self.emit(Instr::PopExcept);
            ends.push(self.emit(Instr::Jump(0)));
            if let Some(n) = next {
                let here = self.here();
                self.patch(n, here);
            }
        }
        // No handler matched: re-raise the exception on the stack.
        self.emit(Instr::Raise(true));
        let end = self.here();
        for e in ends {
            self.patch(e, end);
        }
        Ok(())
    }

    fn compile_function(&mut self, name: &str, params: &Params, body: &[Stmt]) -> C<()> {
        // Defaults are evaluated here, in the enclosing scope, once.
        let mut ndefaults = 0usize;
        for d in params.defaults.iter().flatten() {
            self.expr(d)?;
            ndefaults = ndefaults.wrapping_add(1);
        }
        let mut child = Ctx::new(self.file, name, false);
        child.line = self.line;
        child.globals_hint = self.globals_hint.clone();
        for p in &params.names {
            child.local_slot(p);
            child.local_set.insert(p.clone());
        }
        let star = params.star.as_ref().map(|s| {
            child.local_set.insert(s.clone());
            child.local_slot(s)
        });
        let dstar = params.dstar.as_ref().map(|s| {
            child.local_set.insert(s.clone());
            child.local_slot(s)
        });
        for n in assigned_names(body) {
            child.local_set.insert(n);
        }
        child.compile_body(body)?;
        let c = child.const_index(Value::None);
        child.emit(Instr::Const(c));
        child.emit(Instr::Return);
        let code = child.finish(params.names.len(), star, dstar);
        self.children.push(Rc::new(code));
        let idx = self.children.len().wrapping_sub(1);
        self.emit(Instr::MakeFunction {
            child: idx,
            ndefaults,
        });
        Ok(())
    }

    fn store_target(&mut self, t: &Target) -> C<()> {
        match t {
            Target::Name(n) => self.store_name(n),
            Target::Subscript(obj, idx) => {
                self.expr(obj)?;
                self.expr(idx)?;
                self.emit(Instr::StoreSubscript);
                Ok(())
            }
            Target::Attr(obj, name) => {
                self.expr(obj)?;
                let idx = self.name_index(name);
                self.emit(Instr::StoreAttr(idx));
                Ok(())
            }
            Target::Tuple(targets, star) => {
                self.emit(Instr::Unpack {
                    n: targets.len(),
                    star: *star,
                });
                for t in targets {
                    self.store_target(t)?;
                }
                Ok(())
            }
        }
    }

    // -------------------------------------------------- expressions ---

    fn expr(&mut self, e: &Expr) -> C<()> {
        let saved = self.line;
        self.line = e.line;
        let r = self.expr_inner(e);
        self.line = saved;
        r
    }

    fn expr_inner(&mut self, e: &Expr) -> C<()> {
        match &e.kind {
            ExprKind::Num(n) => {
                let c = self.const_index(Value::Num(*n));
                self.emit(Instr::Const(c));
            }
            ExprKind::Str(s) => {
                let c = self.const_index(Value::str(s));
                self.emit(Instr::Const(c));
            }
            ExprKind::Bool(b) => {
                let c = self.const_index(Value::Bool(*b));
                self.emit(Instr::Const(c));
            }
            ExprKind::None => {
                let c = self.const_index(Value::None);
                self.emit(Instr::Const(c));
            }
            ExprKind::Name(n) => self.load_name(n)?,
            ExprKind::BinOp(l, op, r) => {
                self.expr(l)?;
                self.expr(r)?;
                self.emit(Instr::BinOp(*op));
            }
            ExprKind::UnaryOp(op, x) => {
                self.expr(x)?;
                self.emit(Instr::UnaryOp(*op));
            }
            ExprKind::BoolOp {
                is_and,
                left,
                right,
            } => {
                self.expr(left)?;
                let j = if *is_and {
                    self.emit(Instr::JumpIfFalseKeep(0))
                } else {
                    self.emit(Instr::JumpIfTrueKeep(0))
                };
                self.emit(Instr::Pop);
                self.expr(right)?;
                let end = self.here();
                self.patch(j, end);
            }
            ExprKind::Compare { first, rest } => {
                self.expr(first)?;
                if rest.len() == 1 {
                    let (op, r) = &rest[0];
                    self.expr(r)?;
                    self.emit(Instr::Compare(*op));
                } else {
                    self.compile_chain(rest)?;
                }
            }
            ExprKind::IfExp { cond, then, orelse } => {
                self.expr(cond)?;
                let j = self.emit(Instr::JumpIfFalsePop(0));
                self.expr(then)?;
                let jend = self.emit(Instr::Jump(0));
                let els = self.here();
                self.patch(j, els);
                self.expr(orelse)?;
                let end = self.here();
                self.patch(jend, end);
            }
            ExprKind::Call { func, args } => {
                self.expr(func)?;
                for a in &args.positional {
                    if matches!(a.kind, ExprKind::Starred(_)) {
                        return Err(self.err(
                            "a starred argument is written `*xs`, once, after the positionals",
                        ));
                    }
                    self.expr(a)?;
                }
                if let Some(s) = &args.star {
                    self.expr(s)?;
                }
                for (_, v) in &args.keywords {
                    self.expr(v)?;
                }
                if let Some(d) = &args.dstar {
                    self.expr(d)?;
                }
                let kwnames = if args.keywords.is_empty() {
                    usize::MAX
                } else {
                    let names: Vec<Value> =
                        args.keywords.iter().map(|(k, _)| Value::str(k)).collect();
                    self.consts.push(Value::tuple(names));
                    self.consts.len().wrapping_sub(1)
                };
                self.emit(Instr::Call {
                    argc: args.positional.len(),
                    kwnames,
                    star: args.star.is_some(),
                    dstar: args.dstar.is_some(),
                });
            }
            ExprKind::Attr(obj, name) => {
                self.expr(obj)?;
                let idx = self.name_index(name);
                self.emit(Instr::LoadAttr(idx));
            }
            ExprKind::Subscript(obj, idx) => {
                self.expr(obj)?;
                self.expr(idx)?;
                self.emit(Instr::Subscript);
            }
            ExprKind::Slice { lo, hi, step } => {
                for part in [lo, hi, step] {
                    match part {
                        Some(e) => self.expr(e)?,
                        None => {
                            let c = self.const_index(Value::None);
                            self.emit(Instr::Const(c));
                        }
                    }
                }
                self.emit(Instr::BuildSlice);
            }
            ExprKind::List(items) => self.display(items, Instr::BuildList(0))?,
            ExprKind::Tuple(items) => self.display(items, Instr::BuildTuple(0))?,
            ExprKind::Set(items) => {
                for i in items {
                    if matches!(i.kind, ExprKind::Starred(_)) {
                        return Err(self.err("a starred element is not allowed in a set display"));
                    }
                    self.expr(i)?;
                }
                self.emit(Instr::BuildSet(items.len()));
            }
            ExprKind::Dict(items) => {
                for (k, v) in items {
                    self.expr(k)?;
                    self.expr(v)?;
                }
                self.emit(Instr::BuildDict(items.len()));
            }
            ExprKind::ListComp { elt, gens } => {
                let acc = self.hidden_slot("list");
                self.emit(Instr::BuildList(0));
                self.emit(Instr::StoreLocal(acc));
                self.comprehension(gens, 0, &|c: &mut Ctx| {
                    c.expr(elt)?;
                    c.emit(Instr::CompAppend(acc));
                    Ok(())
                })?;
                self.emit(Instr::LoadLocal(acc));
            }
            ExprKind::SetComp { elt, gens } => {
                let acc = self.hidden_slot("set");
                self.emit(Instr::BuildSet(0));
                self.emit(Instr::StoreLocal(acc));
                self.comprehension(gens, 0, &|c: &mut Ctx| {
                    c.expr(elt)?;
                    c.emit(Instr::CompAdd(acc));
                    Ok(())
                })?;
                self.emit(Instr::LoadLocal(acc));
            }
            ExprKind::DictComp { key, value, gens } => {
                let acc = self.hidden_slot("dict");
                self.emit(Instr::BuildDict(0));
                self.emit(Instr::StoreLocal(acc));
                self.comprehension(gens, 0, &|c: &mut Ctx| {
                    c.expr(key)?;
                    c.expr(value)?;
                    c.emit(Instr::CompSet(acc));
                    Ok(())
                })?;
                self.emit(Instr::LoadLocal(acc));
            }
            ExprKind::Lambda { params, body } => self.compile_lambda(params, body)?,
            ExprKind::FString(parts) => {
                let mut n = 0usize;
                for p in parts {
                    match p {
                        FPart::Lit(s) => {
                            let c = self.const_index(Value::str(s));
                            self.emit(Instr::Const(c));
                        }
                        FPart::Expr { expr, spec } => {
                            self.expr(expr)?;
                            let spec_idx = match spec {
                                Some(s) => self.const_index(Value::str(s)),
                                None => usize::MAX,
                            };
                            self.emit(Instr::FormatValue(spec_idx));
                        }
                    }
                    n = n.wrapping_add(1);
                }
                self.emit(Instr::BuildString(n));
            }
            ExprKind::Starred(_) => {
                return Err(self.err("a starred expression is not allowed here"));
            }
        }
        Ok(())
    }

    /// `a < b <= c` with `a` already on the stack: each further operand is
    /// evaluated once and kept in a hidden slot for the next comparison.
    fn compile_chain(&mut self, rest: &[(CmpOp, Expr)]) -> C<()> {
        let mut fails = Vec::new();
        for (i, (op, r)) in rest.iter().enumerate() {
            let last = i.wrapping_add(1) == rest.len();
            self.expr(r)?;
            if !last {
                let slot = self.hidden_slot("cmp");
                self.emit(Instr::Dup);
                self.emit(Instr::StoreLocal(slot));
                self.emit(Instr::Compare(*op));
                fails.push(self.emit(Instr::JumpIfFalseKeep(0)));
                self.emit(Instr::Pop);
                self.emit(Instr::LoadLocal(slot));
            } else {
                self.emit(Instr::Compare(*op));
            }
        }
        let end = self.here();
        for f in fails {
            self.patch(f, end);
        }
        Ok(())
    }

    fn compile_lambda(&mut self, params: &Params, body: &Expr) -> C<()> {
        let mut ndefaults = 0usize;
        for d in params.defaults.iter().flatten() {
            self.expr(d)?;
            ndefaults = ndefaults.wrapping_add(1);
        }
        let mut child = Ctx::new(self.file, "<lambda>", false);
        child.line = self.line;
        for p in &params.names {
            child.local_slot(p);
            child.local_set.insert(p.clone());
        }
        let star = params.star.as_ref().map(|s| {
            child.local_set.insert(s.clone());
            child.local_slot(s)
        });
        let dstar = params.dstar.as_ref().map(|s| {
            child.local_set.insert(s.clone());
            child.local_slot(s)
        });
        if !self.is_module {
            for n in &self.local_set {
                if !child.local_set.contains(n) {
                    child.forbidden.insert(n.clone());
                }
            }
        }
        for (n, _) in &self.overrides {
            if !child.local_set.contains(n) {
                child.forbidden.insert(n.clone());
            }
        }
        child.expr(body)?;
        child.emit(Instr::Return);
        let code = child.finish(params.names.len(), star, dstar);
        self.children.push(Rc::new(code));
        let idx = self.children.len().wrapping_sub(1);
        self.emit(Instr::MakeFunction {
            child: idx,
            ndefaults,
        });
        Ok(())
    }

    fn display(&mut self, items: &[Expr], build: Instr) -> C<()> {
        let has_star = items.iter().any(|i| matches!(i.kind, ExprKind::Starred(_)));
        if !has_star {
            for i in items {
                self.expr(i)?;
            }
            let n = items.len();
            self.emit(match build {
                Instr::BuildTuple(_) => Instr::BuildTuple(n),
                _ => Instr::BuildList(n),
            });
            return Ok(());
        }
        // Build a list incrementally, extending at each star, then convert.
        self.emit(Instr::BuildList(0));
        for i in items {
            match &i.kind {
                ExprKind::Starred(inner) => {
                    self.expr(inner)?;
                    self.emit(Instr::ListExtend);
                }
                _ => {
                    self.expr(i)?;
                    self.emit(Instr::CompAppend(usize::MAX));
                }
            }
        }
        if matches!(build, Instr::BuildTuple(_)) {
            // `BuildTuple(MAX)` converts the list on top into a tuple.
            self.emit(Instr::BuildTuple(usize::MAX));
        }
        Ok(())
    }

    fn comprehension(
        &mut self,
        gens: &[Comp],
        i: usize,
        body: &dyn Fn(&mut Ctx) -> C<()>,
    ) -> C<()> {
        let Some(g) = gens.get(i) else {
            return body(self);
        };
        // The outermost iterable is evaluated before any target exists.
        self.expr(&g.iter)?;
        self.emit(Instr::GetIter);
        let start = self.here();
        let jexit = self.emit(Instr::ForIter(0));
        let names = target_names(&g.target);
        let before = self.overrides.len();
        for n in &names {
            let slot = self.hidden_slot(&format!("comp:{n}"));
            self.overrides.push((n.clone(), slot));
        }
        self.store_target(&g.target)?;
        let mut skips = Vec::new();
        for cond in &g.ifs {
            self.expr(cond)?;
            skips.push(self.emit(Instr::JumpIfFalsePop(0)));
        }
        self.comprehension(gens, i.wrapping_add(1), body)?;
        let cont = self.here();
        for s in skips {
            self.patch(s, cont);
        }
        self.emit(Instr::Jump(start));
        self.overrides.truncate(before);
        let exit = self.here();
        self.patch(jexit, exit);
        Ok(())
    }
}

fn target_names(t: &Target) -> Vec<String> {
    match t {
        Target::Name(n) => vec![n.clone()],
        Target::Tuple(ts, _) => ts.iter().flat_map(target_names).collect(),
        _ => vec![],
    }
}

/// Every name assigned anywhere in a body: assignment targets, `for`
/// targets, `def` names, `except … as` names. Comprehension targets are not
/// included, since they are private to the comprehension.
fn assigned_names(stmts: &[Stmt]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    fn walk(stmts: &[Stmt], out: &mut BTreeSet<String>) {
        for s in stmts {
            match &s.kind {
                StmtKind::Assign { targets, .. } => {
                    for t in targets {
                        out.extend(target_names(t));
                    }
                }
                StmtKind::AugAssign { target, .. } => out.extend(target_names(target)),
                StmtKind::For {
                    target,
                    body,
                    orelse,
                    ..
                } => {
                    out.extend(target_names(target));
                    walk(body, out);
                    walk(orelse, out);
                }
                StmtKind::If { body, orelse, .. } | StmtKind::While { body, orelse, .. } => {
                    walk(body, out);
                    walk(orelse, out);
                }
                StmtKind::Def { name, .. } | StmtKind::Class { name, .. } => {
                    out.insert(name.clone());
                }
                StmtKind::Try {
                    body,
                    handlers,
                    orelse,
                    finalbody,
                } => {
                    walk(body, out);
                    for h in handlers {
                        if let Some(n) = &h.name {
                            out.insert(n.clone());
                        }
                        walk(&h.body, out);
                    }
                    walk(orelse, out);
                    walk(finalbody, out);
                }
                _ => {}
            }
        }
    }
    walk(stmts, &mut out);
    out
}

//! The editor's type check (`docs/07`, Q61): a red underline wherever the
//! annotations (Q56) and the builtins' signatures say a value cannot fit.
//! It underlines only what is certainly wrong, so a value whose type it
//! cannot work out passes, and so does a union with any member that fits.
//! It reads the tree as text; nothing here reaches the sim or a peer, and
//! nothing it says stops a deploy.

use crate::hover::{BuiltinDocs, Kind};
use lang::ast::*;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

/// One underline: the chars of its file it covers, and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

/// What the check knows of a value's type. `Any` is "unknown", and fits
/// everything both ways.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ty {
    Any,
    None,
    Bool,
    /// An integral `num`.
    Int,
    Num,
    Str,
    List(Box<Ty>),
    Set(Box<Ty>),
    Dict(Box<Ty>, Box<Ty>),
    Tuple(Vec<Ty>),
    /// `tuple[T, ...]`: any length.
    TupleOf(Box<Ty>),
    Union(Vec<Ty>),
    /// `Machine`, `Tile` or `Sound`.
    Record(&'static str),
    /// An instance of a player's class: its file and its name.
    Instance(String, String),
}

impl Ty {
    /// `A | B | …`, flattened and without repeats; any `Any` makes it all
    /// `Any`, since then anything might be there.
    fn union(items: impl IntoIterator<Item = Ty>) -> Ty {
        let mut out: Vec<Ty> = Vec::new();
        for t in items {
            match t {
                Ty::Any => return Ty::Any,
                Ty::Union(ts) => {
                    for t in ts {
                        if !out.contains(&t) {
                            out.push(t);
                        }
                    }
                }
                t => {
                    if !out.contains(&t) {
                        out.push(t);
                    }
                }
            }
        }
        match out.len() {
            0 => Ty::Any,
            1 => out.pop().unwrap_or(Ty::Any),
            _ => Ty::Union(out),
        }
    }

    fn members(&self) -> Vec<&Ty> {
        match self {
            Ty::Union(ts) => ts.iter().collect(),
            t => vec![t],
        }
    }

    fn numeric(&self) -> bool {
        matches!(self, Ty::Bool | Ty::Int | Ty::Num)
    }

    fn tuple_like(&self) -> bool {
        matches!(self, Ty::Tuple(_) | Ty::TupleOf(_))
    }

    /// As an annotation writes it.
    pub fn show(&self) -> String {
        match self {
            Ty::Any => "unknown".into(),
            Ty::None => "None".into(),
            Ty::Bool => "bool".into(),
            Ty::Int => "int".into(),
            Ty::Num => "num".into(),
            Ty::Str => "str".into(),
            Ty::List(t) => format!("list[{}]", t.show_inner()),
            Ty::Set(t) => format!("set[{}]", t.show_inner()),
            Ty::Dict(k, v) => format!("dict[{}, {}]", k.show_inner(), v.show_inner()),
            Ty::Tuple(ts) => {
                let ts: Vec<String> = ts.iter().map(Ty::show_inner).collect();
                format!("tuple[{}]", ts.join(", "))
            }
            Ty::TupleOf(t) => format!("tuple[{}, ...]", t.show_inner()),
            Ty::Union(ts) => {
                let ts: Vec<String> = ts.iter().map(Ty::show).collect();
                ts.join(" | ")
            }
            Ty::Record(r) => (*r).into(),
            Ty::Instance(_, c) => c.clone(),
        }
    }

    /// Inside brackets, where an unknown element is just left open.
    fn show_inner(&self) -> String {
        match self {
            Ty::Any => "...".into(),
            t => t.show(),
        }
    }
}

/// The element type iterating a value of type `t` gives.
fn elem_of(t: &Ty) -> Ty {
    match t {
        Ty::List(e) | Ty::Set(e) | Ty::TupleOf(e) => (**e).clone(),
        Ty::Tuple(ts) => Ty::union(ts.iter().cloned()),
        Ty::Dict(k, _) => (**k).clone(),
        Ty::Str => Ty::Str,
        Ty::Union(ts) => Ty::union(ts.iter().map(elem_of)),
        _ => Ty::Any,
    }
}

/// A callable's parameters, for checking a call.
#[derive(Debug, Clone)]
struct Sig {
    /// For messages: `go_to`, `Point.norm`.
    name: String,
    /// Name, type (`Any` when unannotated), has a default.
    params: Vec<(String, Ty, bool)>,
    /// `*args`, with its elements' type.
    star: Option<Ty>,
    dstar: bool,
    returns: Ty,
    /// Too few arguments, or a keyword with no parameter, is an error.
    strict: bool,
    /// Too many positional arguments is an error. A game builtin with no
    /// parameters ignores any it is given.
    strict_extra: bool,
    /// A game builtin takes no keyword arguments.
    positional_only: bool,
}

impl Sig {
    fn arity(&self) -> String {
        let max = self.params.len();
        let min = self.params.iter().filter(|p| !p.2).count();
        let n = |k: usize| if k == 1 { "argument" } else { "arguments" };
        if min == max {
            format!("takes {max} {}", n(max))
        } else {
            format!("takes {min} to {max} arguments")
        }
    }
}

/// The game's and the language's builtins, as `docs.toml` signs them.
#[derive(Debug, Clone, Default)]
pub struct Builtins {
    sigs: BTreeMap<String, Sig>,
}

impl Builtins {
    /// Every builtin whose signature is a plain `def` header. One the
    /// language cannot write, such as `log`'s keyword after `*values` or
    /// `range`'s two forms, is not checked at all.
    pub fn new(docs: &BuiltinDocs) -> Builtins {
        let mut sigs = BTreeMap::new();
        for (kind, name, signature) in docs.signatures() {
            if kind == Kind::Exception || name == "log" {
                continue;
            }
            let src = format!("def {signature}:\n    pass\n");
            let Ok(stmts) = lang::parser::parse_file("docs.toml", &src) else {
                continue;
            };
            let Some(Stmt {
                kind: StmtKind::Def {
                    params, returns, ..
                },
                ..
            }) = stmts.first()
            else {
                continue;
            };
            let ty = |a: &Option<Annotation>| {
                a.as_ref()
                    .map_or(Ty::Any, |a| plain_ty(&a.ty).unwrap_or(Ty::Any))
            };
            let game = kind == Kind::Game;
            let ps: Vec<(String, Ty, bool)> = params
                .names
                .iter()
                .zip(&params.annotations)
                .zip(&params.defaults)
                .map(|((n, a), d)| (n.clone(), ty(a), d.is_some()))
                .collect();
            sigs.insert(
                name.to_string(),
                Sig {
                    name: name.to_string(),
                    strict_extra: game && !ps.is_empty(),
                    params: ps,
                    star: params.star.as_ref().map(|_| ty(&params.star_annotation)),
                    dstar: params.dstar.is_some(),
                    returns: ty(returns),
                    strict: game,
                    positional_only: game,
                },
            );
        }
        Builtins { sigs }
    }
}

/// A type expression that names no player class: the builtins' own.
fn plain_ty(t: &TypeExpr) -> Option<Ty> {
    Some(match t {
        TypeExpr::None => Ty::None,
        TypeExpr::Name(n) => builtin_type(n)?,
        TypeExpr::Generic(base, args) => {
            let TypeExpr::Name(b) = &**base else {
                return None;
            };
            generic(
                b,
                args.iter().map(plain_ty).collect::<Option<Vec<_>>>()?,
                args,
            )?
        }
        TypeExpr::Union(ts) => Ty::union(ts.iter().map(plain_ty).collect::<Option<Vec<_>>>()?),
        _ => return None,
    })
}

/// A type name every file knows.
fn builtin_type(n: &str) -> Option<Ty> {
    Some(match n {
        "num" => Ty::Num,
        "int" => Ty::Int,
        "str" => Ty::Str,
        "bool" => Ty::Bool,
        "list" => Ty::List(Box::new(Ty::Any)),
        "set" => Ty::Set(Box::new(Ty::Any)),
        "dict" => Ty::Dict(Box::new(Ty::Any), Box::new(Ty::Any)),
        "tuple" => Ty::TupleOf(Box::new(Ty::Any)),
        "Machine" => Ty::Record("Machine"),
        "Tile" => Ty::Record("Tile"),
        "Sound" => Ty::Record("Sound"),
        _ => return None,
    })
}

/// `list[T]`, `set[T]`, `dict[K, V]`, `tuple[…]` from their resolved
/// arguments, or `None` for any other shape.
fn generic(base: &str, mut args: Vec<Ty>, written: &[TypeExpr]) -> Option<Ty> {
    let ellipsis = matches!(written, [_, TypeExpr::Ellipsis]);
    Some(match (base, args.len()) {
        ("list", 1) => Ty::List(Box::new(args.pop()?)),
        ("set", 1) => Ty::Set(Box::new(args.pop()?)),
        ("dict", 2) => {
            let v = args.pop()?;
            Ty::Dict(Box::new(args.pop()?), Box::new(v))
        }
        ("tuple", 2) if ellipsis => Ty::TupleOf(Box::new(args.swap_remove(0))),
        ("tuple", _) => Ty::Tuple(args),
        _ => return None,
    })
}

/// What a name a file imports stands for.
#[derive(Debug, Clone)]
enum Import {
    Module(String),
    Name(String, String),
}

/// One file of the tree, parsed.
struct File<'a> {
    path: &'a str,
    stmts: &'a [Stmt],
    /// The text, for a file being checked; `None` for one only read for
    /// its signatures (it does not parse as it stands).
    chars: Option<Vec<char>>,
    imports: BTreeMap<String, Import>,
    defs: BTreeMap<String, &'a Stmt>,
    classes: BTreeMap<String, &'a Stmt>,
    scope: Scope<'a>,
}

/// Where a name's value comes from, in one scope. The check is
/// flow-insensitive: a name's type is every source's, joined.
#[derive(Debug)]
enum Src<'a> {
    Expr(&'a Expr),
    /// The elements of iterating this.
    Elem(&'a Expr),
    Ty(Ty),
}

/// The names one scope binds: those an annotation declares, and the
/// sources of the rest. A name bound any way the check does not follow
/// (`+=`, unpacking, `except … as`) has a source of `Ty::Any`.
#[derive(Debug, Default)]
struct Scope<'a> {
    id: usize,
    decl: BTreeMap<String, &'a Annotation>,
    srcs: BTreeMap<String, Vec<Src<'a>>>,
}

impl<'a> Scope<'a> {
    fn binds(&self, n: &str) -> bool {
        self.decl.contains_key(n) || self.srcs.contains_key(n)
    }

    fn add(&mut self, n: &str, s: Src<'a>) {
        self.srcs.entry(n.to_string()).or_default().push(s);
    }

    fn opaque(&mut self, n: &str) {
        self.add(n, Src::Ty(Ty::Any));
    }

    /// Record what `stmts` bind, not looking inside a `def` or `class`
    /// body, which are scopes of their own.
    fn collect(&mut self, stmts: &'a [Stmt]) {
        for s in stmts {
            match &s.kind {
                StmtKind::Assign { targets, value } => {
                    for t in targets {
                        self.target(t, Some(value));
                    }
                }
                StmtKind::AnnAssign {
                    target, annotation, ..
                } => match target {
                    Target::Name(n) => {
                        self.decl.insert(n.clone(), annotation);
                    }
                    t => self.target(t, None),
                },
                StmtKind::AugAssign { target, .. } => self.target(target, None),
                StmtKind::For {
                    target,
                    iter,
                    body,
                    orelse,
                } => {
                    match target {
                        Target::Name(n) => self.add(n, Src::Elem(iter)),
                        t => self.target(t, None),
                    }
                    self.collect(body);
                    self.collect(orelse);
                }
                StmtKind::If { body, orelse, .. } | StmtKind::While { body, orelse, .. } => {
                    self.collect(body);
                    self.collect(orelse);
                }
                StmtKind::Def { name, .. } | StmtKind::Class { name, .. } => self.opaque(name),
                StmtKind::Import { module, alias } => {
                    self.opaque(alias.as_deref().unwrap_or(module));
                }
                StmtKind::FromImport { names, .. } => {
                    for (n, alias) in names {
                        self.opaque(alias.as_deref().unwrap_or(n));
                    }
                }
                StmtKind::Match { arms, .. } => {
                    for a in arms {
                        let mut names = Vec::new();
                        a.pattern.bound_names(&mut names);
                        names.iter().for_each(|n| self.opaque(n));
                        self.collect(&a.body);
                    }
                }
                StmtKind::Try {
                    body,
                    handlers,
                    orelse,
                    finalbody,
                } => {
                    self.collect(body);
                    for h in handlers {
                        if let Some(n) = &h.name {
                            self.opaque(n);
                        }
                        self.collect(&h.body);
                    }
                    self.collect(orelse);
                    self.collect(finalbody);
                }
                _ => {}
            }
        }
    }

    fn target(&mut self, t: &'a Target, value: Option<&'a Expr>) {
        match t {
            Target::Name(n) => match value {
                Some(v) => self.add(n, Src::Expr(v)),
                None => self.opaque(n),
            },
            Target::Tuple(items, star) => {
                // `a, b = x, y` pairs up; any other unpacking is not
                // followed.
                let pairs = match value.map(|v| &v.kind) {
                    Some(ExprKind::Tuple(vs) | ExprKind::List(vs))
                        if vs.len() == items.len()
                            && star.is_none()
                            && !vs.iter().any(|v| matches!(v.kind, ExprKind::Starred(_))) =>
                    {
                        Some(vs)
                    }
                    _ => None,
                };
                for (i, item) in items.iter().enumerate() {
                    self.target(item, pairs.and_then(|vs| vs.get(i)));
                }
            }
            Target::Subscript(..) | Target::Attr(..) => {}
        }
    }
}

/// Where an expression is being read: its file, the function it is in,
/// the class that function is a method of, and any comprehension or
/// `lambda` names in force, innermost last.
#[derive(Clone)]
struct Cx<'s, 'a> {
    file: &'s File<'a>,
    local: Option<&'s Scope<'a>>,
    comps: Vec<BTreeMap<String, Ty>>,
}

impl<'s, 'a> Cx<'s, 'a> {
    fn with(&self, names: BTreeMap<String, Ty>) -> Cx<'s, 'a> {
        let mut c = self.clone();
        c.comps.push(names);
        c
    }
}

/// The whole tree, ready to check.
struct Tree<'a> {
    files: Vec<File<'a>>,
    builtins: &'a Builtins,
    /// Finished name types by scope and name; `None` while one is being
    /// worked out, so a cycle reads as unknown.
    memo: RefCell<BTreeMap<(usize, String), Option<Ty>>>,
    out: RefCell<Vec<Diag>>,
}

/// The diagnostics for every file of `files` that parses as it stands,
/// by path. `files` gives each file's text and its last good parse, and
/// whether that parse is of the text as it stands.
pub fn check(
    files: &BTreeMap<String, (String, Vec<Stmt>, bool)>,
    builtins: &Builtins,
) -> BTreeMap<String, Vec<Diag>> {
    let ids = Cell::new(0usize);
    let next_id = || {
        ids.set(ids.get() + 1);
        ids.get()
    };
    let mut parsed = Vec::new();
    for (path, (text, stmts, current)) in files {
        let mut f = File {
            path,
            stmts,
            chars: current.then(|| text.chars().collect()),
            imports: BTreeMap::new(),
            defs: BTreeMap::new(),
            classes: BTreeMap::new(),
            scope: Scope {
                id: next_id(),
                ..Scope::default()
            },
        };
        for s in stmts {
            match &s.kind {
                StmtKind::Def { name, .. } => {
                    f.defs.insert(name.clone(), s);
                }
                StmtKind::Class { name, .. } => {
                    f.classes.insert(name.clone(), s);
                }
                StmtKind::Import { module, alias } => {
                    let local = alias.clone().unwrap_or_else(|| module.clone());
                    f.imports.insert(local, Import::Module(module.clone()));
                }
                StmtKind::FromImport { module, names } => {
                    for (n, alias) in names {
                        let local = alias.clone().unwrap_or_else(|| n.clone());
                        f.imports
                            .insert(local, Import::Name(module.clone(), n.clone()));
                    }
                }
                _ => {}
            }
        }
        f.scope.collect(stmts);
        parsed.push(f);
    }
    let tree = Tree {
        files: parsed,
        builtins,
        memo: RefCell::new(BTreeMap::new()),
        out: RefCell::new(Vec::new()),
    };
    let mut result = BTreeMap::new();
    for f in tree.files.iter().filter(|f| f.chars.is_some()) {
        tree.out.borrow_mut().clear();
        tree.file(f, &next_id);
        let mut diags = std::mem::take(&mut *tree.out.borrow_mut());
        diags.sort_by_key(|d| (d.start, d.end));
        diags.dedup();
        result.insert(f.path.to_string(), diags);
    }
    result
}

/// What a callee is.
enum Callee<'s, 'a> {
    Def(&'s File<'a>, &'a Stmt),
    Class(&'s File<'a>, &'a Stmt),
}

impl<'a> Tree<'a> {
    fn diag(&self, span: Span, message: String) {
        self.out.borrow_mut().push(Diag {
            start: span.start as usize,
            end: span.end.max(span.start + 1) as usize,
            message,
        });
    }

    /// The file a module name is: one outside `robots/`, by its bare name.
    fn module(&self, name: &str) -> Option<&File<'a>> {
        self.files.iter().find(|f| {
            f.path
                .rsplit('/')
                .next()
                .and_then(|s| s.strip_suffix(".py"))
                == Some(name)
                && sim::script::robot_of(f.path).is_none()
        })
    }

    /// A def or class a file defines or imports under `name`.
    fn member<'s>(&'s self, f: &'s File<'a>, name: &str, depth: u32) -> Option<Callee<'s, 'a>> {
        if depth > 8 {
            return None;
        }
        if let Some(d) = f.defs.get(name) {
            return Some(Callee::Def(f, d));
        }
        if let Some(c) = f.classes.get(name) {
            return Some(Callee::Class(f, c));
        }
        match f.imports.get(name)? {
            Import::Name(m, x) => self.member(self.module(m)?, x, depth + 1),
            Import::Module(_) => None,
        }
    }

    // ── annotations ─────────────────────────────────────────────────────

    /// An annotation as a type, read in `f`; with `report`, each name
    /// that is no type underlined.
    fn ann(&self, f: &File<'a>, a: &Annotation, report: bool) -> Ty {
        let mut unknown = Vec::new();
        let t = self.type_expr(f, &a.ty, &mut unknown);
        if report {
            for (name, hint) in unknown {
                let span = f
                    .chars
                    .as_ref()
                    .and_then(|c| find_word(c, a.span, &name))
                    .unwrap_or(a.span);
                self.diag(span, hint);
            }
        }
        t
    }

    fn type_expr(&self, f: &File<'a>, t: &TypeExpr, unknown: &mut Vec<(String, String)>) -> Ty {
        match t {
            TypeExpr::None => Ty::None,
            TypeExpr::Ellipsis => Ty::Any,
            TypeExpr::Name(n) => {
                if let Some(t) = builtin_type(n) {
                    return t;
                }
                if let Some(Callee::Class(cf, _)) = self.member(f, n, 0) {
                    return Ty::Instance(cf.path.to_string(), n.clone());
                }
                unknown.push((n.clone(), unknown_type(n)));
                Ty::Any
            }
            TypeExpr::Attr(m, c) => {
                let class = match f.imports.get(m) {
                    Some(Import::Module(m)) => self.module(m).and_then(|mf| {
                        mf.classes
                            .contains_key(c)
                            .then(|| Ty::Instance(mf.path.to_string(), c.clone()))
                    }),
                    _ => None,
                };
                class.unwrap_or_else(|| {
                    unknown.push((c.clone(), format!("`{m}.{c}` is not a class")));
                    Ty::Any
                })
            }
            TypeExpr::Generic(base, args) => {
                let tys: Vec<Ty> = args.iter().map(|a| self.type_expr(f, a, unknown)).collect();
                let TypeExpr::Name(b) = &**base else {
                    return Ty::Any;
                };
                match generic(b, tys, args) {
                    Some(t) => t,
                    None => {
                        let hint = match b.as_str() {
                            "list" | "set" => format!("`{b}[T]` takes one type"),
                            "dict" => "`dict[K, V]` takes two types".to_string(),
                            _ if builtin_type(b).is_some() => format!("`{b}` takes no types"),
                            _ => unknown_type(b),
                        };
                        unknown.push((b.clone(), hint));
                        Ty::Any
                    }
                }
            }
            TypeExpr::Union(ts) => Ty::union(ts.iter().map(|t| self.type_expr(f, t, unknown))),
            TypeExpr::Str(s) => {
                // A string holding a type: read what it holds, quietly.
                let src = format!("x: {s}\n");
                match lang::parser::parse_file("annotation", &src).as_deref() {
                    Ok(
                        [
                            Stmt {
                                kind: StmtKind::AnnAssign { annotation, .. },
                                ..
                            },
                        ],
                    ) => self.type_expr(f, &annotation.ty, &mut Vec::new()),
                    _ => Ty::Any,
                }
            }
        }
    }

    // ── signatures ──────────────────────────────────────────────────────

    /// A def's signature, its types read in its own file. `drop_self`
    /// for a method called on an instance.
    fn def_sig(&self, f: &File<'a>, def: &Stmt, label: String, drop_self: bool) -> Option<Sig> {
        let StmtKind::Def {
            params, returns, ..
        } = &def.kind
        else {
            return None;
        };
        let ty = |a: &Option<Annotation>| a.as_ref().map_or(Ty::Any, |a| self.ann(f, a, false));
        let mut ps: Vec<(String, Ty, bool)> = params
            .names
            .iter()
            .zip(&params.annotations)
            .zip(&params.defaults)
            .map(|((n, a), d)| (n.clone(), ty(a), d.is_some()))
            .collect();
        if drop_self {
            if ps.is_empty() {
                return None;
            }
            ps.remove(0);
        }
        Some(Sig {
            name: label,
            params: ps,
            star: params.star.as_ref().map(|_| ty(&params.star_annotation)),
            dstar: params.dstar.is_some(),
            returns: ty(returns),
            strict: true,
            strict_extra: true,
            positional_only: false,
        })
    }

    /// A method of a class or of a base, and the file defining it; `None`
    /// if the chain passes through a class the check cannot see.
    fn method<'s>(
        &'s self,
        f: &'s File<'a>,
        class: &str,
        name: &str,
        depth: u32,
    ) -> Option<(&'s File<'a>, &'a Stmt)> {
        let Some(Callee::Class(cf, c)) = self.member(f, class, depth) else {
            return None;
        };
        let StmtKind::Class { base, body, .. } = &c.kind else {
            return None;
        };
        for s in body {
            if let StmtKind::Def { name: n, .. } = &s.kind
                && n == name
            {
                return Some((cf, s));
            }
        }
        match base.as_ref().map(|b| &b.kind) {
            Some(ExprKind::Name(b)) => self.method(cf, b, name, depth + 1),
            _ => None,
        }
    }

    /// Whether a class's bases are all classes of the tree, so a method it
    /// lacks is truly missing.
    fn chain_known(&self, f: &File<'a>, class: &str, depth: u32) -> bool {
        if depth > 8 {
            return false;
        }
        let Some(Callee::Class(cf, c)) = self.member(f, class, depth) else {
            return false;
        };
        let StmtKind::Class { base, .. } = &c.kind else {
            return false;
        };
        match base.as_ref().map(|b| &b.kind) {
            None => true,
            Some(ExprKind::Name(b)) => self.chain_known(cf, b, depth + 1),
            Some(_) => false,
        }
    }

    /// The signature a call's callee has, if the check knows it.
    fn callee_sig(&self, cx: &Cx<'_, 'a>, func: &Expr) -> Option<Sig> {
        match &func.kind {
            ExprKind::Name(n) => {
                if self.shadowed(cx, n) {
                    return None;
                }
                match self.member(cx.file, n, 0) {
                    Some(Callee::Def(df, d)) => self.def_sig(df, d, n.clone(), false),
                    Some(Callee::Class(cf, _)) => self.class_sig(cf, n),
                    None if cx.file.scope.binds(n) => None,
                    None => self.builtins.sigs.get(n).cloned(),
                }
            }
            ExprKind::Attr(owner, name) => {
                if let ExprKind::Name(m) = &owner.kind
                    && !self.shadowed(cx, m)
                    && let Some(Import::Module(m)) = cx.file.imports.get(m)
                {
                    let mf = self.module(m)?;
                    return match self.member(mf, name, 0)? {
                        Callee::Def(df, d) => self.def_sig(df, d, format!("{m}.{name}"), false),
                        Callee::Class(cf, _) => self.class_sig(cf, name),
                    };
                }
                let Ty::Instance(path, class) = self.synth(cx, owner) else {
                    return None;
                };
                let cf = self.files.iter().find(|f| f.path == path)?;
                let (mf, m) = self.method(cf, &class, name, 0)?;
                self.def_sig(mf, m, format!("{class}.{name}"), true)
            }
            _ => None,
        }
    }

    /// What calling a class takes: its `__init__`'s parameters, or none if
    /// no class in its chain has one.
    fn class_sig(&self, f: &File<'a>, class: &str) -> Option<Sig> {
        let returns = Ty::Instance(f.path.to_string(), class.to_string());
        if let Some((mf, init)) = self.method(f, class, "__init__", 0) {
            let mut sig = self.def_sig(mf, init, class.to_string(), true)?;
            sig.returns = returns;
            return Some(sig);
        }
        self.chain_known(f, class, 0).then(|| Sig {
            name: class.to_string(),
            params: Vec::new(),
            star: None,
            dstar: false,
            returns,
            strict: true,
            strict_extra: true,
            positional_only: false,
        })
    }

    /// Whether `n` is bound by the function or a comprehension, which hides
    /// the module's `def` or a builtin of that name.
    fn shadowed(&self, cx: &Cx<'_, 'a>, n: &str) -> bool {
        cx.comps.iter().any(|c| c.contains_key(n)) || cx.local.is_some_and(|s| s.binds(n))
    }

    // ── types of expressions ────────────────────────────────────────────

    fn name_ty(&self, cx: &Cx<'_, 'a>, n: &str) -> Ty {
        if let Some(t) = cx.comps.iter().rev().find_map(|c| c.get(n)) {
            return t.clone();
        }
        if let Some(s) = cx.local
            && s.binds(n)
        {
            return self.scope_ty(cx, s, n);
        }
        if cx.file.scope.binds(n) {
            let module = Cx {
                file: cx.file,
                local: None,
                comps: Vec::new(),
            };
            return self.scope_ty(&module, &cx.file.scope, n);
        }
        Ty::Any
    }

    fn scope_ty(&self, cx: &Cx<'_, 'a>, s: &Scope<'a>, n: &str) -> Ty {
        if let Some(a) = s.decl.get(n) {
            return self.ann(cx.file, a, false);
        }
        let key = (s.id, n.to_string());
        if let Some(done) = self.memo.borrow().get(&key) {
            return done.clone().unwrap_or(Ty::Any);
        }
        self.memo.borrow_mut().insert(key.clone(), None);
        let inner = Cx {
            file: cx.file,
            local: cx.local,
            comps: Vec::new(),
        };
        let t = Ty::union(s.srcs.get(n).into_iter().flatten().map(|src| match src {
            Src::Expr(e) => self.synth(&inner, e),
            Src::Elem(e) => elem_of(&self.synth(&inner, e)),
            Src::Ty(t) => t.clone(),
        }));
        self.memo.borrow_mut().insert(key, Some(t.clone()));
        t
    }

    /// The names a comprehension's `for`s bind, with their element types.
    fn comp_names<'s>(&self, cx: &Cx<'s, 'a>, gens: &[Comp]) -> Cx<'s, 'a> {
        let mut c = cx.clone();
        for g in gens {
            let elem = elem_of(&self.synth(&c, &g.iter));
            let mut names = BTreeMap::new();
            bind(&g.target, elem, &mut names);
            c = c.with(names);
        }
        c
    }

    fn synth(&self, cx: &Cx<'_, 'a>, e: &Expr) -> Ty {
        match &e.kind {
            ExprKind::Num(n) => {
                if n.is_integral() {
                    Ty::Int
                } else {
                    Ty::Num
                }
            }
            ExprKind::Str(_) | ExprKind::FString(_) => Ty::Str,
            ExprKind::Bool(_) => Ty::Bool,
            ExprKind::None => Ty::None,
            ExprKind::Name(n) => self.name_ty(cx, n),
            ExprKind::BinOp(l, op, r) => binop_ty(*op, &self.synth(cx, l), &self.synth(cx, r)),
            ExprKind::UnaryOp(UnaryOp::Not, _) | ExprKind::Compare { .. } => Ty::Bool,
            ExprKind::UnaryOp(_, x) => match self.synth(cx, x) {
                Ty::Bool | Ty::Int => Ty::Int,
                Ty::Num => Ty::Num,
                _ => Ty::Any,
            },
            ExprKind::BoolOp { left, right, .. } => {
                Ty::union([self.synth(cx, left), self.synth(cx, right)])
            }
            ExprKind::IfExp { then, orelse, .. } => {
                Ty::union([self.synth(cx, then), self.synth(cx, orelse)])
            }
            ExprKind::Call { func, .. } => self.callee_sig(cx, func).map_or(Ty::Any, |s| s.returns),
            ExprKind::Subscript(v, idx) => {
                let slice = matches!(idx.kind, ExprKind::Slice { .. });
                match self.synth(cx, v) {
                    Ty::List(t) if slice => Ty::List(t),
                    Ty::List(t) | Ty::TupleOf(t) | Ty::Dict(_, t) if !slice => *t,
                    Ty::TupleOf(t) => Ty::TupleOf(t),
                    Ty::Tuple(ts) if slice => Ty::TupleOf(Box::new(Ty::union(ts))),
                    Ty::Tuple(ts) => match index_of(idx).and_then(|i| pick(&ts, i)) {
                        Some(t) => t,
                        None => Ty::union(ts),
                    },
                    Ty::Str => Ty::Str,
                    _ => Ty::Any,
                }
            }
            ExprKind::List(items) => Ty::List(Box::new(self.elems(cx, items))),
            ExprKind::Set(items) => Ty::Set(Box::new(self.elems(cx, items))),
            ExprKind::Tuple(items) => {
                if items.iter().any(|i| matches!(i.kind, ExprKind::Starred(_))) {
                    Ty::TupleOf(Box::new(Ty::Any))
                } else {
                    Ty::Tuple(items.iter().map(|i| self.synth(cx, i)).collect())
                }
            }
            ExprKind::Dict(pairs) => {
                if pairs.is_empty() {
                    return Ty::Dict(Box::new(Ty::Any), Box::new(Ty::Any));
                }
                let k = Ty::union(pairs.iter().map(|(k, _)| self.synth(cx, k)));
                let v = Ty::union(pairs.iter().map(|(_, v)| self.synth(cx, v)));
                Ty::Dict(Box::new(k), Box::new(v))
            }
            ExprKind::ListComp { elt, gens } => {
                Ty::List(Box::new(self.synth(&self.comp_names(cx, gens), elt)))
            }
            ExprKind::SetComp { elt, gens } => {
                Ty::Set(Box::new(self.synth(&self.comp_names(cx, gens), elt)))
            }
            ExprKind::DictComp { key, value, gens } => {
                let c = self.comp_names(cx, gens);
                Ty::Dict(
                    Box::new(self.synth(&c, key)),
                    Box::new(self.synth(&c, value)),
                )
            }
            ExprKind::Attr(..)
            | ExprKind::Slice { .. }
            | ExprKind::Lambda { .. }
            | ExprKind::Starred(_) => Ty::Any,
        }
    }

    /// A display's elements, joined; unknown if any is starred or there
    /// are none.
    fn elems(&self, cx: &Cx<'_, 'a>, items: &[Expr]) -> Ty {
        if items.iter().any(|i| matches!(i.kind, ExprKind::Starred(_))) {
            return Ty::Any;
        }
        Ty::union(items.iter().map(|i| self.synth(cx, i)))
    }

    // ── the walk ────────────────────────────────────────────────────────

    fn file(&self, f: &File<'a>, next_id: &dyn Fn() -> usize) {
        let cx = Cx {
            file: f,
            local: None,
            comps: Vec::new(),
        };
        self.stmts(&cx, f.stmts, None, next_id);
    }

    /// Check `stmts`; `ret` is the enclosing def's name and declared return
    /// type, when it declares one.
    fn stmts(
        &self,
        cx: &Cx<'_, 'a>,
        stmts: &'a [Stmt],
        ret: Option<(&str, &Ty)>,
        next_id: &dyn Fn() -> usize,
    ) {
        for s in stmts {
            self.stmt(cx, s, ret, next_id);
        }
    }

    fn stmt(
        &self,
        cx: &Cx<'_, 'a>,
        s: &'a Stmt,
        ret: Option<(&str, &Ty)>,
        next_id: &dyn Fn() -> usize,
    ) {
        let scope = cx.local.unwrap_or(&cx.file.scope);
        match &s.kind {
            StmtKind::Expr(e) => self.expr(cx, e),
            StmtKind::Assign { targets, value } => {
                self.expr(cx, value);
                for t in targets {
                    self.target_exprs(cx, t);
                    if let Target::Name(n) = t
                        && let Some(a) = scope.decl.get(n)
                    {
                        let want = self.ann(cx.file, a, false);
                        self.fits_or(cx, value, &want, |got| {
                            format!("`{n}` is declared {}, but this is {got}", want.show())
                        });
                    }
                }
            }
            StmtKind::AnnAssign {
                target,
                annotation,
                value,
            } => {
                let want = self.ann(cx.file, annotation, true);
                self.target_exprs(cx, target);
                if let Some(v) = value {
                    self.expr(cx, v);
                    self.fits_or(cx, v, &want, |got| {
                        format!("declared {}, but this is {got}", want.show())
                    });
                }
            }
            StmtKind::AugAssign { target, value, .. } => {
                self.target_exprs(cx, target);
                self.expr(cx, value);
            }
            StmtKind::If { cond, body, orelse } | StmtKind::While { cond, body, orelse } => {
                self.expr(cx, cond);
                self.stmts(cx, body, ret, next_id);
                self.stmts(cx, orelse, ret, next_id);
            }
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => {
                self.target_exprs(cx, target);
                self.expr(cx, iter);
                self.stmts(cx, body, ret, next_id);
                self.stmts(cx, orelse, ret, next_id);
            }
            StmtKind::Def {
                name,
                params,
                returns,
                body,
            } => self.def(cx, name, params, returns.as_ref(), body, None, next_id),
            StmtKind::Class { name, base, body } => {
                if let Some(b) = base {
                    self.expr(cx, b);
                }
                // The class body's own names declare nothing of the module's.
                let own = Scope {
                    id: next_id(),
                    ..Scope::default()
                };
                let in_class = Cx {
                    file: cx.file,
                    local: Some(&own),
                    comps: Vec::new(),
                };
                for m in body {
                    match &m.kind {
                        StmtKind::Def {
                            name: mname,
                            params,
                            returns,
                            body,
                        } => self.def(
                            cx,
                            &format!("{name}.{mname}"),
                            params,
                            returns.as_ref(),
                            body,
                            Some(name),
                            next_id,
                        ),
                        _ => self.stmt(&in_class, m, None, next_id),
                    }
                }
            }
            StmtKind::Return(value) => {
                let Some((fname, want)) = ret else {
                    if let Some(v) = value {
                        self.expr(cx, v);
                    }
                    return;
                };
                match value {
                    Some(v) => {
                        self.expr(cx, v);
                        self.fits_or(cx, v, want, |got| {
                            format!(
                                "returns {got}, but `{fname}` is declared -> {}",
                                want.show()
                            )
                        });
                    }
                    None => {
                        if !fits(&Ty::None, want, self) {
                            let span = cx
                                .file
                                .chars
                                .as_ref()
                                .and_then(|c| word_on_line(c, s.line, "return"));
                            if let Some(span) = span {
                                self.diag(
                                    span,
                                    format!(
                                        "returns None, but `{fname}` is declared -> {}",
                                        want.show()
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            StmtKind::Match { subject, arms } => {
                self.expr(cx, subject);
                for a in arms {
                    if let Some(g) = &a.guard {
                        self.expr(cx, g);
                    }
                    self.stmts(cx, &a.body, ret, next_id);
                }
            }
            StmtKind::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                self.stmts(cx, body, ret, next_id);
                for h in handlers {
                    if let Some(t) = &h.types {
                        self.expr(cx, t);
                    }
                    self.stmts(cx, &h.body, ret, next_id);
                }
                self.stmts(cx, orelse, ret, next_id);
                self.stmts(cx, finalbody, ret, next_id);
            }
            StmtKind::Raise(Some(e)) => self.expr(cx, e),
            StmtKind::Raise(None)
            | StmtKind::Break
            | StmtKind::Continue
            | StmtKind::Pass
            | StmtKind::Import { .. }
            | StmtKind::FromImport { .. } => {}
        }
    }

    /// A `def`, or a method of `class`: its annotations, its defaults
    /// against them, and its body in a scope of its own.
    #[allow(clippy::too_many_arguments)]
    fn def(
        &self,
        cx: &Cx<'_, 'a>,
        name: &str,
        params: &'a Params,
        returns: Option<&'a Annotation>,
        body: &'a [Stmt],
        class: Option<&str>,
        next_id: &dyn Fn() -> usize,
    ) {
        let module = Cx {
            file: cx.file,
            local: None,
            comps: Vec::new(),
        };
        let mut scope = Scope {
            id: next_id(),
            ..Scope::default()
        };
        for (i, ((p, a), d)) in params
            .names
            .iter()
            .zip(&params.annotations)
            .zip(&params.defaults)
            .enumerate()
        {
            match a {
                Some(a) => {
                    let want = self.ann(cx.file, a, true);
                    if let Some(d) = d {
                        self.expr(&module, d);
                        self.fits_or(&module, d, &want, |got| {
                            format!(
                                "`{p}` is declared {}, but its default is {got}",
                                want.show()
                            )
                        });
                    }
                    scope.decl.insert(p.clone(), a);
                }
                None => {
                    if let Some(d) = d {
                        self.expr(&module, d);
                    }
                    let t = match class {
                        Some(c) if i == 0 => Ty::Instance(cx.file.path.to_string(), c.to_string()),
                        _ => Ty::Any,
                    };
                    scope.add(p, Src::Ty(t));
                }
            }
        }
        for (n, a, wrap) in [
            (&params.star, &params.star_annotation, true),
            (&params.dstar, &params.dstar_annotation, false),
        ] {
            if let Some(n) = n {
                let t = a.as_ref().map_or(Ty::Any, |a| self.ann(cx.file, a, true));
                let t = if wrap {
                    Ty::TupleOf(Box::new(t))
                } else {
                    Ty::Dict(Box::new(Ty::Str), Box::new(t))
                };
                scope.add(n, Src::Ty(t));
            }
        }
        let want = returns.map(|a| self.ann(cx.file, a, true));
        scope.collect(body);
        let inner = Cx {
            file: cx.file,
            local: Some(&scope),
            comps: Vec::new(),
        };
        let ret = want.as_ref().map(|w| (name, w));
        self.stmts(&inner, body, ret, next_id);
    }

    /// The expressions inside an assignment target: `a[i]`'s `a` and `i`.
    fn target_exprs(&self, cx: &Cx<'_, 'a>, t: &Target) {
        match t {
            Target::Name(_) => {}
            Target::Subscript(a, i) => {
                self.expr(cx, a);
                self.expr(cx, i);
            }
            Target::Attr(a, _) => self.expr(cx, a),
            Target::Tuple(items, _) => items.iter().for_each(|t| self.target_exprs(cx, t)),
        }
    }

    /// Underline `e` with `message(its type)` if its type cannot fit
    /// `want`.
    fn fits_or(&self, cx: &Cx<'_, 'a>, e: &Expr, want: &Ty, message: impl Fn(String) -> String) {
        let got = self.synth(cx, e);
        if !fits(&got, want, self) {
            self.diag(e.span, message(got.show()));
        }
    }

    /// Check an expression and everything inside it.
    fn expr(&self, cx: &Cx<'_, 'a>, e: &Expr) {
        match &e.kind {
            ExprKind::Num(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::None
            | ExprKind::Name(_) => {}
            ExprKind::BinOp(l, op, r) => {
                self.expr(cx, l);
                self.expr(cx, r);
                let (a, b) = (self.synth(cx, l), self.synth(cx, r));
                if !binop_ok(*op, &a, &b) {
                    self.diag(
                        e.span,
                        format!("{} {} {} is a TypeError", a.show(), op_text(*op), b.show()),
                    );
                }
            }
            ExprKind::UnaryOp(_, x) => self.expr(cx, x),
            ExprKind::BoolOp { left, right, .. } => {
                self.expr(cx, left);
                self.expr(cx, right);
            }
            ExprKind::Compare { first, rest } => {
                self.expr(cx, first);
                let mut prev = self.synth(cx, first);
                for (op, x) in rest {
                    self.expr(cx, x);
                    let next = self.synth(cx, x);
                    if !compare_ok(*op, &prev, &next) {
                        self.diag(
                            e.span,
                            format!(
                                "{} {} {} is a TypeError",
                                prev.show(),
                                cmp_text(*op),
                                next.show()
                            ),
                        );
                    }
                    prev = next;
                }
            }
            ExprKind::IfExp { cond, then, orelse } => {
                self.expr(cx, cond);
                self.expr(cx, then);
                self.expr(cx, orelse);
            }
            ExprKind::Call { func, args } => {
                self.expr(cx, func);
                for a in &args.positional {
                    self.expr(cx, a);
                }
                for (_, v) in &args.keywords {
                    self.expr(cx, v);
                }
                for x in args.star.iter().chain(&args.dstar) {
                    self.expr(cx, x);
                }
                if let Some(sig) = self.callee_sig(cx, func) {
                    self.call(cx, e, args, &sig);
                }
            }
            ExprKind::Attr(x, _) => self.expr(cx, x),
            ExprKind::Subscript(a, i) => {
                self.expr(cx, a);
                self.expr(cx, i);
            }
            ExprKind::Slice { lo, hi, step } => {
                for x in [lo, hi, step].into_iter().flatten() {
                    self.expr(cx, x);
                }
            }
            ExprKind::List(items) | ExprKind::Tuple(items) | ExprKind::Set(items) => {
                items.iter().for_each(|i| self.expr(cx, i));
            }
            ExprKind::Dict(pairs) => {
                for (k, v) in pairs {
                    self.expr(cx, k);
                    self.expr(cx, v);
                }
            }
            ExprKind::ListComp { elt, gens } | ExprKind::SetComp { elt, gens } => {
                let c = self.comp(cx, gens);
                self.expr(&c, elt);
            }
            ExprKind::DictComp { key, value, gens } => {
                let c = self.comp(cx, gens);
                self.expr(&c, key);
                self.expr(&c, value);
            }
            ExprKind::Lambda { params, body } => {
                let names = params
                    .names
                    .iter()
                    .chain(&params.star)
                    .chain(&params.dstar)
                    .map(|n| (n.clone(), Ty::Any))
                    .collect();
                for d in params.defaults.iter().flatten() {
                    self.expr(cx, d);
                }
                self.expr(&cx.with(names), body);
            }
            ExprKind::FString(parts) => {
                for p in parts {
                    if let FPart::Expr { expr, .. } = p {
                        self.expr(cx, expr);
                    }
                }
            }
            ExprKind::Starred(x) => self.expr(cx, x),
        }
    }

    /// Check a comprehension's iterables and conditions, and return the
    /// context its element is read in.
    fn comp<'s>(&self, cx: &Cx<'s, 'a>, gens: &[Comp]) -> Cx<'s, 'a> {
        let mut c = cx.clone();
        for g in gens {
            self.expr(&c, &g.iter);
            let elem = elem_of(&self.synth(&c, &g.iter));
            let mut names = BTreeMap::new();
            bind(&g.target, elem, &mut names);
            c = c.with(names);
            for i in &g.ifs {
                self.expr(&c, i);
            }
        }
        c
    }

    /// A call against its callee's signature: the count of arguments,
    /// keywords, and each argument's type.
    fn call(&self, cx: &Cx<'_, 'a>, call: &Expr, args: &CallArgs, sig: &Sig) {
        let splat = args.star.is_some() || args.dstar.is_some();
        let n = &sig.name;
        for (i, a) in args.positional.iter().enumerate() {
            match sig.params.get(i) {
                Some((p, want, _)) => self.fits_or(cx, a, want, |got| {
                    format!("`{n}`'s `{p}` is {}, but this is {got}", want.show())
                }),
                None => match &sig.star {
                    Some(want) => self.fits_or(cx, a, want, |got| {
                        format!("`{n}` takes {}, but this is {got}", want.show())
                    }),
                    None => {
                        if sig.strict_extra && !splat {
                            self.diag(
                                a.span,
                                format!(
                                    "`{n}` {}, but {} were given",
                                    sig.arity(),
                                    args.positional.len()
                                ),
                            );
                            break;
                        }
                    }
                },
            }
        }
        let mut named = BTreeSet::new();
        for (k, v) in &args.keywords {
            if sig.positional_only {
                self.diag(v.span, format!("`{n}` takes no keyword arguments"));
                continue;
            }
            match sig.params.iter().position(|p| p.0 == *k) {
                Some(i) => {
                    named.insert(k.as_str());
                    if i < args.positional.len() && sig.strict {
                        self.diag(v.span, format!("`{n}` was given `{k}` twice"));
                    }
                    let want = &sig.params[i].1;
                    self.fits_or(cx, v, want, |got| {
                        format!("`{n}`'s `{k}` is {}, but this is {got}", want.show())
                    });
                }
                None => {
                    if !sig.dstar && sig.strict {
                        self.diag(v.span, format!("`{n}` has no parameter `{k}`"));
                    }
                }
            }
        }
        // Keywords to a builtin that takes none are the mistake to show,
        // not the arguments they failed to supply.
        let keywords_refused = sig.positional_only && !args.keywords.is_empty();
        if sig.strict && !splat && !keywords_refused {
            let missing: Vec<String> = sig
                .params
                .iter()
                .enumerate()
                .filter(|(i, p)| {
                    !p.2 && *i >= args.positional.len() && !named.contains(p.0.as_str())
                })
                .map(|(_, p)| format!("`{}`", p.0))
                .collect();
            if !missing.is_empty() {
                self.diag(
                    call.span,
                    format!("`{n}` is missing {}", missing.join(" and ")),
                );
            }
        }
    }
}

/// Bind an assignment or comprehension target to a value's type,
/// unpacking a tuple of the same length.
fn bind(t: &Target, ty: Ty, out: &mut BTreeMap<String, Ty>) {
    match t {
        Target::Name(n) => {
            out.insert(n.clone(), ty);
        }
        Target::Tuple(items, None) => {
            for (i, item) in items.iter().enumerate() {
                let part = match &ty {
                    Ty::Tuple(ts) if ts.len() == items.len() => ts[i].clone(),
                    Ty::TupleOf(e) | Ty::List(e) => (**e).clone(),
                    _ => Ty::Any,
                };
                bind(item, part, out);
            }
        }
        Target::Tuple(items, Some(_)) => {
            for item in items {
                bind(item, Ty::Any, out);
            }
        }
        Target::Subscript(..) | Target::Attr(..) => {}
    }
}

/// A literal integer index, negative ones included.
fn index_of(idx: &Expr) -> Option<i128> {
    match &idx.kind {
        ExprKind::Num(n) if n.is_integral() => Some(n.raw() / lang::num::Num::ONE.raw()),
        ExprKind::UnaryOp(UnaryOp::Neg, x) => index_of(x).map(|i| -i),
        _ => None,
    }
}

fn pick(ts: &[Ty], i: i128) -> Option<Ty> {
    let len = i128::try_from(ts.len()).ok()?;
    let i = if i < 0 { len + i } else { i };
    ts.get(usize::try_from(i).ok()?).cloned()
}

/// Whether a value of type `got` might be a `want`: `false` only when it
/// certainly is not.
fn fits(got: &Ty, want: &Ty, tree: &Tree<'_>) -> bool {
    match (got, want) {
        (Ty::Any, _) | (_, Ty::Any) => true,
        (_, Ty::Union(ws)) => ws.iter().any(|w| fits(got, w, tree)),
        (Ty::Union(gs), _) => gs.iter().any(|g| fits(g, want, tree)),
        (Ty::None, Ty::None) | (Ty::Str, Ty::Str) => true,
        // `bool` is a `num`, and a `num` may be integral.
        (Ty::Bool | Ty::Int | Ty::Num, Ty::Int | Ty::Num) => true,
        (Ty::Bool, Ty::Bool) => true,
        (Ty::List(g), Ty::List(w)) | (Ty::Set(g), Ty::Set(w)) => fits(g, w, tree),
        (Ty::Dict(gk, gv), Ty::Dict(wk, wv)) => fits(gk, wk, tree) && fits(gv, wv, tree),
        (Ty::Tuple(gs), Ty::Tuple(ws)) => {
            gs.len() == ws.len() && gs.iter().zip(ws).all(|(g, w)| fits(g, w, tree))
        }
        (Ty::Tuple(gs), Ty::TupleOf(w)) => gs.iter().all(|g| fits(g, w, tree)),
        (Ty::TupleOf(g), Ty::Tuple(ws)) => ws.iter().all(|w| fits(g, w, tree)),
        (Ty::TupleOf(g), Ty::TupleOf(w)) => fits(g, w, tree),
        (Ty::Record(a), Ty::Record(b)) => a == b,
        (Ty::Instance(gp, gc), Ty::Instance(wp, wc)) => subclass(tree, gp, gc, wp, wc, 0),
        _ => false,
    }
}

/// Whether class `gc` of `gp` might be `wc` of `wp` or derive from it:
/// `false` only when its whole chain is known and misses it.
fn subclass(tree: &Tree<'_>, gp: &str, gc: &str, wp: &str, wc: &str, depth: u32) -> bool {
    if gp == wp && gc == wc {
        return true;
    }
    if depth > 8 {
        return true;
    }
    let Some(f) = tree.files.iter().find(|f| f.path == gp) else {
        return true;
    };
    let Some(Callee::Class(cf, c)) = tree.member(f, gc, 0) else {
        return true;
    };
    let StmtKind::Class { base, .. } = &c.kind else {
        return true;
    };
    match base.as_ref().map(|b| &b.kind) {
        None => false,
        Some(ExprKind::Name(b)) => match tree.member(cf, b, 0) {
            Some(Callee::Class(bf, _)) => subclass(tree, bf.path, b, wp, wc, depth + 1),
            // A builtin exception class, which no class of the tree is.
            None if builtin_exception(b) => false,
            _ => true,
        },
        Some(_) => true,
    }
}

fn builtin_exception(n: &str) -> bool {
    lang::errors::ExcClass::ALL.iter().any(|c| c.name() == n)
}

/// The type an arithmetic operator gives, as far as the check can tell.
fn binop_ty(op: BinOp, a: &Ty, b: &Ty) -> Ty {
    let int = |t: &Ty| matches!(t, Ty::Bool | Ty::Int);
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::FloorDiv | BinOp::Mod if int(a) && int(b) => {
            Ty::Int
        }
        BinOp::BitOr | BinOp::BitAnd | BinOp::BitXor if int(a) && int(b) => Ty::Int,
        _ if a.numeric() && b.numeric() => Ty::Num,
        BinOp::Add => match (a, b) {
            (Ty::Str, Ty::Str) => Ty::Str,
            (Ty::List(x), Ty::List(y)) => {
                Ty::List(Box::new(Ty::union([(**x).clone(), (**y).clone()])))
            }
            (Ty::Tuple(x), Ty::Tuple(y)) => Ty::Tuple(x.iter().chain(y).cloned().collect()),
            (x, y) if x.tuple_like() && y.tuple_like() => {
                Ty::TupleOf(Box::new(Ty::union([elem_of(x), elem_of(y)])))
            }
            _ => Ty::Any,
        },
        BinOp::Mul => match (a, b) {
            (Ty::Str, n) | (n, Ty::Str) if n.numeric() => Ty::Str,
            (Ty::List(x), n) | (n, Ty::List(x)) if n.numeric() => Ty::List(x.clone()),
            (t, n) | (n, t) if t.tuple_like() && n.numeric() => Ty::TupleOf(Box::new(elem_of(t))),
            _ => Ty::Any,
        },
        BinOp::Mod if *a == Ty::Str => Ty::Str,
        _ => Ty::Any,
    }
}

/// Whether some member of `a` with some member of `b` is an operation
/// the language has.
fn binop_ok(op: BinOp, a: &Ty, b: &Ty) -> bool {
    a.members().iter().any(|x| {
        b.members().iter().any(|y| {
            let (x, y) = (*x, *y);
            if matches!(x, Ty::Any | Ty::Instance(..)) || matches!(y, Ty::Any | Ty::Instance(..)) {
                return true;
            }
            if x.numeric() && y.numeric() {
                return true;
            }
            let seq = |t: &Ty| matches!(t, Ty::Str | Ty::List(_)) || t.tuple_like();
            match op {
                BinOp::Add => {
                    matches!((x, y), (Ty::Str, Ty::Str) | (Ty::List(_), Ty::List(_)))
                        || (x.tuple_like() && y.tuple_like())
                }
                BinOp::Mul => (seq(x) && y.numeric()) || (x.numeric() && seq(y)),
                BinOp::Mod => *x == Ty::Str,
                BinOp::Sub | BinOp::BitAnd | BinOp::BitXor => {
                    matches!((x, y), (Ty::Set(_), Ty::Set(_)))
                }
                BinOp::BitOr => {
                    matches!(
                        (x, y),
                        (Ty::Set(_), Ty::Set(_)) | (Ty::Dict(..), Ty::Dict(..))
                    )
                }
                BinOp::Div | BinOp::FloorDiv | BinOp::Pow => false,
            }
        })
    })
}

/// Whether some pairing of the two sides' members compares.
fn compare_ok(op: CmpOp, a: &Ty, b: &Ty) -> bool {
    a.members().iter().any(|x| {
        b.members().iter().any(|y| {
            let (x, y) = (*x, *y);
            if matches!(x, Ty::Any | Ty::Instance(..)) || matches!(y, Ty::Any | Ty::Instance(..)) {
                return true;
            }
            match op {
                CmpOp::Lt | CmpOp::Gt | CmpOp::Le | CmpOp::Ge => {
                    (x.numeric() && y.numeric())
                        || matches!(
                            (x, y),
                            (Ty::Str, Ty::Str)
                                | (Ty::List(_), Ty::List(_))
                                | (Ty::Set(_), Ty::Set(_))
                        )
                        || (x.tuple_like() && y.tuple_like())
                }
                CmpOp::In | CmpOp::NotIn => match y {
                    Ty::Str => *x == Ty::Str,
                    Ty::List(_)
                    | Ty::Set(_)
                    | Ty::Dict(..)
                    | Ty::Tuple(_)
                    | Ty::TupleOf(_)
                    | Ty::Record(_) => true,
                    _ => false,
                },
                CmpOp::Eq | CmpOp::Ne | CmpOp::Is | CmpOp::IsNot => true,
            }
        })
    })
}

fn op_text(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::FloorDiv => "//",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
        BinOp::BitOr => "|",
        BinOp::BitAnd => "&",
        BinOp::BitXor => "^",
    }
}

fn cmp_text(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Lt => "<",
        CmpOp::Gt => ">",
        CmpOp::Le => "<=",
        CmpOp::Ge => ">=",
        CmpOp::Eq => "==",
        CmpOp::Ne => "!=",
        CmpOp::In => "in",
        CmpOp::NotIn => "not in",
        CmpOp::Is => "is",
        CmpOp::IsNot => "is not",
    }
}

/// What an annotation naming no type says, with a pointer for the names
/// Python's `typing` would have supplied.
fn unknown_type(n: &str) -> String {
    let hint = match n {
        "Optional" => ": write `T | None`",
        "Union" => ": write `A | B`",
        "List" | "Dict" | "Set" | "Tuple" => ": the language spells it in lower case",
        "Any" => ": there is no `Any`; leave the annotation off",
        "float" => ": there is no `float`; every number is a `num`",
        _ => "",
    };
    format!("`{n}` is not a type{hint}")
}

/// The chars of `word`, as a whole identifier, inside `span`.
fn find_word(chars: &[char], span: Span, word: &str) -> Option<Span> {
    let w: Vec<char> = word.chars().collect();
    let ident = |c: Option<&char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_');
    let (lo, hi) = (span.start as usize, (span.end as usize).min(chars.len()));
    (lo..hi).find_map(|i| {
        let end = i + w.len();
        (end <= hi
            && chars.get(i..end) == Some(&w[..])
            && !ident(i.checked_sub(1).and_then(|j| chars.get(j)))
            && !ident(chars.get(end)))
        .then_some(Span {
            start: i as u32,
            end: end as u32,
        })
    })
}

/// The chars of `word` on 1-based `line`.
fn word_on_line(chars: &[char], line: u32, word: &str) -> Option<Span> {
    let mut start = 0usize;
    let mut n = 1u32;
    for (i, c) in chars.iter().enumerate() {
        if n == line {
            break;
        }
        if *c == '\n' {
            n += 1;
            start = i + 1;
        }
    }
    let end = chars[start..]
        .iter()
        .position(|c| *c == '\n')
        .map_or(chars.len(), |p| start + p);
    find_word(
        chars,
        Span {
            start: start as u32,
            end: end as u32,
        },
        word,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each underline in `path` as (the text it covers, its message).
    fn diags(files: &[(&str, &str)], path: &str) -> Vec<(String, String)> {
        let docs = BuiltinDocs::parse(crate::hover::DOCS_TOML).expect("docs");
        let parsed = files
            .iter()
            .map(|(p, t)| {
                let stmts = lang::parser::parse_file(p, t).expect("parses");
                (p.to_string(), (t.to_string(), stmts, true))
            })
            .collect();
        let all = check(&parsed, &Builtins::new(&docs));
        let text: Vec<char> = files
            .iter()
            .find(|(p, _)| *p == path)
            .expect("file")
            .1
            .chars()
            .collect();
        all.get(path)
            .into_iter()
            .flatten()
            .map(|d| (text[d.start..d.end].iter().collect(), d.message.clone()))
            .collect()
    }

    fn one(src: &str) -> Vec<(String, String)> {
        diags(&[("robots/bot.py", src)], "robots/bot.py")
    }

    /// Just the underlined texts.
    fn marked(src: &str) -> Vec<String> {
        one(src).into_iter().map(|(t, _)| t).collect()
    }

    #[test]
    fn the_starter_tree_has_no_underlines() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/starter");
        let tree = sim::script::read_tree_files(&root).expect("starter tree");
        let files: Vec<(&str, &str)> = tree.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
        for (p, _) in &files {
            assert_eq!(diags(&files, p), vec![], "{p}");
        }
    }

    #[test]
    fn a_call_is_checked_against_its_def() {
        let src = "\
def f(a: int, b: str = \"x\") -> int:
    return a
f(1)
f(1, \"y\", 3)
f()
f(1, c=2)
f(1, a=2)
f(\"one\")
f(1, b=2)
";
        assert_eq!(
            one(src),
            vec![
                (
                    "3".into(),
                    "`f` takes 1 to 2 arguments, but 3 were given".into()
                ),
                ("f()".into(), "`f` is missing `a`".into()),
                ("2".into(), "`f` has no parameter `c`".into()),
                ("2".into(), "`f` was given `a` twice".into()),
                ("\"one\"".into(), "`f`'s `a` is int, but this is str".into()),
                ("2".into(), "`f`'s `b` is str, but this is int".into()),
            ]
        );
    }

    #[test]
    fn game_builtins_take_exact_positional_arguments() {
        assert_eq!(
            marked("move_to(me().pos)\nmove_to(1, 2, 3)\nmove_to(x=1, y=2)\nwait(\"a\")\n"),
            vec!["move_to(me().pos)", "3", "1", "2", "\"a\""]
        );
        // `see`, `hear` and `me` ignore what they are given.
        assert_eq!(marked("see(1)\nme(2, 3)\n"), Vec::<String>::new());
        // A name the program binds is not the builtin.
        assert_eq!(
            marked("def move(a, b, c):\n    pass\nmove(1, 2, 3)\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn only_a_certain_mismatch_is_underlined() {
        let src = "\
def want_tile(t: Tile) -> bool:
    return True
def want_int(n: int) -> int:
    return n
want_tile(tile(1, 2))
want_tile(None)
want_int(len([1]))
want_int(me().pos)
x = 1
if want_int(2):
    x = \"a\"
want_int(x)
y = \"s\"
want_int(y)
want_int(1.5)
want_int(True)
";
        // A union with a member that fits passes, and so does an unknown;
        // a `num` may be integral, and a `bool` is a `num`.
        assert_eq!(marked(src), vec!["None", "y"]);
    }

    #[test]
    fn returns_and_declared_names() {
        let src = "\
def f(a: int) -> str:
    if a:
        return a
    return
def g() -> None:
    return
n: int = \"a\"
n = 2
n = \"b\"
def h(x: int = \"z\"):
    x = \"w\"
";
        assert_eq!(
            one(src),
            vec![
                ("a".into(), "returns int, but `f` is declared -> str".into()),
                (
                    "return".into(),
                    "returns None, but `f` is declared -> str".into()
                ),
                ("\"a\"".into(), "declared int, but this is str".into()),
                (
                    "\"b\"".into(),
                    "`n` is declared int, but this is str".into()
                ),
                (
                    "\"z\"".into(),
                    "`x` is declared int, but its default is str".into()
                ),
                (
                    "\"w\"".into(),
                    "`x` is declared int, but this is str".into()
                ),
            ]
        );
    }

    #[test]
    fn operators_the_language_lacks() {
        let src = "\
a = \"s\" + 1
b = None * 2
c = \"ab\" * 3 + \"c\"
d = [1] + [2]
e = \"x\" < 3
f = 1 in 5
g = unknown + 1
h = (1, 2) + (3,)
";
        assert_eq!(
            one(src),
            vec![
                ("\"s\" + 1".into(), "str + int is a TypeError".into()),
                ("None * 2".into(), "None * int is a TypeError".into()),
                ("\"x\" < 3".into(), "str < int is a TypeError".into()),
                ("1 in 5".into(), "int in int is a TypeError".into()),
            ]
        );
    }

    #[test]
    fn an_annotation_naming_no_type_is_underlined() {
        let src = "\
from geo import Point
import geo
def f(p: Pos, q: Optional[int], r: Point, s: geo.Point, t: Later, u: geo.Nope) -> List[int]:
    v: list[int, str] = []
class Later:
    pass
";
        let geo = "class Point:\n    pass\n";
        assert_eq!(
            diags(
                &[("robots/bot.py", src), ("lib/geo.py", geo)],
                "robots/bot.py"
            ),
            vec![
                ("Pos".into(), "`Pos` is not a type".into()),
                (
                    "Optional".into(),
                    "`Optional` is not a type: write `T | None`".into()
                ),
                ("Nope".into(), "`geo.Nope` is not a class".into()),
                (
                    "List".into(),
                    "`List` is not a type: the language spells it in lower case".into()
                ),
                ("list".into(), "`list[T]` takes one type".into()),
            ]
        );
    }

    #[test]
    fn classes_methods_and_inheritance() {
        let src = "\
class A:
    def __init__(self, n: int):
        self.n = n
    def get(self, k: str) -> int:
        return self.n
class B(A):
    def twice(self) -> int:
        return self.get(1) * 2
class C:
    pass
def use(a: A) -> int:
    return a.get(\"k\")
use(B(1))
use(C())
A()
b = B(2)
b.get(\"k\", 3)
";
        assert_eq!(
            one(src),
            vec![
                ("1".into(), "`B.get`'s `k` is str, but this is int".into()),
                ("C()".into(), "`use`'s `a` is A, but this is C".into()),
                ("A()".into(), "`A` is missing `n`".into()),
                (
                    "3".into(),
                    "`B.get` takes 1 argument, but 2 were given".into()
                ),
            ]
        );
    }

    #[test]
    fn comprehensions_and_loops_carry_element_types() {
        let src = "\
def names(xs: list[str]) -> int:
    return 0
pts: list[tuple[int, int]] = [(1, 2)]
names([p for p in pts])
names([str(p[0]) for p in pts])
for q in pts:
    names(q)
";
        assert_eq!(marked(src), vec!["[p for p in pts]", "q"]);
    }

    #[test]
    fn the_game_s_builtins_take_the_arguments_docs_toml_says() {
        // The check underlines a game builtin's call by its signature in
        // docs.toml, so the host must refuse exactly what that signature
        // refuses: one argument too many, one too few, and a keyword.
        // A bot tries each and faults if any goes through.
        let docs = BuiltinDocs::parse(crate::hover::DOCS_TOML).expect("docs");
        let builtins = Builtins::new(&docs);
        let mut bot = String::new();
        let mut tried = 0;
        for (kind, name, _) in docs.signatures() {
            let Some(sig) = builtins.sigs.get(name).filter(|_| kind == Kind::Game) else {
                continue;
            };
            let n = sig.params.len();
            let zeros = |k: usize| vec!["0"; k].join(", ");
            let mut calls = vec![format!("{name}(x=0)")];
            if n > 0 {
                calls.push(format!("{name}({})", zeros(n + 1)));
                calls.push(format!("{name}({})", zeros(n - 1)));
            }
            for call in calls {
                tried += 1;
                bot.push_str(&format!(
                    "try:\n    {call}\n    raise ValueError(\"{call} went through\")\nexcept TypeError:\n    pass\n"
                ));
            }
        }
        bot.push_str("rename(\"checked\")\nwhile True:\n    wait(10)\n");
        assert!(tried > 40, "only {tried} calls tried");

        let map = r#"
name = "t"
speed = 10
width = 5
height = 5
terrain = """
o....
.....
.....
.....
....o
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-1, 0]
[[team]]
start = [1, -1]
"#;
        let data = sim::Data::load().expect("data");
        let mut world = sim::Sim::new(sim::Map::parse(map, &data).expect("map"));
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/starter");
        let mut tree = sim::script::read_tree_files(&root).expect("starter tree");
        tree.insert("robots/bot.py".into(), bot);
        let deploys: Vec<sim::Command> = sim::script::compose(&tree)
            .expect("composes")
            .into_iter()
            .enumerate()
            .map(|(i, (deployment, bundle))| {
                sim::Command::new(
                    0,
                    sim::TeamId(0),
                    i as u32,
                    sim::CommandKind::Deploy { deployment, bundle },
                )
            })
            .collect();
        let mut checked = false;
        for tick in 0..200 {
            let cmds = if tick == 0 { deploys.clone() } else { vec![] };
            world.step(&cmds);
            for m in &world.snapshot().machines {
                if m.record.team == sim::TeamId(0) {
                    assert!(m.fault.is_none(), "{}: {:?}", m.record.name, m.fault);
                    checked |= m.record.name == "checked";
                }
            }
            if checked {
                break;
            }
        }
        assert!(checked, "no bot finished the calls");
    }

    #[test]
    fn calls_across_files_read_the_other_file_s_annotations() {
        let lib = "def go(p: tuple[int, int]) -> bool:\n    return True\n";
        let bot = "import nav\nfrom nav import go\ngo(me())\nnav.go((1, 2))\nnav.go(1)\n";
        assert_eq!(
            diags(
                &[("robots/bot.py", bot), ("lib/nav.py", lib)],
                "robots/bot.py"
            )
            .into_iter()
            .map(|(t, _)| t)
            .collect::<Vec<_>>(),
            vec!["me()", "1"]
        );
    }
}

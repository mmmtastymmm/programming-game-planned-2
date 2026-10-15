//! What resting the pointer on a name in a file window shows (Q57): the
//! signature as written, its docstring, and where it comes from.
//!
//! Plain Rust over the language's own parser, with no Bevy in it, so it is
//! tested on its own. Each file is outlined from its syntax tree — its
//! `def`s, `class`es and their methods, its imports, and its docstrings —
//! and a name under the pointer resolves the way the language resolves it:
//! a `def` or `class` of the file, an import, a module's name, `module.f`,
//! `self.method` or `Class.method`, and only then a builtin, whose text
//! comes from `data/language/docs.toml`. A name that resolves to nothing
//! shows nothing: a guess is worse than silence.

use lang::ast::{ExprKind, Stmt, StmtKind, Target};
use std::collections::{BTreeMap, BTreeSet};

/// The builtins' signatures and descriptions.
pub const DOCS_TOML: &str = include_str!("../../../data/language/docs.toml");

/// What a hover shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Hover {
    /// As written in the source, annotations included, on one line.
    pub signature: String,
    /// The docstring, cleaned as Python's `inspect.cleandoc` does.
    pub doc: Option<String>,
    pub origin: Origin,
}

/// Where a hovered name is defined.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    /// A file of the tree, by path, and the line of its `def`, `class` or
    /// (for a module) its first line.
    File { path: String, line: u32 },
    /// A game builtin (`docs/02`).
    Game,
    /// A builtin of the language (`docs/01`).
    Builtin,
    /// One of the language's exception classes.
    Exception,
}

impl Origin {
    /// The line under a hover saying where the name comes from.
    pub fn label(&self) -> String {
        match self {
            Origin::File { path, line } => format!("{path}:{line}"),
            Origin::Game => "game builtin".into(),
            Origin::Builtin => "language builtin".into(),
            Origin::Exception => "language exception".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Def {
    line: u32,
    signature: String,
    doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct Class {
    def: Def,
    /// The base class's name, if it has one written as a plain name.
    base: Option<String>,
    /// The last line of the class's body: the line before the next
    /// top-level statement, or the end of the file.
    last: u32,
    methods: BTreeMap<String, Def>,
}

#[derive(Debug, Clone, PartialEq)]
enum Import {
    /// `import m` or `import m as a`.
    Module(String),
    /// `from m import x` or `from m import x as a`.
    Name(String, String),
}

/// One file's outline: everything a hover can name in it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Outline {
    doc: Option<String>,
    defs: BTreeMap<String, Def>,
    classes: BTreeMap<String, Class>,
    /// Local name to what it imports.
    imports: BTreeMap<String, Import>,
    /// Names the module assigns at its top level, which hide a builtin of
    /// the same name.
    assigned: BTreeSet<String>,
    /// Each `from m import …` statement's first and last lines and its
    /// module, so a name hovered there is looked up in `m`.
    from_lines: Vec<(u32, u32, String)>,
}

/// Outline `src`, or `None` if it does not parse.
pub fn outline(src: &str) -> Option<Outline> {
    let stmts = lang::parser::parse_file("hover.py", src).ok()?;
    let lines: Vec<&str> = src.lines().collect();
    let mut o = Outline {
        doc: docstring(&stmts),
        ..Outline::default()
    };
    for (i, s) in stmts.iter().enumerate() {
        let next = stmts.get(i.saturating_add(1)).map(|n| n.line);
        match &s.kind {
            StmtKind::Def { name, body, .. } => {
                o.defs.insert(name.clone(), def(&lines, s.line, body));
            }
            StmtKind::Class { name, base, body } => {
                let methods = body
                    .iter()
                    .filter_map(|m| match &m.kind {
                        StmtKind::Def { name, body, .. } => {
                            Some((name.clone(), def(&lines, m.line, body)))
                        }
                        _ => None,
                    })
                    .collect();
                let base = base.as_ref().and_then(|b| match &b.kind {
                    ExprKind::Name(n) => Some(n.clone()),
                    _ => None,
                });
                let last = next.map_or(u32::MAX, |n| n.saturating_sub(1));
                o.classes.insert(
                    name.clone(),
                    Class {
                        def: def(&lines, s.line, body),
                        base,
                        last,
                        methods,
                    },
                );
            }
            StmtKind::Import { module, alias } => {
                let local = alias.clone().unwrap_or_else(|| module.clone());
                o.imports.insert(local, Import::Module(module.clone()));
            }
            StmtKind::FromImport { module, names } => {
                let last = next.map_or(u32::MAX, |n| n.saturating_sub(1));
                o.from_lines.push((s.line, last, module.clone()));
                for (name, alias) in names {
                    let local = alias.clone().unwrap_or_else(|| name.clone());
                    o.imports
                        .insert(local, Import::Name(module.clone(), name.clone()));
                }
            }
            StmtKind::Assign { targets, .. } => {
                for t in targets {
                    if let Target::Name(n) = t {
                        o.assigned.insert(n.clone());
                    }
                }
            }
            StmtKind::AnnAssign {
                target: Target::Name(n),
                ..
            } => {
                o.assigned.insert(n.clone());
            }
            _ => {}
        }
    }
    Some(o)
}

fn def(lines: &[&str], line: u32, body: &[Stmt]) -> Def {
    Def {
        line,
        signature: header(lines, line),
        doc: docstring(body),
    }
}

/// The text of a `def` or `class` header from its first line to the `:`
/// that ends it, outside brackets and strings, joined onto one line.
fn header(lines: &[&str], line: u32) -> String {
    let start = usize::try_from(line.saturating_sub(1)).unwrap_or(usize::MAX);
    let mut out = String::new();
    let mut depth = 0u32;
    let mut quote: Option<char> = None;
    for text in lines.iter().skip(start) {
        let text = text.trim();
        for c in text.chars() {
            match quote {
                Some(q) => {
                    if c == q {
                        quote = None;
                    }
                }
                None => match c {
                    '\'' | '"' => quote = Some(c),
                    '(' | '[' | '{' => depth = depth.saturating_add(1),
                    ')' | ']' | '}' => depth = depth.saturating_sub(1),
                    ':' if depth == 0 => return out,
                    '#' => break,
                    _ => {}
                },
            }
            out.push(c);
        }
        if !out.is_empty() && !out.ends_with(['(', '[', '{']) {
            out.push(' ');
        }
    }
    out.trim_end().to_string()
}

/// A body's docstring: its first statement, when that is a plain string
/// literal (`01`, Q52), cleaned as `inspect.cleandoc` does — the first
/// line stripped, the common indentation of the rest removed, and blank
/// lines dropped from both ends.
fn docstring(body: &[Stmt]) -> Option<String> {
    let StmtKind::Expr(e) = &body.first()?.kind else {
        return None;
    };
    let ExprKind::Str(s) = &e.kind else {
        return None;
    };
    let mut lines = s.lines();
    let first = lines.next().unwrap_or("").trim().to_string();
    let rest: Vec<&str> = lines.collect();
    let indent = rest
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len().saturating_sub(l.trim_start().len()))
        .min()
        .unwrap_or(0);
    let mut out: Vec<String> = vec![first];
    out.extend(
        rest.iter()
            .map(|l| l.get(indent..).unwrap_or("").trim_end().to_string()),
    );
    while out.first().is_some_and(|l| l.is_empty()) {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    (!out.is_empty()).then(|| out.join("\n"))
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct Entry {
    signature: String,
    doc: String,
}

/// The builtins' entries in `data/language/docs.toml`, by kind and name.
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
pub struct BuiltinDocs {
    #[serde(default)]
    game: BTreeMap<String, Entry>,
    #[serde(default)]
    builtin: BTreeMap<String, Entry>,
    #[serde(default)]
    exception: BTreeMap<String, Entry>,
}

impl BuiltinDocs {
    pub fn parse(text: &str) -> Result<BuiltinDocs, String> {
        toml::from_str(text).map_err(|e| format!("data/language/docs.toml: {e}"))
    }

    fn lookup(&self, name: &str) -> Option<Hover> {
        let (e, origin) = if let Some(e) = self.game.get(name) {
            (e, Origin::Game)
        } else if let Some(e) = self.builtin.get(name) {
            (e, Origin::Builtin)
        } else {
            (self.exception.get(name)?, Origin::Exception)
        };
        Some(Hover {
            signature: e.signature.clone(),
            doc: Some(e.doc.trim().to_string()),
            origin,
        })
    }
}

/// Every file's outline, kept across edits: a file that stops parsing
/// while it is being typed keeps its last good outline, so hovers go on
/// working.
#[derive(Debug, Clone)]
pub struct Index {
    files: BTreeMap<String, (String, Option<Outline>)>,
    docs: BuiltinDocs,
}

impl Default for Index {
    /// With the builtins' entries loaded, which a derived default would
    /// leave empty.
    fn default() -> Index {
        Index::new()
    }
}

impl Index {
    pub fn new() -> Index {
        Index {
            files: BTreeMap::new(),
            docs: BuiltinDocs::parse(DOCS_TOML).expect("docs.toml"),
        }
    }

    /// Bring the index up to date with the tree: outline each file whose
    /// text changed, keeping the old outline where the new text does not
    /// parse, and forget files that are gone.
    pub fn update<'a>(&mut self, files: impl IntoIterator<Item = (&'a str, &'a str)>) {
        let mut seen = BTreeSet::new();
        for (path, text) in files {
            seen.insert(path.to_string());
            let entry = self
                .files
                .entry(path.to_string())
                .or_insert_with(|| (String::new(), None));
            if entry.0 != text || entry.1.is_none() {
                entry.0 = text.to_string();
                if let Some(o) = outline(text) {
                    entry.1 = Some(o);
                }
            }
        }
        self.files.retain(|p, _| seen.contains(p));
    }

    fn by_path(&self, path: &str) -> Option<&Outline> {
        self.files.get(path).and_then(|(_, o)| o.as_ref())
    }

    /// The file a module name is, among the files every bundle shares:
    /// a file outside `robots/` by its bare name (`docs/07`). A robot file
    /// is its bundle's `main.py` and never a module.
    fn module(&self, name: &str) -> Option<(&str, &Outline)> {
        self.files.iter().find_map(|(path, (_, o))| {
            let stem = path.rsplit('/').next()?.strip_suffix(".py")?;
            (stem == name && sim::script::robot_of(path).is_none())
                .then_some(())
                .and(o.as_ref().map(|o| (path.as_str(), o)))
        })
    }

    /// What the pointer at char `at` of `path`'s `text` names, if anything.
    pub fn hover(&self, path: &str, text: &str, at: usize) -> Option<Hover> {
        let chain = chain_at(text, at)?;
        let line = line_of(text, at);
        let here = self.by_path(path)?;
        match chain.as_slice() {
            [name] => self
                .in_import(here, name, line)
                .or_else(|| self.in_file(path, here, name, 0))
                .or_else(|| method_defined_here(path, here, name, line))
                .or_else(|| {
                    (!here.assigned.contains(name.as_str()))
                        .then(|| self.docs.lookup(name))
                        .flatten()
                }),
            [owner, name] => {
                if owner == "self" {
                    let class = enclosing_class(here, line)?;
                    return self.method(path, here, class, name, 0);
                }
                // `module.f` or `module.Class`.
                if let Some(Import::Module(m)) = here.imports.get(owner.as_str()) {
                    let (mpath, mo) = self.module(m)?;
                    return self.in_file(mpath, mo, name, 0);
                }
                // `Class.method`, the class defined here or imported.
                let (cpath, co, class) = self.class(path, here, owner, 0)?;
                self.method(cpath, co, class, name, 0)
            }
            _ => None,
        }
    }

    /// A name on a `from m import …` line: `m` itself, or a name `m`
    /// defines, so the original of `from geo import Point as P` resolves.
    fn in_import(&self, here: &Outline, name: &str, line: u32) -> Option<Hover> {
        let (_, _, m) = here
            .from_lines
            .iter()
            .find(|(first, last, _)| *first <= line && line <= *last)?;
        let (mpath, mo) = self.module(m)?;
        if name == m {
            return Some(module_hover(m, mpath, mo));
        }
        self.in_file(mpath, mo, name, 0)
    }

    /// `name` as file `path` defines or imports it: a `def`, a `class`,
    /// an imported name followed to where it is defined, or a module.
    fn in_file(&self, path: &str, o: &Outline, name: &str, depth: u32) -> Option<Hover> {
        if depth > 8 {
            return None;
        }
        if let Some(d) = o.defs.get(name) {
            return Some(from_def(path, d));
        }
        if let Some(c) = o.classes.get(name) {
            return Some(from_def(path, &c.def));
        }
        match o.imports.get(name)? {
            Import::Module(m) => {
                let (mpath, mo) = self.module(m)?;
                Some(module_hover(m, mpath, mo))
            }
            Import::Name(m, x) => {
                let (mpath, mo) = self.module(m)?;
                self.in_file(mpath, mo, x, depth.saturating_add(1))
            }
        }
    }

    /// The class `name` as file `path` sees it: its own, or imported.
    fn class<'a>(
        &'a self,
        path: &'a str,
        o: &'a Outline,
        name: &str,
        depth: u32,
    ) -> Option<(&'a str, &'a Outline, &'a Class)> {
        if depth > 8 {
            return None;
        }
        if let Some(c) = o.classes.get(name) {
            return Some((path, o, c));
        }
        match o.imports.get(name)? {
            Import::Name(m, x) => {
                let (mpath, mo) = self.module(m)?;
                self.class(mpath, mo, x, depth.saturating_add(1))
            }
            Import::Module(_) => None,
        }
    }

    /// A method of `class`, or of its base classes, which single
    /// inheritance makes a chain.
    fn method(
        &self,
        path: &str,
        o: &Outline,
        class: &Class,
        name: &str,
        depth: u32,
    ) -> Option<Hover> {
        if depth > 8 {
            return None;
        }
        if let Some(d) = class.methods.get(name) {
            return Some(from_def(path, d));
        }
        let base = class.base.as_deref()?;
        let (bpath, bo, bclass) = self.class(path, o, base, 0)?;
        self.method(bpath, bo, bclass, name, depth.saturating_add(1))
    }
}

/// A method's own name on the line of its `def`.
fn method_defined_here(path: &str, o: &Outline, name: &str, line: u32) -> Option<Hover> {
    let d = enclosing_class(o, line)?.methods.get(name)?;
    (d.line == line).then(|| from_def(path, d))
}

fn enclosing_class(o: &Outline, line: u32) -> Option<&Class> {
    o.classes
        .values()
        .find(|c| c.def.line <= line && line <= c.last)
}

fn module_hover(name: &str, path: &str, o: &Outline) -> Hover {
    Hover {
        signature: format!("module {name}"),
        doc: o.doc.clone(),
        origin: Origin::File {
            path: path.to_string(),
            line: 1,
        },
    }
}

fn from_def(path: &str, d: &Def) -> Hover {
    Hover {
        signature: d.signature.clone(),
        doc: d.doc.clone(),
        origin: Origin::File {
            path: path.to_string(),
            line: d.line,
        },
    }
}

/// The 1-based line holding char `at`.
fn line_of(text: &str, at: usize) -> u32 {
    let n = text.chars().take(at).filter(|&c| c == '\n').count();
    u32::try_from(n).unwrap_or(u32::MAX).saturating_add(1)
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The dotted name ending at the identifier under char `at`: hovering
/// `go_to` in `nav.go_to(p)` gives `["nav", "go_to"]`, and hovering `nav`
/// gives `["nav"]`. `None` off an identifier, and for an attribute of
/// anything but a plain name (`me().pos`), which only a type could
/// resolve.
fn chain_at(text: &str, at: usize) -> Option<Vec<String>> {
    let chars: Vec<char> = text.chars().collect();
    if !chars.get(at).copied().is_some_and(is_ident) {
        return None;
    }
    let mut start = at;
    while start > 0
        && chars
            .get(start.saturating_sub(1))
            .copied()
            .is_some_and(is_ident)
    {
        start = start.saturating_sub(1);
    }
    let mut end = at;
    while chars.get(end).copied().is_some_and(is_ident) {
        end = end.saturating_add(1);
    }
    if chars.get(start).is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut chain = vec![chars.get(start..end)?.iter().collect::<String>()];
    let mut i = start;
    while i > 0 && chars.get(i.saturating_sub(1)) == Some(&'.') {
        let dot = i.saturating_sub(1);
        let mut s = dot;
        while s > 0
            && chars
                .get(s.saturating_sub(1))
                .copied()
                .is_some_and(is_ident)
        {
            s = s.saturating_sub(1);
        }
        if s == dot || chars.get(s).is_some_and(|c| c.is_ascii_digit()) {
            // `).name`, `].name`, `"x".name`: an attribute of a value.
            return None;
        }
        chain.insert(0, chars.get(s..dot)?.iter().collect());
        i = s;
    }
    Some(chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(files: &[(&str, &str)]) -> Index {
        let mut i = Index::new();
        i.update(files.iter().copied());
        i
    }

    /// Hover at the first occurrence of `marker` in `path`'s text, at the
    /// char `offset` into it.
    fn at(
        i: &Index,
        files: &[(&str, &str)],
        path: &str,
        marker: &str,
        offset: usize,
    ) -> Option<Hover> {
        let text = files.iter().find(|(p, _)| *p == path).unwrap().1;
        let byte = text.find(marker).unwrap_or_else(|| panic!("no {marker:?}"));
        let ch = text[..byte].chars().count().saturating_add(offset);
        i.hover(path, text, ch)
    }

    const NAV: &str = "\"\"\"Getting a bot from here to there.\n\nMore below.\n\"\"\"\n\nfrom geo import step\n\n\ndef go_to(p: tuple[int, int],\n          limit: int = 4) -> bool | None:\n    \"\"\"One step toward p; True once there.\n\n    False after a step.\n    \"\"\"\n    return step(p, 'n')\n";
    const GEO: &str = "def step(p, d):\n    \"\"\"The tile one step from p.\"\"\"\n    return p\n\nclass Base:\n    def hello(self):\n        \"\"\"Says hello.\"\"\"\n        pass\n\nclass Point(Base):\n    \"\"\"A point.\"\"\"\n\n    def norm(self) -> int:\n        return self.hello()\n";
    const BOT: &str = "import nav\nfrom geo import Point as P\nimport geo\n\nnav.go_to((1, 2))\nmove_to(1, 2)\nP.norm\ngeo.step\nP.hello\nlen([1])\nnav.missing\nme().pos\n";

    fn files() -> Vec<(&'static str, &'static str)> {
        vec![
            ("lib/nav.py", NAV),
            ("lib/geo.py", GEO),
            ("robots/bot.py", BOT),
        ]
    }

    #[test]
    fn a_module_function_shows_its_signature_docstring_and_file() {
        let f = files();
        let i = index(&f);
        let h = at(&i, &f, "robots/bot.py", "go_to", 2).expect("go_to");
        assert_eq!(
            h.signature,
            "def go_to(p: tuple[int, int], limit: int = 4) -> bool | None"
        );
        assert_eq!(
            h.doc.as_deref(),
            Some("One step toward p; True once there.\n\nFalse after a step.")
        );
        assert_eq!(
            h.origin,
            Origin::File {
                path: "lib/nav.py".into(),
                line: 9
            }
        );
        // The module's own name shows its docstring.
        let h = at(&i, &f, "robots/bot.py", "nav.go_to", 1).expect("nav");
        assert_eq!(h.signature, "module nav");
        assert_eq!(
            h.doc.as_deref(),
            Some("Getting a bot from here to there.\n\nMore below.")
        );
    }

    #[test]
    fn imports_aliases_classes_and_inherited_methods_resolve() {
        let f = files();
        let i = index(&f);
        // `from geo import step` inside nav, followed to geo.
        let h = at(&i, &f, "lib/nav.py", "step(p", 0).expect("step");
        assert_eq!(h.signature, "def step(p, d)");
        assert_eq!(h.doc.as_deref(), Some("The tile one step from p."));
        // `geo.step` through `import geo`.
        let h = at(&i, &f, "robots/bot.py", "geo.step", 5).expect("geo.step");
        assert_eq!(h.signature, "def step(p, d)");
        // An aliased class and its method, and a method of its base.
        let h = at(&i, &f, "robots/bot.py", "P.norm", 3).expect("P.norm");
        assert_eq!(h.signature, "def norm(self) -> int");
        assert_eq!(h.doc, None);
        let h = at(&i, &f, "robots/bot.py", "P.hello", 3).expect("P.hello");
        assert_eq!(h.doc.as_deref(), Some("Says hello."));
        // `self.hello` inside the class, inherited from Base.
        let h = at(&i, &f, "lib/geo.py", "self.hello", 6).expect("self.hello");
        assert_eq!(h.signature, "def hello(self)");
        // A class's own name and docstring, its base in the header.
        let h = at(&i, &f, "robots/bot.py", "Point as P", 0).expect("Point");
        assert_eq!(h.signature, "class Point(Base)");
        assert_eq!(h.doc.as_deref(), Some("A point."));
        // A method's own name, on its def line.
        let h = at(&i, &f, "lib/geo.py", "norm", 0).expect("def norm");
        assert_eq!(h.signature, "def norm(self) -> int");
    }

    #[test]
    fn builtins_show_their_entry_and_unknowns_show_nothing() {
        let f = files();
        let i = index(&f);
        let h = at(&i, &f, "robots/bot.py", "move_to", 0).expect("move_to");
        assert_eq!(h.signature, "move_to(x: int, y: int) -> None");
        assert_eq!(h.origin, Origin::Game);
        assert!(h.doc.unwrap().contains("ONE step"));
        let h = at(&i, &f, "robots/bot.py", "len", 0).expect("len");
        assert_eq!(h.origin, Origin::Builtin);
        // A name a module lacks, a field of a value, and a literal: nothing.
        assert_eq!(at(&i, &f, "robots/bot.py", "missing", 0), None);
        assert_eq!(at(&i, &f, "robots/bot.py", "pos", 0), None);
        assert_eq!(at(&i, &f, "robots/bot.py", "1, 2))", 0), None);
        // A module-level name hides the builtin it shares a name with.
        let shadow = [("robots/bot.py", "tile = 3\ntile\n")];
        let i = index(&shadow);
        assert_eq!(at(&i, &shadow, "robots/bot.py", "tile\n", 0), None);
    }

    #[test]
    fn a_file_that_stops_parsing_keeps_its_last_outline() {
        let mut f = files();
        let mut i = index(&f);
        let broken = "def go_to(p):\n    \"\"\"Still here.\"\"\"\n    return (\n";
        f[0] = ("lib/nav.py", broken);
        i.update(f.iter().copied());
        // The old outline stands: the hover still finds the docstring.
        let h = at(&i, &f, "robots/bot.py", "go_to", 0).expect("go_to");
        assert!(h.doc.unwrap().starts_with("One step toward p"));
    }

    #[test]
    fn a_robot_file_is_never_a_module() {
        let f = [
            ("robots/printer.py", "def wait_more():\n    pass\n"),
            ("robots/bot.py", "import printer\nprinter.wait_more\n"),
        ];
        let i = index(&f);
        assert_eq!(at(&i, &f, "robots/bot.py", "wait_more", 0), None);
    }

    #[test]
    fn every_builtin_has_an_entry_and_every_entry_is_a_builtin() {
        let docs = BuiltinDocs::parse(DOCS_TOML).expect("docs.toml parses");
        let costs = lang::Costs::parse(lang::data::COSTS_TOML).expect("costs");
        let game: BTreeSet<&str> = costs.game.keys().map(String::as_str).collect();
        let documented: BTreeSet<&str> = docs.game.keys().map(String::as_str).collect();
        assert_eq!(
            documented, game,
            "docs.toml [game] against costs.toml [game]"
        );

        let mut builtins: BTreeSet<&str> = lang::builtins::BUILTINS.iter().copied().collect();
        builtins.insert("tuple");
        let documented: BTreeSet<&str> = docs.builtin.keys().map(String::as_str).collect();
        assert_eq!(
            documented, builtins,
            "docs.toml [builtin] against the language's"
        );

        let classes: BTreeSet<&str> = lang::ExcClass::ALL.iter().map(|c| c.name()).collect();
        let documented: BTreeSet<&str> = docs.exception.keys().map(String::as_str).collect();
        assert_eq!(
            documented, classes,
            "docs.toml [exception] against ExcClass"
        );

        for (name, e) in docs.game.iter().chain(&docs.builtin).chain(&docs.exception) {
            assert!(!e.doc.trim().is_empty(), "{name} has no description");
            assert!(
                e.signature.contains(name.as_str()),
                "{name}'s signature does not name it: {}",
                e.signature
            );
        }
    }

    #[test]
    fn the_starter_library_documents_every_function() {
        // Every function and method the starter tree defines has a
        // docstring for its hover, and each resolves from its own file.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/starter");
        let tree = sim::script::read_tree_files(&root).expect("starter tree");
        for (path, text) in &tree {
            if !path.starts_with("lib/") {
                continue;
            }
            let o = outline(text).unwrap_or_else(|| panic!("{path} does not parse"));
            assert!(o.doc.is_some(), "{path} has no module docstring");
            for (name, d) in &o.defs {
                assert!(d.doc.is_some(), "{path}: {name} has no docstring");
            }
        }
    }
}

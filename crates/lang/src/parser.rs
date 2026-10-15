//! A recursive-descent parser for the procedural core. Anything Python has
//! that is not in `docs/01-language/syntax.md`'s tables is a parse error at
//! load, never a runtime surprise.

use crate::ast::*;
use crate::errors::LoadError;
use crate::lexer::{Lexer, Tok, Token, escape_char};

pub struct Parser<'a> {
    file: &'a str,
    toks: Vec<Token>,
    pos: usize,
    /// Depth of `def` bodies, to refuse a nested `def`.
    def_depth: u32,
    /// Whether a `def` may open here: at the module's top level and directly
    /// in a `class` body, not inside any other compound statement.
    def_allowed: bool,
    /// Whether a `class` may open here: the module's top level only.
    top_level: bool,
    /// Set by `class` for the block it is about to parse.
    next_block_is_class: bool,
    /// Depth of indented blocks, bounded by `max_depth` (`execution.md`,
    /// Limits: "brackets and blocks nested in source have the same bound").
    block_depth: u32,
    max_depth: u32,
}

type P<T> = Result<T, LoadError>;

/// Parse a whole file into its statements, with source nesting unbounded.
pub fn parse_file(file: &str, src: &str) -> P<Vec<Stmt>> {
    parse_file_bounded(file, src, u32::MAX)
}

/// Parse a whole file, refusing brackets or blocks nested deeper than
/// `nesting` levels.
pub fn parse_file_bounded(file: &str, src: &str, nesting: u32) -> P<Vec<Stmt>> {
    let toks = Lexer::new(file, src).nesting(nesting).tokenize()?;
    let mut p = Parser {
        file,
        toks,
        pos: 0,
        def_depth: 0,
        def_allowed: true,
        top_level: true,
        next_block_is_class: false,
        block_depth: 0,
        max_depth: nesting,
    };
    let mut stmts = Vec::new();
    while !p.at(&Tok::Eof) {
        stmts.extend(p.statement()?);
    }
    Ok(stmts)
}

/// Parse a single expression (an f-string's `{…}` segment). Its tokens
/// take the line and the span of the whole f-string.
fn parse_expr_src(file: &str, line: u32, span: Span, src: &str) -> P<Expr> {
    let toks = Lexer::new(file, src).tokenize()?;
    let toks: Vec<Token> = toks
        .into_iter()
        .filter(|t| !matches!(t.tok, Tok::Newline | Tok::Indent | Tok::Dedent))
        .map(|mut t| {
            t.line = line;
            t.start = span.start;
            t.end = span.end;
            t
        })
        .collect();
    let mut p = Parser {
        file,
        toks,
        pos: 0,
        def_depth: 1,
        def_allowed: false,
        top_level: false,
        next_block_is_class: false,
        block_depth: 0,
        max_depth: u32::MAX,
    };
    let e = p.expr()?;
    if !p.at(&Tok::Eof) {
        return Err(p.err("an f-string field holds one expression"));
    }
    Ok(e)
}

impl<'a> Parser<'a> {
    fn err(&self, message: impl Into<String>) -> LoadError {
        LoadError {
            file: self.file.to_string(),
            line: self.line(),
            message: message.into(),
        }
    }

    fn line(&self) -> u32 {
        self.toks.get(self.pos).map(|t| t.line).unwrap_or(0)
    }

    /// Where the next token begins.
    fn start(&self) -> u32 {
        self.toks.get(self.pos).map_or(0, |t| t.start)
    }

    /// From `start` to the end of the last token read.
    fn since(&self, start: u32) -> Span {
        let end = self
            .pos
            .checked_sub(1)
            .and_then(|i| self.toks.get(i))
            .map_or(start, |t| t.end);
        Span {
            start,
            end: end.max(start),
        }
    }

    fn peek(&self) -> &Tok {
        self.toks.get(self.pos).map(|t| &t.tok).unwrap_or(&Tok::Eof)
    }

    fn peek_at(&self, k: usize) -> &Tok {
        self.toks
            .get(self.pos.wrapping_add(k))
            .map(|t| &t.tok)
            .unwrap_or(&Tok::Eof)
    }

    fn at(&self, t: &Tok) -> bool {
        self.peek() == t
    }

    fn at_op(&self, op: &str) -> bool {
        matches!(self.peek(), Tok::Op(o) if *o == op)
    }

    fn at_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Keyword(k) if *k == kw)
    }

    fn bump(&mut self) -> Tok {
        let t = self.peek().clone();
        if !matches!(t, Tok::Eof) {
            self.pos = self.pos.wrapping_add(1);
        }
        t
    }

    fn eat_op(&mut self, op: &str) -> bool {
        if self.at_op(op) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.at_kw(kw) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_op(&mut self, op: &str) -> P<()> {
        if self.eat_op(op) {
            Ok(())
        } else {
            Err(self.err(format!("expected `{op}`")))
        }
    }

    fn expect_kw(&mut self, kw: &str) -> P<()> {
        if self.eat_kw(kw) {
            Ok(())
        } else {
            Err(self.err(format!("expected `{kw}`")))
        }
    }

    fn expect_name(&mut self) -> P<String> {
        match self.bump() {
            Tok::Name(n) => Ok(n),
            Tok::Keyword(k) => Err(self.err(format!("`{k}` is a reserved word"))),
            _ => Err(self.err("expected a name")),
        }
    }

    fn expect_newline(&mut self) -> P<()> {
        if self.at(&Tok::Newline) {
            self.bump();
            Ok(())
        } else if self.at(&Tok::Eof) {
            Ok(())
        } else {
            Err(self.err("expected the end of the line"))
        }
    }

    // ------------------------------------------------------ statements ---

    fn block(&mut self) -> P<Vec<Stmt>> {
        let saved = (self.def_allowed, self.top_level);
        self.def_allowed = self.next_block_is_class;
        self.next_block_is_class = false;
        self.top_level = false;
        self.block_depth = self.block_depth.wrapping_add(1);
        let out = if self.block_depth > self.max_depth {
            Err(self.err(format!(
                "blocks nest deeper than {} levels, the nesting-depth limit",
                self.max_depth
            )))
        } else {
            self.block_inner()
        };
        self.block_depth = self.block_depth.wrapping_sub(1);
        (self.def_allowed, self.top_level) = saved;
        out
    }

    fn block_inner(&mut self) -> P<Vec<Stmt>> {
        self.expect_op(":")?;
        if self.at(&Tok::Newline) {
            self.bump();
            if !self.at(&Tok::Indent) {
                return Err(self.err("expected an indented block"));
            }
            self.bump();
            let mut stmts = Vec::new();
            while !self.at(&Tok::Dedent) && !self.at(&Tok::Eof) {
                stmts.extend(self.statement()?);
            }
            self.bump();
            Ok(stmts)
        } else {
            // A simple statement on the same line.
            self.simple_line()
        }
    }

    /// One logical line of simple statements, `;`-separated.
    fn simple_line(&mut self) -> P<Vec<Stmt>> {
        let mut out = vec![self.simple_statement()?];
        while self.eat_op(";") {
            if self.at(&Tok::Newline) || self.at(&Tok::Eof) {
                break;
            }
            out.push(self.simple_statement()?);
        }
        self.expect_newline()?;
        Ok(out)
    }

    fn statement(&mut self) -> P<Vec<Stmt>> {
        let line = self.line();
        if self.at(&Tok::Indent) {
            return Err(self.err("unexpected indent"));
        }
        let kind = match self.peek().clone() {
            Tok::Keyword("if") => {
                self.bump();
                Some(self.if_stmt()?)
            }
            Tok::Keyword("while") => {
                self.bump();
                let cond = self.expr()?;
                let body = self.block()?;
                let orelse = if self.eat_kw("else") {
                    self.block()?
                } else {
                    vec![]
                };
                Some(StmtKind::While { cond, body, orelse })
            }
            Tok::Keyword("for") => {
                self.bump();
                let target = self.target_list()?;
                self.expect_kw("in")?;
                let iter = self.expr_list()?;
                let body = self.block()?;
                let orelse = if self.eat_kw("else") {
                    self.block()?
                } else {
                    vec![]
                };
                Some(StmtKind::For {
                    target,
                    iter,
                    body,
                    orelse,
                })
            }
            Tok::Keyword("def") => {
                self.bump();
                if self.def_depth > 0 {
                    return Err(self.err(
                        "functions do not nest: a `def` inside a `def` is not in the language",
                    ));
                }
                if !self.def_allowed {
                    return Err(self.err(
                        "a `def` belongs at the module's top level or directly in a `class` body, not inside a compound statement",
                    ));
                }
                let name = self.expect_name()?;
                self.expect_op("(")?;
                let params = self.params(")")?;
                let returns = if self.eat_op("->") {
                    Some(self.annotation()?)
                } else {
                    None
                };
                self.def_depth = self.def_depth.wrapping_add(1);
                let body = self.block();
                self.def_depth = self.def_depth.wrapping_sub(1);
                Some(StmtKind::Def {
                    name,
                    params,
                    returns,
                    body: body?,
                })
            }
            Tok::Keyword("try") => {
                self.bump();
                Some(self.try_stmt()?)
            }
            Tok::Keyword("class") => {
                self.bump();
                if !self.top_level {
                    return Err(self.err("a `class` belongs at the module's top level only"));
                }
                let name = self.expect_name()?;
                let base = if self.eat_op("(") {
                    if self.eat_op(")") {
                        None
                    } else {
                        let b = self.expr()?;
                        if self.eat_op(",") {
                            return Err(self.err(
                                "multiple inheritance is not in the language: a class has one base or none",
                            ));
                        }
                        self.expect_op(")")?;
                        Some(b)
                    }
                } else {
                    None
                };
                self.next_block_is_class = true;
                let body = self.block()?;
                for st in &body {
                    match &st.kind {
                        StmtKind::Def { .. }
                        | StmtKind::Assign { .. }
                        | StmtKind::AnnAssign { .. }
                        | StmtKind::AugAssign { .. }
                        | StmtKind::Pass
                        | StmtKind::Expr(_) => {}
                        _ => {
                            return Err(LoadError {
                                file: self.file.to_string(),
                                line: st.line,
                                message: "a `class` body holds `def`s, assignments, `pass` and expression statements only".into(),
                            });
                        }
                    }
                }
                Some(StmtKind::Class { name, base, body })
            }
            Tok::Keyword("match") => {
                self.bump();
                Some(self.match_stmt()?)
            }
            Tok::Keyword("import") => {
                self.bump();
                let module = self.module_name()?;
                let alias = if self.eat_kw("as") {
                    Some(self.expect_name()?)
                } else {
                    None
                };
                if self.at(&Tok::Op(",")) {
                    return Err(self.err("one module per `import`; the forms are `import name` and `from name import a, b`"));
                }
                self.expect_newline()?;
                return Ok(vec![Stmt {
                    line,
                    kind: StmtKind::Import { module, alias },
                }]);
            }
            Tok::Keyword("from") => {
                self.bump();
                if self.at(&Tok::Op(".")) {
                    return Err(self.err("relative imports are not in the language"));
                }
                let module = self.module_name()?;
                if !self.eat_kw("import") {
                    return Err(self.err("expected `import`"));
                }
                if self.eat_op("*") {
                    return Err(self
                        .err("`from name import *` is not in the language; name what you import"));
                }
                let mut names = Vec::new();
                loop {
                    let n = self.expect_name()?;
                    let alias = if self.eat_kw("as") {
                        Some(self.expect_name()?)
                    } else {
                        None
                    };
                    names.push((n, alias));
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_newline()?;
                return Ok(vec![Stmt {
                    line,
                    kind: StmtKind::FromImport { module, names },
                }]);
            }
            Tok::Keyword(
                k @ ("del" | "assert" | "with" | "async" | "await" | "yield" | "global"
                | "nonlocal"),
            ) => {
                return Err(self.err(format!("`{k}` is not in the language")));
            }
            _ => None,
        };
        match kind {
            Some(kind) => Ok(vec![Stmt { line, kind }]),
            None => self.simple_line(),
        }
    }

    fn if_stmt(&mut self) -> P<StmtKind> {
        let cond = self.expr()?;
        let body = self.block()?;
        let orelse = if self.at_kw("elif") {
            let line = self.line();
            self.bump();
            vec![Stmt {
                line,
                kind: self.if_stmt()?,
            }]
        } else if self.eat_kw("else") {
            self.block()?
        } else {
            vec![]
        };
        Ok(StmtKind::If { cond, body, orelse })
    }

    fn try_stmt(&mut self) -> P<StmtKind> {
        let body = self.block()?;
        let mut handlers = Vec::new();
        while self.at_kw("except") {
            let line = self.line();
            self.bump();
            let (types, name) = if self.at_op(":") {
                (None, None)
            } else {
                let types = self.expr()?;
                let name = if self.eat_kw("as") {
                    Some(self.expect_name()?)
                } else {
                    None
                };
                (Some(types), name)
            };
            let body = self.block()?;
            handlers.push(Handler {
                line,
                types,
                name,
                body,
            });
        }
        let orelse = if self.eat_kw("else") {
            if handlers.is_empty() {
                return Err(self.err("`else` on a `try` needs an `except`"));
            }
            self.block()?
        } else {
            vec![]
        };
        let finalbody = if self.eat_kw("finally") {
            self.block()?
        } else {
            vec![]
        };
        if handlers.is_empty() && finalbody.is_empty() {
            return Err(self.err("a `try` needs an `except` or a `finally`"));
        }
        Ok(StmtKind::Try {
            body,
            handlers,
            orelse,
            finalbody,
        })
    }

    fn params(&mut self, close: &str) -> P<Params> {
        let mut params = Params::default();
        let mut seen_default = false;
        loop {
            if self.eat_op(close) {
                break;
            }
            if self.eat_op("**") {
                params.dstar = Some(self.expect_name()?);
                params.dstar_annotation = self.param_annotation(close)?;
                self.eat_op(",");
                self.expect_op(close)?;
                break;
            }
            if self.eat_op("*") {
                if params.star.is_some() {
                    return Err(self.err("one `*args` per function"));
                }
                params.star = Some(self.expect_name()?);
                params.star_annotation = self.param_annotation(close)?;
            } else {
                let name = self.expect_name()?;
                if params.names.contains(&name) {
                    return Err(self.err(format!("duplicate parameter `{name}`")));
                }
                let annotation = self.param_annotation(close)?;
                let default = if self.eat_op("=") {
                    seen_default = true;
                    Some(self.expr()?)
                } else {
                    if seen_default {
                        return Err(
                            self.err("a parameter without a default follows one with a default")
                        );
                    }
                    None
                };
                if params.star.is_some() {
                    return Err(self.err("keyword-only parameters are not in the language"));
                }
                params.names.push(name);
                params.defaults.push(default);
                params.annotations.push(annotation);
            }
            if !self.eat_op(",") {
                self.expect_op(close)?;
                break;
            }
        }
        Ok(params)
    }

    /// A parameter's `: T`, in a `def` only: a `lambda`'s parameter list
    /// ends at its `:`, so it can have none, as in Python.
    fn param_annotation(&mut self, close: &str) -> P<Option<Annotation>> {
        if close == ")" && self.eat_op(":") {
            Ok(Some(self.annotation()?))
        } else {
            Ok(None)
        }
    }

    // --------------------------------------------------- annotations ---

    /// A type annotation (Q56, `syntax.md` Annotations): a name, a
    /// `module.Name`, `None`, a string, a name subscripted with types, or
    /// a union of these with `|`. Read and kept, never evaluated, and never
    /// checked against the names in scope: that is the editor's.
    fn annotation(&mut self) -> P<Annotation> {
        let line = self.line();
        let start = self.start();
        let ty = self.type_union(false)?;
        Ok(Annotation {
            line,
            span: self.since(start),
            ty,
        })
    }

    fn type_union(&mut self, in_subscript: bool) -> P<TypeExpr> {
        let first = self.type_atom(in_subscript)?;
        if !self.at_op("|") {
            return Ok(first);
        }
        let mut arms = vec![first];
        while self.eat_op("|") {
            arms.push(self.type_atom(false)?);
        }
        Ok(TypeExpr::Union(arms))
    }

    fn type_atom(&mut self, in_subscript: bool) -> P<TypeExpr> {
        let base = match self.peek().clone() {
            Tok::Name(n) => {
                self.bump();
                if self.eat_op(".") {
                    TypeExpr::Attr(n, self.expect_name()?)
                } else {
                    TypeExpr::Name(n)
                }
            }
            Tok::Keyword("None") => {
                self.bump();
                return Ok(TypeExpr::None);
            }
            Tok::Str(s) => {
                self.bump();
                return Ok(TypeExpr::Str(s));
            }
            Tok::Op("...") if in_subscript => {
                self.bump();
                return Ok(TypeExpr::Ellipsis);
            }
            _ => {
                return Err(self.err(
                    "an annotation is a type: a name, `None`, a string, `name[…]` or `A | B`",
                ));
            }
        };
        if !self.eat_op("[") {
            return Ok(base);
        }
        let mut args = Vec::new();
        loop {
            args.push(self.type_union(true)?);
            if !self.eat_op(",") || self.at_op("]") {
                break;
            }
        }
        self.expect_op("]")?;
        Ok(TypeExpr::Generic(Box::new(base), args))
    }

    /// A module name: one identifier, never dotted.
    fn module_name(&mut self) -> P<String> {
        let n = self.expect_name()?;
        if self.at(&Tok::Op(".")) {
            return Err(self.err("dotted module names and packages are not in the language"));
        }
        Ok(n)
    }

    // --------------------------------------------------------- match ---

    fn match_stmt(&mut self) -> P<StmtKind> {
        if self.at(&Tok::Op("=")) || self.at(&Tok::Op(".")) || self.at(&Tok::Op("(")) {
            return Err(self.err("`match` is a reserved word"));
        }
        let subject = self.expr()?;
        self.expect_op(":")?;
        if !self.at(&Tok::Newline) {
            return Err(self.err("`match` takes a block of `case` arms"));
        }
        self.bump();
        if !self.at(&Tok::Indent) {
            return Err(self.err("expected an indented block of `case` arms"));
        }
        self.bump();
        let mut arms = Vec::new();
        while self.eat_kw("case") {
            let line = self.line();
            let pattern = self.pattern()?;
            let mut names = Vec::new();
            pattern.bound_names(&mut names);
            let mut seen: Vec<&String> = Vec::new();
            for n in &names {
                if seen.contains(&n) {
                    return Err(self.err(format!("a pattern binds `{n}` twice")));
                }
                seen.push(n);
            }
            let guard = if self.eat_kw("if") {
                Some(self.expr()?)
            } else {
                None
            };
            let body = self.block()?;
            arms.push(Arm {
                pattern,
                guard,
                body,
                line,
            });
        }
        if arms.is_empty() {
            return Err(self.err("a `match` needs at least one `case`"));
        }
        if !self.at(&Tok::Dedent) {
            return Err(self.err("a `match` block holds `case` arms only"));
        }
        self.bump();
        Ok(StmtKind::Match { subject, arms })
    }

    /// An or-pattern, then an optional `as name`.
    fn pattern(&mut self) -> P<Pattern> {
        let mut alts = vec![self.closed_pattern()?];
        while self.eat_op("|") {
            alts.push(self.closed_pattern()?);
        }
        let mut p = if alts.len() == 1 {
            alts.pop().unwrap_or(Pattern::Wildcard)
        } else {
            // Alternatives must bind the same names (`syntax.md`).
            let mut first = Vec::new();
            alts[0].bound_names(&mut first);
            first.sort();
            for alt in &alts[1..] {
                let mut names = Vec::new();
                alt.bound_names(&mut names);
                names.sort();
                if names != first {
                    return Err(
                        self.err("the alternatives of a `|` pattern must bind the same names")
                    );
                }
            }
            Pattern::Or(alts)
        };
        if self.eat_kw("as") {
            let n = self.expect_name()?;
            if n == "_" {
                return Err(self.err("`as _` binds nothing; drop the `as`"));
            }
            p = Pattern::As(Box::new(p), n);
        }
        Ok(p)
    }

    fn literal_pattern(&mut self) -> P<Option<Expr>> {
        let line = self.line();
        let start = self.start();
        let kind = match self.peek().clone() {
            Tok::Num(n) => {
                self.bump();
                ExprKind::Num(n)
            }
            Tok::Op("-") => {
                if let Tok::Num(n) = self.peek_at(1).clone() {
                    self.bump();
                    self.bump();
                    ExprKind::Num(n.checked_neg().map_err(|_| self.err("literal"))?)
                } else {
                    return Err(self.err("`-` in a pattern precedes a number"));
                }
            }
            Tok::Str(s) => {
                self.bump();
                let mut s = s;
                while let Tok::Str(next) = self.peek().clone() {
                    self.bump();
                    s.push_str(&next);
                }
                ExprKind::Str(s)
            }
            Tok::Keyword("True") => {
                self.bump();
                ExprKind::Bool(true)
            }
            Tok::Keyword("False") => {
                self.bump();
                ExprKind::Bool(false)
            }
            Tok::Keyword("None") => {
                self.bump();
                ExprKind::None
            }
            _ => return Ok(None),
        };
        Ok(Some(Expr {
            kind,
            line,
            span: self.since(start),
        }))
    }

    fn closed_pattern(&mut self) -> P<Pattern> {
        if let Some(lit) = self.literal_pattern()? {
            return Ok(Pattern::Literal(lit));
        }
        let line = self.line();
        let start = self.start();
        match self.peek().clone() {
            Tok::Name(n) if n == "_" => {
                self.bump();
                Ok(Pattern::Wildcard)
            }
            Tok::Name(n) => {
                self.bump();
                let name_span = self.since(start);
                if self.eat_op("(") {
                    let mut kwargs = Vec::new();
                    while !self.at(&Tok::Op(")")) {
                        let Tok::Name(k) = self.peek().clone() else {
                            return Err(self.err(
                                "a class pattern takes keyword sub-patterns only: positional ones need `__match_args__`, which does not exist",
                            ));
                        };
                        if !matches!(self.peek_at(1), Tok::Op("=")) {
                            return Err(self.err(
                                "a class pattern takes keyword sub-patterns only: positional ones need `__match_args__`, which does not exist",
                            ));
                        }
                        self.bump();
                        self.bump();
                        if kwargs.iter().any(|(name, _)| *name == k) {
                            return Err(self.err(format!("`{k}` appears twice in a class pattern")));
                        }
                        kwargs.push((k, self.pattern()?));
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.expect_op(")")?;
                    return Ok(Pattern::Class {
                        class: Expr {
                            kind: ExprKind::Name(n),
                            line,
                            span: name_span,
                        },
                        kwargs,
                    });
                }
                if self.at(&Tok::Op(".")) {
                    return Err(self
                        .err("a dotted value pattern is not in the language; compare in a guard"));
                }
                Ok(Pattern::Capture(n))
            }
            Tok::Op("*") => {
                self.bump();
                let n = self.expect_name()?;
                Ok(Pattern::Star(if n == "_" { None } else { Some(n) }))
            }
            Tok::Op("[") => {
                self.bump();
                let items = self.sequence_items("]")?;
                self.checked_sequence(items)
            }
            Tok::Op("(") => {
                self.bump();
                if self.eat_op(")") {
                    return Ok(Pattern::Sequence(vec![]));
                }
                let first = self.pattern_or_star()?;
                if self.eat_op(",") {
                    let mut items = vec![first];
                    items.extend(self.sequence_items(")")?);
                    return self.checked_sequence(items);
                }
                self.expect_op(")")?;
                if matches!(first, Pattern::Star(_)) {
                    return Err(self.err("a `*` pattern belongs inside a sequence pattern"));
                }
                Ok(first)
            }
            Tok::Op("{") => {
                self.bump();
                let mut pairs = Vec::new();
                let mut rest = None;
                while !self.at(&Tok::Op("}")) {
                    if self.eat_op("**") {
                        let n = self.expect_name()?;
                        if rest.is_some() {
                            return Err(self.err("one `**rest` per mapping pattern"));
                        }
                        rest = Some(n);
                    } else {
                        let Some(key) = self.literal_pattern()? else {
                            return Err(self.err("a mapping pattern's keys are literals"));
                        };
                        self.expect_op(":")?;
                        pairs.push((key, self.pattern()?));
                    }
                    if !self.eat_op(",") {
                        break;
                    }
                }
                self.expect_op("}")?;
                Ok(Pattern::Mapping { pairs, rest })
            }
            Tok::Keyword(k) => Err(self.err(format!("`{k}` cannot start a pattern"))),
            _ => Err(self.err("expected a pattern")),
        }
    }

    fn pattern_or_star(&mut self) -> P<Pattern> {
        if self.at(&Tok::Op("*")) {
            self.closed_pattern()
        } else {
            self.pattern()
        }
    }

    /// Comma-separated patterns up to `close`, which is consumed.
    fn sequence_items(&mut self, close: &'static str) -> P<Vec<Pattern>> {
        let mut items = Vec::new();
        while !self.at(&Tok::Op(close)) {
            items.push(self.pattern_or_star()?);
            if !self.eat_op(",") {
                break;
            }
        }
        self.expect_op(close)?;
        Ok(items)
    }

    fn checked_sequence(&self, items: Vec<Pattern>) -> P<Pattern> {
        let stars = items
            .iter()
            .filter(|p| matches!(p, Pattern::Star(_)))
            .count();
        if stars > 1 {
            return Err(self.err("one `*` per sequence pattern"));
        }
        Ok(Pattern::Sequence(items))
    }

    fn simple_statement(&mut self) -> P<Stmt> {
        let line = self.line();
        let kind = match self.peek().clone() {
            Tok::Keyword("pass") => {
                self.bump();
                StmtKind::Pass
            }
            Tok::Keyword("break") => {
                self.bump();
                StmtKind::Break
            }
            Tok::Keyword("continue") => {
                self.bump();
                StmtKind::Continue
            }
            Tok::Keyword("return") => {
                self.bump();
                if self.def_depth == 0 {
                    return Err(self.err("`return` outside a function"));
                }
                if self.at(&Tok::Newline) || self.at(&Tok::Eof) || self.at_op(";") {
                    StmtKind::Return(None)
                } else {
                    StmtKind::Return(Some(self.expr_list()?))
                }
            }
            Tok::Keyword("raise") => {
                self.bump();
                if self.at(&Tok::Newline) || self.at(&Tok::Eof) || self.at_op(";") {
                    StmtKind::Raise(None)
                } else {
                    StmtKind::Raise(Some(self.expr()?))
                }
            }
            _ => {
                let first = self.expr_list()?;
                if self.eat_op(":") {
                    let target = self.to_target(first)?;
                    if matches!(target, Target::Tuple(..)) {
                        return Err(self.err("an annotated assignment takes a single target"));
                    }
                    let annotation = self.annotation()?;
                    let value = if self.eat_op("=") {
                        Some(self.expr_list()?)
                    } else {
                        None
                    };
                    if self.at_op("=") {
                        return Err(self.err("an annotated assignment takes a single target"));
                    }
                    StmtKind::AnnAssign {
                        target,
                        annotation,
                        value,
                    }
                } else if self.at_op("=") {
                    let mut targets = vec![self.to_target(first)?];
                    let mut value;
                    loop {
                        self.bump();
                        value = self.expr_list()?;
                        if self.at_op("=") {
                            targets.push(self.to_target(value)?);
                        } else {
                            break;
                        }
                    }
                    StmtKind::Assign { targets, value }
                } else if let Tok::Op(op) = self.peek().clone()
                    && let Some(bin) = aug_op(op)
                {
                    self.bump();
                    let target = self.to_target(first)?;
                    if matches!(target, Target::Tuple(..)) {
                        return Err(self.err("augmented assignment needs a single target"));
                    }
                    let value = self.expr_list()?;
                    StmtKind::AugAssign {
                        target,
                        op: bin,
                        value,
                    }
                } else {
                    StmtKind::Expr(first)
                }
            }
        };
        Ok(Stmt { line, kind })
    }

    fn to_target(&self, e: Expr) -> P<Target> {
        let line = e.line;
        let bad = |m: &str| LoadError {
            file: self.file.to_string(),
            line,
            message: m.to_string(),
        };
        Ok(match e.kind {
            ExprKind::Name(n) => Target::Name(n),
            ExprKind::Subscript(obj, idx) => Target::Subscript(obj, idx),
            ExprKind::Attr(obj, name) => Target::Attr(obj, name),
            ExprKind::Tuple(items) | ExprKind::List(items) => {
                let mut targets = Vec::new();
                let mut star = None;
                for (i, item) in items.into_iter().enumerate() {
                    if let ExprKind::Starred(inner) = item.kind {
                        if star.is_some() {
                            return Err(bad("one `*name` per unpacking"));
                        }
                        star = Some(i);
                        targets.push(self.to_target(*inner)?);
                    } else {
                        targets.push(self.to_target(item)?);
                    }
                }
                Target::Tuple(targets, star)
            }
            _ => return Err(bad("cannot assign to this expression")),
        })
    }

    /// A `for` target: a name, or a tuple of targets without parentheses.
    fn target_list(&mut self) -> P<Target> {
        let e = self.expr_list_no_in()?;
        self.to_target(e)
    }

    // ----------------------------------------------------- expressions ---

    /// `a, b, c` — a tuple if there is a comma, else the expression.
    fn expr_list(&mut self) -> P<Expr> {
        let line = self.line();
        let first = self.expr_or_star()?;
        if !self.at_op(",") {
            return Ok(first);
        }
        let mut items = vec![first];
        while self.eat_op(",") {
            if self.expr_ends() {
                break;
            }
            items.push(self.expr_or_star()?);
        }
        let span = self.since(items.first().map_or(0, |e| e.span.start));
        Ok(Expr {
            line,
            span,
            kind: ExprKind::Tuple(items),
        })
    }

    /// Like `expr_list` but the `in` of a `for` ends it: parses at the
    /// bitwise-or level so `in` is not read as a comparison.
    fn expr_list_no_in(&mut self) -> P<Expr> {
        let line = self.line();
        let first = self.star_or(Self::bit_or)?;
        if !self.at_op(",") {
            return Ok(first);
        }
        let mut items = vec![first];
        while self.eat_op(",") {
            if self.at_kw("in") {
                break;
            }
            items.push(self.star_or(Self::bit_or)?);
        }
        let span = self.since(items.first().map_or(0, |e| e.span.start));
        Ok(Expr {
            line,
            span,
            kind: ExprKind::Tuple(items),
        })
    }

    fn expr_ends(&self) -> bool {
        matches!(self.peek(), Tok::Newline | Tok::Eof)
            || matches!(self.peek(), Tok::Op(")" | "]" | "}" | "=" | ";" | ":"))
            || matches!(self.peek(), Tok::Op(o) if aug_op(o).is_some())
    }

    fn expr_or_star(&mut self) -> P<Expr> {
        self.star_or(Self::expr)
    }

    fn star_or(&mut self, inner: fn(&mut Self) -> P<Expr>) -> P<Expr> {
        let line = self.line();
        let start = self.start();
        if self.eat_op("*") {
            let e = inner(self)?;
            return Ok(Expr {
                line,
                span: self.since(start),
                kind: ExprKind::Starred(Box::new(e)),
            });
        }
        inner(self)
    }

    pub fn expr(&mut self) -> P<Expr> {
        if self.at_kw("lambda") {
            return self.lambda();
        }
        let line = self.line();
        let then = self.or_test()?;
        if self.eat_kw("if") {
            let cond = self.or_test()?;
            self.expect_kw("else")?;
            let orelse = self.expr()?;
            return Ok(Expr {
                line,
                span: self.since(then.span.start),
                kind: ExprKind::IfExp {
                    cond: Box::new(cond),
                    then: Box::new(then),
                    orelse: Box::new(orelse),
                },
            });
        }
        Ok(then)
    }

    fn lambda(&mut self) -> P<Expr> {
        let line = self.line();
        let start = self.start();
        self.expect_kw("lambda")?;
        let params = self.params(":")?;
        let body = self.expr()?;
        Ok(Expr {
            line,
            span: self.since(start),
            kind: ExprKind::Lambda {
                params: Box::new(params),
                body: Box::new(body),
            },
        })
    }

    fn or_test(&mut self) -> P<Expr> {
        let mut left = self.and_test()?;
        while self.at_kw("or") {
            let line = self.line();
            self.bump();
            let right = self.and_test()?;
            left = Expr {
                line,
                span: self.since(left.span.start),
                kind: ExprKind::BoolOp {
                    is_and: false,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn and_test(&mut self) -> P<Expr> {
        let mut left = self.not_test()?;
        while self.at_kw("and") {
            let line = self.line();
            self.bump();
            let right = self.not_test()?;
            left = Expr {
                line,
                span: self.since(left.span.start),
                kind: ExprKind::BoolOp {
                    is_and: true,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn not_test(&mut self) -> P<Expr> {
        let line = self.line();
        let start = self.start();
        if self.eat_kw("not") {
            let e = self.not_test()?;
            return Ok(Expr {
                line,
                span: self.since(start),
                kind: ExprKind::UnaryOp(UnaryOp::Not, Box::new(e)),
            });
        }
        self.comparison()
    }

    fn comparison(&mut self) -> P<Expr> {
        let line = self.line();
        let first = self.bit_or()?;
        let mut rest = Vec::new();
        loop {
            let op = match self.peek() {
                Tok::Op("<") => CmpOp::Lt,
                Tok::Op(">") => CmpOp::Gt,
                Tok::Op("<=") => CmpOp::Le,
                Tok::Op(">=") => CmpOp::Ge,
                Tok::Op("==") => CmpOp::Eq,
                Tok::Op("!=") => CmpOp::Ne,
                Tok::Keyword("in") => CmpOp::In,
                Tok::Keyword("not") if matches!(self.peek_at(1), Tok::Keyword("in")) => {
                    self.bump();
                    CmpOp::NotIn
                }
                Tok::Keyword("is") => {
                    if matches!(self.peek_at(1), Tok::Keyword("not")) {
                        self.bump();
                        CmpOp::IsNot
                    } else {
                        CmpOp::Is
                    }
                }
                _ => break,
            };
            self.bump();
            let right = self.bit_or()?;
            if matches!(op, CmpOp::Is | CmpOp::IsNot) && !matches!(right.kind, ExprKind::None) {
                return Err(self.err("`is` compares against `None` only"));
            }
            rest.push((op, right));
        }
        if rest.is_empty() {
            return Ok(first);
        }
        Ok(Expr {
            line,
            span: self.since(first.span.start),
            kind: ExprKind::Compare {
                first: Box::new(first),
                rest,
            },
        })
    }

    fn bit_or(&mut self) -> P<Expr> {
        let mut left = self.bit_xor()?;
        while self.at_op("|") {
            let line = self.line();
            self.bump();
            let right = self.bit_xor()?;
            left = binop(line, left, BinOp::BitOr, right);
        }
        Ok(left)
    }

    fn bit_xor(&mut self) -> P<Expr> {
        let mut left = self.bit_and()?;
        while self.at_op("^") {
            let line = self.line();
            self.bump();
            let right = self.bit_and()?;
            left = binop(line, left, BinOp::BitXor, right);
        }
        Ok(left)
    }

    fn bit_and(&mut self) -> P<Expr> {
        let mut left = self.arith()?;
        while self.at_op("&") {
            let line = self.line();
            self.bump();
            let right = self.arith()?;
            left = binop(line, left, BinOp::BitAnd, right);
        }
        Ok(left)
    }

    fn arith(&mut self) -> P<Expr> {
        let mut left = self.term()?;
        loop {
            let op = match self.peek() {
                Tok::Op("+") => BinOp::Add,
                Tok::Op("-") => BinOp::Sub,
                _ => break,
            };
            let line = self.line();
            self.bump();
            let right = self.term()?;
            left = binop(line, left, op, right);
        }
        Ok(left)
    }

    fn term(&mut self) -> P<Expr> {
        let mut left = self.factor()?;
        loop {
            let op = match self.peek() {
                Tok::Op("*") => BinOp::Mul,
                Tok::Op("/") => BinOp::Div,
                Tok::Op("//") => BinOp::FloorDiv,
                Tok::Op("%") => BinOp::Mod,
                _ => break,
            };
            let line = self.line();
            self.bump();
            let right = self.factor()?;
            left = binop(line, left, op, right);
        }
        Ok(left)
    }

    fn factor(&mut self) -> P<Expr> {
        let line = self.line();
        let start = self.start();
        let op = match self.peek() {
            Tok::Op("-") => Some(UnaryOp::Neg),
            Tok::Op("+") => Some(UnaryOp::Pos),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let e = self.factor()?;
            return Ok(Expr {
                line,
                span: self.since(start),
                kind: ExprKind::UnaryOp(op, Box::new(e)),
            });
        }
        self.power()
    }

    fn power(&mut self) -> P<Expr> {
        let base = self.postfix()?;
        if self.at_op("**") {
            let line = self.line();
            self.bump();
            let exp = self.factor()?;
            return Ok(binop(line, base, BinOp::Pow, exp));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> P<Expr> {
        let mut e = self.atom()?;
        loop {
            let line = self.line();
            if self.eat_op("(") {
                let args = self.call_args()?;
                e = Expr {
                    line,
                    span: self.since(e.span.start),
                    kind: ExprKind::Call {
                        func: Box::new(e),
                        args,
                    },
                };
            } else if self.eat_op("[") {
                let idx = self.subscript()?;
                self.expect_op("]")?;
                e = Expr {
                    line,
                    span: self.since(e.span.start),
                    kind: ExprKind::Subscript(Box::new(e), Box::new(idx)),
                };
            } else if self.eat_op(".") {
                let name = self.expect_name()?;
                e = Expr {
                    line,
                    span: self.since(e.span.start),
                    kind: ExprKind::Attr(Box::new(e), name),
                };
            } else {
                return Ok(e);
            }
        }
    }

    fn subscript(&mut self) -> P<Expr> {
        let line = self.line();
        let start = self.start();
        let lo = if self.at_op(":") {
            None
        } else {
            Some(Box::new(self.expr()?))
        };
        if !self.at_op(":") {
            return lo.map(|b| *b).ok_or_else(|| self.err("expected an index"));
        }
        self.bump();
        let hi = if self.at_op("]") || self.at_op(":") {
            None
        } else {
            Some(Box::new(self.expr()?))
        };
        let step = if self.eat_op(":") {
            if self.at_op("]") {
                None
            } else {
                Some(Box::new(self.expr()?))
            }
        } else {
            None
        };
        Ok(Expr {
            line,
            span: self.since(start),
            kind: ExprKind::Slice { lo, hi, step },
        })
    }

    fn call_args(&mut self) -> P<CallArgs> {
        let mut args = CallArgs {
            positional: vec![],
            keywords: vec![],
            star: None,
            dstar: None,
        };
        loop {
            if self.eat_op(")") {
                break;
            }
            if self.eat_op("**") {
                args.dstar = Some(Box::new(self.expr()?));
            } else if self.eat_op("*") {
                if args.star.is_some() {
                    return Err(self.err("one `*args` per call"));
                }
                args.star = Some(Box::new(self.expr()?));
            } else if matches!(self.peek(), Tok::Name(_)) && matches!(self.peek_at(1), Tok::Op("="))
            {
                let name = self.expect_name()?;
                self.bump();
                let value = self.expr()?;
                if args.keywords.iter().any(|(k, _)| *k == name) {
                    return Err(self.err(format!("keyword argument `{name}` repeated")));
                }
                args.keywords.push((name, value));
            } else {
                if !args.keywords.is_empty() || args.star.is_some() {
                    return Err(self.err("positional argument after keyword or `*` argument"));
                }
                args.positional.push(self.expr()?);
            }
            if !self.eat_op(",") {
                self.expect_op(")")?;
                break;
            }
        }
        Ok(args)
    }

    fn atom(&mut self) -> P<Expr> {
        let line = self.line();
        let start = self.start();
        let kind = match self.bump() {
            Tok::Num(n) => ExprKind::Num(n),
            Tok::Str(s) => {
                let mut s = s;
                // Adjacent string literals concatenate.
                while let Tok::Str(next) = self.peek().clone() {
                    self.bump();
                    s.push_str(&next);
                }
                ExprKind::Str(s)
            }
            Tok::FStr(body) => ExprKind::FString(self.fstring(line, self.since(start), &body)?),
            Tok::Keyword("True") => ExprKind::Bool(true),
            Tok::Keyword("False") => ExprKind::Bool(false),
            Tok::Keyword("None") => ExprKind::None,
            Tok::Name(n) => ExprKind::Name(n),
            Tok::Op("(") => {
                if self.eat_op(")") {
                    ExprKind::Tuple(vec![])
                } else {
                    let first = self.expr_or_star()?;
                    if self.at_kw("for") {
                        return Err(self.err("generator expressions are not in the language; write a list comprehension"));
                    }
                    if self.eat_op(")") {
                        if matches!(first.kind, ExprKind::Starred(_)) {
                            return Err(self.err("a starred expression needs a tuple around it"));
                        }
                        return Ok(first);
                    }
                    let mut items = vec![first];
                    while self.eat_op(",") {
                        if self.at_op(")") {
                            break;
                        }
                        items.push(self.expr_or_star()?);
                    }
                    self.expect_op(")")?;
                    ExprKind::Tuple(items)
                }
            }
            Tok::Op("[") => {
                if self.eat_op("]") {
                    ExprKind::List(vec![])
                } else {
                    let first = self.expr_or_star()?;
                    if self.at_kw("for") {
                        let gens = self.comp_for()?;
                        self.expect_op("]")?;
                        ExprKind::ListComp {
                            elt: Box::new(first),
                            gens,
                        }
                    } else {
                        let mut items = vec![first];
                        while self.eat_op(",") {
                            if self.at_op("]") {
                                break;
                            }
                            items.push(self.expr_or_star()?);
                        }
                        self.expect_op("]")?;
                        ExprKind::List(items)
                    }
                }
            }
            Tok::Op("{") => {
                if self.eat_op("}") {
                    ExprKind::Dict(vec![])
                } else {
                    let first = self.expr()?;
                    if self.eat_op(":") {
                        let value = self.expr()?;
                        if self.at_kw("for") {
                            let gens = self.comp_for()?;
                            self.expect_op("}")?;
                            ExprKind::DictComp {
                                key: Box::new(first),
                                value: Box::new(value),
                                gens,
                            }
                        } else {
                            let mut items = vec![(first, value)];
                            while self.eat_op(",") {
                                if self.at_op("}") {
                                    break;
                                }
                                let k = self.expr()?;
                                self.expect_op(":")?;
                                let v = self.expr()?;
                                items.push((k, v));
                            }
                            self.expect_op("}")?;
                            ExprKind::Dict(items)
                        }
                    } else if self.at_kw("for") {
                        let gens = self.comp_for()?;
                        self.expect_op("}")?;
                        ExprKind::SetComp {
                            elt: Box::new(first),
                            gens,
                        }
                    } else {
                        let mut items = vec![first];
                        while self.eat_op(",") {
                            if self.at_op("}") {
                                break;
                            }
                            items.push(self.expr()?);
                        }
                        self.expect_op("}")?;
                        ExprKind::Set(items)
                    }
                }
            }
            Tok::Keyword(k) => return Err(self.err(format!("`{k}` cannot start an expression"))),
            Tok::Op(o) => return Err(self.err(format!("`{o}` cannot start an expression"))),
            _ => return Err(self.err("expected an expression")),
        };
        Ok(Expr {
            line,
            span: self.since(start),
            kind,
        })
    }

    fn comp_for(&mut self) -> P<Vec<Comp>> {
        let mut gens = Vec::new();
        while self.eat_kw("for") {
            let target = self.target_list()?;
            self.expect_kw("in")?;
            let iter = self.or_test()?;
            let mut ifs = Vec::new();
            while self.eat_kw("if") {
                ifs.push(self.or_test()?);
            }
            gens.push(Comp { target, iter, ifs });
        }
        Ok(gens)
    }

    /// Split an f-string body into literal and expression parts.
    fn fstring(&self, line: u32, span: Span, body: &str) -> P<Vec<FPart>> {
        let chars: Vec<char> = body.chars().collect();
        let mut parts = Vec::new();
        let mut lit = String::new();
        let mut i = 0usize;
        let flush = |lit: &mut String, parts: &mut Vec<FPart>| {
            if !lit.is_empty() {
                parts.push(FPart::Lit(std::mem::take(lit)));
            }
        };
        while i < chars.len() {
            let c = chars[i];
            match c {
                '{' if chars.get(i.wrapping_add(1)) == Some(&'{') => {
                    lit.push('{');
                    i = i.wrapping_add(2);
                }
                '}' if chars.get(i.wrapping_add(1)) == Some(&'}') => {
                    lit.push('}');
                    i = i.wrapping_add(2);
                }
                '}' => return Err(self.err("a `}` in an f-string needs a field or doubling")),
                '{' => {
                    flush(&mut lit, &mut parts);
                    let mut j = i.wrapping_add(1);
                    let mut depth: u32 = 0;
                    let mut field = String::new();
                    while j < chars.len() {
                        let d = chars[j];
                        if d == '{' {
                            return Err(self.err("f-strings do not nest"));
                        }
                        if d == '}' && depth == 0 {
                            break;
                        }
                        match d {
                            '(' | '[' => depth = depth.wrapping_add(1),
                            ')' | ']' => depth = depth.saturating_sub(1),
                            _ => {}
                        }
                        field.push(d);
                        j = j.wrapping_add(1);
                    }
                    if j >= chars.len() {
                        return Err(self.err("an f-string field is never closed"));
                    }
                    let (expr_src, spec) = split_field(&field);
                    if expr_src.trim().is_empty() {
                        return Err(self.err("an empty f-string field"));
                    }
                    let expr = parse_expr_src(self.file, line, span, &expr_src)?;
                    parts.push(FPart::Expr { expr, spec });
                    i = j.wrapping_add(1);
                }
                '\\' => {
                    let n = *chars
                        .get(i.wrapping_add(1))
                        .ok_or_else(|| self.err("a string ends in a backslash"))?;
                    let mut k = i.wrapping_add(2);
                    let ch = escape_char(n, |count| {
                        let mut s = String::new();
                        for _ in 0..count {
                            s.push(
                                *chars
                                    .get(k)
                                    .ok_or_else(|| self.err("an escape runs off the string"))?,
                            );
                            k = k.wrapping_add(1);
                        }
                        Ok(s)
                    })
                    .map_err(|m| self.err(m))?;
                    lit.push(ch);
                    i = k;
                }
                _ => {
                    lit.push(c);
                    i = i.wrapping_add(1);
                }
            }
        }
        flush(&mut lit, &mut parts);
        Ok(parts)
    }
}

/// `expr!s:spec` → (`expr`, spec). `!s` is the only conversion and is
/// dropped; `!r` and `=` do not exist.
fn split_field(field: &str) -> (String, Option<String>) {
    let (expr, spec) = match field.find(':') {
        Some(i) => (&field[..i], Some(field[i.wrapping_add(1)..].to_string())),
        None => (field, None),
    };
    let expr = expr.strip_suffix("!s").unwrap_or(expr);
    (expr.to_string(), spec)
}

fn binop(line: u32, left: Expr, op: BinOp, right: Expr) -> Expr {
    Expr {
        line,
        span: Span {
            start: left.span.start,
            end: right.span.end,
        },
        kind: ExprKind::BinOp(Box::new(left), op, Box::new(right)),
    }
}

fn aug_op(op: &str) -> Option<BinOp> {
    Some(match op {
        "+=" => BinOp::Add,
        "-=" => BinOp::Sub,
        "*=" => BinOp::Mul,
        "/=" => BinOp::Div,
        "//=" => BinOp::FloorDiv,
        "%=" => BinOp::Mod,
        "**=" => BinOp::Pow,
        "|=" => BinOp::BitOr,
        "&=" => BinOp::BitAnd,
        "^=" => BinOp::BitXor,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<Stmt> {
        parse_file("t.py", src).expect("parses")
    }

    fn fails(src: &str) -> bool {
        parse_file("t.py", src).is_err()
    }

    #[test]
    fn the_procedural_core_parses() {
        let s = parse(
            "def f(a, b=2, *rest, **kw):\n    if a < b <= 3:\n        return [x * 2 for x in rest if x]\n    elif a:\n        pass\n    else:\n        a, *b = b, a\n    while True:\n        break\n    for i, j in zip(a, b):\n        continue\n    try:\n        raise ValueError(1)\n    except (ValueError, KeyError) as e:\n        x = e.args\n    finally:\n        y = f\"{a:.2f} and {b!s}\"\n    return lambda z, k=1: z + k\n",
        );
        assert_eq!(s.len(), 1);
        assert!(matches!(s[0].kind, StmtKind::Def { .. }));
    }

    #[test]
    fn the_boundary_is_a_parse_error() {
        assert!(fails("def f():\n    def g():\n        pass\n"));
        assert!(fails("x = (y for y in z)"));
        assert!(fails("a is b"));
        assert!(fails("del x"));
        assert!(fails("class A(B, C):\n    pass\n"));
        assert!(fails("if x:\n    class A:\n        pass\n"));
        assert!(fails("if x:\n    def f():\n        pass\n"));
        assert!(fails("class A:\n    if x:\n        pass\n"));
        assert!(fails("import m.n"));
        assert!(fails("from m import *"));
        assert!(fails("def f(a=1, b):\n    pass\n"));
        assert!(fails("return 1"));
        assert!(fails("f(x=1, 2)"));
        assert!(fails("x = f'{a{b}}'"));
        assert!(fails("try:\n    pass\n"));
    }

    #[test]
    fn an_expression_records_the_chars_it_was_read_from() {
        let src = "x = f(a, b + 1)\ny: list[int] = [g(2)]\n";
        let s = parse(src);
        let text = |sp: Span| -> String {
            src.chars()
                .skip(sp.start as usize)
                .take((sp.end - sp.start) as usize)
                .collect()
        };
        let StmtKind::Assign { value, .. } = &s[0].kind else {
            panic!()
        };
        assert_eq!(text(value.span), "f(a, b + 1)");
        let ExprKind::Call { args, .. } = &value.kind else {
            panic!()
        };
        let spans: Vec<String> = args.positional.iter().map(|a| text(a.span)).collect();
        assert_eq!(spans, ["a", "b + 1"]);
        let StmtKind::AnnAssign {
            annotation, value, ..
        } = &s[1].kind
        else {
            panic!()
        };
        assert_eq!(text(annotation.span), "list[int]");
        assert_eq!(text(value.as_ref().expect("value").span), "[g(2)]");
    }

    #[test]
    fn annotations_parse_in_python_s_three_places() {
        let s = parse(
            "def f(a: num, b: list[str] = [], *rest: num, **kw: dict[str, num]) -> tuple[num, ...]:\n    return a\n",
        );
        let StmtKind::Def {
            params, returns, ..
        } = &s[0].kind
        else {
            panic!()
        };
        let name = |n: &str| TypeExpr::Name(n.into());
        let tys: Vec<Option<TypeExpr>> = params
            .annotations
            .iter()
            .map(|a| a.as_ref().map(|a| a.ty.clone()))
            .collect();
        assert_eq!(
            tys,
            [
                Some(name("num")),
                Some(TypeExpr::Generic(Box::new(name("list")), vec![name("str")]))
            ]
        );
        assert_eq!(params.defaults.len(), 2);
        assert!(params.defaults[1].is_some());
        assert_eq!(
            params.star_annotation.as_ref().map(|a| &a.ty),
            Some(&name("num"))
        );
        assert!(params.dstar_annotation.is_some());
        assert_eq!(
            returns.as_ref().map(|a| &a.ty),
            Some(&TypeExpr::Generic(
                Box::new(name("tuple")),
                vec![name("num"), TypeExpr::Ellipsis]
            ))
        );

        let s = parse("x: num | None = None\ny: Machine\nz: geo.Point\nw: 'Node' = n\n");
        let anns: Vec<(&TypeExpr, bool)> = s
            .iter()
            .map(|st| match &st.kind {
                StmtKind::AnnAssign {
                    annotation, value, ..
                } => (&annotation.ty, value.is_some()),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            anns,
            [
                (&TypeExpr::Union(vec![name("num"), TypeExpr::None]), true),
                (&name("Machine"), false),
                (&TypeExpr::Attr("geo".into(), "Point".into()), false),
                (&TypeExpr::Str("Node".into()), true),
            ]
        );

        // An attribute target in a method, and annotations in a class body.
        parse(
            "class C:\n    n: num = 0\n    m: str\n    def __init__(self, v: num) -> None:\n        self.v: num = v\n",
        );
        // A lambda's parameters take none: its `:` ends them.
        assert!(matches!(
            &parse("f = lambda x: x")[0].kind,
            StmtKind::Assign { .. }
        ));
    }

    #[test]
    fn a_malformed_annotation_is_a_parse_error() {
        assert!(fails("x: 1 + 2\n"));
        assert!(fails("x: f(y)\n"));
        assert!(fails("a, b: num = 1, 2\n"));
        assert!(fails("x: num = y = 1\n"));
        assert!(fails("x: ...\n"));
        assert!(fails("x: list[]\n"));
        assert!(fails("def f() -> :\n    pass\n"));
        assert!(fails("def f(x:) :\n    pass\n"));
        assert!(fails("x: num |\n"));
    }

    #[test]
    fn expressions_bind_as_python_binds_them() {
        let s = parse("x = -2 ** 2 + 3 * 4 // 2 or not a and b");
        let StmtKind::Assign { value, .. } = &s[0].kind else {
            panic!()
        };
        // `or` at the top: (-2**2 + 3*4//2) or (not a and b)
        assert!(matches!(value.kind, ExprKind::BoolOp { is_and: false, .. }));
    }
}

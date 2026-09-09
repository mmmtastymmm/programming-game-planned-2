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
}

type P<T> = Result<T, LoadError>;

/// Parse a whole file into its statements.
pub fn parse_file(file: &str, src: &str) -> P<Vec<Stmt>> {
    let toks = Lexer::new(file, src).tokenize()?;
    let mut p = Parser {
        file,
        toks,
        pos: 0,
        def_depth: 0,
        def_allowed: true,
        top_level: true,
        next_block_is_class: false,
    };
    let mut stmts = Vec::new();
    while !p.at(&Tok::Eof) {
        stmts.extend(p.statement()?);
    }
    Ok(stmts)
}

/// Parse a single expression (an f-string's `{…}` segment).
fn parse_expr_src(file: &str, line: u32, src: &str) -> P<Expr> {
    let toks = Lexer::new(file, src).tokenize()?;
    let toks: Vec<Token> = toks
        .into_iter()
        .filter(|t| !matches!(t.tok, Tok::Newline | Tok::Indent | Tok::Dedent))
        .map(|mut t| {
            t.line = line;
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
        let out = self.block_inner();
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
                self.def_depth = self.def_depth.wrapping_add(1);
                let body = self.block();
                self.def_depth = self.def_depth.wrapping_sub(1);
                Some(StmtKind::Def {
                    name,
                    params,
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
                return Err(self.err("`match` is not in this slice of the language yet (T12)"));
            }
            Tok::Keyword("import") | Tok::Keyword("from") => {
                return Err(self.err("`import` is not in this slice of the language yet (T13)"));
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
                self.eat_op(",");
                self.expect_op(close)?;
                break;
            }
            if self.eat_op("*") {
                if params.star.is_some() {
                    return Err(self.err("one `*args` per function"));
                }
                params.star = Some(self.expect_name()?);
            } else {
                let name = self.expect_name()?;
                if params.names.contains(&name) {
                    return Err(self.err(format!("duplicate parameter `{name}`")));
                }
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
            }
            if !self.eat_op(",") {
                self.expect_op(close)?;
                break;
            }
        }
        Ok(params)
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
                if self.at_op("=") {
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
        Ok(Expr {
            line,
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
        Ok(Expr {
            line,
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
        if self.eat_op("*") {
            let e = inner(self)?;
            return Ok(Expr {
                line,
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
        self.expect_kw("lambda")?;
        let params = self.params(":")?;
        let body = self.expr()?;
        Ok(Expr {
            line,
            kind: ExprKind::Lambda {
                params,
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
        if self.eat_kw("not") {
            let e = self.not_test()?;
            return Ok(Expr {
                line,
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
                    kind: ExprKind::Subscript(Box::new(e), Box::new(idx)),
                };
            } else if self.eat_op(".") {
                let name = self.expect_name()?;
                e = Expr {
                    line,
                    kind: ExprKind::Attr(Box::new(e), name),
                };
            } else {
                return Ok(e);
            }
        }
    }

    fn subscript(&mut self) -> P<Expr> {
        let line = self.line();
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
            Tok::FStr(body) => ExprKind::FString(self.fstring(line, &body)?),
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
        Ok(Expr { line, kind })
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
    fn fstring(&self, line: u32, body: &str) -> P<Vec<FPart>> {
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
                    let expr = parse_expr_src(self.file, line, &expr_src)?;
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
        assert!(fails("import m"));
        assert!(fails("def f(a=1, b):\n    pass\n"));
        assert!(fails("return 1"));
        assert!(fails("f(x=1, 2)"));
        assert!(fails("x = f'{a{b}}'"));
        assert!(fails("try:\n    pass\n"));
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

//! The syntax tree: exactly the boundary `docs/01-language/syntax.md` draws,
//! for the procedural core (T10). `class`, `match` and `import` are T11–T13
//! and have no node here yet.

use crate::num::Num;

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub line: u32,
    pub kind: StmtKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Expr(Expr),
    Assign {
        targets: Vec<Target>,
        value: Expr,
    },
    AugAssign {
        target: Target,
        op: BinOp,
        value: Expr,
    },
    If {
        cond: Expr,
        body: Vec<Stmt>,
        orelse: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        orelse: Vec<Stmt>,
    },
    For {
        target: Target,
        iter: Expr,
        body: Vec<Stmt>,
        orelse: Vec<Stmt>,
    },
    Break,
    Continue,
    Pass,
    Def {
        name: String,
        params: Params,
        /// The `-> T` annotation, if any (Q56): never evaluated.
        returns: Option<Annotation>,
        body: Vec<Stmt>,
    },
    /// `target: T` or `target: T = value` (Q56). The annotation is never
    /// evaluated; with a value it assigns as `target = value` does, and
    /// without one it compiles to nothing.
    AnnAssign {
        target: Target,
        annotation: Annotation,
        value: Option<Expr>,
    },
    /// `class Name(Base):` — at most one base, named by an expression.
    Class {
        name: String,
        base: Option<Expr>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    /// `import module [as alias]`.
    Import {
        module: String,
        alias: Option<String>,
    },
    /// `from module import a [as b], …`.
    FromImport {
        module: String,
        names: Vec<(String, Option<String>)>,
    },
    /// `match subject:` with its `case` arms, tried top to bottom.
    Match {
        subject: Expr,
        arms: Vec<Arm>,
    },
    Try {
        body: Vec<Stmt>,
        handlers: Vec<Handler>,
        orelse: Vec<Stmt>,
        finalbody: Vec<Stmt>,
    },
    Raise(Option<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
    pub line: u32,
}

/// The pattern forms of `syntax.md`'s `match` table.
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    /// A number, string, `True`, `False` or `None`, as an expression.
    Literal(Expr),
    Capture(String),
    Wildcard,
    /// `*name` or `*_`, inside a sequence only.
    Star(Option<String>),
    Sequence(Vec<Pattern>),
    /// Literal keys to sub-patterns, and `**rest`.
    Mapping {
        pairs: Vec<(Expr, Pattern)>,
        rest: Option<String>,
    },
    /// `Name(k=p, …)`: keyword sub-patterns only.
    Class {
        class: Expr,
        kwargs: Vec<(String, Pattern)>,
    },
    Or(Vec<Pattern>),
    As(Box<Pattern>, String),
}

impl Pattern {
    /// Every name the pattern binds, in order, duplicates included.
    pub fn bound_names(&self, out: &mut Vec<String>) {
        match self {
            Pattern::Literal(_) | Pattern::Wildcard | Pattern::Star(None) => {}
            Pattern::Capture(n) | Pattern::Star(Some(n)) => out.push(n.clone()),
            Pattern::Sequence(items) => items.iter().for_each(|p| p.bound_names(out)),
            Pattern::Mapping { pairs, rest } => {
                pairs.iter().for_each(|(_, p)| p.bound_names(out));
                if let Some(r) = rest {
                    out.push(r.clone());
                }
            }
            Pattern::Class { kwargs, .. } => kwargs.iter().for_each(|(_, p)| p.bound_names(out)),
            // Alternatives bind the same names, so the first one's suffice.
            Pattern::Or(alts) => {
                if let Some(a) = alts.first() {
                    a.bound_names(out);
                }
            }
            Pattern::As(p, n) => {
                p.bound_names(out);
                out.push(n.clone());
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Handler {
    pub line: u32,
    /// `None` for a bare `except`.
    pub types: Option<Expr>,
    pub name: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Params {
    pub names: Vec<String>,
    /// One per name; `None` where there is no default.
    pub defaults: Vec<Option<Expr>>,
    pub star: Option<String>,
    pub dstar: Option<String>,
    /// One per name, then `*args`'s and `**kwargs`'s: each parameter's
    /// annotation, if any (Q56). Never evaluated; a `lambda` has none.
    pub annotations: Vec<Option<Annotation>>,
    pub star_annotation: Option<Annotation>,
    pub dstar_annotation: Option<Annotation>,
}

/// A type annotation (Q56): a type expression the parser reads and the
/// compiler drops. It exists for the editor, so it is kept as written.
#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub line: u32,
    pub span: Span,
    pub ty: TypeExpr,
}

/// The type expressions an annotation may hold (`syntax.md`, Annotations).
#[derive(Clone, Debug, PartialEq)]
pub enum TypeExpr {
    /// `num`, `str`, a class, `Machine`: any name, known or not, since an
    /// annotation is never checked at load.
    Name(String),
    /// `module.Name`: a class from an imported module.
    Attr(String, String),
    /// `None`.
    None,
    /// `list[T]`, `dict[K, V]`, `tuple[A, B]`: a name subscripted.
    Generic(Box<TypeExpr>, Vec<TypeExpr>),
    /// `...`, inside a subscript only: `tuple[num, ...]`.
    Ellipsis,
    /// `A | B | …`, two or more.
    Union(Vec<TypeExpr>),
    /// A string holding a type expression, as Python allows.
    Str(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Name(String),
    Subscript(Box<Expr>, Box<Expr>),
    Attr(Box<Expr>, String),
    /// A tuple or list target; `star` is the index of the `*name`, if any.
    Tuple(Vec<Target>, Option<usize>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub line: u32,
    pub span: Span,
    pub kind: ExprKind,
}

/// The chars of its file an expression was read from, end exclusive: for
/// the editor's underlines (Q59). The compiler never reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    BitOr,
    BitAnd,
    BitXor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    In,
    NotIn,
    Is,
    IsNot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CallArgs {
    pub positional: Vec<Expr>,
    pub keywords: Vec<(String, Expr)>,
    pub star: Option<Box<Expr>>,
    pub dstar: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comp {
    pub target: Target,
    pub iter: Expr,
    pub ifs: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FPart {
    Lit(String),
    Expr {
        expr: Expr,
        /// `!s` is the only conversion and means nothing extra.
        spec: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Num(Num),
    Str(String),
    Bool(bool),
    None,
    Name(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    /// `and` / `or`, short-circuiting, returning an operand.
    BoolOp {
        is_and: bool,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `a < b <= c`: `first` then pairs of operator and operand.
    Compare {
        first: Box<Expr>,
        rest: Vec<(CmpOp, Expr)>,
    },
    IfExp {
        cond: Box<Expr>,
        then: Box<Expr>,
        orelse: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: CallArgs,
    },
    Attr(Box<Expr>, String),
    Subscript(Box<Expr>, Box<Expr>),
    Slice {
        lo: Option<Box<Expr>>,
        hi: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
    },
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Set(Vec<Expr>),
    ListComp {
        elt: Box<Expr>,
        gens: Vec<Comp>,
    },
    SetComp {
        elt: Box<Expr>,
        gens: Vec<Comp>,
    },
    DictComp {
        key: Box<Expr>,
        value: Box<Expr>,
        gens: Vec<Comp>,
    },
    Lambda {
        /// Boxed, so every expression node stays small: the parser recurses
        /// once per bracket, and a node as large as a `Params` overflowed
        /// the stack at the nesting limit.
        params: Box<Params>,
        body: Box<Expr>,
    },
    FString(Vec<FPart>),
    /// `*expr` inside a display or a call; only where the parser allows it.
    Starred(Box<Expr>),
}

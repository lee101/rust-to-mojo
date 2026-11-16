//! A small Mojo IR. The lowering pass builds these nodes from `syn` trees and
//! `printer.rs` turns them into text. Nothing outside the printer formats Mojo
//! source by hand, so precedence, indentation and escaping live in one place.

use std::collections::BTreeSet;

/// A Mojo type expression.
#[derive(Clone, Debug, PartialEq)]
pub enum MType {
    /// `Name` or `Name[Arg, ...]`. `Self.T` is spelled as a name with a dot.
    Named(String, Vec<MTypeArg>),
    /// `Tuple[A, B]`
    Tuple(Vec<MType>),
    /// `def(A, B) -> R` closure/function type.
    Func(Vec<MType>, Option<Box<MType>>),
    /// `Some[Trait & Trait]` (opaque argument type).
    Some(Vec<String>),
    /// `_` (inferred) — only valid in a few positions, e.g. `List[_]`.
    Infer,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MTypeArg {
    Type(MType),
    Expr(MExpr),
}

impl MType {
    pub fn named(n: impl Into<String>) -> MType {
        MType::Named(n.into(), vec![])
    }
    pub fn generic(n: impl Into<String>, args: Vec<MType>) -> MType {
        MType::Named(n.into(), args.into_iter().map(MTypeArg::Type).collect())
    }
    pub fn is_named(&self, n: &str) -> bool {
        matches!(self, MType::Named(x, _) if x == n)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    In,
    NotIn,
    Is,
    IsNot,
}

impl BinOp {
    pub fn text(self) -> &'static str {
        use BinOp::*;
        match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Mod => "%",
            Pow => "**",
            BitAnd => "&",
            BitOr => "|",
            BitXor => "^",
            Shl => "<<",
            Shr => ">>",
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            And => "and",
            Or => "or",
            In => "in",
            NotIn => "not in",
            Is => "is",
            IsNot => "is not",
        }
    }
    /// Python-style binding power (higher binds tighter).
    pub fn prec(self) -> u8 {
        use BinOp::*;
        match self {
            Or => 2,
            And => 3,
            Eq | Ne | Lt | Le | Gt | Ge | In | NotIn | Is | IsNot => 5,
            BitOr => 6,
            BitXor => 7,
            BitAnd => 8,
            Shl | Shr => 9,
            Add | Sub => 10,
            Mul | Div | Mod => 11,
            Pow => 13,
        }
    }
    pub fn is_comparison(self) -> bool {
        self.prec() == 5
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Invert,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MArgVal {
    pub name: Option<String>,
    pub value: MExpr,
}

impl MArgVal {
    pub fn pos(value: MExpr) -> Self {
        MArgVal { name: None, value }
    }
    pub fn kw(name: impl Into<String>, value: MExpr) -> Self {
        MArgVal { name: Some(name.into()), value }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MExpr {
    Name(String),
    Int(String),
    Float(String),
    Str(String),
    Bool(bool),
    NoneLit,
    Bin(Box<MExpr>, BinOp, Box<MExpr>),
    Unary(UnOp, Box<MExpr>),
    Call(Box<MExpr>, Vec<MArgVal>),
    Attr(Box<MExpr>, String),
    Index(Box<MExpr>, Vec<MExpr>),
    /// `a:b` inside an index.
    Slice(Option<Box<MExpr>>, Option<Box<MExpr>>),
    /// Compile-time parameter application: `List[Int]`, `x.isa[T]`.
    Param(Box<MExpr>, Vec<MTypeArg>),
    /// A type used in expression position (`Float32(x)` callee, etc).
    Type(MType),
    Tuple(Vec<MExpr>),
    List(Vec<MExpr>),
    /// `a if cond else b`
    Ternary(Box<MExpr>, Box<MExpr>, Box<MExpr>),
    /// `x^`
    Transfer(Box<MExpr>),
}

impl MExpr {
    pub fn name(n: impl Into<String>) -> MExpr {
        MExpr::Name(n.into())
    }
    pub fn call(f: MExpr, args: Vec<MExpr>) -> MExpr {
        MExpr::Call(Box::new(f), args.into_iter().map(MArgVal::pos).collect())
    }
    pub fn call_named(f: &str, args: Vec<MExpr>) -> MExpr {
        MExpr::call(MExpr::name(f), args)
    }
    pub fn method(recv: MExpr, m: &str, args: Vec<MExpr>) -> MExpr {
        MExpr::call(MExpr::Attr(Box::new(recv), m.to_string()), args)
    }
    pub fn attr(recv: MExpr, a: &str) -> MExpr {
        MExpr::Attr(Box::new(recv), a.to_string())
    }
    pub fn bin(l: MExpr, op: BinOp, r: MExpr) -> MExpr {
        MExpr::Bin(Box::new(l), op, Box::new(r))
    }
    pub fn not(e: MExpr) -> MExpr {
        match e {
            MExpr::Bin(a, BinOp::Is, b) => MExpr::Bin(a, BinOp::IsNot, b),
            MExpr::Bin(a, BinOp::IsNot, b) => MExpr::Bin(a, BinOp::Is, b),
            MExpr::Bin(a, BinOp::Eq, b) => MExpr::Bin(a, BinOp::Ne, b),
            MExpr::Bin(a, BinOp::Ne, b) => MExpr::Bin(a, BinOp::Eq, b),
            MExpr::Bin(a, BinOp::In, b) => MExpr::Bin(a, BinOp::NotIn, b),
            MExpr::Bin(a, BinOp::NotIn, b) => MExpr::Bin(a, BinOp::In, b),
            MExpr::Unary(UnOp::Not, inner) => *inner,
            e => MExpr::Unary(UnOp::Not, Box::new(e)),
        }
    }
    pub fn and_all(mut conds: Vec<MExpr>) -> MExpr {
        if conds.is_empty() {
            return MExpr::Bool(true);
        }
        let first = conds.remove(0);
        conds.into_iter().fold(first, |a, b| MExpr::bin(a, BinOp::And, b))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conv {
    /// default immutable borrow (no keyword)
    Default,
    Mut,
    Var,
    Out,
    Deinit,
    Ref,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MArg {
    pub conv: Conv,
    pub name: String,
    pub ty: Option<MType>,
    /// Marks a `*` keyword-only separator *before* this argument.
    pub kw_only: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MParam {
    pub name: String,
    /// e.g. `Copyable & Deinitable`, `Int`, `def(Int) -> Int`
    pub bound: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum RetType {
    #[default]
    None,
    Value(MType),
    /// `ref [origin] T`
    Ref(String, MType),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MFunction {
    pub decorators: Vec<String>,
    pub name: String,
    pub params: Vec<MParam>,
    pub args: Vec<MArg>,
    pub raises: bool,
    pub ret: RetType,
    pub where_clauses: Vec<String>,
    pub doc: Option<String>,
    /// `None` means a trait requirement (`...`).
    pub body: Option<Vec<MStmt>>,
    /// Comment lines printed above the def (TODO/NOTE/HINT).
    pub comments: Vec<String>,
    /// Closure capture list, printed as `{imm a, mut b}` after the arguments.
    pub captures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindKind {
    Var,
    Ref,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfArm {
    pub cond: MExpr,
    pub body: Vec<MStmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MStmt {
    Var {
        kind: BindKind,
        name: String,
        ty: Option<MType>,
        init: Option<MExpr>,
    },
    Assign(MExpr, MExpr),
    AugAssign(MExpr, BinOp, MExpr),
    Expr(MExpr),
    Return(Option<MExpr>),
    Raise(MExpr),
    If {
        arms: Vec<IfArm>,
        else_body: Option<Vec<MStmt>>,
    },
    While(MExpr, Vec<MStmt>),
    For {
        is_ref: bool,
        targets: Vec<String>,
        iter: MExpr,
        body: Vec<MStmt>,
    },
    Break,
    Continue,
    Pass,
    Comment(String),
    NestedFn(Box<MFunction>),
    /// Inline TODO with the Rust source it replaces (used inside bodies only
    /// when the statement is known to be semantically inert, e.g. a dropped
    /// debug-only statement).
    Todo { reason: String, rust: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MField {
    pub name: String,
    pub ty: MType,
    pub doc: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MComptime {
    pub name: String,
    pub ty: Option<MType>,
    pub value: Option<MExpr>,
    pub params: Vec<MParam>,
    pub comments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MMember {
    Field(MField),
    Comptime(MComptime),
    Method(MFunction),
    AssocType { name: String, bound: String },
    Stub(MStub),
    Comment(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MStruct {
    pub decorators: Vec<String>,
    pub name: String,
    pub params: Vec<MParam>,
    pub traits: Vec<String>,
    pub doc: Option<String>,
    pub members: Vec<MMember>,
    pub comments: Vec<String>,
}

impl MStruct {
    pub fn add_trait(&mut self, t: &str) {
        if !self.traits.iter().any(|x| x == t) {
            self.traits.push(t.to_string());
        }
    }
    pub fn has_method(&self, name: &str) -> bool {
        self.members.iter().any(|m| matches!(m, MMember::Method(f) if f.name == name))
    }
    pub fn field_count(&self) -> usize {
        self.members.iter().filter(|m| matches!(m, MMember::Field(_))).count()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MTrait {
    pub name: String,
    pub parents: Vec<String>,
    pub doc: Option<String>,
    pub members: Vec<MMember>,
    pub comments: Vec<String>,
}

/// An item that could not be translated. Printed as a TODO header plus the
/// original Rust source, commented out.
#[derive(Clone, Debug, PartialEq)]
pub struct MStub {
    pub kind: String,
    pub name: String,
    pub reason: String,
    pub rust: String,
    /// Best-effort translated signature, shown as a hint for later passes.
    pub signature: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MItem {
    Struct(MStruct),
    Trait(MTrait),
    Function(MFunction),
    Comptime(MComptime),
    Stub(MStub),
    Comment(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MImport {
    pub module: String,
    /// `None` = `import module`
    pub name: Option<String>,
    pub alias: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct MModule {
    pub doc: Option<String>,
    pub header_comments: Vec<String>,
    pub imports: BTreeSet<MImport>,
    pub items: Vec<MItem>,
}

/// Apply `f` bottom-up to every expression in `stmts` (nested functions
/// included). Used for whole-body rewrites after lowering.
pub fn rewrite_exprs(stmts: &mut [MStmt], f: &dyn Fn(&mut MExpr)) {
    fn ex(e: &mut MExpr, f: &dyn Fn(&mut MExpr)) {
        match e {
            MExpr::Bin(a, _, b) => {
                ex(a, f);
                ex(b, f);
            }
            MExpr::Unary(_, a) | MExpr::Attr(a, _) | MExpr::Transfer(a) => ex(a, f),
            MExpr::Call(c, args) => {
                ex(c, f);
                for a in args {
                    ex(&mut a.value, f);
                }
            }
            MExpr::Index(b, ix) => {
                ex(b, f);
                for i in ix {
                    ex(i, f);
                }
            }
            MExpr::Slice(a, b) => {
                if let Some(a) = a {
                    ex(a, f);
                }
                if let Some(b) = b {
                    ex(b, f);
                }
            }
            MExpr::Param(b, args) => {
                ex(b, f);
                for a in args {
                    if let MTypeArg::Expr(e) = a {
                        ex(e, f);
                    }
                }
            }
            MExpr::Tuple(v) | MExpr::List(v) => {
                for x in v {
                    ex(x, f);
                }
            }
            MExpr::Ternary(a, b, c) => {
                ex(a, f);
                ex(b, f);
                ex(c, f);
            }
            _ => {}
        }
        f(e);
    }
    for s in stmts {
        match s {
            MStmt::Var { init: Some(e), .. } | MStmt::Expr(e) | MStmt::Return(Some(e)) | MStmt::Raise(e) => ex(e, f),
            MStmt::Assign(a, b) | MStmt::AugAssign(a, _, b) => {
                ex(a, f);
                ex(b, f);
            }
            MStmt::If { arms, else_body } => {
                for a in arms {
                    ex(&mut a.cond, f);
                    rewrite_exprs(&mut a.body, f);
                }
                if let Some(b) = else_body {
                    rewrite_exprs(b, f);
                }
            }
            MStmt::While(c, b) => {
                ex(c, f);
                rewrite_exprs(b, f);
            }
            MStmt::For { iter, body, .. } => {
                ex(iter, f);
                rewrite_exprs(body, f);
            }
            MStmt::NestedFn(nf) => {
                if let Some(b) = &mut nf.body {
                    rewrite_exprs(b, f);
                }
            }
            _ => {}
        }
    }
}

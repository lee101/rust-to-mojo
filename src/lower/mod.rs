//! syn -> Mojo IR lowering.
//!
//! The contract: every function here either produces IR that is believed to
//! be semantically faithful, or returns `Err(Unsupported)`. The item driver in
//! `items.rs` turns an `Err` into a stub (TODO + original Rust as comments).
//! Lossy-but-safe choices (dropped trait bounds, unsupported derives) are
//! recorded as TODO notes on the item via `FileCtx::todo`.

pub mod derive;
pub mod expr;
pub mod items;
pub mod iters;
pub mod macros;
pub mod names;
pub mod pat;
pub mod types;

use crate::ir::*;
use crate::report::Reason;
use crate::symbols::Symbols;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Unsupported {
    pub key: String,
    pub detail: String,
}

pub type LResult<T> = Result<T, Unsupported>;

pub fn unsup<T>(key: impl Into<String>, detail: impl Into<String>) -> LResult<T> {
    Err(Unsupported { key: key.into(), detail: detail.into() })
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub optimize_hints: bool,
}

/// A local name binding inside a function body.
#[derive(Clone, Debug)]
pub struct Binding {
    /// What a Rust reference to this name lowers to (usually `Name(mojo)`;
    /// for pattern bindings substituted into guards, the place expression).
    pub expr: MExpr,
    /// Rust type, if known (references stripped).
    pub rty: Option<syn::Type>,
    /// Declared as a Rust reference (`&T` / `&mut T`), so `*x` is identity.
    pub is_ref: bool,
    /// `var`-owned binding that may be transferred with `^` on its only use.
    pub transferable: bool,
    pub loop_depth: usize,
    /// The Mojo identifier of this binding (for use counting).
    pub mojo_name: String,
}

#[derive(Clone, Debug, Default)]
pub struct FnState {
    pub scopes: Vec<HashMap<String, Binding>>,
    pub declared: HashSet<String>,
    /// Function lowered with `raises` (Rust returned `Result`).
    pub raises: bool,
    /// Rust return type with `Result<T, _>` unwrapped to `T`.
    pub ret_rty: Option<syn::Type>,
    /// Mojo return type, used as the expected type of tail expressions.
    pub ret_mty: Option<MType>,
    pub returns_option: bool,
    /// Number of uses of each Rust identifier in the body (for `^`).
    pub use_counts: HashMap<String, usize>,
    pub loop_depth: usize,
    /// Statements hoisted in front of the statement being lowered.
    pub pending: Vec<MStmt>,
    /// Inside a Display/Debug `fmt`: the Rust name of the Formatter param.
    pub fmt_writer: Option<String>,
    /// Inside an `__init__(out self)`: `return v` becomes `self = v; return`.
    pub init_self: bool,
    /// Inferred numeric type for the `let` currently being lowered.
    pub let_hint: Option<syn::Type>,
}

/// Generic-parameter context while lowering a struct, impl or trait.
#[derive(Clone, Debug, Default)]
pub struct TypeScope {
    /// Rust type-param name -> Mojo spelling (`Self.T` inside a struct body,
    /// `T` in a free function).
    pub params: HashMap<String, String>,
    /// Rust type the current impl is for, e.g. `Point<T>`.
    pub self_rty: Option<syn::Type>,
    /// Name of the struct whose body we are in.
    pub self_name: Option<String>,
    pub in_trait: bool,
    /// Associated types declared in the current impl (`type Output = X;`).
    pub assoc: HashMap<String, syn::Type>,
}

pub struct FileCtx<'a> {
    pub syms: &'a Symbols,
    pub src: &'a str,
    pub opts: &'a Options,
    pub module_path: Vec<String>,
    pub crate_pkg: String,
    pub imports: BTreeSet<MImport>,
    pub file_notes: Vec<Reason>,
    /// `use Enum::*` / `use Enum::{A, B}` -> variant name -> enum name.
    pub variant_imports: HashMap<String, String>,
    pub todos: Vec<Reason>,
    pub hints: Vec<String>,
    pub f: FnState,
    pub ts: TypeScope,
    pub tmp_counter: usize,
    /// Types in this module that were stubbed, with the root-cause key.
    pub stubbed_types: HashMap<String, String>,
}

impl<'a> FileCtx<'a> {
    pub fn new(syms: &'a Symbols, src: &'a str, opts: &'a Options, module_path: Vec<String>, crate_pkg: String) -> Self {
        FileCtx {
            syms,
            src,
            opts,
            module_path,
            crate_pkg,
            imports: BTreeSet::new(),
            file_notes: vec![],
            variant_imports: HashMap::new(),
            todos: vec![],
            hints: vec![],
            f: FnState::default(),
            ts: TypeScope::default(),
            tmp_counter: 0,
            stubbed_types: HashMap::new(),
        }
    }

    pub fn import(&mut self, module: &str, name: &str) {
        self.imports.insert(MImport { module: module.to_string(), name: Some(name.to_string()), alias: None });
    }

    pub fn rt(&mut self, module: &str, name: &str) -> MExpr {
        self.import(&format!("mojo_rt.{}", module), name);
        MExpr::name(name)
    }

    /// Record a TODO note on the current item (item becomes `partial`).
    pub fn todo(&mut self, key: impl Into<String>, detail: impl Into<String>) {
        let key = key.into();
        let detail = detail.into();
        if !self.todos.iter().any(|r| r.key == key && r.detail == detail) {
            self.todos.push(Reason { key, detail });
        }
    }

    pub fn file_note(&mut self, key: impl Into<String>, detail: impl Into<String>) {
        let key = key.into();
        if !self.file_notes.iter().any(|r| r.key == key) {
            self.file_notes.push(Reason { key, detail: detail.into() });
        }
    }

    pub fn fresh(&mut self, base: &str) -> String {
        self.tmp_counter += 1;
        format!("__{}{}", base, self.tmp_counter)
    }

    /// Original source lines covered by a span (1-based inclusive lines).
    pub fn source_of(&self, span: proc_macro2::Span) -> String {
        let start = span.start().line;
        let end = span.end().line;
        if start == 0 || end < start {
            return String::from("<source unavailable>");
        }
        let lines: Vec<&str> = self.src.lines().collect();
        let s = (start - 1).min(lines.len());
        let e = end.min(lines.len());
        let body = &lines[s..e];
        // De-indent by the common leading whitespace.
        let indent = body
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        let mut out: Vec<String> = body.iter().map(|l| if l.len() >= indent { l[indent..].to_string() } else { l.trim_start().to_string() }).collect();
        const MAX: usize = 400;
        if out.len() > MAX {
            let extra = out.len() - MAX;
            out.truncate(MAX);
            out.push(format!("... ({} more lines)", extra));
        }
        out.join("\n")
    }

    // ---- function-scope helpers -------------------------------------------------

    pub fn begin_fn(&mut self) -> FnState {
        std::mem::replace(&mut self.f, FnState { scopes: vec![HashMap::new()], ..Default::default() })
    }

    pub fn end_fn(&mut self, saved: FnState) {
        self.f = saved;
    }

    pub fn push_scope(&mut self) {
        self.f.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.f.scopes.pop();
        if self.f.scopes.is_empty() {
            self.f.scopes.push(HashMap::new());
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        self.f.scopes.iter().rev().find_map(|s| s.get(name))
    }

    /// Declare a Rust local. Returns the Mojo name to use (renamed if this
    /// would shadow an earlier declaration in the same function).
    pub fn declare(&mut self, rust_name: &str, rty: Option<syn::Type>, is_ref: bool, transferable: bool) -> String {
        let base = names::ident(rust_name);
        let mut mojo = base.clone();
        let mut i = 1;
        while self.f.declared.contains(&mojo) {
            mojo = format!("{}_{}", base, i);
            i += 1;
        }
        self.f.declared.insert(mojo.clone());
        let b = Binding {
            expr: MExpr::name(&mojo),
            rty,
            is_ref,
            transferable,
            loop_depth: self.f.loop_depth,
            mojo_name: mojo.clone(),
        };
        if let Some(s) = self.f.scopes.last_mut() {
            s.insert(rust_name.to_string(), b);
        }
        mojo
    }

    /// Bind a Rust name to an arbitrary expression (no declaration emitted).
    pub fn bind_expr(&mut self, rust_name: &str, expr: MExpr, rty: Option<syn::Type>) {
        let b = Binding { expr, rty, is_ref: true, transferable: false, loop_depth: self.f.loop_depth, mojo_name: String::new() };
        if let Some(s) = self.f.scopes.last_mut() {
            s.insert(rust_name.to_string(), b);
        }
    }

    pub fn take_pending(&mut self) -> Vec<MStmt> {
        std::mem::take(&mut self.f.pending)
    }
}

/// Extract `///` doc text from attributes.
pub fn doc_of(attrs: &[syn::Attribute]) -> Option<String> {
    let mut lines = vec![];
    for a in attrs {
        if a.path().is_ident("doc") {
            if let syn::Meta::NameValue(nv) = &a.meta {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                    let v = s.value();
                    lines.push(v.strip_prefix(' ').unwrap_or(&v).to_string());
                }
            }
        }
    }
    if lines.is_empty() {
        None
    } else {
        // strip fenced code examples: they are Rust, not Mojo
        let mut out = vec![];
        let mut in_code = false;
        let mut dropped = false;
        for l in lines {
            if l.trim_start().starts_with("```") {
                in_code = !in_code;
                dropped = true;
                continue;
            }
            if !in_code {
                out.push(l);
            }
        }
        while out.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
            out.pop();
        }
        if dropped {
            out.push(String::new());
            out.push("(Rust code examples omitted by rust-to-mojo.)".to_string());
        }
        let s = out.join("\n");
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

/// Does this attribute list contain `#[cfg(test)]` or `#[test]`?
pub fn is_test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if a.path().is_ident("test") {
            return true;
        }
        if a.path().is_ident("cfg") {
            let s = quote::quote!(#a).to_string().replace(' ', "");
            return s.contains("cfg(test)") || s.starts_with("#[cfg(all(test") || s.contains("cfg(any(test,feature=\"test-support\"))");
        }
        // gpui-style test helpers
        if let Some(seg) = a.path().segments.last() {
            if seg.ident == "test" || seg.ident == "bench" {
                return true;
            }
        }
        false
    })
}

/// Textual `cfg(...)` predicate, if present (excluding test).
pub fn cfg_of(attrs: &[syn::Attribute]) -> Option<String> {
    for a in attrs {
        if a.path().is_ident("cfg") {
            if let syn::Meta::List(l) = &a.meta {
                return Some(l.tokens.to_string());
            }
        }
    }
    None
}

pub fn derives_of(attrs: &[syn::Attribute]) -> Vec<String> {
    let mut out = vec![];
    for a in attrs {
        if a.path().is_ident("derive") {
            let _ = a.parse_nested_meta(|m| {
                if let Some(seg) = m.path.segments.last() {
                    out.push(seg.ident.to_string());
                }
                Ok(())
            });
        }
    }
    out
}

pub fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.path().is_ident(name))
}

pub fn path_last(p: &syn::Path) -> String {
    p.segments.last().map(|s| s.ident.to_string()).unwrap_or_default()
}

pub fn path_str(p: &syn::Path) -> String {
    p.segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::")
}

pub fn tokens_str<T: quote::ToTokens>(t: &T) -> String {
    let s = quote::quote!(#t).to_string();
    let mut s = s.replace(" :: ", "::").replace(" . ", ".").replace("& ", "&").replace(" ,", ",").replace(" ;", ";");
    for (a, b) in [(" < ", "<"), (" >", ">"), (" (", "("), ("( ", "("), (" )", ")"), (" ! (", "!("), ("! (", "!("), (" [", "["), ("[ ", "["), (" ]", "]"), (" .. ", ".."), (" ..= ", "..=")] {
        s = s.replace(a, b);
    }
    if s.chars().count() > 120 {
        let t: String = s.chars().take(117).collect();
        format!("{}...", t)
    } else {
        s
    }
}

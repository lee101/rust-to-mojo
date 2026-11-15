//! Item lowering: modules, structs, enums, traits, impls, functions, consts,
//! `use` declarations. Also owns the item-level honesty contract: each item
//! produces exactly one coverage record, and a failed item becomes a stub.

use super::expr::{count_uses, Sink};
use super::names;
use super::types::*;
use super::*;
use crate::report::{ItemRecord, Status};
use std::panic::{catch_unwind, AssertUnwindSafe};
use syn::spanned::Spanned;
use syn::{FnArg, ImplItem, Item, ReturnType, TraitItem, Type};

pub enum ChildMod {
    Inline { name: String, items: Vec<Item>, attrs: Vec<syn::Attribute>, is_pub: bool },
    External { name: String, path_attr: Option<String>, is_pub: bool, cfg: Option<String> },
}

pub struct ModuleOut {
    pub module: MModule,
    pub records: Vec<ItemRecord>,
    pub children: Vec<ChildMod>,
    pub file_notes: Vec<Reason>,
    pub internal_errors: usize,
    pub hints: Vec<crate::hints::Hint>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FnMode {
    Normal,
    /// `impl Mul<X> for f32` lowered to `X.__rmul__`
    Reflected,
    /// `Clone::clone` -> `__init__(out self, *, copy: Self)`
    CopyInit,
    /// `Display::fmt` / `Debug::fmt` -> `write_to` / `write_repr_to`
    Fmt,
    /// `Drop::drop` -> `__deinit__(deinit self)`
    Deinit,
}

pub struct FnSpec<'s> {
    pub sig: &'s syn::Signature,
    pub block: Option<&'s syn::Block>,
    pub attrs: &'s [syn::Attribute],
    pub mojo_name: Option<String>,
    pub in_type: bool,
    pub mode: FnMode,
    pub static_decorator: bool,
    /// Extra where clauses (from impl specialisation).
    pub wheres: Vec<String>,
    /// For `Reflected`: Rust type of the foreign `self`.
    pub lhs_rty: Option<Type>,
    /// Impl-level generic params not bound by the self type
    /// (`impl<T, Rhs> Mul<Rhs> for Point<T>`): become method parameters.
    pub extra_generics: Option<syn::Generics>,
}

impl<'s> FnSpec<'s> {
    pub fn free(f: &'s syn::ItemFn) -> Self {
        FnSpec { sig: &f.sig, block: Some(&f.block), attrs: &f.attrs, mojo_name: None, in_type: false, mode: FnMode::Normal, static_decorator: false, wheres: vec![], lhs_rty: None, extra_generics: None }
    }
}

/// A struct (or enum lowered to a struct) awaiting impl methods.
#[allow(dead_code)]
struct TypeDecl {
    name: String,
    pre: Vec<MStruct>,
    main: MStruct,
    generics: Vec<String>,
    /// mapped trait names declared on each generic parameter
    param_traits: HashMap<String, Vec<String>>,
    record: usize,
}

enum Slot {
    Type(String),
    Item(MItem),
}

type ItemResult<T> = (Result<T, Unsupported>, Vec<Reason>);

impl<'a> FileCtx<'a> {
    /// Run one item's lowering with panic isolation and TODO collection.
    fn run_item<T>(&mut self, f: impl FnOnce(&mut Self) -> LResult<T>) -> ItemResult<T> {
        self.todos.clear();
        let saved_ts = self.ts.clone();
        let saved_f = self.begin_fn();
        let r = catch_unwind(AssertUnwindSafe(|| f(self)));
        self.end_fn(saved_f);
        self.ts = saved_ts;
        let todos = std::mem::take(&mut self.todos);
        match r {
            Ok(r) => (r, todos),
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "panic".into());
                (Err(Unsupported { key: "internal error (translator panic)".into(), detail: msg }), todos)
            }
        }
    }

    fn record(kind: &str, name: &str, span: proc_macro2::Span, r: &Result<(), &Unsupported>, todos: &[Reason]) -> ItemRecord {
        let (status, reasons) = match r {
            Err(u) => (Status::Stubbed, vec![Reason { key: u.key.clone(), detail: u.detail.clone() }]),
            Ok(()) if todos.is_empty() => (Status::Translated, vec![]),
            Ok(()) => (Status::Partial, todos.to_vec()),
        };
        ItemRecord { kind: kind.into(), name: name.into(), line: span.start().line, status, reasons }
    }

    fn stub(&self, kind: &str, name: &str, span: proc_macro2::Span, u: &Unsupported) -> MStub {
        let reason = if u.detail.is_empty() || u.detail.len() > 160 || u.detail == name { u.key.clone() } else { format!("{} ({})", u.key, u.detail) };
        MStub { kind: kind.into(), name: name.into(), reason, rust: self.source_of(span), signature: None }
    }

    fn todo_comments(todos: &[Reason]) -> Vec<String> {
        todos.iter().map(|t| format!("TODO(rust-to-mojo): {} -- {}", t.key, t.detail)).collect()
    }

    // ==================================================================================
    // Module
    // ==================================================================================

    pub fn lower_module(&mut self, inner_attrs: &[syn::Attribute], items: &[Item]) -> ModuleOut {
        let mut module = MModule { doc: doc_of(inner_attrs), ..Default::default() };
        let mut records: Vec<ItemRecord> = vec![];
        let mut children = vec![];
        let mut internal_errors = 0;
        let mut header: Vec<String> = vec![];

        // Pass A: use declarations (imports + enum variant imports)
        for item in items {
            if let Item::Use(u) = item {
                if is_test_only(&u.attrs) {
                    continue;
                }
                self.lower_use(&u.tree, vec![], u.leading_colon.is_some(), &mut header);
            }
        }

        // Pass B: types
        let mut decls: HashMap<String, TypeDecl> = HashMap::new();
        let mut slots: Vec<Slot> = vec![];
        let mut type_slots: HashMap<usize, Slot> = HashMap::new();
        for (idx, item) in items.iter().enumerate() {
            let (attrs, name, span, kind) = match item {
                Item::Struct(s) => (&s.attrs, s.ident.to_string(), item.span(), "struct"),
                Item::Enum(e) => (&e.attrs, e.ident.to_string(), item.span(), "enum"),
                _ => continue,
            };
            if is_test_only(attrs) {
                records.push(ItemRecord { kind: kind.into(), name, line: span.start().line, status: Status::Skipped, reasons: vec![] });
                continue;
            }
            let (r, todos) = self.run_item(|this| match item {
                Item::Struct(s) => this.lower_struct(s),
                Item::Enum(e) => this.lower_enum(e),
                _ => unreachable!(),
            });
            let rec = Self::record(kind, &name, span, &r.as_ref().map(|_| ()), &todos);
            match r {
                Ok(mut d) => {
                    d.main.comments.splice(0..0, Self::todo_comments(&todos));
                    d.record = records.len();
                    records.push(rec);
                    type_slots.insert(idx, Slot::Type(name.clone()));
                    decls.insert(name, d);
                }
                Err(u) => {
                    if u.key.starts_with("internal error") {
                        internal_errors += 1;
                    }
                    records.push(rec);
                    self.stubbed_types.insert(name.clone(), u.key.clone());
                    type_slots.insert(idx, Slot::Item(MItem::Stub(self.stub(kind, &name, span, &u))));
                }
            }
        }

        // Pass C: everything else, in order
        for (idx, item) in items.iter().enumerate() {
            match item {
                Item::Struct(_) | Item::Enum(_) => {
                    if let Some(s) = type_slots.remove(&idx) {
                        slots.push(s);
                    }
                }
                Item::Use(_) => {}
                Item::Mod(m) => {
                    if is_test_only(&m.attrs) {
                        records.push(ItemRecord { kind: "mod".into(), name: m.ident.to_string(), line: item.span().start().line, status: Status::Skipped, reasons: vec![] });
                        continue;
                    }
                    let is_pub = matches!(m.vis, syn::Visibility::Public(_));
                    match &m.content {
                        Some((_, items)) => children.push(ChildMod::Inline { name: m.ident.to_string(), items: items.clone(), attrs: m.attrs.clone(), is_pub }),
                        None => {
                            let path_attr = m.attrs.iter().find(|a| a.path().is_ident("path")).and_then(|a| {
                                if let syn::Meta::NameValue(nv) = &a.meta {
                                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                                        return Some(s.value());
                                    }
                                }
                                None
                            });
                            children.push(ChildMod::External { name: m.ident.to_string(), path_attr, is_pub, cfg: cfg_of(&m.attrs) });
                        }
                    }
                    if is_pub {
                        module.imports.insert(MImport { module: ".".into(), name: Some(names::ident(&m.ident.to_string())), alias: None });
                    }
                }
                Item::Impl(imp) => {
                    if is_test_only(&imp.attrs) {
                        continue;
                    }
                    self.lower_impl(imp, &mut decls, &mut slots, &mut records, &mut internal_errors);
                }
                _ => {
                    let (kind, name, attrs) = item_meta(item);
                    let span = item.span();
                    if let Some(a) = attrs {
                        if is_test_only(a) {
                            records.push(ItemRecord { kind: kind.into(), name, line: span.start().line, status: Status::Skipped, reasons: vec![] });
                            continue;
                        }
                    }
                    let (r, todos) = self.run_item(|this| this.lower_plain_item(item));
                    let rec = Self::record(kind, &name, span, &r.as_ref().map(|_| ()), &todos);
                    records.push(rec);
                    match r {
                        Ok(Some(mut mi)) => {
                            let c = Self::todo_comments(&todos);
                            let target: Option<&mut Vec<String>> = match &mut mi {
                                MItem::Function(f) => Some(&mut f.comments),
                                MItem::Trait(t) => Some(&mut t.comments),
                                MItem::Comptime(k) => Some(&mut k.comments),
                                _ => None,
                            };
                            if let Some(t) = target {
                                t.splice(0..0, c);
                            }
                            slots.push(Slot::Item(mi));
                        }
                        Ok(None) => {
                            records.pop();
                        }
                        Err(u) => {
                            if u.key.starts_with("internal error") {
                                internal_errors += 1;
                            }
                            let mut st = self.stub(kind, &name, span, &u);
                            if let Item::Fn(f) = item {
                                st.signature = self.signature_hint(&f.sig);
                            }
                            slots.push(Slot::Item(MItem::Stub(st)));
                        }
                    }
                }
            }
        }

        records.sort_by_key(|r| r.line);
        // Assemble
        for s in slots {
            match s {
                Slot::Item(i) => module.items.push(i),
                Slot::Type(n) => {
                    if let Some(d) = decls.remove(&n) {
                        for p in d.pre {
                            module.items.push(MItem::Struct(p));
                        }
                        module.items.push(MItem::Struct(d.main));
                    }
                }
            }
        }
        module.imports.extend(std::mem::take(&mut self.imports));
        // `from . import x` for pub child modules is only meaningful in packages.
        module.header_comments = header;
        ModuleOut { module, records, children, file_notes: std::mem::take(&mut self.file_notes), internal_errors, hints: vec![] }
    }

    fn signature_hint(&mut self, sig: &syn::Signature) -> Option<String> {
        let saved_todos = std::mem::take(&mut self.todos);
        let fake_block: syn::Block = syn::parse_quote!({});
        let saved = self.begin_fn();
        let r = catch_unwind(AssertUnwindSafe(|| {
            let spec = FnSpec { sig, block: None, attrs: &[], mojo_name: None, in_type: false, mode: FnMode::Normal, static_decorator: false, wheres: vec![], lhs_rty: None, extra_generics: None };
            self.lower_fn_item(&spec)
        }));
        self.end_fn(saved);
        self.todos = saved_todos;
        let _ = fake_block;
        match r {
            Ok(Ok(f)) => Some(crate::printer::signature_str(&f)),
            _ => None,
        }
    }

    fn lower_plain_item(&mut self, item: &Item) -> LResult<Option<MItem>> {
        match item {
            Item::Fn(f) => {
                let mut mf = self.lower_fn_item(&FnSpec::free(f))?;
                self.hint_fn(&f.block, &mut mf);
                Ok(Some(MItem::Function(mf)))
            }
            Item::Const(c) => {
                let ty = self.lower_type(&c.ty)?;
                let v = self.lower_expr_exp(&c.expr, Some(&ty))?;
                if !self.f.pending.is_empty() {
                    return unsup("const initialiser needing statements", tokens_str(&c.expr));
                }
                Ok(Some(MItem::Comptime(MComptime { name: names::ident(&c.ident.to_string()), ty: Some(ty), value: Some(v), params: vec![], comments: vec![] })))
            }
            Item::Static(s) => {
                if !matches!(s.mutability, syn::StaticMutability::None) {
                    return unsup("`static mut` global", tokens_str(&s.ident));
                }
                let ty = self.lower_type(&s.ty)?;
                let v = self.lower_expr_exp(&s.expr, Some(&ty))?;
                if !self.f.pending.is_empty() {
                    return unsup("static initialiser needing statements", tokens_str(&s.expr));
                }
                self.todo("`static` lowered to `comptime` (no runtime address identity)", s.ident.to_string());
                Ok(Some(MItem::Comptime(MComptime { name: names::ident(&s.ident.to_string()), ty: Some(ty), value: Some(v), params: vec![], comments: vec![] })))
            }
            Item::Type(t) => {
                let params = self.lower_generics(&t.generics, false, true)?;
                let ty = self.lower_type(&t.ty)?;
                Ok(Some(MItem::Comptime(MComptime { name: t.ident.to_string(), ty: None, value: Some(MExpr::Type(ty)), params, comments: vec![] })))
            }
            Item::Trait(t) => Ok(Some(MItem::Trait(self.lower_trait(t)?))),
            Item::Macro(m) => {
                if m.mac.path.is_ident("macro_rules") {
                    return unsup("`macro_rules!` definition (no Mojo macro system)", m.ident.as_ref().map(|i| i.to_string()).unwrap_or_default());
                }
                unsup(format!("item-level macro `{}!`", path_last(&m.mac.path)), tokens_str(&m.mac.path))
            }
            Item::ForeignMod(_) => unsup("`extern` block (FFI)", String::new()),
            Item::Union(_) => unsup("union", String::new()),
            Item::ExternCrate(_) => Ok(None),
            Item::TraitAlias(_) => unsup("trait alias", String::new()),
            _ => unsup("unsupported item kind", String::new()),
        }
    }

    // ==================================================================================
    // use
    // ==================================================================================

    fn lower_use(&mut self, tree: &syn::UseTree, prefix: Vec<String>, _leading: bool, header: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(p) => {
                let mut pre = prefix;
                pre.push(p.ident.to_string());
                self.lower_use(&p.tree, pre, _leading, header);
            }
            syn::UseTree::Group(g) => {
                for t in &g.items {
                    self.lower_use(t, prefix.clone(), _leading, header);
                }
            }
            syn::UseTree::Name(n) => self.use_one(&prefix, &n.ident.to_string(), None, header),
            syn::UseTree::Rename(r) => {
                let alias = r.rename.to_string();
                if alias == "_" {
                    return; // trait import for method resolution only
                }
                self.use_one(&prefix, &r.ident.to_string(), Some(alias), header)
            }
            syn::UseTree::Glob(_) => self.use_one(&prefix, "*", None, header),
        }
    }

    fn use_one(&mut self, prefix: &[String], name: &str, alias: Option<String>, header: &mut Vec<String>) {
        let full = {
            let mut v = prefix.to_vec();
            v.push(name.to_string());
            v.join("::")
        };
        if prefix.is_empty() {
            // `use foo;` of an external crate or `use Enum::*` style with no path
            if self.syms.enums.contains_key(name) {
                return;
            }
            self.file_note(format!("external crate `{}`", name), full.clone());
            push_ext(header, name, &full);
            return;
        }
        // enum variant imports: `use Axis::*`, `use crate::x::Axis::{A, B}`
        let owner = prefix.last().unwrap();
        if let Some(info) = self.syms.enums.get(owner).cloned() {
            if name == "*" {
                for (v, _) in &info.variants {
                    self.variant_imports.insert(v.clone(), owner.clone());
                }
            } else if info.variant(name).is_some() {
                self.variant_imports.insert(name.to_string(), owner.clone());
            }
            // still import the enum itself if it lives elsewhere
            if let Some(m) = self.resolve_module_path(&prefix[..prefix.len() - 1]) {
                if self.module_mojo_path() != m {
                    self.import(&m, owner);
                }
            }
            return;
        }
        let root = prefix[0].as_str();
        match root {
            "std" | "core" | "alloc" => {
                // Mapped std items are imported on use (Dict, ArcPointer, ...);
                // traits and modules have no import in Mojo.
                return;
            }
            "crate" | "self" | "super" => {}
            _ => {
                let mut m = self.module_path.clone();
                m.push(root.to_string());
                if !self.syms.modules.contains(&m.join("::")) {
                    let key = format!("external crate `{}`", root);
                    self.file_note(key, full.clone());
                    push_ext(header, root, &full);
                    return;
                }
            }
        }
        // Importing a module itself?
        let mut target = prefix.to_vec();
        target.push(name.to_string());
        if name != "*" {
            if let Some(abs) = self.abs_module(&target) {
                if self.syms.modules.contains(&abs.join("::")) {
                    let mut parts = vec![self.crate_pkg.clone()];
                    parts.extend(abs.iter().cloned());
                    let parent = parts[..parts.len() - 1].join(".");
                    self.imports.insert(MImport { module: parent, name: Some(names::ident(name)), alias: alias.map(|a| names::ident(&a)) });
                    return;
                }
            }
        }
        if let Some(m) = self.resolve_module_path(prefix) {
            if m == self.module_mojo_path() {
                return;
            }
            self.imports.insert(MImport { module: m, name: Some(if name == "*" { "*".into() } else { name.to_string() }), alias });
        }
    }

    fn abs_module(&self, segs: &[String]) -> Option<Vec<String>> {
        let mut base: Vec<String> = match segs.first()?.as_str() {
            "crate" => vec![],
            "self" => self.module_path.clone(),
            "super" => {
                let mut m = self.module_path.clone();
                m.pop();
                m
            }
            _ => self.module_path.clone(),
        };
        let start = if ["crate", "self", "super"].contains(&segs[0].as_str()) { 1 } else { 0 };
        for s in &segs[start..] {
            if s == "super" {
                base.pop();
            } else if s != "self" {
                base.push(s.clone());
            }
        }
        Some(base)
    }

    pub fn module_mojo_path(&self) -> String {
        let mut parts = vec![self.crate_pkg.clone()];
        parts.extend(self.module_path.iter().cloned());
        parts.join(".")
    }

    // ==================================================================================
    // Structs
    // ==================================================================================

    fn lower_struct(&mut self, s: &syn::ItemStruct) -> LResult<TypeDecl> {
        let name = s.ident.to_string();
        self.ts = TypeScope { self_name: Some(name.clone()), ..Default::default() };
        let params = self.lower_generics(&s.generics, true, true)?;
        let generics: Vec<String> = s.generics.type_params().map(|t| t.ident.to_string()).collect();
        let param_traits = param_trait_map(&params);
        let mut st = MStruct {
            decorators: vec!["fieldwise_init".into()],
            name: name.clone(),
            params,
            traits: vec![],
            doc: doc_of(&s.attrs),
            members: vec![],
            comments: vec![],
        };
        let mut fields: Vec<(String, MType)> = vec![];
        let iter: Vec<(String, &syn::Field)> = match &s.fields {
            syn::Fields::Named(n) => n.named.iter().map(|f| (names::ident(&f.ident.as_ref().unwrap().to_string()), f)).collect(),
            syn::Fields::Unnamed(u) => u.unnamed.iter().enumerate().map(|(i, f)| (format!("_{}", i), f)).collect(),
            syn::Fields::Unit => vec![],
        };
        for (fname, f) in iter {
            if rty_last(&f.ty).as_deref() == Some("PhantomData") {
                self.todo("PhantomData field dropped", format!("field `{}`", fname));
                continue;
            }
            let ty = self.lower_type(&f.ty)?;
            if type_mentions(&ty, &name) {
                return unsup("recursive struct (Mojo forbids self-referential structs; needs pointers)", name.clone());
            }
            fields.push((fname.clone(), ty.clone()));
            st.members.push(MMember::Field(MField { name: fname, ty, doc: doc_of(&f.attrs) }));
        }
        for a in &s.attrs {
            if a.path().is_ident("repr") {
                st.comments.push(format!("NOTE(rust-to-mojo): Rust {} -- verify Mojo layout if this crosses FFI", tokens_str(&a.meta)));
            }
        }
        if let Some(c) = cfg_of(&s.attrs) {
            self.todo("cfg-gated item translated unconditionally", format!("cfg({})", c));
        }
        let derives = derives_of(&s.attrs);
        self.apply_struct_derives(&mut st, &derives, &fields, &generics, &param_traits)?;
        if self.opts.optimize_hints {
            self.hint_struct(&mut st, &fields);
        }
        Ok(TypeDecl { name, pre: vec![], main: st, generics, param_traits, record: 0 })
    }

    // ==================================================================================
    // Enums
    // ==================================================================================

    fn lower_enum(&mut self, e: &syn::ItemEnum) -> LResult<TypeDecl> {
        let name = e.ident.to_string();
        self.ts = TypeScope { self_name: Some(name.clone()), ..Default::default() };
        let params = self.lower_generics(&e.generics, true, true)?;
        let generics: Vec<String> = e.generics.type_params().map(|t| t.ident.to_string()).collect();
        let param_traits = param_trait_map(&params);
        let derives = derives_of(&e.attrs);
        if let Some(c) = cfg_of(&e.attrs) {
            self.todo("cfg-gated item translated unconditionally", format!("cfg({})", c));
        }
        let c_like = e.variants.iter().all(|v| matches!(v.fields, syn::Fields::Unit));
        let default_variant = e.variants.iter().find(|v| has_attr(&v.attrs, "default")).map(|v| v.ident.to_string());
        let mut st = MStruct {
            decorators: vec!["fieldwise_init".into()],
            name: name.clone(),
            params: params.clone(),
            traits: vec![],
            doc: doc_of(&e.attrs),
            members: vec![],
            comments: vec![],
        };
        if c_like {
            if !generics.is_empty() {
                return unsup("generic fieldless enum", name);
            }
            st.traits.push("ImplicitlyCopyable".into());
            st.traits.push("Equatable".into());
            st.traits.push("Intable".into());
            st.members.push(MMember::Field(MField { name: "_tag".into(), ty: MType::named("Int"), doc: None }));
            let mut next: i128 = 0;
            let mut tags: Vec<(String, MExpr)> = vec![];
            for v in &e.variants {
                let val = match &v.discriminant {
                    Some((_, d)) => {
                        let lit = match d {
                            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. }) => i.base10_parse::<i128>().ok(),
                            syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => match &*u.expr {
                                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. }) => i.base10_parse::<i128>().ok().map(|x| -x),
                                _ => None,
                            },
                            _ => None,
                        };
                        match lit {
                            Some(x) => x,
                            None => return unsup("non-literal enum discriminant", tokens_str(d)),
                        }
                    }
                    None => next,
                };
                next = val + 1;
                let vexpr = MExpr::Int(val.to_string());
                tags.push((v.ident.to_string(), vexpr.clone()));
                let doc = doc_of(&v.attrs);
                let mut comments = vec![];
                if let Some(d) = doc {
                    comments.extend(d.lines().map(|l| l.to_string()));
                }
                st.members.push(MMember::Comptime(MComptime {
                    name: names::ident(&v.ident.to_string()),
                    ty: None,
                    value: Some(MExpr::call_named("Self", vec![vexpr])),
                    params: vec![],
                    comments,
                }));
            }
            let other_tag = MExpr::attr(MExpr::name("other"), "_tag");
            let self_tag = MExpr::attr(MExpr::name("self"), "_tag");
            st.members.push(MMember::Method(MFunction {
                name: "__eq__".into(),
                args: vec![self_arg(Conv::Default), MArg { conv: Conv::Default, name: "other".into(), ty: Some(MType::named("Self")), kw_only: false }],
                ret: RetType::Value(MType::named("Bool")),
                body: Some(vec![MStmt::Return(Some(MExpr::bin(self_tag.clone(), BinOp::Eq, other_tag.clone())))]),
                ..Default::default()
            }));
            st.members.push(MMember::Method(MFunction {
                name: "__ne__".into(),
                args: vec![self_arg(Conv::Default), MArg { conv: Conv::Default, name: "other".into(), ty: Some(MType::named("Self")), kw_only: false }],
                ret: RetType::Value(MType::named("Bool")),
                body: Some(vec![MStmt::Return(Some(MExpr::bin(self_tag.clone(), BinOp::Ne, other_tag.clone())))]),
                ..Default::default()
            }));
            st.members.push(MMember::Method(MFunction {
                name: "__int__".into(),
                args: vec![self_arg(Conv::Default)],
                ret: RetType::Value(MType::named("Int")),
                body: Some(vec![MStmt::Return(Some(self_tag.clone()))]),
                ..Default::default()
            }));
            for d in &derives {
                match d.as_str() {
                    "Clone" | "Copy" | "PartialEq" | "Eq" => {}
                    "Hash" => st.add_trait("Hashable"),
                    "Debug" => {
                        st.add_trait("Writable");
                        let arms: Vec<IfArm> = tags
                            .iter()
                            .map(|(n, v)| IfArm {
                                cond: MExpr::bin(self_tag.clone(), BinOp::Eq, v.clone()),
                                body: vec![MStmt::Expr(MExpr::method(MExpr::name("writer"), "write", vec![MExpr::Str(n.clone())]))],
                            })
                            .collect();
                        // Debug is repr; it may only take `write_to` when there is no Display impl.
                        let writer_name = if self.syms.display_types.contains(&name) { "write_repr_to" } else { "write_to" };
                        st.members.push(MMember::Method(MFunction {
                            name: writer_name.into(),
                            params: vec![MParam { name: "W".into(), bound: Some("Writer".into()) }],
                            args: vec![self_arg(Conv::Default), MArg { conv: Conv::Mut, name: "writer".into(), ty: Some(MType::named("W")), kw_only: false }],
                            body: Some(vec![MStmt::If { arms, else_body: None }]),
                            ..Default::default()
                        }));
                    }
                    "PartialOrd" | "Ord" => {
                        if !st.has_method("__lt__") {
                            st.add_trait("Comparable");
                            for (n, op) in [("__lt__", BinOp::Lt), ("__le__", BinOp::Le), ("__gt__", BinOp::Gt), ("__ge__", BinOp::Ge)] {
                                st.members.push(MMember::Method(MFunction {
                                    name: n.into(),
                                    args: vec![self_arg(Conv::Default), MArg { conv: Conv::Default, name: "other".into(), ty: Some(MType::named("Self")), kw_only: false }],
                                    ret: RetType::Value(MType::named("Bool")),
                                    body: Some(vec![MStmt::Return(Some(MExpr::bin(self_tag.clone(), op, other_tag.clone())))]),
                                    ..Default::default()
                                }));
                            }
                        }
                    }
                    "Default" => {
                        let Some(dv) = &default_variant else {
                            return unsup("derive(Default) on enum without #[default] variant", name);
                        };
                        st.add_trait("Defaultable");
                        st.members.push(MMember::Method(MFunction {
                            name: "__init__".into(),
                            args: vec![self_arg(Conv::Out)],
                            body: Some(vec![MStmt::Assign(MExpr::name("self"), MExpr::attr(MExpr::name("Self"), &names::ident(dv)))]),
                            ..Default::default()
                        }));
                    }
                    other => self.unsupported_derive(other),
                }
            }
            return Ok(TypeDecl { name, pre: vec![], main: st, generics, param_traits, record: 0 });
        }

        // ---- enum with data: payload structs + Variant wrapper ----
        let mut pre = vec![];
        let mut payload_types = vec![];
        let param_args: Vec<MType> = generics.iter().map(|g| MType::named(format!("Self.{}", g))).collect();
        let payload_traits: Vec<String> = derives
            .iter()
            .filter_map(|d| match d.as_str() {
                "Clone" => Some("Copyable"),
                "Copy" => Some("ImplicitlyCopyable"),
                "PartialEq" | "Eq" => Some("Equatable"),
                "Hash" => Some("Hashable"),
                "Debug" => Some("Writable"),
                _ => None,
            })
            .map(|s| s.to_string())
            .fold(vec![], |mut acc, t| {
                if !acc.contains(&t) {
                    acc.push(t);
                }
                acc
            });
        let mut ctors = vec![];
        let mut accessors = vec![];
        for v in &e.variants {
            let vname = v.ident.to_string();
            let pname = format!("{}_{}", name, vname);
            let mut pst = MStruct {
                decorators: vec!["fieldwise_init".into()],
                name: pname.clone(),
                params: params.clone(),
                traits: payload_traits.clone(),
                doc: Some(format!("Payload of `{}::{}`.", name, vname)),
                members: vec![],
                comments: vec![],
            };
            let mut ctor_args = vec![];
            let mut ctor_vals = vec![];
            let fields: Vec<(String, &syn::Field)> = match &v.fields {
                syn::Fields::Named(n) => n.named.iter().map(|f| (names::ident(&f.ident.as_ref().unwrap().to_string()), f)).collect(),
                syn::Fields::Unnamed(u) => u.unnamed.iter().enumerate().map(|(i, f)| (format!("_{}", i), f)).collect(),
                syn::Fields::Unit => vec![],
            };
            for (fname, f) in fields {
                let ty = self.lower_type(&f.ty)?;
                if type_mentions(&ty, &name) {
                    return unsup("recursive enum (Mojo forbids self-referential structs; needs pointers)", name.clone());
                }
                pst.members.push(MMember::Field(MField { name: fname.clone(), ty: ty.clone(), doc: doc_of(&f.attrs) }));
                let prim = is_prim_rty(&f.ty);
                ctor_args.push(MArg { conv: if prim { Conv::Default } else { Conv::Var }, name: fname.clone(), ty: Some(ty), kw_only: false });
                ctor_vals.push(if prim { MExpr::name(&fname) } else { MExpr::Transfer(Box::new(MExpr::name(&fname))) });
            }
            let ptype = if generics.is_empty() { MType::named(&pname) } else { MType::generic(&pname, param_args.clone()) };
            payload_types.push(ptype.clone());
            let payload_val = MExpr::call(MExpr::Type(ptype.clone()), ctor_vals);
            ctors.push(MMember::Method(MFunction {
                decorators: vec!["staticmethod".into()],
                name: names::ident(&vname),
                args: ctor_args,
                ret: RetType::Value(MType::named("Self")),
                doc: doc_of(&v.attrs),
                body: Some(vec![MStmt::Return(Some(MExpr::call_named("Self", vec![payload_val])))]),
                ..Default::default()
            }));
            let v_field = MExpr::attr(MExpr::name("self"), "_v");
            accessors.push(MMember::Method(MFunction {
                name: format!("isa_{}", vname),
                args: vec![self_arg(Conv::Default)],
                ret: RetType::Value(MType::named("Bool")),
                body: Some(vec![MStmt::Return(Some(MExpr::call(MExpr::Param(Box::new(MExpr::attr(v_field.clone(), "isa")), vec![MTypeArg::Type(ptype.clone())]), vec![])))]),
                ..Default::default()
            }));
            accessors.push(MMember::Method(MFunction {
                name: format!("as_{}", vname),
                args: vec![self_arg(Conv::Ref)],
                ret: RetType::Ref("self".into(), ptype.clone()),
                body: Some(vec![MStmt::Return(Some(MExpr::Param(Box::new(v_field), vec![MTypeArg::Type(ptype)])))]),
                ..Default::default()
            }));
            pre.push(pst);
        }
        self.import("std.utils", "Variant");
        st.traits = payload_traits.clone();
        st.members.push(MMember::Field(MField { name: "_v".into(), ty: MType::generic("Variant", payload_types), doc: None }));
        st.members.extend(ctors);
        st.members.extend(accessors);
        for d in &derives {
            match d.as_str() {
                "Clone" | "Copy" | "PartialEq" | "Eq" | "Hash" | "Debug" => {}
                "Default" => {
                    let Some(dv) = &default_variant else {
                        return unsup("derive(Default) on enum without #[default] variant", name);
                    };
                    st.add_trait("Defaultable");
                    st.members.push(MMember::Method(MFunction {
                        name: "__init__".into(),
                        args: vec![self_arg(Conv::Out)],
                        body: Some(vec![MStmt::Assign(MExpr::name("self"), MExpr::call(MExpr::attr(MExpr::name("Self"), &names::ident(dv)), vec![]))]),
                        ..Default::default()
                    }));
                }
                "PartialOrd" | "Ord" => self.todo("derive(PartialOrd/Ord) on data enum dropped", name.clone()),
                other => self.unsupported_derive(other),
            }
        }
        if derives.iter().any(|d| d == "Debug") {
            self.todo("Debug on data enum maps to Variant's Writable (format differs from Rust)", name.clone());
        }
        st.comments.push(format!("Rust enum `{}` lowered to a tagged struct over Variant (see MAPPING.md#enums).", name));
        Ok(TypeDecl { name, pre, main: st, generics, param_traits, record: 0 })
    }

    pub fn unsupported_derive(&mut self, d: &str) {
        self.todo(format!("derive `{}` has no Mojo equivalent", d), format!("#[derive({})] dropped", d));
    }

    // ==================================================================================
    // Traits
    // ==================================================================================

    fn lower_trait(&mut self, t: &syn::ItemTrait) -> LResult<MTrait> {
        if t.generics.params.iter().any(|p| !matches!(p, syn::GenericParam::Lifetime(_))) {
            return unsup("generic trait (Mojo traits take no type parameters)", t.ident.to_string());
        }
        if t.unsafety.is_some() {
            return unsup("unsafe trait", t.ident.to_string());
        }
        self.ts = TypeScope { in_trait: true, ..Default::default() };
        let mut parents = vec![];
        for b in &t.supertraits {
            if let syn::TypeParamBound::Trait(tb) = b {
                let n = path_last(&tb.path);
                if names::erased_trait(&n) {
                    continue;
                }
                if let Some(m) = names::std_trait(&n) {
                    if !parents.contains(&m.to_string()) {
                        parents.push(m.to_string());
                    }
                } else if self.syms.traits.contains(&n) {
                    parents.push(n);
                } else if names::op_trait(&n).is_some() {
                    self.todo(format!("dropped supertrait `{}` (no Mojo std trait)", n), tokens_str(&tb.path));
                } else {
                    self.todo(format!("supertrait on external trait `{}`", n), tokens_str(&tb.path));
                    parents.push(n);
                }
            }
        }
        let mut members = vec![];
        for item in &t.items {
            match item {
                TraitItem::Type(ty) => {
                    let bound = self.lower_bounds(ty.bounds.iter(), &ty.ident.to_string(), true)?;
                    if ty.default.is_some() {
                        self.todo("dropped default for associated type", ty.ident.to_string());
                    }
                    members.push(MMember::AssocType { name: ty.ident.to_string(), bound });
                }
                TraitItem::Const(c) => {
                    let ty = self.lower_type(&c.ty)?;
                    let value = match &c.default {
                        Some((_, e)) => Some(self.lower_expr_exp(e, Some(&ty))?),
                        None => None,
                    };
                    members.push(MMember::Comptime(MComptime { name: c.ident.to_string(), ty: Some(ty), value, params: vec![], comments: vec![] }));
                }
                TraitItem::Fn(f) => {
                    let saved = self.begin_fn();
                    let spec = FnSpec { sig: &f.sig, block: f.default.as_ref(), attrs: &f.attrs, mojo_name: None, in_type: true, mode: FnMode::Normal, static_decorator: false, wheres: vec![], lhs_rty: None, extra_generics: None };
                    let r = self.lower_fn_item(&spec);
                    self.end_fn(saved);
                    members.push(MMember::Method(r?));
                }
                TraitItem::Macro(_) => return unsup("macro inside trait", t.ident.to_string()),
                _ => return unsup("unsupported trait item", t.ident.to_string()),
            }
        }
        Ok(MTrait { name: t.ident.to_string(), parents, doc: doc_of(&t.attrs), members, comments: vec![] })
    }

    // ==================================================================================
    // Impl blocks
    // ==================================================================================

    fn lower_impl(&mut self, imp: &syn::ItemImpl, decls: &mut HashMap<String, TypeDecl>, slots: &mut Vec<Slot>, records: &mut Vec<ItemRecord>, internal_errors: &mut usize) {
        let saved_ts = self.ts.clone();
        let saved_todos = std::mem::take(&mut self.todos);
        self.lower_impl_inner(imp, decls, slots, records, internal_errors);
        self.ts = saved_ts;
        self.todos = saved_todos;
    }

    fn lower_impl_inner(&mut self, imp: &syn::ItemImpl, decls: &mut HashMap<String, TypeDecl>, slots: &mut Vec<Slot>, records: &mut Vec<ItemRecord>, internal_errors: &mut usize) {
        let self_name = match &*imp.self_ty {
            Type::Path(p) if p.qself.is_none() => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        };
        let trait_name = imp.trait_.as_ref().map(|(_, p, _)| path_last(p));
        let trait_path = imp.trait_.as_ref().map(|(_, p, _)| p.clone());
        let negative = imp.trait_.as_ref().map(|(n, _, _)| n.is_some()).unwrap_or(false);
        let type_label = self_name.clone().unwrap_or_else(|| tokens_str(&imp.self_ty));

        // Reflected operator impls: `impl Mul<Pixels> for f32`
        if let (Some(tn), Some(sn)) = (&trait_name, &self_name) {
            if !decls.contains_key(sn) && names::op_trait(tn).map(|o| o.2.is_some()).unwrap_or(false) {
                if let Some(rhs) = trait_path.as_ref().and_then(|p| first_type_arg(p)) {
                    if let Some(rn) = rty_last(&rhs) {
                        if decls.contains_key(&rn) && (names::prim(sn).is_some()) {
                            self.lower_reflected_impl(imp, tn, &rn, decls, records, internal_errors);
                            return;
                        }
                    }
                }
            }
        }

        let method_items: Vec<&syn::ImplItemFn> = imp.items.iter().filter_map(|i| if let ImplItem::Fn(f) = i { Some(f) } else { None }).collect();

        // Decide whether the whole impl is unsupported.
        let whole_reason: Option<Unsupported> = (|| {
            if negative {
                return Some(Unsupported { key: "negative impl".into(), detail: type_label.clone() });
            }
            if imp.unsafety.is_some() {
                if matches!(trait_name.as_deref(), Some("Send") | Some("Sync")) {
                    return None;
                }
                return Some(Unsupported { key: "unsafe impl".into(), detail: type_label.clone() });
            }
            let Some(sn) = &self_name else {
                return Some(Unsupported { key: "impl for non-path type (reference, tuple, slice, dyn)".into(), detail: tokens_str(&imp.self_ty) });
            };
            if let Some(tn) = &trait_name {
                if imp.generics.type_params().any(|t| t.ident == sn.as_str()) {
                    return Some(Unsupported { key: "blanket impl `impl<T> Trait for T`".into(), detail: tn.clone() });
                }
            }
            if let Some(cause) = self.stubbed_types.get(sn) {
                return Some(Unsupported { key: format!("method of an untranslated type (cause: {})", cause), detail: sn.clone() });
            }
            if !decls.contains_key(sn) {
                let key = if self.syms.structs.contains_key(sn) || self.syms.enums.contains_key(sn) {
                    if decls.is_empty() && !self.syms.structs.contains_key(sn) && !self.syms.enums.contains_key(sn) {
                        "impl for unknown type"
                    } else {
                        "impl for a type defined in another module or not translated (Mojo needs methods inside the struct)"
                    }
                } else if names::prim(sn).is_some() || ["Vec", "String", "Option", "str", "HashMap", "Box", "Arc", "Rc"].contains(&sn.as_str()) {
                    "impl on a std/primitive type (Mojo has no extensions)"
                } else {
                    "impl for an external type (Mojo has no extensions)"
                };
                return Some(Unsupported { key: key.into(), detail: format!("{}{}", trait_name.as_ref().map(|t| format!("{} for ", t)).unwrap_or_default(), sn) });
            }
            if let Some(tn) = &trait_name {
                let is_std = names::std_trait(tn).is_some() || names::op_trait(tn).is_some() || names::erased_trait(tn) || ["From", "Drop", "Index", "IndexMut", "Hash", "Deref", "DerefMut", "Iterator", "IntoIterator", "AsRef", "AsMut", "Borrow", "BorrowMut", "Into", "TryFrom", "TryInto", "FromStr", "Sum", "Product", "Extend", "FromIterator", "Error", "Unpin"].contains(&tn.as_str());
                if !is_std && !self.syms.traits.contains(tn) {
                    return Some(Unsupported { key: format!("impl of external trait `{}`", tn), detail: format!("{} for {}", tn, sn) });
                }
            }
            None
        })();

        if let Some(u) = whole_reason {
            if method_items.is_empty() {
                // marker / assoc-only impl: one record for the impl itself
                let span = imp.span();
                records.push(Self::record("impl", &format!("{} for {}", trait_name.clone().unwrap_or_default(), type_label), span, &Err(&u), &[]));
                let st = self.stub("impl", &format!("{}{}", trait_path.as_ref().map(|t| format!("{} for ", tokens_str(t))).unwrap_or_default(), type_label), span, &u);
                self.place_stub(decls, slots, self_name.as_deref(), st);
                return;
            }
            // One stub for the whole impl block, one record per method.
            for f in &method_items {
                records.push(Self::record("method", &format!("{}.{}", type_label, f.sig.ident), f.span(), &Err(&u), &[]));
            }
            let span = imp.span();
            let st = self.stub("impl", &format!("{}{}", trait_path.as_ref().map(|t| format!("{} for ", tokens_str(t))).unwrap_or_default(), type_label), span, &u);
            self.place_stub(decls, slots, self_name.as_deref(), st);
            return;
        }

        let sn = self_name.unwrap();
        let tn = trait_name.clone();

        // Generic parameter mapping and specialisation where-clauses.
        let mut base_ts = TypeScope { self_name: Some(sn.clone()), self_rty: Some((*imp.self_ty).clone()), ..Default::default() };
        let mut wheres: Vec<String> = vec![];
        let mut cond_conformance: Vec<String> = vec![];
        let mut setup_err: Option<Unsupported> = None;
        let mut setup_todos: Vec<Reason> = vec![];
        let mut extra_generics: Option<syn::Generics> = None;
        {
            let d = decls.get(&sn).unwrap();
            let decl_generics = d.generics.clone();
            let decl_traits = d.param_traits.clone();
            let args: Vec<Type> = rty_args(&imp.self_ty);
            let impl_params: Vec<String> = imp.generics.type_params().map(|t| t.ident.to_string()).collect();
            let mut mapped: HashMap<String, String> = HashMap::new();
            if args.len() != decl_generics.len() {
                setup_err = Some(Unsupported { key: "impl type arguments do not match the type's parameters".into(), detail: tokens_str(&imp.self_ty) });
            } else {
                self.todos.clear();
                self.ts = base_ts.clone();
                for (i, a) in args.iter().enumerate() {
                    let g = &decl_generics[i];
                    let a_name = match a {
                        Type::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 && p.path.segments[0].arguments.is_empty() => Some(p.path.segments[0].ident.to_string()),
                        _ => None,
                    };
                    match a_name.filter(|n| impl_params.contains(n)) {
                        Some(n) => {
                            mapped.insert(n.clone(), format!("Self.{}", g));
                        }
                        None => match self.lower_type(a) {
                            Ok(t) => {
                                wheres.push(format!("Self.{} == {}", g, crate::printer::type_str(&t)));
                                self.todo("impl specialised on a concrete type argument", format!("method gated with `where Self.{} == {}`", g, crate::printer::type_str(&t)));
                            }
                            Err(u) => setup_err = Some(u),
                        },
                    }
                }
                let unbound: Vec<String> = impl_params.iter().filter(|p| !mapped.contains_key(*p)).cloned().collect();
                if !unbound.is_empty() {
                    let mut eg = imp.generics.clone();
                    eg.params = eg.params.into_iter().filter(|p| matches!(p, syn::GenericParam::Type(t) if unbound.contains(&t.ident.to_string()))).collect();
                    extra_generics = Some(eg);
                }
                if setup_err.is_none() {
                    base_ts.params = mapped.clone();
                    self.ts = base_ts.clone();
                    // extra bounds on impl params
                    let mut g2 = imp.generics.clone();
                    g2.params = g2.params.into_iter().filter(|p| matches!(p, syn::GenericParam::Type(t) if mapped.contains_key(&t.ident.to_string()))).collect();
                    // where-predicates on method-level (unbound) params are handled per method
                    if let Some(w) = &mut g2.where_clause {
                        w.predicates = w.predicates.clone().into_iter().filter(|p| match p {
                            syn::WherePredicate::Type(pt) => !matches!(&pt.bounded_ty, Type::Path(tp) if tp.path.segments.len() == 1 && !mapped.contains_key(&tp.path.segments[0].ident.to_string())),
                            _ => true,
                        }).collect();
                    }
                    match self.lower_generics(&g2, true, false) {
                        Ok(ps) => {
                            for p in ps {
                                let target = mapped.get(&p.name).cloned().unwrap_or_default();
                                let decl_name = target.trim_start_matches("Self.").to_string();
                                let have = decl_traits.get(&decl_name).cloned().unwrap_or_default();
                                for t in p.bound.unwrap_or_default().split(" & ") {
                                    let t = t.trim();
                                    if t.is_empty() || t == "AnyType" || t == "Movable" || t == "Deinitable" || have.iter().any(|h| h == t) {
                                        continue;
                                    }
                                    if t.starts_with("def(") {
                                        setup_err = Some(Unsupported { key: "closure-typed impl parameter".into(), detail: t.into() });
                                        continue;
                                    }
                                    wheres.push(format!("conforms_to({}, {})", target, t));
                                    self.todo("method gated on an extra trait bound (`where conforms_to`)", format!("{}: {}", target, t));
                                    cond_conformance.push(format!("conforms_to({}, {})", decl_name, t));
                                }
                            }
                        }
                        Err(u) => setup_err = Some(u),
                    }
                    base_ts = self.ts.clone();
                    base_ts.params = mapped;
                }
                setup_todos = std::mem::take(&mut self.todos);
            }
        }
        // associated types of the impl
        for i in &imp.items {
            if let ImplItem::Type(t) = i {
                base_ts.assoc.insert(t.ident.to_string(), t.ty.clone());
            }
        }
        if let Some(u) = setup_err {
            for f in &method_items {
                records.push(Self::record("method", &format!("{}.{}", sn, f.sig.ident), f.span(), &Err(&u), &[]));
            }
            let st = self.stub("impl", &format!("{}{}", tn.as_ref().map(|t| format!("{} for ", t)).unwrap_or_default(), sn), imp.span(), &u);
            self.place_stub(decls, slots, Some(&sn), st);
            return;
        }

        // Conformance + special trait handling.
        let mut add_conformance: Option<String> = None;
        let mut extra_members: Vec<MMember> = vec![];
        if let Some(t) = &tn {
            match t.as_str() {
                "Copy" => add_conformance = Some("ImplicitlyCopyable".into()),
                "Eq" => add_conformance = Some("Equatable".into()),
                "Send" | "Sync" | "Unpin" => {}
                "PartialEq" => {
                    let rhs_is_self = trait_path.as_ref().and_then(first_type_arg).map(|t| rty_last(&t) == Some(sn.clone()) || rty_last(&t).as_deref() == Some("Self")).unwrap_or(true);
                    if rhs_is_self {
                        add_conformance = Some("Equatable".into());
                    }
                }
                "Display" | "Debug" | "Default" | "Clone" => {
                    add_conformance = names::std_trait(t).map(|s| s.to_string());
                }
                x if self.syms.traits.contains(x) => {
                    add_conformance = Some(x.to_string());
                    for i in &imp.items {
                        if let ImplItem::Type(ty) = i {
                            self.ts = base_ts.clone();
                            if let Ok(mt) = self.lower_type(&ty.ty) {
                                extra_members.push(MMember::Comptime(MComptime { name: ty.ident.to_string(), ty: None, value: Some(MExpr::Type(mt)), params: vec![], comments: vec![] }));
                            }
                        }
                        if let ImplItem::Const(c) = i {
                            self.ts = base_ts.clone();
                            if let (Ok(mt), true) = (self.lower_type(&c.ty), true) {
                                let saved = self.begin_fn();
                                let v = self.lower_expr_exp(&c.expr, Some(&mt));
                                self.end_fn(saved);
                                if let Ok(v) = v {
                                    extra_members.push(MMember::Comptime(MComptime { name: c.ident.to_string(), ty: Some(mt), value: Some(v), params: vec![], comments: vec![] }));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        } else {
            // inherent impl consts
            for i in &imp.items {
                if let ImplItem::Const(c) = i {
                    self.todos.clear();
                    self.ts = base_ts.clone();
                    let saved = self.begin_fn();
                    let r = (|| -> LResult<MComptime> {
                        let mt = self.lower_type(&c.ty)?;
                        let v = self.lower_expr_exp(&c.expr, Some(&mt))?;
                        if !self.f.pending.is_empty() {
                            return unsup("const initialiser needing statements", tokens_str(&c.expr));
                        }
                        Ok(MComptime { name: names::ident(&c.ident.to_string()), ty: Some(mt), value: Some(v), params: vec![], comments: vec![] })
                    })();
                    self.end_fn(saved);
                    let todos = std::mem::take(&mut self.todos);
                    records.push(Self::record("const", &format!("{}.{}", sn, c.ident), c.span(), &r.as_ref().map(|_| ()), &todos));
                    match r {
                        Ok(mut k) => {
                            k.comments = Self::todo_comments(&todos);
                            if !wheres.is_empty() {
                                // specialised consts cannot be gated: stub them
                                let u = Unsupported { key: "associated const in a specialised impl".into(), detail: c.ident.to_string() };
                                let rec = records.last_mut().unwrap();
                                rec.status = Status::Stubbed;
                                rec.reasons = vec![Reason { key: u.key.clone(), detail: u.detail.clone() }];
                                extra_members.push(MMember::Stub(self.stub("const", &c.ident.to_string(), c.span(), &u)));
                            } else {
                                extra_members.push(MMember::Comptime(k));
                            }
                        }
                        Err(u) => extra_members.push(MMember::Stub(self.stub("const", &c.ident.to_string(), c.span(), &u))),
                    }
                }
            }
        }

        // Methods
        let mut new_members: Vec<MMember> = vec![];
        let mut have_eq = false;
        let mut have_ne = false;
        for f in &method_items {
            let rust_name = f.sig.ident.to_string();
            let rec_name = format!("{}.{}", sn, rust_name);
            let (mojo_name, mode, is_static_extra): (Option<String>, FnMode, bool) = match tn.as_deref() {
                Some(t) if names::op_trait(t).is_some() => {
                    let (m, dunder, _) = names::op_trait(t).unwrap();
                    if rust_name == m {
                        (Some(dunder.to_string()), FnMode::Normal, false)
                    } else {
                        (None, FnMode::Normal, false)
                    }
                }
                Some("PartialEq") => match rust_name.as_str() {
                    "eq" => {
                        have_eq = true;
                        (Some("__eq__".into()), FnMode::Normal, false)
                    }
                    "ne" => {
                        have_ne = true;
                        (Some("__ne__".into()), FnMode::Normal, false)
                    }
                    _ => (None, FnMode::Normal, false),
                },
                Some("Display") if rust_name == "fmt" => (Some("write_to".into()), FnMode::Fmt, false),
                Some("Debug") if rust_name == "fmt" => (Some("write_repr_to".into()), FnMode::Fmt, false),
                Some("Default") if rust_name == "default" => (Some("default".into()), FnMode::Normal, true),
                Some("From") if rust_name == "from" => (Some("from_".into()), FnMode::Normal, true),
                Some("Clone") if rust_name == "clone" => (Some("__init__".into()), FnMode::CopyInit, false),
                Some("Drop") if rust_name == "drop" => (Some("__deinit__".into()), FnMode::Deinit, false),
                Some("Index") if rust_name == "index" => (Some("__getitem__".into()), FnMode::Normal, false),
                Some(t) if names::std_trait(t).is_some() || ["Hash", "Deref", "DerefMut", "Iterator", "IntoIterator", "AsRef", "AsMut", "Borrow", "BorrowMut", "Into", "TryFrom", "TryInto", "FromStr", "Sum", "Product", "Extend", "FromIterator", "Error", "IndexMut"].contains(&t) => {
                    let u = Unsupported { key: format!("impl of std trait `{}` (no Mojo mapping yet)", t), detail: format!("{} for {}", t, sn) };
                    records.push(Self::record("method", &rec_name, f.span(), &Err(&u), &[]));
                    new_members.push(MMember::Stub(self.stub("method", &rec_name, f.span(), &u)));
                    continue;
                }
                _ => (None, FnMode::Normal, false),
            };
            // Clone whose body is `*self` is just a Copyable conformance.
            if mode == FnMode::CopyInit && is_deref_self(&f.block) {
                records.push(Self::record("method", &rec_name, f.span(), &Ok(()), &[]));
                continue;
            }
            if is_test_only(&f.attrs) {
                records.push(ItemRecord { kind: "method".into(), name: rec_name, line: f.span().start().line, status: Status::Skipped, reasons: vec![] });
                continue;
            }
            let ts = base_ts.clone();
            let wh = wheres.clone();
            let st_todos = setup_todos.clone();
            let (r, mut todos) = self.run_item(|this| {
                this.ts = ts;
                for t in st_todos {
                    this.todos.push(t);
                }
                let is_static = !matches!(f.sig.inputs.first(), Some(FnArg::Receiver(_)));
                let spec = FnSpec {
                    sig: &f.sig,
                    block: Some(&f.block),
                    attrs: &f.attrs,
                    mojo_name: mojo_name.clone(),
                    in_type: true,
                    mode,
                    static_decorator: is_static || is_static_extra,
                    wheres: wh,
                    lhs_rty: None,
                    extra_generics: extra_generics.clone(),
                };
                let mut mf = this.lower_fn_item(&spec)?;
                this.hint_fn(&f.block, &mut mf);
                Ok(mf)
            });
            if let Some(c) = cfg_of(&f.attrs) {
                todos.push(Reason { key: "cfg-gated item translated unconditionally".into(), detail: format!("cfg({})", c) });
            }
            // Rust allows an inherent method and a trait method with the same
            // name; Mojo needs exactly one definition per signature.
            let r = match r {
                Ok(mf) => {
                    let key = method_key(&mf);
                    let clash = decls.get(&sn).map(|d| d.main.members.iter().any(|m| matches!(m, MMember::Method(o) if method_key(o) == key))).unwrap_or(false)
                        || new_members.iter().any(|m| matches!(m, MMember::Method(o) if method_key(o) == key));
                    if clash {
                        Err(Unsupported { key: "method signature collides with another impl's method (Mojo needs one definition)".into(), detail: format!("{}.{}", sn, mf.name) })
                    } else {
                        Ok(mf)
                    }
                }
                e => e,
            };
            records.push(Self::record("method", &rec_name, f.span(), &r.as_ref().map(|_| ()), &todos));
            match r {
                Ok(mut mf) => {
                    mf.comments.splice(0..0, Self::todo_comments(&todos));
                    new_members.push(MMember::Method(mf));
                }
                Err(u) => {
                    if u.key.starts_with("internal error") {
                        *internal_errors += 1;
                    }
                    let mut stb = self.stub("method", &rec_name, f.span(), &u);
                    self.ts = base_ts.clone();
                    stb.signature = self.signature_hint_method(&f.sig, base_ts.clone());
                    new_members.push(MMember::Stub(stb));
                }
            }
        }
        if have_eq && !have_ne {
            new_members.push(MMember::Method(MFunction {
                name: "__ne__".into(),
                args: vec![self_arg(Conv::Default), MArg { conv: Conv::Default, name: "other".into(), ty: Some(MType::named("Self")), kw_only: false }],
                ret: RetType::Value(MType::named("Bool")),
                body: Some(vec![MStmt::Return(Some(MExpr::not(MExpr::method(MExpr::name("self"), "__eq__", vec![MExpr::name("other")]))))]),
                where_clauses: wheres.clone(),
                ..Default::default()
            }));
        }
        let d = decls.get_mut(&sn).unwrap();
        if tn.as_deref() == Some("Default") && d.main.field_count() > 0 && !d.main.members.iter().any(|m| matches!(m, MMember::Method(f) if f.name == "__init__" && f.args.len() == 1)) && new_members.iter().any(|m| matches!(m, MMember::Method(f) if f.name == "default")) {
            new_members.push(MMember::Method(MFunction {
                name: "__init__".into(),
                args: vec![self_arg(Conv::Out)],
                body: Some(vec![MStmt::Assign(MExpr::name("self"), MExpr::call(MExpr::attr(MExpr::name("Self"), "default"), vec![]))]),
                where_clauses: wheres.clone(),
                ..Default::default()
            }));
        }
        if add_conformance.as_deref() == Some("Copyable") && d.main.traits.iter().any(|t| t.starts_with("ImplicitlyCopyable")) {
            add_conformance = None;
        }
        if let Some(c) = add_conformance {
            // only conform when every method made it through
            let all_ok = !new_members.iter().any(|m| matches!(m, MMember::Stub(_)));
            if all_ok {
                if cond_conformance.is_empty() && wheres.is_empty() {
                    d.main.add_trait(&c);
                } else if wheres.iter().all(|w| w.starts_with("conforms_to")) {
                    d.main.add_trait(&format!("{} where {}", c, cond_conformance.join(" and ")));
                }
            }
        }
        d.main.members.extend(extra_members);
        d.main.members.extend(new_members);
        if !records.is_empty() {
            let _ = d.record;
        }
    }

    fn signature_hint_method(&mut self, sig: &syn::Signature, ts: TypeScope) -> Option<String> {
        let saved_todos = std::mem::take(&mut self.todos);
        let saved_ts = std::mem::replace(&mut self.ts, ts);
        let saved = self.begin_fn();
        let r = catch_unwind(AssertUnwindSafe(|| {
            let spec = FnSpec { sig, block: None, attrs: &[], mojo_name: None, in_type: true, mode: FnMode::Normal, static_decorator: !matches!(sig.inputs.first(), Some(FnArg::Receiver(_))), wheres: vec![], lhs_rty: None, extra_generics: None };
            self.lower_fn_item(&spec)
        }));
        self.end_fn(saved);
        self.ts = saved_ts;
        self.todos = saved_todos;
        match r {
            Ok(Ok(f)) => Some(crate::printer::signature_str(&f)),
            _ => None,
        }
    }

    fn place_stub(&mut self, decls: &mut HashMap<String, TypeDecl>, slots: &mut Vec<Slot>, target: Option<&str>, st: MStub) {
        if let Some(t) = target {
            if let Some(d) = decls.get_mut(t) {
                d.main.members.push(MMember::Stub(st));
                return;
            }
        }
        slots.push(Slot::Item(MItem::Stub(st)));
    }

    fn lower_reflected_impl(&mut self, imp: &syn::ItemImpl, trait_name: &str, rhs_name: &str, decls: &mut HashMap<String, TypeDecl>, records: &mut Vec<ItemRecord>, internal_errors: &mut usize) {
        let (m, _, reflected) = names::op_trait(trait_name).unwrap();
        let reflected = reflected.unwrap();
        for item in &imp.items {
            let ImplItem::Fn(f) = item else { continue };
            let rec_name = format!("{}.{} (reflected {} for {})", rhs_name, reflected, trait_name, tokens_str(&imp.self_ty));
            if f.sig.ident != m {
                continue;
            }
            let mut ts = TypeScope { self_name: Some(rhs_name.to_string()), self_rty: parse_rty(rhs_name), ..Default::default() };
            for i in &imp.items {
                if let ImplItem::Type(t) = i {
                    ts.assoc.insert(t.ident.to_string(), t.ty.clone());
                }
            }
            let lhs = (*imp.self_ty).clone();
            let (r, todos) = self.run_item(|this| {
                this.ts = ts;
                let spec = FnSpec { sig: &f.sig, block: Some(&f.block), attrs: &f.attrs, mojo_name: Some(reflected.to_string()), in_type: true, mode: FnMode::Reflected, static_decorator: false, wheres: vec![], lhs_rty: Some(lhs), extra_generics: None };
                this.lower_fn_item(&spec)
            });
            records.push(Self::record("method", &rec_name, f.span(), &r.as_ref().map(|_| ()), &todos));
            let d = decls.get_mut(rhs_name).unwrap();
            match r {
                Ok(mut mf) => {
                    mf.comments.splice(0..0, Self::todo_comments(&todos));
                    mf.comments.insert(0, format!("Rust: impl {} for {}", tokens_str(&imp.trait_.as_ref().unwrap().1), tokens_str(&imp.self_ty)));
                    d.main.members.push(MMember::Method(mf));
                }
                Err(u) => {
                    if u.key.starts_with("internal error") {
                        *internal_errors += 1;
                    }
                    let st = self.stub("method", &rec_name, f.span(), &u);
                    d.main.members.push(MMember::Stub(st));
                }
            }
        }
    }

    // ==================================================================================
    // Functions
    // ==================================================================================

    pub fn lower_fn_item(&mut self, spec: &FnSpec) -> LResult<MFunction> {
        let sig = spec.sig;
        if sig.asyncness.is_some() {
            return unsup("async fn", sig.ident.to_string());
        }
        if sig.unsafety.is_some() {
            return unsup("unsafe fn", sig.ident.to_string());
        }
        if sig.abi.is_some() {
            return unsup("extern ABI fn (FFI)", sig.ident.to_string());
        }
        if sig.variadic.is_some() {
            return unsup("C-variadic fn", sig.ident.to_string());
        }
        let rust_name = sig.ident.to_string();
        let mut f = MFunction { name: spec.mojo_name.clone().unwrap_or_else(|| names::ident(&rust_name)), doc: doc_of(spec.attrs), ..Default::default() };
        for a in spec.attrs {
            if a.path().is_ident("inline") {
                if let syn::Meta::List(l) = &a.meta {
                    if l.tokens.to_string() == "always" {
                        f.decorators.push("always_inline".into());
                    }
                }
            }
        }
        // method-level generics are plain names
        let saved_params = self.ts.params.clone();
        let r = self.lower_fn_inner(spec, &mut f);
        self.ts.params = saved_params;
        r?;
        f.where_clauses.extend(spec.wheres.iter().cloned());
        if spec.static_decorator && spec.mode == FnMode::Normal && spec.in_type && !self.ts.in_trait {
            f.decorators.insert(0, "staticmethod".into());
        } else if spec.static_decorator && self.ts.in_trait {
            f.decorators.insert(0, "staticmethod".into());
        }
        Ok(f)
    }

    fn lower_fn_inner(&mut self, spec: &FnSpec, f: &mut MFunction) -> LResult<()> {
        let sig = spec.sig;
        f.params = self.lower_generics(&sig.generics, false, true)?;
        if let Some(eg) = &spec.extra_generics {
            // only keep where-predicates that constrain the method-level params
            let mut eg = eg.clone();
            let names: Vec<String> = eg.type_params().map(|t| t.ident.to_string()).collect();
            if let Some(w) = &mut eg.where_clause {
                w.predicates = w.predicates.clone().into_iter().filter(|p| match p {
                    syn::WherePredicate::Type(pt) => matches!(&pt.bounded_ty, Type::Path(tp) if tp.path.segments.len() == 1 && names.contains(&tp.path.segments[0].ident.to_string())),
                    _ => false,
                }).collect();
            }
            let mut ep = self.lower_generics(&eg, false, true)?;
            ep.extend(std::mem::take(&mut f.params));
            f.params = ep;
        }
        if let Some(b) = spec.block {
            self.f.use_counts = count_uses(b);
        }
        let mut impl_trait_n = 0;
        let mut receiver_seen = false;
        let mut pending_lhs: Option<MArg> = None;
        let mut prologue: Vec<MStmt> = vec![];
        let inputs: Vec<&FnArg> = sig.inputs.iter().collect();
        for (idx, input) in inputs.iter().enumerate() {
            match input {
                FnArg::Receiver(r) => {
                    receiver_seen = true;
                    if r.colon_token.is_some() {
                        // `self: Box<Self>` etc.
                        let ok = matches!(&*r.ty, Type::Reference(_)) || rty_last(&r.ty).as_deref() == Some("Self");
                        if !ok {
                            return unsup("typed `self` receiver (Box<Self>, Rc<Self>, Pin<..>)", tokens_str(&r.ty));
                        }
                    }
                    let self_rty = self.ts.self_rty.clone();
                    match spec.mode {
                        FnMode::Reflected => {
                            let lt = spec.lhs_rty.clone().unwrap();
                            let mt = self.lower_type(&lt)?;
                            // Rust `self` (the foreign lhs) is called `lhs` in Mojo
                            self.bind_expr("self", MExpr::name("lhs"), spec.lhs_rty.clone());
                            pending_lhs = Some(MArg { conv: Conv::Default, name: "lhs".into(), ty: Some(mt), kw_only: false });
                        }
                        FnMode::CopyInit => {
                            f.args.push(self_arg(Conv::Out));
                            f.args.push(MArg { conv: Conv::Default, name: "copy".into(), ty: Some(MType::named("Self")), kw_only: true });
                            self.bind_expr("self", MExpr::name("copy"), self_rty);
                            self.f.init_self = true;
                        }
                        FnMode::Deinit => {
                            f.args.push(self_arg(Conv::Deinit));
                            self.bind_expr("self", MExpr::name("self"), self_rty);
                        }
                        _ => {
                            let conv = if r.reference.is_some() {
                                if r.mutability.is_some() { Conv::Mut } else { Conv::Default }
                            } else if r.mutability.is_some() {
                                Conv::Var
                            } else {
                                Conv::Default
                            };
                            f.args.push(self_arg(conv));
                            self.bind_expr("self", MExpr::name("self"), self_rty);
                        }
                    }
                }
                FnArg::Typed(pt) => {
                    if spec.mode == FnMode::Fmt {
                        // the Formatter
                        if let syn::Pat::Ident(pi) = &*pt.pat {
                            self.f.fmt_writer = Some(pi.ident.to_string());
                        }
                        f.params.push(MParam { name: "W".into(), bound: Some("Writer".into()) });
                        f.args.push(MArg { conv: Conv::Mut, name: "writer".into(), ty: Some(MType::named("W")), kw_only: false });
                        continue;
                    }
                    if spec.mode == FnMode::Reflected && pending_lhs.is_some() && !f.args.iter().any(|a| a.name == "self") {
                        // Rust rhs becomes Mojo self
                        let syn::Pat::Ident(pi) = &*pt.pat else {
                            return unsup("destructuring rhs in operator impl", tokens_str(&pt.pat));
                        };
                        f.args.push(self_arg(Conv::Default));
                        self.bind_expr(&pi.ident.to_string(), MExpr::name("self"), Some(strip_ref(&pt.ty).clone()));
                        f.args.push(pending_lhs.take().unwrap());
                        continue;
                    }
                    // impl Trait / impl Fn arguments
                    let mut ty: &Type = &pt.ty;
                    let mut by_ref = false;
                    let mut by_mut = false;
                    if let Type::Reference(r) = ty {
                        if let Type::ImplTrait(_) = &*r.elem {
                            by_ref = true;
                            by_mut = r.mutability.is_some();
                            ty = &r.elem;
                        }
                    }
                    let (conv, mty, is_ref_ty) = if let Type::ImplTrait(it) = ty {
                        impl_trait_n += 1;
                        let pname = format!("F{}", impl_trait_n);
                        let bound = self.lower_bounds(it.bounds.iter(), &pname, false)?;
                        if bound.starts_with("def(") {
                            f.params.push(MParam { name: pname.clone(), bound: Some(bound) });
                            (Conv::Default, MType::named(pname), false)
                        } else {
                            let traits: Vec<String> = bound.split(" & ").map(|s| s.to_string()).collect();
                            (if by_mut { Conv::Mut } else { Conv::Default }, MType::Some(traits), by_ref)
                        }
                    } else {
                        let mutable = matches!(&*pt.pat, syn::Pat::Ident(pi) if pi.mutability.is_some());
                        let (mut conv, mty) = self.lower_arg_type(&pt.ty, mutable)?;
                        let is_ref_ty = matches!(&*pt.ty, Type::Reference(_));
                        // by-value non-Copy argument: take ownership so it can be moved
                        let fn_param = rty_last(&pt.ty).map(|n| f.params.iter().any(|p| p.name == n && p.bound.as_deref().map(|b| b.starts_with("def(")).unwrap_or(false))).unwrap_or(false);
                        if conv == Conv::Default && !is_ref_ty && !fn_param && !self.is_copy_rty(&pt.ty) {
                            conv = Conv::Var;
                        }
                        (conv, mty, is_ref_ty)
                    };
                    let rty = strip_ref(&pt.ty).clone();
                    match &*pt.pat {
                        syn::Pat::Ident(pi) if pi.subpat.is_none() => {
                            let transferable = conv == Conv::Var;
                            let n = self.declare(&pi.ident.to_string(), Some(rty), is_ref_ty, transferable);
                            f.args.push(MArg { conv, name: n, ty: Some(mty), kw_only: false });
                        }
                        syn::Pat::Wild(_) => {
                            f.args.push(MArg { conv: if conv == Conv::Var { Conv::Default } else { conv }, name: format!("_unused{}", idx), ty: Some(mty), kw_only: false });
                        }
                        other => {
                            let n = format!("__arg{}", idx);
                            f.args.push(MArg { conv, name: n.clone(), ty: Some(mty), kw_only: false });
                            let po = self.lower_pat(other, &MExpr::name(&n), Some(&rty))?;
                            if !po.conds.is_empty() {
                                return unsup("refutable pattern in argument", tokens_str(other));
                            }
                            self.emit_binds(&po.binds, &mut prologue);
                        }
                    }
                }
            }
        }
        if spec.in_type && !receiver_seen && spec.mode == FnMode::Normal {
            // static method
        }
        // Return type
        match &sig.output {
            ReturnType::Default => {}
            ReturnType::Type(_, t) => {
                if spec.mode == FnMode::Fmt || spec.mode == FnMode::CopyInit || spec.mode == FnMode::Deinit {
                    // handled by mode
                } else {
                    self.lower_return_type(t, f, receiver_seen, &inputs)?;
                }
            }
        }
        let Some(block) = spec.block else {
            if self.ts.in_trait {
                f.body = None;
            } else {
                f.body = Some(vec![MStmt::Pass]);
            }
            return Ok(());
        };
        let mut body = prologue;
        let sink = if self.f.init_self { Sink::Assign(MExpr::name("self")) } else { Sink::Return };
        if self.f.init_self {
            // `return v` inside an initializer is handled as assignment to self
            self.f.ret_mty = None;
        }
        self.lower_block(block, &sink, &mut body)?;
        if !self.f.pending.is_empty() {
            return unsup("internal: unflushed hoisted statements", rust_fn_name(sig));
        }
        while matches!(body.last(), Some(MStmt::Return(None))) {
            body.pop();
        }
        f.body = Some(body);
        Ok(())
    }

    fn lower_return_type(&mut self, t: &Type, f: &mut MFunction, has_self: bool, inputs: &[&FnArg]) -> LResult<()> {
        if let Type::Tuple(tt) = t {
            if tt.elems.is_empty() {
                return Ok(());
            }
        }
        if let Type::ImplTrait(_) = t {
            return unsup("`impl Trait` return type", tokens_str(t));
        }
        if let Type::Never(_) = t {
            return unsup("diverging `-> !` return type", tokens_str(t));
        }
        if let Some(last) = rty_last(t) {
            if last == "Result" && !matches!(t, Type::Reference(_)) {
                let args = rty_args(t);
                f.raises = true;
                self.f.raises = true;
                let ok = args.first().cloned();
                if let Some(ok) = ok {
                    if !matches!(&ok, Type::Tuple(tt) if tt.elems.is_empty()) {
                        let mt = self.lower_type(&ok)?;
                        f.ret = RetType::Value(mt.clone());
                        self.f.ret_mty = Some(mt);
                        self.f.ret_rty = Some(ok);
                    }
                }
                let err = args.get(1).map(tokens_str).unwrap_or_else(|| "anyhow::Error".into());
                self.todo("Result<T, E> lowered to `raises` (error type erased)", format!("E = {}", err));
                return Ok(());
            }
            if last == "Option" {
                self.f.returns_option = true;
            }
        }
        if let Type::Reference(r) = t {
            if rty_last(&r.elem).as_deref() == Some("str") {
                self.todo("`&str` return lowered to owned String", tokens_str(t));
                f.ret = RetType::Value(MType::named("String"));
                self.f.ret_mty = Some(MType::named("String"));
                return Ok(());
            }
            let origin = if has_self {
                "self".to_string()
            } else {
                let refs: Vec<String> = inputs
                    .iter()
                    .filter_map(|i| match i {
                        FnArg::Typed(pt) if matches!(&*pt.ty, Type::Reference(_)) => match &*pt.pat {
                            syn::Pat::Ident(pi) => Some(names::ident(&pi.ident.to_string())),
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect();
                if refs.len() == 1 {
                    refs[0].clone()
                } else {
                    return unsup("returned reference with ambiguous origin (lifetime elision across several args)", tokens_str(t));
                }
            };
            if let Some(n) = rty_last(&r.elem) {
                if n == "str" {
                    // &str return -> owned String copy
                    self.todo("`&str` return lowered to owned String", tokens_str(t));
                    f.ret = RetType::Value(MType::named("String"));
                    self.f.ret_mty = Some(MType::named("String"));
                    return Ok(());
                }
            }
            if let Type::Slice(_) | Type::TraitObject(_) = &*r.elem {
                return unsup("returning a slice / trait-object reference", tokens_str(t));
            }
            let mt = self.lower_type(&r.elem)?;
            self.f.ret_rty = Some((*r.elem).clone());
            f.ret = RetType::Ref(origin, mt.clone());
            self.f.ret_mty = Some(mt);
            return Ok(());
        }
        let mt = self.lower_type(t)?;
        self.f.ret_rty = Some(t.clone());
        f.ret = RetType::Value(mt.clone());
        self.f.ret_mty = Some(mt);
        Ok(())
    }

    pub fn is_copy_rty(&self, t: &Type) -> bool {
        if is_prim_rty(t) {
            return true;
        }
        match rty_last(t) {
            Some(n) => {
                if self.syms.copy_types.contains(&n) {
                    return true;
                }
                if n == "Option" {
                    return rty_args(t).first().map(|a| self.is_copy_rty(a)).unwrap_or(false);
                }
                if n == "Self" {
                    if let Some(s) = &self.ts.self_name {
                        return self.syms.copy_types.contains(s);
                    }
                }
                false
            }
            None => match strip_ref(t) {
                Type::Tuple(tt) => tt.elems.iter().all(|e| self.is_copy_rty(e)),
                Type::Array(a) => self.is_copy_rty(&a.elem),
                _ => false,
            },
        }
    }
}

/// One header line per external crate, listing every dropped `use`.
fn push_ext(header: &mut Vec<String>, krate: &str, full: &str) {
    let prefix = format!("TODO(rust-to-mojo): external crate `{}` has no Mojo port; dropped: use ", krate);
    if let Some(line) = header.iter_mut().find(|l| l.starts_with(&prefix)) {
        line.push_str(", ");
        line.push_str(full);
    } else {
        header.push(format!("{}{}", prefix, full));
    }
}

/// Overload identity: name + argument types/conventions + where clauses.
fn method_key(f: &MFunction) -> String {
    let args: Vec<String> = f.args.iter().map(|a| format!("{:?}:{}", a.conv, a.ty.as_ref().map(crate::printer::type_str).unwrap_or_default())).collect();
    format!("{}({})|{}|{}", f.name, args.join(","), f.where_clauses.join("&"), f.params.len())
}

fn rust_fn_name(sig: &syn::Signature) -> String {
    sig.ident.to_string()
}

pub fn self_arg(conv: Conv) -> MArg {
    MArg { conv, name: "self".into(), ty: None, kw_only: false }
}

fn item_meta(item: &Item) -> (&'static str, String, Option<&Vec<syn::Attribute>>) {
    match item {
        Item::Fn(f) => ("fn", f.sig.ident.to_string(), Some(&f.attrs)),
        Item::Const(c) => ("const", c.ident.to_string(), Some(&c.attrs)),
        Item::Static(s) => ("static", s.ident.to_string(), Some(&s.attrs)),
        Item::Type(t) => ("type", t.ident.to_string(), Some(&t.attrs)),
        Item::Trait(t) => ("trait", t.ident.to_string(), Some(&t.attrs)),
        Item::Macro(m) => ("macro", m.ident.as_ref().map(|i| i.to_string()).unwrap_or_else(|| format!("{}!", path_last(&m.mac.path))), Some(&m.attrs)),
        Item::ForeignMod(f) => ("extern", "extern block".into(), Some(&f.attrs)),
        Item::Union(u) => ("union", u.ident.to_string(), Some(&u.attrs)),
        Item::ExternCrate(e) => ("extern crate", e.ident.to_string(), Some(&e.attrs)),
        Item::TraitAlias(t) => ("trait alias", t.ident.to_string(), Some(&t.attrs)),
        _ => ("item", "?".into(), None),
    }
}

fn param_trait_map(params: &[MParam]) -> HashMap<String, Vec<String>> {
    params
        .iter()
        .map(|p| (p.name.clone(), p.bound.clone().unwrap_or_default().split(" & ").map(|s| s.trim().to_string()).collect()))
        .collect()
}

fn first_type_arg(p: &syn::Path) -> Option<Type> {
    let seg = p.segments.last()?;
    if let syn::PathArguments::AngleBracketed(a) = &seg.arguments {
        for g in &a.args {
            if let syn::GenericArgument::Type(t) = g {
                return Some(t.clone());
            }
        }
    }
    None
}

fn is_deref_self(b: &syn::Block) -> bool {
    if b.stmts.len() != 1 {
        return false;
    }
    if let syn::Stmt::Expr(syn::Expr::Unary(u), None) = &b.stmts[0] {
        if matches!(u.op, syn::UnOp::Deref(_)) {
            if let syn::Expr::Path(p) = &*u.expr {
                return p.path.is_ident("self");
            }
        }
    }
    false
}

fn type_mentions(t: &MType, name: &str) -> bool {
    match t {
        MType::Named(n, a) => n == name || a.iter().any(|x| matches!(x, MTypeArg::Type(t) if type_mentions(t, name))),
        MType::Tuple(ts) => ts.iter().any(|t| type_mentions(t, name)),
        MType::Func(a, r) => a.iter().any(|t| type_mentions(t, name)) || r.as_ref().map(|r| type_mentions(r, name)).unwrap_or(false),
        _ => false,
    }
}

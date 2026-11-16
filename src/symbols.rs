//! Crate-wide symbol collection (pass 1). The lowering pass uses this to tell
//! enum variants from functions, tuple structs from tuples, user methods from
//! std methods, and to find struct field types for `..base` expansion.

use std::collections::{HashMap, HashSet};
use syn::{Fields, Item, ReturnType, Type};

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub name: String,
    pub generics: Vec<String>,
    /// (field name, type). Tuple-struct fields are named `0`, `1`, ...
    pub fields: Vec<(String, Type)>,
    pub tuple: bool,
    pub module: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VariantShape {
    Unit,
    Tuple(usize),
    Struct(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct EnumInfo {
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<(String, VariantShape)>,
    /// Field types of each variant, in declaration order.
    pub variant_types: HashMap<String, Vec<Type>>,
    pub c_like: bool,
    pub module: Vec<String>,
}

impl EnumInfo {
    pub fn variant(&self, name: &str) -> Option<&VariantShape> {
        self.variants.iter().find(|v| v.0 == name).map(|v| &v.1)
    }
}

/// How a struct field that has no direct Mojo type was lowered (decided by
/// the field probe pass, `FileCtx::probe_fields`, before any module is lowered
/// so that every module agrees on it).
#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    /// Type erased to `mojo_rt.erased.Erased`; the payload is the root-cause
    /// key (e.g. "`dyn Trait` ..."). Code touching the field is stubbed.
    Erased(String),
    /// `&'a T` lowered to `Pointer[T, Self.origin_a]`; reads need `[]`.
    Pointer,
    /// `&'static T` / `&'static [T]` stored by value (immutable static data).
    StaticValue,
}

/// A function or method signature in the crate (for closure-parameter and
/// `.into()` target inference at call sites).
#[derive(Clone, Debug)]
pub struct FnSig {
    /// Self type of the impl (last path segment), or the trait name.
    pub owner: Option<String>,
    pub sig: syn::Signature,
    /// Generics of the enclosing impl/trait (bounds on `F` may live there).
    pub impl_generics: syn::Generics,
    /// The impl's self type (`Context<T>`), for substituting impl params.
    pub self_ty: Option<Type>,
}

#[derive(Clone, Debug, Default)]
pub struct Symbols {
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub traits: HashSet<String>,
    /// Method names defined in any user `impl` block.
    pub user_methods: HashSet<String>,
    /// Free functions and methods (by simple name) whose return type is `Result`.
    pub result_fns: HashSet<String>,
    pub consts: HashSet<String>,
    pub type_aliases: HashSet<String>,
    /// Types that derive or implement `Copy`.
    pub copy_types: HashSet<String>,
    /// Types with a hand-written `Display` impl.
    pub display_types: HashSet<String>,
    /// Module paths in the crate, joined with `::` (crate root = "").
    pub modules: HashSet<String>,
    pub crate_name: String,
    /// Type names defined in more than one module (ambiguous by simple name).
    pub dup_types: HashSet<String>,
    /// Function/method name -> every signature with that name.
    pub fn_sigs: HashMap<String, Vec<FnSig>>,
    /// struct name -> field name -> non-direct lowering.
    pub field_kinds: HashMap<String, HashMap<String, FieldKind>>,
    /// Field names erased in some struct -> cause (used when the receiver
    /// type of a field access is unknown).
    pub erased_names: HashMap<String, String>,
    /// Field names lowered to `Pointer` in some struct.
    pub pointer_names: HashSet<String>,
    /// struct name -> lifetimes that became `origin_<lt>` parameters.
    pub origin_params: HashMap<String, Vec<String>>,
}

impl Symbols {
    /// Module defining type `name`, if it is a unique crate type.
    pub fn type_home(&self, name: &str) -> Option<&Vec<String>> {
        if self.dup_types.contains(name) {
            return None;
        }
        self.structs.get(name).map(|s| &s.module).or_else(|| self.enums.get(name).map(|e| &e.module))
    }

    pub fn field_kind(&self, strukt: &str, field: &str) -> Option<&FieldKind> {
        self.field_kinds.get(strukt).and_then(|m| m.get(field))
    }

    pub fn add_field_plans(&mut self, plans: Vec<(String, String, FieldKind)>, origins: Vec<(String, Vec<String>)>) {
        for (st, f, k) in plans {
            match &k {
                FieldKind::Erased(cause) => {
                    self.erased_names.entry(f.clone()).or_insert_with(|| cause.clone());
                }
                FieldKind::Pointer => {
                    self.pointer_names.insert(f.clone());
                }
                FieldKind::StaticValue => {}
            }
            self.field_kinds.entry(st).or_default().insert(f, k);
        }
        for (st, o) in origins {
            if !o.is_empty() {
                self.origin_params.insert(st, o);
            }
        }
    }
}

fn returns_result(ret: &ReturnType) -> bool {
    if let ReturnType::Type(_, t) = ret {
        if let Type::Path(p) = &**t {
            if let Some(seg) = p.path.segments.last() {
                return seg.ident == "Result";
            }
        }
    }
    false
}

impl Symbols {
    pub fn collect_items(&mut self, items: &[Item], module: &[String]) {
        for item in items {
            match item {
                Item::Struct(s) => {
                    let (fields, tuple) = match &s.fields {
                        Fields::Named(n) => (
                            n.named
                                .iter()
                                .map(|f| (f.ident.as_ref().unwrap().to_string(), f.ty.clone()))
                                .collect(),
                            false,
                        ),
                        Fields::Unnamed(u) => (
                            u.unnamed
                                .iter()
                                .enumerate()
                                .map(|(i, f)| (i.to_string(), f.ty.clone()))
                                .collect(),
                            true,
                        ),
                        Fields::Unit => (vec![], false),
                    };
                    let name = s.ident.to_string();
                    if self.structs.get(&name).map(|o| o.module != module).unwrap_or(false) || self.enums.contains_key(&name) {
                        self.dup_types.insert(name.clone());
                    }
                    if crate::lower::derives_of(&s.attrs).iter().any(|d| d == "Copy") {
                        self.copy_types.insert(name.clone());
                    }
                    self.structs.insert(
                        name.clone(),
                        StructInfo {
                            name,
                            generics: s.generics.type_params().map(|t| t.ident.to_string()).collect(),
                            fields,
                            tuple,
                            module: module.to_vec(),
                        },
                    );
                }
                Item::Enum(e) => {
                    let variants: Vec<(String, VariantShape)> = e
                        .variants
                        .iter()
                        .map(|v| {
                            let shape = match &v.fields {
                                Fields::Unit => VariantShape::Unit,
                                Fields::Unnamed(u) => VariantShape::Tuple(u.unnamed.len()),
                                Fields::Named(n) => VariantShape::Struct(
                                    n.named.iter().map(|f| f.ident.as_ref().unwrap().to_string()).collect(),
                                ),
                            };
                            (v.ident.to_string(), shape)
                        })
                        .collect();
                    if self.enums.get(&e.ident.to_string()).map(|o| o.module != module).unwrap_or(false) || self.structs.contains_key(&e.ident.to_string()) {
                        self.dup_types.insert(e.ident.to_string());
                    }
                    let c_like = variants.iter().all(|v| v.1 == VariantShape::Unit);
                    let variant_types: HashMap<String, Vec<Type>> = e.variants.iter().map(|v| (v.ident.to_string(), v.fields.iter().map(|f| f.ty.clone()).collect())).collect();
                    let name = e.ident.to_string();
                    if c_like || crate::lower::derives_of(&e.attrs).iter().any(|d| d == "Copy") {
                        self.copy_types.insert(name.clone());
                    }
                    self.enums.insert(
                        name.clone(),
                        EnumInfo {
                            name,
                            generics: e.generics.type_params().map(|t| t.ident.to_string()).collect(),
                            variants,
                            variant_types,
                            c_like,
                            module: module.to_vec(),
                        },
                    );
                }
                Item::Trait(t) => {
                    self.traits.insert(t.ident.to_string());
                    for ti in &t.items {
                        if let syn::TraitItem::Fn(f) = ti {
                            self.fn_sigs.entry(f.sig.ident.to_string()).or_default().push(FnSig { owner: Some(t.ident.to_string()), sig: f.sig.clone(), impl_generics: t.generics.clone(), self_ty: None });
                            self.user_methods.insert(f.sig.ident.to_string());
                            if returns_result(&f.sig.output) {
                                self.result_fns.insert(f.sig.ident.to_string());
                            }
                        }
                    }
                }
                Item::Impl(i) => {
                    if let Some((_, tp, _)) = &i.trait_ {
                        if tp.segments.last().map(|s| s.ident == "Display").unwrap_or(false) {
                            if let Type::Path(p) = &*i.self_ty {
                                if let Some(seg) = p.path.segments.last() {
                                    self.display_types.insert(seg.ident.to_string());
                                }
                            }
                        }
                        if tp.segments.last().map(|s| s.ident == "Copy").unwrap_or(false) {
                            if let Type::Path(p) = &*i.self_ty {
                                if let Some(seg) = p.path.segments.last() {
                                    self.copy_types.insert(seg.ident.to_string());
                                }
                            }
                        }
                    }
                    let owner = if let Type::Path(p) = &*i.self_ty { p.path.segments.last().map(|s| s.ident.to_string()) } else { None };
                    for ii in &i.items {
                        if let syn::ImplItem::Fn(f) = ii {
                            self.fn_sigs.entry(f.sig.ident.to_string()).or_default().push(FnSig { owner: owner.clone(), sig: f.sig.clone(), impl_generics: i.generics.clone(), self_ty: Some((*i.self_ty).clone()) });
                            self.user_methods.insert(f.sig.ident.to_string());
                            if returns_result(&f.sig.output) {
                                self.result_fns.insert(f.sig.ident.to_string());
                            }
                        }
                    }
                }
                Item::Fn(f) => {
                    self.fn_sigs.entry(f.sig.ident.to_string()).or_default().push(FnSig { owner: None, sig: f.sig.clone(), impl_generics: syn::Generics::default(), self_ty: None });
                    if returns_result(&f.sig.output) {
                        self.result_fns.insert(f.sig.ident.to_string());
                    }
                }
                Item::Const(c) => {
                    self.consts.insert(c.ident.to_string());
                }
                Item::Static(c) => {
                    self.consts.insert(c.ident.to_string());
                }
                Item::Type(t) => {
                    self.type_aliases.insert(t.ident.to_string());
                }
                Item::Mod(m) => {
                    let mut sub = module.to_vec();
                    sub.push(m.ident.to_string());
                    self.modules.insert(sub.join("::"));
                    if let Some((_, items)) = &m.content {
                        self.collect_items(items, &sub);
                    }
                }
                _ => {}
            }
        }
    }
}

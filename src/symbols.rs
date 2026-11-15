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
    pub c_like: bool,
    pub module: Vec<String>,
}

impl EnumInfo {
    pub fn variant(&self, name: &str) -> Option<&VariantShape> {
        self.variants.iter().find(|v| v.0 == name).map(|v| &v.1)
    }
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
    /// Module paths in the crate, joined with `::` (crate root = "").
    pub modules: HashSet<String>,
    pub crate_name: String,
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
                    let c_like = variants.iter().all(|v| v.1 == VariantShape::Unit);
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
                            c_like,
                            module: module.to_vec(),
                        },
                    );
                }
                Item::Trait(t) => {
                    self.traits.insert(t.ident.to_string());
                    for ti in &t.items {
                        if let syn::TraitItem::Fn(f) = ti {
                            self.user_methods.insert(f.sig.ident.to_string());
                            if returns_result(&f.sig.output) {
                                self.result_fns.insert(f.sig.ident.to_string());
                            }
                        }
                    }
                }
                Item::Impl(i) => {
                    if let Some((_, tp, _)) = &i.trait_ {
                        if tp.segments.last().map(|s| s.ident == "Copy").unwrap_or(false) {
                            if let Type::Path(p) = &*i.self_ty {
                                if let Some(seg) = p.path.segments.last() {
                                    self.copy_types.insert(seg.ident.to_string());
                                }
                            }
                        }
                    }
                    for ii in &i.items {
                        if let syn::ImplItem::Fn(f) = ii {
                            self.user_methods.insert(f.sig.ident.to_string());
                            if returns_result(&f.sig.output) {
                                self.result_fns.insert(f.sig.ident.to_string());
                            }
                        }
                    }
                }
                Item::Fn(f) => {
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

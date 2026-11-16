//! Call-site type inference for closures and `.into()`.
//!
//! Rust closures usually leave their parameter types to inference. Mojo
//! nested `def`s need them spelled out. The type is recovered from the
//! context the closure is passed into:
//!
//! * a crate function or method whose parameter is `impl Fn(A) -> R`,
//!   `F` with `F: Fn(A) -> R` (inline or in a `where` clause), `&dyn Fn(A)`,
//!   or `Box<dyn Fn(A)>`. Impl-level type parameters are substituted from the
//!   receiver's type (`Context<T>` called on a `Context<Foo>` gives `T = Foo`);
//! * `Option<T>` combinators (`map`, `and_then`, `unwrap_or_else`, ...): `T`.
//!
//! The return type, when the Rust closure does not annotate it, comes from the
//! same signature, or else from the type of a single-expression body.
//! Anything still unknown stays a stub ("closure with untyped parameters").

use super::types::*;
use super::*;
use crate::symbols::FnSig;
use syn::{PathArguments, ReturnType, Type, TypeParamBound};

/// What the context says about a closure about to be lowered.
#[derive(Clone, Debug, Default)]
pub struct ClosureHint {
    /// Rust type of each parameter (`None` = unknown).
    pub inputs: Vec<Option<Type>>,
    /// Rust return type: `Some(Some(t))` known, `Some(None)` unit, `None` unknown.
    pub output: Option<Option<Type>>,
}

/// `Fn(A, B) -> R` arguments of a bound list.
fn fn_sugar_of<'b>(bounds: impl Iterator<Item = &'b TypeParamBound>) -> Option<(Vec<Type>, Option<Type>)> {
    for b in bounds {
        if let TypeParamBound::Trait(tb) = b {
            let seg = tb.path.segments.last()?;
            if seg.ident == "Fn" || seg.ident == "FnMut" || seg.ident == "FnOnce" {
                if let PathArguments::Parenthesized(p) = &seg.arguments {
                    let out = match &p.output {
                        ReturnType::Default => None,
                        ReturnType::Type(_, t) => Some((**t).clone()),
                    };
                    return Some((p.inputs.iter().cloned().collect(), out));
                }
            }
        }
    }
    None
}

/// The `Fn` signature a parameter of type `ty` accepts, given the generics in scope.
pub fn fn_shape(ty: &Type, generics: &[&syn::Generics]) -> Option<(Vec<Type>, Option<Type>)> {
    match ty {
        Type::Reference(r) => fn_shape(&r.elem, generics),
        Type::Paren(p) => fn_shape(&p.elem, generics),
        Type::ImplTrait(it) => fn_sugar_of(it.bounds.iter()),
        Type::TraitObject(to) => fn_sugar_of(to.bounds.iter()),
        Type::Path(p) if p.qself.is_none() => {
            let seg = p.path.segments.last()?;
            let name = seg.ident.to_string();
            if ["Box", "Rc", "Arc", "Option"].contains(&name.as_str()) {
                let inner = rty_args(ty).into_iter().next()?;
                return fn_shape(&inner, generics);
            }
            if p.path.segments.len() != 1 || !seg.arguments.is_empty() {
                return None;
            }
            for g in generics {
                for tp in g.type_params() {
                    if tp.ident == name {
                        if let Some(s) = fn_sugar_of(tp.bounds.iter()) {
                            return Some(s);
                        }
                    }
                }
                if let Some(w) = &g.where_clause {
                    for pred in &w.predicates {
                        if let syn::WherePredicate::Type(pt) = pred {
                            if matches!(&pt.bounded_ty, Type::Path(bp) if bp.path.is_ident(&name)) {
                                if let Some(s) = fn_sugar_of(pt.bounds.iter()) {
                                    return Some(s);
                                }
                            }
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Does `t` mention any of `names` (as a path segment)?
pub fn mentions(t: &Type, names: &[String]) -> bool {
    use syn::visit::Visit;
    struct V<'n>(&'n [String], bool);
    impl<'n, 'ast> Visit<'ast> for V<'n> {
        fn visit_path(&mut self, p: &'ast syn::Path) {
            if p.segments.len() == 1 && self.0.iter().any(|n| p.segments[0].ident == n.as_str()) {
                self.1 = true;
            }
            syn::visit::visit_path(self, p);
        }
    }
    let mut v = V(names, false);
    v.visit_type(t);
    v.1
}

fn has_infer(t: &Type) -> bool {
    use syn::visit::Visit;
    struct V(bool);
    impl<'ast> Visit<'ast> for V {
        fn visit_type_infer(&mut self, _: &'ast syn::TypeInfer) {
            self.0 = true;
        }
    }
    let mut v = V(false);
    v.visit_type(t);
    v.0
}

impl<'a> FileCtx<'a> {
    /// Pick the signature a call refers to: by owner type when known, else
    /// the unique candidate with a matching argument count.
    pub fn find_sig(&self, name: &str, owner: Option<&str>, nargs: usize, method_call: bool) -> Option<FnSig> {
        let cands = self.syms.fn_sigs.get(name)?;
        let arity_ok = |s: &&FnSig| {
            let has_self = matches!(s.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
            let n = s.sig.inputs.len() - if has_self && method_call { 1 } else { 0 };
            n == nargs && (!method_call || has_self)
        };
        if let Some(o) = owner {
            let v: Vec<&FnSig> = cands.iter().filter(|s| s.owner.as_deref() == Some(o)).filter(arity_ok).collect();
            if v.len() == 1 {
                return Some(v[0].clone());
            }
            if !v.is_empty() {
                return None;
            }
        }
        if owner.is_some() && !method_call {
            return None;
        }
        // a plain `f(x)` call only names free functions
        let v: Vec<&FnSig> = cands.iter().filter(arity_ok).filter(|s| method_call || s.owner.is_none()).collect();
        if v.len() == 1 {
            Some(v[0].clone())
        } else {
            None
        }
    }

    /// Substitution of the impl's type parameters from the receiver type.
    fn impl_subst(&self, sig: &FnSig, recv_t: Option<&Type>) -> HashMap<String, Type> {
        let mut map = HashMap::new();
        let (Some(st), Some(rt)) = (&sig.self_ty, recv_t) else { return map };
        let params = rty_args(st);
        let args = rty_args(rt);
        if params.len() == args.len() {
            for (p, a) in params.iter().zip(args) {
                if let Some(n) = rty_last(p) {
                    if sig.impl_generics.type_params().any(|t| t.ident == n.as_str()) && !has_infer(&a) {
                        map.insert(n, a);
                    }
                }
            }
        }
        if let Some(rt) = recv_t {
            map.insert("Self".into(), strip_ref(rt).clone());
        }
        map
    }

    /// The closure hint for argument `idx` (0-based, receiver excluded for
    /// method calls) of a call to `sig`.
    pub fn closure_hint_for(&self, sig: &FnSig, idx: usize, method_call: bool, recv_t: Option<&Type>) -> Option<ClosureHint> {
        let has_self = matches!(sig.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
        let pos = idx + if has_self && method_call { 1 } else { 0 };
        let syn::FnArg::Typed(pt) = sig.sig.inputs.iter().nth(pos)? else { return None };
        let (inputs, output) = fn_shape(&pt.ty, &[&sig.sig.generics, &sig.impl_generics])?;
        let subst = self.impl_subst(sig, recv_t);
        // generic names that remain unknown after substitution
        let mut unknown: Vec<String> = sig.sig.generics.type_params().map(|t| t.ident.to_string()).collect();
        unknown.extend(sig.impl_generics.type_params().map(|t| t.ident.to_string()).filter(|n| !subst.contains_key(n)));
        if !subst.contains_key("Self") {
            unknown.push("Self".into());
        }
        let fix = |t: &Type| -> Option<Type> {
            let t = subst_rty(t, &subst);
            if mentions(&t, &unknown) || matches!(t, Type::ImplTrait(_)) {
                None
            } else {
                Some(t)
            }
        };
        Some(ClosureHint {
            inputs: inputs.iter().map(&fix).collect(),
            output: match &output {
                None => Some(None),
                Some(t) => fix(t).map(Some),
            },
        })
    }

    /// Lower call arguments, giving closures the parameter types `sig` implies.
    pub fn lower_args_with_sig(&mut self, args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>, sig: Option<&FnSig>, method_call: bool, recv_t: Option<&Type>) -> LResult<Vec<MExpr>> {
        let mut v = vec![];
        for (i, a) in args.iter().enumerate() {
            if let (Some(s), syn::Expr::Closure(_)) = (sig, strip_closure_wrappers(a)) {
                self.f.closure_hint = self.closure_hint_for(s, i, method_call, recv_t);
            }
            let exp = match sig {
                Some(s) if wants_expected_type(a) => self.param_mtype(s, i, method_call, recv_t),
                _ => None,
            };
            let x = self.lower_expr_exp(a, exp.as_ref());
            self.f.closure_hint = None;
            let x = x?;
            v.push(self.maybe_transfer(a, x));
        }
        Ok(v)
    }

    /// Mojo type of parameter `idx` of `sig`, when it is concrete (no unknown
    /// generic parameters, no `impl Trait`).
    pub fn param_mtype(&mut self, sig: &FnSig, idx: usize, method_call: bool, recv_t: Option<&Type>) -> Option<MType> {
        let has_self = matches!(sig.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
        let pos = idx + if has_self && method_call { 1 } else { 0 };
        let syn::FnArg::Typed(pt) = sig.sig.inputs.iter().nth(pos)? else { return None };
        let subst = self.impl_subst(sig, recv_t);
        let mut unknown: Vec<String> = sig.sig.generics.type_params().map(|t| t.ident.to_string()).collect();
        unknown.extend(sig.impl_generics.type_params().map(|t| t.ident.to_string()).filter(|n| !subst.contains_key(n)));
        unknown.push("Self".into());
        let t = subst_rty(strip_ref(&pt.ty), &subst);
        if mentions(&t, &unknown) || matches!(t, Type::ImplTrait(_) | Type::TraitObject(_)) {
            return None;
        }
        let saved = self.todos.len();
        let r = self.lower_type(&t).ok();
        self.todos.truncate(saved);
        r
    }

    /// Element type `T` of an `Option<T>` receiver, as a closure hint with one input.
    pub fn option_closure_hint(&self, recv_t: Option<&Type>, method: &str) -> Option<ClosureHint> {
        let t = recv_t?;
        if rty_last(t).as_deref() != Some("Option") {
            return None;
        }
        let elem = rty_args(t).into_iter().next()?;
        Some(match method {
            "unwrap_or_else" | "or_else" => ClosureHint { inputs: vec![], output: Some(Some(if method == "or_else" { t.clone() } else { elem })) },
            "is_some_and" | "is_none_or" | "filter" => ClosureHint { inputs: vec![Some(elem)], output: Some(Some(syn::parse_quote!(bool))) },
            "map" | "and_then" | "map_or" | "map_or_else" => ClosureHint { inputs: vec![Some(elem)], output: None },
            "inspect" => ClosureHint { inputs: vec![Some(elem)], output: Some(None) },
            _ => return None,
        })
    }
}

/// Arguments whose lowering needs the parameter type: `.into()` and
/// `Default::default()` (other arguments keep their own inferred types).
pub fn wants_expected_type(e: &syn::Expr) -> bool {
    match e {
        syn::Expr::Reference(r) => wants_expected_type(&r.expr),
        syn::Expr::Paren(p) => wants_expected_type(&p.expr),
        syn::Expr::MethodCall(mc) => mc.method == "into" && mc.args.is_empty(),
        syn::Expr::Call(c) => matches!(&*c.func, syn::Expr::Path(p) if p.path.segments.len() == 2 && p.path.segments[0].ident == "Default" && p.path.segments[1].ident == "default"),
        _ => false,
    }
}

/// `&|x| ..` / `Box::new(|x| ..)` still pass a closure.
pub fn strip_closure_wrappers(e: &syn::Expr) -> &syn::Expr {
    match e {
        syn::Expr::Reference(r) => strip_closure_wrappers(&r.expr),
        syn::Expr::Paren(p) => strip_closure_wrappers(&p.expr),
        _ => e,
    }
}

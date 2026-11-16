//! Type lowering.

use super::names;
use super::*;
use syn::{GenericArgument, PathArguments, ReturnType, Type, TypeParamBound};

/// Strip `&`, `&mut`, parens and groups.
pub fn strip_ref(t: &Type) -> &Type {
    match t {
        Type::Reference(r) => strip_ref(&r.elem),
        Type::Paren(p) => strip_ref(&p.elem),
        Type::Group(g) => strip_ref(&g.elem),
        _ => t,
    }
}

pub fn rty_last(t: &Type) -> Option<String> {
    match strip_ref(t) {
        Type::Path(p) if p.qself.is_none() => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

pub fn rty_args(t: &Type) -> Vec<Type> {
    if let Type::Path(p) = strip_ref(t) {
        if let Some(seg) = p.path.segments.last() {
            if let PathArguments::AngleBracketed(a) = &seg.arguments {
                return a
                    .args
                    .iter()
                    .filter_map(|g| if let GenericArgument::Type(t) = g { Some(t.clone()) } else { None })
                    .collect();
            }
        }
    }
    vec![]
}

pub fn is_float_rty(t: &Type) -> bool {
    rty_last(t).map(|n| names::is_float(&n)).unwrap_or(false)
}

pub fn is_int_rty(t: &Type) -> bool {
    rty_last(t).map(|n| names::is_int(&n)).unwrap_or(false)
}

pub fn is_unit_rty(t: &Type) -> bool {
    matches!(strip_ref(t), Type::Tuple(tt) if tt.elems.is_empty())
}

pub fn is_prim_rty(t: &Type) -> bool {
    rty_last(t).map(|n| names::is_numeric(&n) || n == "bool" || n == "char").unwrap_or(false)
}

pub fn parse_rty(s: &str) -> Option<Type> {
    syn::parse_str::<Type>(s).ok()
}

/// Replace generic parameter names inside a type (e.g. T -> Pixels).
pub fn subst_rty(t: &Type, map: &HashMap<String, Type>) -> Type {
    use syn::visit_mut::VisitMut;
    struct S<'a>(&'a HashMap<String, Type>);
    impl<'a> VisitMut for S<'a> {
        fn visit_type_mut(&mut self, t: &mut Type) {
            if let Type::Path(p) = t {
                if p.qself.is_none() && p.path.segments.len() == 1 && p.path.segments[0].arguments.is_empty() {
                    if let Some(r) = self.0.get(&p.path.segments[0].ident.to_string()) {
                        *t = r.clone();
                        return;
                    }
                }
            }
            syn::visit_mut::visit_type_mut(self, t);
        }
    }
    let mut out = t.clone();
    S(map).visit_type_mut(&mut out);
    out
}

/// Is `seg` (the last segment of an operator-trait bound on `owner`) of the
/// form `Add<Output = T>`, `Add<T, Output = T>`, `AddAssign`, `AddAssign<T>` or
/// `Neg<Output = T>`, i.e. an operator whose operands and result are all the
/// bounded parameter itself? Only those can be lowered to `mojo_rt.ops`.
pub fn self_typed_op_bound(trait_name: &str, seg: &syn::PathSegment, owner: &str) -> bool {
    if !["Add", "Sub", "Mul", "Div", "Rem", "Neg", "AddAssign", "SubAssign", "MulAssign", "DivAssign", "RemAssign"].contains(&trait_name) {
        return false;
    }
    let is_owner = |t: &Type| matches!(t, Type::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 && p.path.segments[0].arguments.is_empty() && (p.path.segments[0].ident == owner || p.path.segments[0].ident == "Self"));
    // `Mul<S, Output = T>` / `Div<f32, Output = T>`: `ops.mul_rhs` / `ops.div_rhs`
    let scalar_rhs = |t: &Type| ["Mul", "Div"].contains(&trait_name) && matches!(t, Type::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 && p.path.segments[0].arguments.is_empty());
    let assign = trait_name.ends_with("Assign");
    let mut output_ok = assign;
    match &seg.arguments {
        PathArguments::None => {}
        PathArguments::AngleBracketed(a) => {
            for g in &a.args {
                match g {
                    GenericArgument::Type(t) => {
                        if !is_owner(t) && !scalar_rhs(t) {
                            return false;
                        }
                    }
                    GenericArgument::AssocType(at) if at.ident == "Output" => {
                        if !is_owner(&at.ty) {
                            return false;
                        }
                        output_ok = true;
                    }
                    GenericArgument::Lifetime(_) => {}
                    _ => return false,
                }
            }
        }
        PathArguments::Parenthesized(_) => return false,
    }
    output_ok
}

/// `mojo_rt.ops` function for an arithmetic operator on a generic parameter.
pub fn ops_fn(op: BinOp) -> Option<&'static str> {
    Some(match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::Mod => "rem",
        _ => return None,
    })
}

impl<'a> FileCtx<'a> {
    /// Is `t` a bare generic type parameter of the current scope (`T`)?
    pub fn is_param_rty(&self, t: &Type) -> bool {
        match strip_ref(t) {
            Type::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 && p.path.segments[0].arguments.is_empty() => self.ts.params.contains_key(&p.path.segments[0].ident.to_string()),
            _ => false,
        }
    }

    /// Lower a type in value position (fields, `var` annotations, generic args).
    pub fn lower_type(&mut self, ty: &Type) -> LResult<MType> {
        match ty {
            Type::Paren(p) => self.lower_type(&p.elem),
            Type::Group(g) => self.lower_type(&g.elem),
            Type::Tuple(t) => {
                if t.elems.is_empty() {
                    return Ok(MType::named("NoneType"));
                }
                let mut v = vec![];
                for e in &t.elems {
                    v.push(self.lower_type(e)?);
                }
                Ok(MType::Tuple(v))
            }
            Type::Array(a) => {
                let elem = self.lower_type(&a.elem)?;
                let len = self.lower_const_expr(&a.len)?;
                Ok(MType::Named("Array".into(), vec![MTypeArg::Type(elem), MTypeArg::Expr(len)]))
            }
            Type::Reference(r) => {
                // `&str` / `&'static str` -> String is a sound value-level lowering.
                if let Some(n) = rty_last(&r.elem) {
                    if n == "str" {
                        return Ok(MType::named("String"));
                    }
                }
                unsup(
                    "reference type in field/value position (needs Pointer + origin)",
                    format!("type `{}`", tokens_str(ty)),
                )
            }
            Type::Slice(_) => unsup("unsized slice type `[T]`", tokens_str(ty)),
            Type::TraitObject(_) => unsup("`dyn Trait` (no existential types in Mojo)", tokens_str(ty)),
            Type::ImplTrait(_) => unsup("`impl Trait` outside argument position", tokens_str(ty)),
            Type::Ptr(_) => unsup("raw pointer type", tokens_str(ty)),
            Type::Never(_) => unsup("never type `!`", tokens_str(ty)),
            Type::Infer(_) => Ok(MType::Infer),
            Type::BareFn(f) => {
                let mut args = vec![];
                for a in &f.inputs {
                    args.push(self.lower_type(&a.ty)?);
                }
                let ret = match &f.output {
                    ReturnType::Default => None,
                    ReturnType::Type(_, t) => Some(Box::new(self.lower_type(t)?)),
                };
                Ok(MType::Func(args, ret))
            }
            Type::Macro(_) => unsup("type macro", tokens_str(ty)),
            Type::Path(p) => {
                if p.qself.is_some() {
                    return unsup("qualified path type `<T as Trait>::X`", tokens_str(ty));
                }
                self.lower_path_type(&p.path, ty)
            }
            _ => unsup("unsupported type syntax", tokens_str(ty)),
        }
    }

    fn lower_generic_args(&mut self, args: &PathArguments) -> LResult<Vec<MTypeArg>> {
        let mut out = vec![];
        match args {
            PathArguments::None => {}
            PathArguments::AngleBracketed(a) => {
                for g in &a.args {
                    match g {
                        GenericArgument::Lifetime(_) => {}
                        GenericArgument::Type(t) => out.push(MTypeArg::Type(self.lower_type(t)?)),
                        GenericArgument::Const(e) => out.push(MTypeArg::Expr(self.lower_const_expr(e)?)),
                        _ => return unsup("associated-type generic argument", tokens_str(g)),
                    }
                }
            }
            PathArguments::Parenthesized(_) => return unsup("Fn-sugar type outside `impl Fn` argument", tokens_str(args)),
        }
        Ok(out)
    }

    pub fn lower_const_expr(&mut self, e: &syn::Expr) -> LResult<MExpr> {
        match e {
            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. }) => Ok(MExpr::Int(i.base10_digits().to_string())),
            syn::Expr::Path(p) if p.path.segments.len() == 1 => {
                let n = p.path.segments[0].ident.to_string();
                if let Some(m) = self.ts.params.get(&n) {
                    return Ok(MExpr::name(m.clone()));
                }
                Ok(MExpr::name(names::ident(&n)))
            }
            syn::Expr::Block(b) if b.block.stmts.len() == 1 => {
                if let syn::Stmt::Expr(inner, None) = &b.block.stmts[0] {
                    return self.lower_const_expr(inner);
                }
                unsup("const generic expression", tokens_str(e))
            }
            _ => unsup("const generic expression", tokens_str(e)),
        }
    }

    fn lower_path_type(&mut self, path: &syn::Path, whole: &Type) -> LResult<MType> {
        let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let last = path.segments.last().unwrap();
        let name = last.ident.to_string();

        // `Self::Unit` / `T::Output`
        if segs.len() == 2 && segs[0] == "Self" {
            if let Some(t) = self.ts.assoc.get(&segs[1]).cloned() {
                return self.lower_type(&t);
            }
            return Ok(MType::named(format!("Self.{}", segs[1])));
        }
        if segs.len() >= 2 && self.ts.params.contains_key(&segs[0]) {
            return unsup("associated type projection on a generic parameter", tokens_str(whole));
        }

        if segs.len() == 1 {
            if let Some(m) = self.ts.params.get(&name) {
                return Ok(MType::named(m.clone()));
            }
            if name == "Self" {
                if let Some(t) = &self.ts.self_concrete {
                    return Ok(t.clone());
                }
                return Ok(MType::named("Self"));
            }
        }
        if let Some(p) = names::prim(&name) {
            if last.arguments.is_empty() {
                return Ok(MType::named(p));
            }
        }
        let args = self.lower_generic_args(&last.arguments)?;
        let arg_types: Vec<MType> = args
            .iter()
            .filter_map(|a| if let MTypeArg::Type(t) = a { Some(t.clone()) } else { None })
            .collect();
        let one = |n: &str| -> LResult<MType> {
            if arg_types.len() == 1 {
                Ok(MType::generic(n, arg_types.clone()))
            } else {
                unsup("unexpected generic arity", tokens_str(whole))
            }
        };
        match name.as_str() {
            "Vec" | "SmallVec" => {
                if name == "SmallVec" {
                    // SmallVec<[T; N]>
                    if let Some(MType::Named(n, a)) = arg_types.first() {
                        if n == "Array" {
                            if let Some(MTypeArg::Type(t)) = a.first() {
                                self.todo("SmallVec lowered to List (no inline storage)", "SmallVec<[T; N]> -> List[T]");
                                return Ok(MType::generic("List", vec![t.clone()]));
                            }
                        }
                    }
                    return unsup("SmallVec with unexpected arguments", tokens_str(whole));
                }
                one("List")
            }
            "VecDeque" => {
                self.import("std.collections", "Deque");
                one("Deque")
            }
            "HashMap" | "FxHashMap" | "IndexMap" => {
                if arg_types.len() >= 2 {
                    if name == "IndexMap" {
                        self.todo("IndexMap lowered to Dict", "Mojo Dict preserves insertion order; verify API parity");
                    }
                    Ok(MType::generic("Dict", arg_types[..2].to_vec()))
                } else {
                    unsup("unexpected generic arity", tokens_str(whole))
                }
            }
            "BTreeMap" => unsup("BTreeMap (ordered map; no Mojo std equivalent)", tokens_str(whole)),
            "BTreeSet" => unsup("BTreeSet (ordered set; no Mojo std equivalent)", tokens_str(whole)),
            "HashSet" | "FxHashSet" | "IndexSet" => {
                self.import("std.collections", "Set");
                one("Set")
            }
            "Option" => one("Optional"),
            "Box" => {
                if let Some(MType::Func(..)) = arg_types.first() {
                    return unsup("boxed closure `Box<dyn Fn>`", tokens_str(whole));
                }
                self.import("std.memory", "OwnedPointer");
                one("OwnedPointer")
            }
            "Rc" | "Arc" => {
                self.import("std.memory", "ArcPointer");
                if name == "Rc" {
                    self.todo("Rc lowered to ArcPointer (atomic refcount)", "Rc<T> -> ArcPointer[T]");
                }
                one("ArcPointer")
            }
            "Weak" => {
                // Mojo 1.0 added WeakPointer[T] for weak refs to ArcPointer[T].
                self.import("std.memory", "WeakPointer");
                self.todo("Weak<T> lowered to WeakPointer[T] (1.0 API unverified; upgrade() calls are not mapped)", tokens_str(whole));
                one("WeakPointer")
            }
            "RefCell" => {
                self.import("mojo_rt.cell", "RefCell");
                one("RefCell")
            }
            "Cell" => {
                self.import("mojo_rt.cell", "Cell");
                one("Cell")
            }
            "Range" => {
                self.import("mojo_rt.range", "RustRange");
                one("RustRange")
            }
            "Cow" => {
                if let Some(t) = arg_types.first() {
                    if t.is_named("String") {
                        self.todo("Cow<str> lowered to owned String", "Cow<'_, str> -> String");
                        return Ok(MType::named("String"));
                    }
                }
                unsup("Cow<T> (copy-on-write)", tokens_str(whole))
            }
            "PhantomData" => unsup("PhantomData (handled at field level)", tokens_str(whole)),
            "Result" => unsup("Result type outside a function return position", tokens_str(whole)),
            "Mutex" | "RwLock" | "ReentrantMutex" => {
                // parking_lot / std / spin locks -> single-owner shim
                let shim = if name == "RwLock" { "RwLock" } else { "Mutex" };
                self.import("mojo_rt.sync", shim);
                self.todo(
                    "lock lowered to mojo_rt.sync shim (no mutual exclusion; guards are plain references)",
                    format!("{} -> mojo_rt.sync.{}", tokens_str(whole), shim),
                );
                one(shim)
            }
            "Pin" => unsup("Pin<T>", tokens_str(whole)),
            "SharedString" | "ArcStr" => {
                // gpui's immutable shared string. Lower to String (owned copy).
                self.todo("shared/immutable string lowered to String", format!("{} -> String", name));
                Ok(MType::named("String"))
            }
            _ => {
                if let Some(os) = self.syms.origin_params.get(&name) {
                    // struct with borrowed-reference fields: origin parameters are
                    // left unbound at use sites (`Foo[_, T]`)
                    let own = self.ts.self_name.as_deref() == Some(name.as_str());
                    let mut a: Vec<MTypeArg> = os.iter().map(|o| MTypeArg::Type(if own { MType::named(format!("Self.origin_{}", o)) } else { MType::Infer })).collect();
                    a.extend(args);
                    return Ok(MType::Named(name, a));
                }
                if self.syms.type_aliases.contains(&name) || self.syms.structs.contains_key(&name) || self.syms.enums.contains_key(&name) {
                    // user type
                } else if segs.len() > 1 || !name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    // path into another crate / std module we do not model
                    if ["std", "core", "alloc"].contains(&segs[0].as_str()) {
                        return unsup("std type with no Mojo mapping", tokens_str(whole));
                    }
                }
                Ok(MType::Named(name, args))
            }
        }
    }

    /// `Option<&T>` in argument/return position -> `Optional[T]` (a copy of the
    /// referent). Shared references are read-only, so only identity and cost
    /// differ (and interior mutability seen through another path).
    pub fn option_ref_type(&mut self, ty: &Type) -> Option<LResult<MType>> {
        let Type::Path(p) = ty else { return None };
        let seg = p.path.segments.last()?;
        if seg.ident != "Option" || p.qself.is_some() {
            return None;
        }
        let inner = rty_args(ty).into_iter().next()?;
        let Type::Reference(r) = &inner else { return None };
        if r.mutability.is_some() || matches!(&*r.elem, Type::TraitObject(_) | Type::Slice(_)) {
            return None;
        }
        Some(self.lower_type(&r.elem).map(|t| {
            self.todo("`Option<&T>` lowered to `Optional[T]` (copies the referent)", tokens_str(ty));
            MType::generic("Optional", vec![t])
        }))
    }

    /// Lower an argument type: returns the convention and the value type.
    pub fn lower_arg_type(&mut self, ty: &Type, mutable_binding: bool) -> LResult<(Conv, MType)> {
        if let Some(r) = self.option_ref_type(ty) {
            return Ok((Conv::Default, r?));
        }
        match ty {
            Type::Reference(r) => {
                let inner = &*r.elem;
                if let Type::Slice(s) = inner {
                    let elem = self.lower_type(&s.elem)?;
                    self.todo(
                        "slice argument `&[T]` lowered to List[T] borrow",
                        "callers passing sub-slices need Span[T, origin]",
                    );
                    let conv = if r.mutability.is_some() { Conv::Mut } else { Conv::Default };
                    return Ok((conv, MType::generic("List", vec![elem])));
                }
                if let Type::TraitObject(_) = inner {
                    return unsup("`&dyn Trait` argument (no existential types in Mojo)", tokens_str(ty));
                }
                if let Type::ImplTrait(_) = inner {
                    return unsup("`&impl Trait` argument", tokens_str(ty));
                }
                let mt = self.lower_type(inner)?;
                Ok((if r.mutability.is_some() { Conv::Mut } else { Conv::Default }, mt))
            }
            _ => {
                let mt = self.lower_type(ty)?;
                Ok((if mutable_binding { Conv::Var } else { Conv::Default }, mt))
            }
        }
    }

    /// Map a Rust bound list to a Mojo bound string. Bounds with no Mojo
    /// counterpart are dropped with a TODO note.
    pub fn lower_bounds<'b>(&mut self, bounds: impl Iterator<Item = &'b TypeParamBound>, owner: &str, add_base: bool) -> LResult<String> {
        let mut out: Vec<String> = vec![];
        let mut fn_bound: Option<String> = None;
        for b in bounds {
            match b {
                TypeParamBound::Trait(tb) => {
                    if matches!(tb.modifier, syn::TraitBoundModifier::Maybe(_)) {
                        continue; // ?Sized
                    }
                    let last = tb.path.segments.last().unwrap();
                    let n = last.ident.to_string();
                    if names::erased_trait(&n) {
                        continue;
                    }
                    if n == "Fn" || n == "FnMut" || n == "FnOnce" {
                        fn_bound = Some(self.fn_sugar_type(last)?);
                        continue;
                    }
                    if let Some(m) = names::std_trait(&n) {
                        if !out.contains(&m.to_string()) {
                            out.push(m.to_string());
                        }
                        continue;
                    }
                    if names::op_trait(&n).is_some() && self_typed_op_bound(&n, last, owner) {
                        // `T: Add<Output = T>`: operators on `T` are lowered to
                        // `mojo_rt.ops` dispatch calls, which check the bound
                        // when the generic is instantiated (see MAPPING.md 6.1).
                        continue;
                    }
                    if names::op_trait(&n).is_some() {
                        // `T: Mul<Rhs, Output = Rhs>`: the result is not `T`, and
                        // `mojo_rt.ops` can only return `T`.
                        let out_ty = match &last.arguments {
                            syn::PathArguments::AngleBracketed(a) => a.args.iter().find_map(|g| match g {
                                syn::GenericArgument::AssocType(at) if at.ident == "Output" => Some(tokens_str(&at.ty)),
                                _ => None,
                            }),
                            _ => None,
                        };
                        if let Some(o) = out_ty {
                            if o != owner && o != "Self" && !owner.ends_with(&format!(".{}", o)) {
                                return unsup("operator bound whose `Output` is not the parameter itself", format!("{}: {}", owner, tokens_str(&tb.path)));
                            }
                        }
                    }
                    if names::op_trait(&n).is_some() || ["Sum", "Product", "Into", "From", "AsRef", "Borrow", "Neg", "Zero", "One"].contains(&n.as_str()) {
                        self.todo(
                            format!("dropped bound `{}` (no Mojo std trait)", n),
                            format!("`{}: {}`: operators on generic params need a user trait or DType parameterisation", owner, tokens_str(&tb.path)),
                        );
                        continue;
                    }
                    if !last.arguments.is_empty() {
                        return unsup("bound on a generic trait", tokens_str(&tb.path));
                    }
                    if self.syms.traits.contains(&n) {
                        out.push(n);
                    } else {
                        self.todo(format!("bound on external trait `{}`", n), format!("`{}: {}` kept; trait must be ported", owner, n));
                        out.push(n);
                    }
                }
                TypeParamBound::Lifetime(_) => {}
                _ => {}
            }
        }
        if let Some(f) = fn_bound {
            if !out.is_empty() {
                return unsup("closure bound combined with other traits", owner.to_string());
            }
            return Ok(f);
        }
        if add_base {
            if !out.iter().any(|t| t == "Copyable" || t == "ImplicitlyCopyable") {
                out.insert(0, "Movable".to_string());
            }
            out.push("Deinitable".to_string());
        }
        if out.is_empty() {
            Ok("AnyType".to_string())
        } else {
            Ok(out.join(" & "))
        }
    }

    /// `Fn(A, B) -> R` -> `def(A, B) -> R`
    pub fn fn_sugar_type(&mut self, seg: &syn::PathSegment) -> LResult<String> {
        if let PathArguments::Parenthesized(p) = &seg.arguments {
            let mut args = vec![];
            let mut muts = vec![];
            for i in &p.inputs {
                if let Type::Reference(r) = i {
                    // `&T` argument == Mojo's default (immutable borrow) convention;
                    // `&mut T` == a `mut` argument (`def(mut Bencher) -> None`)
                    muts.push(r.mutability.is_some());
                    args.push(self.lower_type(&r.elem)?);
                    continue;
                }
                muts.push(false);
                args.push(self.lower_type(i)?);
            }
            let ret = match &p.output {
                ReturnType::Default => None,
                ReturnType::Type(_, t) => {
                    if let Type::Reference(_) = &**t {
                        return unsup("closure returning a reference", tokens_str(seg));
                    }
                    Some(Box::new(self.lower_type(t)?))
                }
            };
            if seg.ident == "FnMut" {
                self.todo("FnMut closure parameter", "Mojo closures mutate captures via `{mut x}` capture lists; verify");
            }
            if muts.iter().any(|m| *m) {
                let a: Vec<String> = args.iter().zip(&muts).map(|(t, m)| format!("{}{}", if *m { "mut " } else { "" }, crate::printer::type_str(t))).collect();
                let r = ret.map(|r| format!(" -> {}", crate::printer::type_str(&r))).unwrap_or_default();
                return Ok(format!("def({}){}", a.join(", "), r));
            }
            return Ok(crate::printer::type_str(&MType::Func(args, ret)));
        }
        unsup("Fn trait without parenthesized arguments", tokens_str(seg))
    }

    /// Lower generic params of an item. `struct_scope` registers the names as
    /// `Self.T` for struct bodies; otherwise as plain `T`.
    pub fn lower_generics(&mut self, g: &syn::Generics, struct_scope: bool, add_base: bool) -> LResult<Vec<MParam>> {
        let mut params = vec![];
        // where-clause bounds, merged into params
        let mut extra: HashMap<String, Vec<TypeParamBound>> = HashMap::new();
        if let Some(w) = &g.where_clause {
            for p in &w.predicates {
                match p {
                    syn::WherePredicate::Type(pt) => {
                        if pt.lifetimes.is_some() {
                            return unsup("higher-ranked where clause", tokens_str(pt));
                        }
                        let bt = &pt.bounded_ty;
                        let key = match bt {
                            Type::Path(tp) if tp.qself.is_none() && tp.path.segments.len() == 1 => tp.path.segments[0].ident.to_string(),
                            _ => {
                                self.todo("dropped complex where clause", tokens_str(pt));
                                continue;
                            }
                        };
                        if !g.type_params().any(|t| t.ident == key) {
                            self.todo("dropped where clause on non-parameter type", tokens_str(pt));
                            continue;
                        }
                        extra.entry(key).or_default().extend(pt.bounds.iter().cloned());
                    }
                    syn::WherePredicate::Lifetime(_) => {}
                    _ => {}
                }
            }
        }
        for gp in &g.params {
            match gp {
                syn::GenericParam::Lifetime(_) => {}
                syn::GenericParam::Type(tp) => {
                    let n = tp.ident.to_string();
                    let mut all: Vec<TypeParamBound> = tp.bounds.iter().cloned().collect();
                    if let Some(e) = extra.get(&n) {
                        all.extend(e.iter().cloned());
                    }
                    if tp.default.is_some() {
                        self.todo("dropped default for generic parameter", format!("{} = ...", n));
                    }
                    let bound = self.lower_bounds(all.iter(), &n, add_base)?;
                    params.push(MParam { name: n.clone(), bound: Some(bound) });
                    self.ts.params.insert(n.clone(), if struct_scope { format!("Self.{}", n) } else { n.clone() });
                }
                syn::GenericParam::Const(cp) => {
                    let n = cp.ident.to_string();
                    let t = self.lower_type(&cp.ty)?;
                    params.push(MParam { name: n.clone(), bound: Some(crate::printer::type_str(&t)) });
                    self.ts.params.insert(n.clone(), if struct_scope { format!("Self.{}", n) } else { n.clone() });
                }
            }
        }
        Ok(params)
    }
}

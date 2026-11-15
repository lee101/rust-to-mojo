//! Expression and statement lowering.

use super::names;
use super::types::*;
use super::*;
use syn::{BinOp as RB, Expr, Lit, Stmt, UnOp as RU};

/// Where the value of a tail expression goes.
#[derive(Clone, Debug)]
pub enum Sink {
    Return,
    Discard,
    Assign(MExpr),
}

fn is_place(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => p.qself.is_none(),
        Expr::Field(f) => is_place(&f.base),
        Expr::Paren(p) => is_place(&p.expr),
        Expr::Reference(r) => is_place(&r.expr),
        Expr::Unary(u) if matches!(u.op, RU::Deref(_)) => is_place(&u.expr),
        Expr::Index(i) => is_place(&i.expr) && matches!(&*i.index, Expr::Lit(_) | Expr::Path(_)),
        _ => false,
    }
}

pub fn single_ident(e: &Expr) -> Option<String> {
    match e {
        Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 && p.path.segments[0].arguments.is_empty() => {
            Some(p.path.segments[0].ident.to_string())
        }
        Expr::Paren(p) => single_ident(&p.expr),
        _ => None,
    }
}

/// Count identifier uses in a function body (used for `^` transfer insertion).
pub fn count_uses(block: &syn::Block) -> HashMap<String, usize> {
    use syn::visit::Visit;
    struct V(HashMap<String, usize>);
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            if p.path.segments.len() == 1 {
                *self.0.entry(p.path.segments[0].ident.to_string()).or_default() += 1;
            }
            syn::visit::visit_expr_path(self, p);
        }
        fn visit_macro(&mut self, m: &'ast syn::Macro) {
            for t in m.tokens.clone() {
                count_tokens(&mut self.0, t);
            }
        }
        fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
            // uses inside closures never get transferred
            let mut inner = V(HashMap::new());
            syn::visit::visit_expr_closure(&mut inner, c);
            for (k, _) in inner.0 {
                *self.0.entry(k).or_default() += 100;
            }
        }
    }
    fn count_tokens(m: &mut HashMap<String, usize>, t: proc_macro2::TokenTree) {
        match t {
            proc_macro2::TokenTree::Ident(i) => *m.entry(i.to_string()).or_default() += 1,
            proc_macro2::TokenTree::Group(g) => {
                for t in g.stream() {
                    count_tokens(m, t);
                }
            }
            _ => {}
        }
    }
    let mut v = V(HashMap::new());
    v.visit_block(block);
    v.0
}

/// Rust iterator/closure-based methods we do not lower yet.
const ITER_METHODS: &[&str] = &[
    "iter", "iter_mut", "into_iter", "map", "filter", "filter_map", "flat_map", "flatten", "fold", "reduce", "collect",
    "sum", "product", "any", "all", "find", "find_map", "position", "rposition", "zip", "chain", "enumerate", "rev",
    "skip", "step_by", "cloned", "copied", "count", "min_by", "max_by", "min_by_key", "max_by_key", "sort_by",
    "sort_by_key", "sort_unstable_by", "sort_unstable_by_key", "sort_by_cached_key", "windows", "chunks", "peekable",
    "next", "for_each", "inspect", "scan", "take_while", "skip_while", "map_while", "dedup_by_key", "retain", "drain",
    "split", "splitn", "rsplit", "lines", "bytes", "char_indices", "split_whitespace", "map_or", "map_or_else",
    "unwrap_or_else", "ok_or", "ok_or_else", "and_then", "or_else", "get_or_insert_with", "entry", "or_insert",
    "or_insert_with", "or_default", "then", "then_some", "map_err", "context", "with_context", "is_some_and",
    "is_none_or", "zip_with", "last", "nth", "binary_search_by", "binary_search_by_key", "partition", "unzip",
    "extend_from_slice", "iter_with", "par_iter", "chars", "into_keys", "into_values", "keys", "values", "values_mut",
    "split_at", "chunks_exact", "fuse", "cycle", "sum_by", "filter_map_ok", "into_iter_sorted",
];

/// Rust std methods that need a mapping we have not written; never pass
/// these through silently on non-user receivers.
const KNOWN_STD_UNMAPPED: &[&str] = &[
    "partial_cmp", "cmp", "fmt", "hash", "as_mut", "as_deref", "as_deref_mut", "unwrap_unchecked", "get_unchecked",
    "to_bits", "from_bits", "to_be_bytes", "to_le_bytes", "to_ne_bytes", "try_into", "try_from", "parse", "as_ptr",
    "as_mut_ptr", "leak", "into_boxed_slice", "into_inner", "get_mut", "split_off", "dedup", "binary_search",
    "starts_with", "ends_with", "strip_prefix", "strip_suffix", "find", "rfind", "char_indices", "is_char_boundary",
    "as_bytes", "upgrade", "downgrade", "ptr_eq", "make_mut", "try_borrow", "try_borrow_mut", "replace_with",
    "swap_remove", "sort_unstable", "concat", "join", "repeat", "is_sorted", "rotate_left", "rotate_right",
    "is_sign_negative", "is_sign_positive", "total_cmp", "lerp", "midpoint", "isqrt", "ilog2", "ilog10",
    "is_power_of_two", "next_power_of_two", "unsigned_abs", "checked_div_euclid", "to_ascii_lowercase",
    "to_ascii_uppercase", "eq_ignore_ascii_case", "is_ascii", "is_alphabetic", "is_alphanumeric", "is_whitespace",
    "is_numeric", "is_ascii_digit", "to_digit", "len_utf8", "encode_utf8", "write_str", "write_fmt", "debug_struct",
    "debug_tuple", "debug_list", "debug_map", "finish", "field", "with_capacity", "capacity", "shrink_to_fit",
];

#[derive(Clone, Debug, PartialEq)]
enum RecvKind {
    User(String),
    Param,
    Vec,
    Map,
    Set,
    Opt,
    Str,
    Float,
    Int,
    Bool,
    Rc,
    Boxed,
    RefCell,
    Cell,
    Tuple,
    Unknown,
}

impl<'a> FileCtx<'a> {
    // ======================================================================
    // Blocks and statements
    // ======================================================================

    pub fn lower_block(&mut self, b: &syn::Block, sink: &Sink, out: &mut Vec<MStmt>) -> LResult<()> {
        self.push_scope();
        let r = self.lower_stmts(&b.stmts, sink, out);
        self.pop_scope();
        r
    }

    fn lower_stmts(&mut self, stmts: &[Stmt], sink: &Sink, out: &mut Vec<MStmt>) -> LResult<()> {
        let n = stmts.len();
        for (i, s) in stmts.iter().enumerate() {
            let last = i + 1 == n;
            match s {
                Stmt::Local(l) => {
                    self.f.let_hint = self.literal_hint(l, &stmts[i + 1..], sink);
                    let r = self.lower_local(l, out);
                    self.f.let_hint = None;
                    r?
                }
                Stmt::Item(item) => self.lower_nested_item(item, out)?,
                Stmt::Expr(e, semi) => {
                    if last && semi.is_none() {
                        self.lower_expr_sink(e, sink, out)?;
                    } else {
                        self.lower_expr_sink(e, &Sink::Discard, out)?;
                    }
                }
                Stmt::Macro(m) => {
                    let as_tail = last && m.semi_token.is_none();
                    self.lower_macro_stmt(&m.mac, if as_tail { sink } else { &Sink::Discard }, out)?;
                }
            }
        }
        Ok(())
    }

    /// For `let x = <unsuffixed literal>;` find the numeric type Rust would
    /// infer from later uses (`x += y`, `x * y`, `return x`), so Mojo does not
    /// default the literal to Int/Float64.
    fn literal_hint(&self, l: &syn::Local, rest: &[Stmt], sink: &Sink) -> Option<syn::Type> {
        let syn::Pat::Ident(pi) = &l.pat else { return None };
        let init = &l.init.as_ref()?.expr;
        let is_float = match &**init {
            Expr::Lit(syn::ExprLit { lit: Lit::Int(i), .. }) if i.suffix().is_empty() => false,
            Expr::Lit(syn::ExprLit { lit: Lit::Float(f), .. }) if f.suffix().is_empty() => true,
            Expr::Unary(u) if matches!(u.op, RU::Neg(_)) => match &*u.expr {
                Expr::Lit(syn::ExprLit { lit: Lit::Int(i), .. }) if i.suffix().is_empty() => false,
                Expr::Lit(syn::ExprLit { lit: Lit::Float(f), .. }) if f.suffix().is_empty() => true,
                _ => return None,
            },
            _ => return None,
        };
        let name = pi.ident.to_string();
        use syn::visit::Visit;
        struct V<'n> {
            name: &'n str,
            cands: Vec<Expr>,
            returned: bool,
        }
        impl<'n, 'ast> Visit<'ast> for V<'n> {
            fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
                let l = single_ident(&b.left).map(|n| n == self.name).unwrap_or(false);
                let r = single_ident(&b.right).map(|n| n == self.name).unwrap_or(false);
                let shift = matches!(b.op, RB::Shl(_) | RB::Shr(_) | RB::ShlAssign(_) | RB::ShrAssign(_));
                if !shift {
                    if l {
                        self.cands.push((*b.right).clone());
                    } else if r {
                        self.cands.push((*b.left).clone());
                    }
                }
                syn::visit::visit_expr_binary(self, b);
            }
            fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
                if single_ident(&a.left).map(|n| n == self.name).unwrap_or(false) {
                    self.cands.push((*a.right).clone());
                }
                if single_ident(&a.right).map(|n| n == self.name).unwrap_or(false) {
                    self.cands.push((*a.left).clone());
                }
                syn::visit::visit_expr_assign(self, a);
            }
            fn visit_expr_return(&mut self, r: &'ast syn::ExprReturn) {
                if let Some(e) = &r.expr {
                    if single_ident(e).map(|n| n == self.name).unwrap_or(false) {
                        self.returned = true;
                    }
                }
                syn::visit::visit_expr_return(self, r);
            }
        }
        let mut v = V { name: &name, cands: vec![], returned: false };
        for s in rest {
            v.visit_stmt(s);
        }
        if let (Some(Stmt::Expr(tail, None)), Sink::Return) = (rest.last(), sink) {
            if single_ident(tail).map(|n| n == name).unwrap_or(false) {
                v.returned = true;
            }
        }
        let ok = |t: &syn::Type| if is_float { is_float_rty(t) } else { is_int_rty(t) };
        for c in &v.cands {
            if let Some(t) = self.type_of(c) {
                if ok(&t) {
                    return Some(strip_ref(&t).clone());
                }
            }
        }
        if v.returned {
            if let Some(t) = &self.f.ret_rty {
                if ok(t) {
                    return Some(t.clone());
                }
            }
        }
        None
    }

    fn lower_nested_item(&mut self, item: &syn::Item, out: &mut Vec<MStmt>) -> LResult<()> {
        match item {
            syn::Item::Fn(f) => {
                let saved = self.begin_fn();
                let r = self.lower_fn_item(&super::items::FnSpec::free(f));
                self.end_fn(saved);
                out.push(MStmt::NestedFn(Box::new(r?)));
                Ok(())
            }
            syn::Item::Use(_) => Ok(()),
            syn::Item::Const(c) => {
                let ty = self.lower_type(&c.ty)?;
                let v = self.lower_expr_exp(&c.expr, Some(&ty))?;
                let name = self.declare(&c.ident.to_string(), Some((*c.ty).clone()), false, false);
                out.push(MStmt::Var { kind: BindKind::Var, name, ty: Some(ty), init: Some(v) });
                Ok(())
            }
            _ => unsup("item declared inside a function body", tokens_str(item)),
        }
    }

    fn lower_local(&mut self, l: &syn::Local, out: &mut Vec<MStmt>) -> LResult<()> {
        let (pat, ann) = match &l.pat {
            syn::Pat::Type(pt) => (&*pt.pat, Some(&*pt.ty)),
            p => (p, None),
        };
        let init = match &l.init {
            Some(i) => {
                if i.diverge.is_some() {
                    return unsup("let-else", tokens_str(l));
                }
                Some(&*i.expr)
            }
            None => None,
        };
        let hinted = if ann.is_none() { self.f.let_hint.clone() } else { None };
        let ann = ann.or(hinted.as_ref());
        let ann_mty = match ann {
            Some(t) => Some(self.lower_type(t)?),
            None => None,
        };
        match pat {
            syn::Pat::Ident(pi) if pi.subpat.is_none() => {
                let rust_name = pi.ident.to_string();
                let rty = ann.cloned().or_else(|| init.and_then(|e| self.type_of(e)));
                let is_ref_ty = ann.map(|t| matches!(t, syn::Type::Reference(_))).unwrap_or(false);
                let Some(init) = init else {
                    let Some(t) = ann_mty else {
                        return unsup("uninitialised `let` without a type annotation", tokens_str(l));
                    };
                    let name = self.declare(&rust_name, rty, is_ref_ty, false);
                    out.push(MStmt::Var { kind: BindKind::Var, name, ty: Some(t), init: None });
                    return Ok(());
                };
                // vec![...] with elements: use a typed list literal.
                if let Expr::Macro(m) = init {
                    if path_last(&m.mac.path) == "vec" {
                        if let Some(elems) = self.vec_elems(&m.mac)? {
                            let elem_ty = match &ann_mty {
                                Some(t @ MType::Named(n, _)) if n == "List" => t.clone(),
                                _ => MType::generic("List", vec![MType::Infer]),
                            };
                            if elems.is_empty() && !matches!(&ann_mty, Some(MType::Named(n, _)) if n == "List") {
                                return unsup("empty `vec![]` without element type", tokens_str(l));
                            }
                            let name = self.declare(&rust_name, rty, false, true);
                            let init = if elems.is_empty() { MExpr::call(MExpr::Type(elem_ty.clone()), vec![]) } else { MExpr::List(elems) };
                            out.push(MStmt::Var { kind: BindKind::Var, name, ty: Some(elem_ty), init: Some(init) });
                            return Ok(());
                        }
                    }
                }
                if needs_stmt_lowering(init) {
                    if let Some(t) = &ann_mty {
                        let name = self.declare(&rust_name, rty, is_ref_ty, true);
                        out.push(MStmt::Var { kind: BindKind::Var, name: name.clone(), ty: Some(t.clone()), init: None });
                        self.lower_expr_sink(init, &Sink::Assign(MExpr::name(name)), out)?;
                        return Ok(());
                    }
                }
                let v = self.lower_expr_exp(init, ann_mty.as_ref())?;
                let v = self.maybe_transfer(init, v);
                out.extend(self.take_pending());
                let transferable = rty.as_ref().map(|t| !is_prim_rty(t)).unwrap_or(false);
                let name = self.declare(&rust_name, rty, is_ref_ty, transferable);
                if let Some(t) = &ann_mty {
                    if matches!(t, MType::Infer) || contains_infer(t) {
                        out.push(MStmt::Var { kind: BindKind::Var, name, ty: None, init: Some(v) });
                        return Ok(());
                    }
                }
                out.push(MStmt::Var { kind: BindKind::Var, name, ty: ann_mty, init: Some(v) });
                Ok(())
            }
            syn::Pat::Wild(_) => {
                if let Some(init) = init {
                    let v = self.lower_expr(init)?;
                    out.extend(self.take_pending());
                    out.push(MStmt::Assign(MExpr::name("_"), v));
                }
                Ok(())
            }
            _ => {
                let Some(init) = init else {
                    return unsup("destructuring `let` without initialiser", tokens_str(l));
                };
                let rty = ann.cloned().or_else(|| self.type_of(init));
                let place = if is_place(init) {
                    let p = self.lower_expr(init)?;
                    out.extend(self.take_pending());
                    p
                } else {
                    let v = self.lower_expr(init)?;
                    out.extend(self.take_pending());
                    let t = self.fresh("tmp");
                    out.push(MStmt::Var { kind: BindKind::Var, name: t.clone(), ty: None, init: Some(v) });
                    MExpr::name(t)
                };
                let po = self.lower_pat(pat, &place, rty.as_ref())?;
                if !po.conds.is_empty() {
                    return unsup("refutable pattern in `let`", tokens_str(l));
                }
                self.emit_binds(&po.binds, out);
                Ok(())
            }
        }
    }

    // ======================================================================
    // Statement-position expressions
    // ======================================================================

    pub fn lower_expr_sink(&mut self, e: &Expr, sink: &Sink, out: &mut Vec<MStmt>) -> LResult<()> {
        match e {
            Expr::Paren(p) => self.lower_expr_sink(&p.expr, sink, out),
            Expr::Group(g) => self.lower_expr_sink(&g.expr, sink, out),
            Expr::If(i) => self.lower_if(i, sink, out),
            Expr::Match(m) => self.lower_match(m, sink, out),
            Expr::Block(b) => {
                if b.label.is_some() {
                    return unsup("labeled block", tokens_str(e));
                }
                self.lower_block(&b.block, sink, out)
            }
            Expr::Unsafe(_) => unsup("unsafe block", tokens_str(e)),
            Expr::Loop(l) => {
                if l.label.is_some() {
                    return unsup("labeled loop (labeled break/continue)", tokens_str(e));
                }
                let mut body = vec![];
                self.f.loop_depth += 1;
                let r = self.lower_block(&l.body, &Sink::Discard, &mut body);
                self.f.loop_depth -= 1;
                r?;
                out.push(MStmt::While(MExpr::Bool(true), body));
                Ok(())
            }
            Expr::While(w) => self.lower_while(w, out),
            Expr::ForLoop(f) => self.lower_for(f, out),
            Expr::Return(r) => {
                match &r.expr {
                    None => out.push(MStmt::Return(None)),
                    Some(v) => {
                        if self.f.ret_mty.is_none() && !self.f.raises {
                            // unit function returning a unit expression
                            self.lower_expr_sink(v, &Sink::Discard, out)?;
                            out.push(MStmt::Return(None));
                        } else {
                            self.lower_expr_sink(v, &Sink::Return, out)?;
                            if !matches!(out.last(), Some(MStmt::Return(_)) | Some(MStmt::Raise(_))) {
                                out.push(MStmt::Return(None));
                            }
                        }
                    }
                }
                Ok(())
            }
            Expr::Break(b) => {
                if b.label.is_some() {
                    return unsup("labeled break", tokens_str(e));
                }
                if b.expr.is_some() {
                    return unsup("break with a value", tokens_str(e));
                }
                out.push(MStmt::Break);
                Ok(())
            }
            Expr::Continue(c) => {
                if c.label.is_some() {
                    return unsup("labeled continue", tokens_str(e));
                }
                out.push(MStmt::Continue);
                Ok(())
            }
            Expr::Assign(a) => {
                let lhs = self.lower_place(&a.left)?;
                let rhs = self.lower_expr(&a.right)?;
                let rhs = self.maybe_transfer(&a.right, rhs);
                out.extend(self.take_pending());
                out.push(MStmt::Assign(lhs, rhs));
                Ok(())
            }
            Expr::Binary(b) if compound_op(&b.op).is_some() => {
                let op = compound_op(&b.op).unwrap();
                let lhs = self.lower_place(&b.left)?;
                let op = if op == BinOp::Mod {
                    // compound remainder: keep Rust semantics through the shim
                    let l2 = lhs.clone();
                    let rhs = self.lower_expr(&b.right)?;
                    let v = self.lower_rem(&b.left, &b.right, l2, rhs);
                    out.extend(self.take_pending());
                    out.push(MStmt::Assign(lhs, v));
                    return Ok(());
                } else {
                    op
                };
                let rhs = self.lower_expr(&b.right)?;
                out.extend(self.take_pending());
                out.push(MStmt::AugAssign(lhs, op, rhs));
                Ok(())
            }
            Expr::Macro(m) => self.lower_macro_stmt(&m.mac, sink, out),
            Expr::MethodCall(mc) if matches!(sink, Sink::Discard) => {
                // HashMap::insert in statement position -> d[k] = v
                let name = mc.method.to_string();
                if name == "insert" && mc.args.len() == 2 && self.recv_kind(&mc.receiver) == RecvKind::Map {
                    let d = self.lower_expr(&mc.receiver)?;
                    let k = self.lower_expr(&mc.args[0])?;
                    let v = self.lower_expr(&mc.args[1])?;
                    let v = self.maybe_transfer(&mc.args[1], v);
                    out.extend(self.take_pending());
                    out.push(MStmt::Assign(MExpr::Index(Box::new(d), vec![k]), v));
                    return Ok(());
                }
                let v = self.lower_expr(e)?;
                out.extend(self.take_pending());
                if !is_pure(&v) {
                    out.push(MStmt::Expr(v));
                }
                Ok(())
            }
            Expr::Call(c) if matches!(sink, Sink::Return) && self.f.raises => {
                // Ok(x) / Err(e) in a raising function
                if let Expr::Path(p) = &*c.func {
                    let last = path_last(&p.path);
                    if last == "Ok" && c.args.len() == 1 {
                        let a = &c.args[0];
                        if matches!(a, Expr::Tuple(t) if t.elems.is_empty()) {
                            out.push(MStmt::Return(None));
                            return Ok(());
                        }
                        let v = self.lower_expr_exp(a, self.f.ret_mty.clone().as_ref())?;
                        let v = self.maybe_transfer(a, v);
                        out.extend(self.take_pending());
                        out.push(MStmt::Return(Some(v)));
                        return Ok(());
                    }
                    if last == "Err" && c.args.len() == 1 {
                        let err = self.lower_error_value(&c.args[0])?;
                        out.extend(self.take_pending());
                        out.push(MStmt::Raise(err));
                        return Ok(());
                    }
                }
                self.lower_value_into_sink(e, sink, out)
            }
            _ => self.lower_value_into_sink(e, sink, out),
        }
    }

    fn lower_value_into_sink(&mut self, e: &Expr, sink: &Sink, out: &mut Vec<MStmt>) -> LResult<()> {
        match sink {
            Sink::Return => {
                if self.f.raises {
                    if let Expr::Call(c) = e {
                        if let Expr::Path(p) = &*c.func {
                            let last = path_last(&p.path);
                            if last == "Ok" || last == "Err" {
                                return unsup("malformed Ok/Err in raising function", tokens_str(e));
                            }
                        }
                    }
                    // A raising function whose tail is another Result-valued
                    // expression (e.g. `foo()` returning Result) is fine only
                    // if that call is itself lowered as raising.
                    if !self.is_result_call(e) {
                        return unsup("Result-typed tail expression that is not Ok/Err/call", tokens_str(e));
                    }
                }
                if self.f.ret_mty.is_none() {
                    let v = self.lower_expr(e)?;
                    out.extend(self.take_pending());
                    if !is_pure(&v) {
                        out.push(MStmt::Expr(v));
                    }
                    return Ok(());
                }
                let exp = self.f.ret_mty.clone();
                let v = self.lower_expr_exp(e, exp.as_ref())?;
                let v = self.maybe_transfer(e, v);
                out.extend(self.take_pending());
                out.push(MStmt::Return(Some(v)));
                Ok(())
            }
            Sink::Discard => {
                if matches!(e, Expr::Tuple(t) if t.elems.is_empty()) {
                    return Ok(());
                }
                let v = self.lower_expr(e)?;
                out.extend(self.take_pending());
                if is_pure(&v) {
                    return Ok(());
                }
                out.push(MStmt::Expr(v));
                Ok(())
            }
            Sink::Assign(target) => {
                let v = self.lower_expr(e)?;
                let v = self.maybe_transfer(e, v);
                out.extend(self.take_pending());
                out.push(MStmt::Assign(target.clone(), v));
                Ok(())
            }
        }
    }

    fn is_result_call(&self, e: &Expr) -> bool {
        match e {
            Expr::Call(c) => {
                if let Expr::Path(p) = &*c.func {
                    return self.syms.result_fns.contains(&path_last(&p.path));
                }
                false
            }
            Expr::MethodCall(mc) => self.syms.result_fns.contains(&mc.method.to_string()),
            Expr::Paren(p) => self.is_result_call(&p.expr),
            _ => false,
        }
    }

    /// Lower the payload of `Err(...)` / `anyhow!(...)` into an `Error` value.
    pub fn lower_error_value(&mut self, e: &Expr) -> LResult<MExpr> {
        if let Expr::Macro(m) = e {
            let n = path_last(&m.mac.path);
            if n == "anyhow" || n == "format_err" || n == "format" {
                let s = self.lower_format_macro(&m.mac, 0)?;
                return Ok(MExpr::call_named("Error", vec![s]));
            }
        }
        if let Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = e {
            return Ok(MExpr::call_named("Error", vec![MExpr::Str(s.value())]));
        }
        let v = self.lower_expr(e)?;
        self.todo("Result error type erased to `Error`", format!("Err({}) -> raise Error(String(...))", tokens_str(e)));
        Ok(MExpr::call_named("Error", vec![MExpr::call_named("String", vec![v])]))
    }

    fn lower_if(&mut self, i: &syn::ExprIf, sink: &Sink, out: &mut Vec<MStmt>) -> LResult<()> {
        self.push_scope();
        let r = (|| -> LResult<()> {
            let (pre, cond, binds) = self.lower_cond(&i.cond)?;
            out.extend(pre);
            let mut then_body = vec![];
            self.redeclare_binds(&binds, &mut then_body);
            self.lower_block(&i.then_branch, sink, &mut then_body)?;
            let mut arms = vec![IfArm { cond, body: then_body }];
            let mut else_body = None;
            if let Some((_, eb)) = &i.else_branch {
                let mut es = vec![];
                self.pop_scope();
                self.push_scope();
                match &**eb {
                    Expr::If(next) => self.lower_if(next, sink, &mut es)?,
                    Expr::Block(b) => self.lower_block(&b.block, sink, &mut es)?,
                    other => self.lower_expr_sink(other, sink, &mut es)?,
                }
                if es.len() == 1 && matches!(es[0], MStmt::If { .. }) {
                    if let MStmt::If { arms: a2, else_body: e2 } = es.pop().unwrap() {
                        arms.extend(a2);
                        else_body = e2;
                    }
                } else {
                    else_body = Some(es);
                }
            }
            out.push(MStmt::If { arms, else_body });
            Ok(())
        })();
        self.pop_scope();
        r
    }

    /// Lower an `if`/`while` condition, including `let` patterns and
    /// `let`-chains. Returns pre-statements (temps), the boolean condition and
    /// the bindings introduced (already registered as substitutions).
    fn lower_cond(&mut self, c: &Expr) -> LResult<(Vec<MStmt>, MExpr, Vec<pat::PatBind>)> {
        let mut pre = vec![];
        let mut conds = vec![];
        let mut binds = vec![];
        let mut parts = vec![];
        flatten_and(c, &mut parts);
        let has_let = parts.iter().any(|p| matches!(p, Expr::Let(_)));
        for (idx, p) in parts.iter().enumerate() {
            match p {
                Expr::Let(l) => {
                    let rty = self.type_of(&l.expr);
                    let place = if is_place(&l.expr) {
                        let v = self.lower_expr(&l.expr)?;
                        pre.extend(self.take_pending());
                        v
                    } else {
                        if idx > 0 {
                            return unsup("let-chain with a non-place scrutinee after the first condition", tokens_str(c));
                        }
                        let v = self.lower_expr(&l.expr)?;
                        pre.extend(self.take_pending());
                        let t = self.fresh("tmp");
                        pre.push(MStmt::Var { kind: BindKind::Var, name: t.clone(), ty: None, init: Some(v) });
                        MExpr::name(t)
                    };
                    let po = self.lower_pat(&l.pat, &place, rty.as_ref())?;
                    conds.extend(po.conds);
                    for b in &po.binds {
                        self.bind_expr(&b.rust, b.expr.clone(), b.rty.clone());
                    }
                    binds.extend(po.binds);
                }
                other => {
                    if has_let && idx > 0 {
                        let pending_before = self.f.pending.len();
                        let v = self.lower_expr(other)?;
                        if self.f.pending.len() != pending_before {
                            return unsup("condition needing temporaries inside a let-chain", tokens_str(c));
                        }
                        conds.push(v);
                    } else {
                        let v = self.lower_expr(other)?;
                        pre.extend(self.take_pending());
                        conds.push(v);
                    }
                }
            }
        }
        let cond = if conds.is_empty() { MExpr::Bool(true) } else { MExpr::and_all(conds) };
        Ok((pre, cond, binds))
    }

    /// Emit declarations for pattern bindings at the top of a branch body, and
    /// rebind the Rust names to the declared Mojo names.
    fn redeclare_binds(&mut self, binds: &[pat::PatBind], out: &mut Vec<MStmt>) {
        self.emit_binds(binds, out);
    }

    pub fn emit_binds(&mut self, binds: &[pat::PatBind], out: &mut Vec<MStmt>) {
        for b in binds {
            let copy_like = b.mutable || b.rty.as_ref().map(is_prim_rty).unwrap_or(false);
            let name = self.declare(&b.rust, b.rty.clone(), !copy_like, false);
            out.push(MStmt::Var {
                kind: if copy_like { BindKind::Var } else { BindKind::Ref },
                name,
                ty: None,
                init: Some(b.expr.clone()),
            });
        }
    }

    fn lower_match(&mut self, m: &syn::ExprMatch, sink: &Sink, out: &mut Vec<MStmt>) -> LResult<()> {
        let rty = self.type_of(&m.expr);
        let place = if is_place(&m.expr) {
            let v = self.lower_expr(&m.expr)?;
            out.extend(self.take_pending());
            v
        } else {
            let v = self.lower_expr(&m.expr)?;
            out.extend(self.take_pending());
            let t = self.fresh("match");
            out.push(MStmt::Var { kind: BindKind::Var, name: t.clone(), ty: None, init: Some(v) });
            MExpr::name(t)
        };
        let mut arms: Vec<IfArm> = vec![];
        let mut else_body: Option<Vec<MStmt>> = None;
        let n = m.arms.len();
        for (i, arm) in m.arms.iter().enumerate() {
            self.push_scope();
            let r = (|| -> LResult<()> {
                let po = self.lower_pat(&arm.pat, &place, rty.as_ref())?;
                for b in &po.binds {
                    self.bind_expr(&b.rust, b.expr.clone(), b.rty.clone());
                }
                let mut conds = po.conds.clone();
                if let Some((_, g)) = &arm.guard {
                    let before = self.f.pending.len();
                    let gv = self.lower_expr(g)?;
                    if self.f.pending.len() != before {
                        return unsup("match guard needing temporaries", tokens_str(g));
                    }
                    conds.push(gv);
                }
                let mut body = vec![];
                self.emit_binds(&po.binds, &mut body);
                self.lower_expr_sink(&arm.body, sink, &mut body)?;
                let is_last = i + 1 == n;
                if conds.is_empty() || (is_last && arm.guard.is_none()) {
                    // catch-all, or last arm of an exhaustive match
                    else_body = Some(body);
                } else {
                    arms.push(IfArm { cond: MExpr::and_all(conds), body });
                }
                Ok(())
            })();
            self.pop_scope();
            r?;
            if else_body.is_some() {
                break;
            }
        }
        if arms.is_empty() {
            out.extend(else_body.unwrap_or_default());
            return Ok(());
        }
        if else_body.is_none() {
            let abort = self.abort_expr("unreachable: non-exhaustive match lowering");
            else_body = Some(vec![MStmt::Expr(abort)]);
        }
        out.push(MStmt::If { arms, else_body });
        Ok(())
    }

    pub fn abort_expr(&mut self, msg: &str) -> MExpr {
        self.import("std.os", "abort");
        MExpr::call_named("abort", vec![MExpr::Str(msg.to_string())])
    }

    fn lower_while(&mut self, w: &syn::ExprWhile, out: &mut Vec<MStmt>) -> LResult<()> {
        if w.label.is_some() {
            return unsup("labeled while loop", tokens_str(w));
        }
        self.push_scope();
        let r = (|| -> LResult<()> {
            let has_let = {
                let mut parts = vec![];
                flatten_and(&w.cond, &mut parts);
                parts.iter().any(|p| matches!(p, Expr::Let(_)))
            };
            if !has_let {
                let before = self.f.pending.len();
                let c = self.lower_expr(&w.cond)?;
                if self.f.pending.len() != before {
                    return unsup("while condition needing temporaries", tokens_str(&w.cond));
                }
                let mut body = vec![];
                self.f.loop_depth += 1;
                let r = self.lower_block(&w.body, &Sink::Discard, &mut body);
                self.f.loop_depth -= 1;
                r?;
                out.push(MStmt::While(c, body));
                return Ok(());
            }
            // while let: while True: <pre>; if not cond: break; binds; body
            let mut body = vec![];
            self.f.loop_depth += 1;
            let r = (|| -> LResult<()> {
                let (pre, cond, binds) = self.lower_cond(&w.cond)?;
                body.extend(pre);
                body.push(MStmt::If { arms: vec![IfArm { cond: MExpr::not(cond), body: vec![MStmt::Break] }], else_body: None });
                self.emit_binds(&binds, &mut body);
                self.lower_block(&w.body, &Sink::Discard, &mut body)
            })();
            self.f.loop_depth -= 1;
            r?;
            out.push(MStmt::While(MExpr::Bool(true), body));
            Ok(())
        })();
        self.pop_scope();
        r
    }

    fn lower_for(&mut self, f: &syn::ExprForLoop, out: &mut Vec<MStmt>) -> LResult<()> {
        if f.label.is_some() {
            return unsup("labeled for loop", tokens_str(f));
        }
        let (iter, is_ref, elem_rty, map_entries) = self.lower_for_iter(&f.expr)?;
        out.extend(self.take_pending());
        self.push_scope();
        let r = (|| -> LResult<()> {
            self.f.loop_depth += 1;
            let mut body = vec![];
            let mut targets = vec![];
            if map_entries {
                // for (k, v) in &map  ->  for e in d.items(): ref k = e.key; ref v = e.value
                let e = self.fresh("entry");
                targets.push(e.clone());
                let pats: Vec<&syn::Pat> = match &*f.pat {
                    syn::Pat::Tuple(t) if t.elems.len() == 2 => t.elems.iter().collect(),
                    _ => return unsup("map iteration with a non-(k, v) pattern", tokens_str(&f.pat)),
                };
                for (p, field) in pats.iter().zip(["key", "value"]) {
                    let place = MExpr::attr(MExpr::name(&e), field);
                    let po = self.lower_pat(p, &place, None)?;
                    self.emit_binds(&po.binds, &mut body);
                }
            } else {
                self.for_targets(&f.pat, elem_rty.as_ref(), &mut targets)?;
            }
            let r = self.lower_block(&f.body, &Sink::Discard, &mut body);
            self.f.loop_depth -= 1;
            r?;
            out.push(MStmt::For { is_ref, targets, iter, body });
            Ok(())
        })();
        self.pop_scope();
        r
    }

    fn for_targets(&mut self, p: &syn::Pat, rty: Option<&syn::Type>, targets: &mut Vec<String>) -> LResult<()> {
        match p {
            syn::Pat::Ident(pi) if pi.subpat.is_none() => {
                let n = self.declare(&pi.ident.to_string(), rty.cloned(), true, false);
                targets.push(n);
                Ok(())
            }
            syn::Pat::Wild(_) => {
                targets.push("_".to_string());
                Ok(())
            }
            syn::Pat::Reference(r) => self.for_targets(&r.pat, rty, targets),
            syn::Pat::Tuple(t) if targets.is_empty() => {
                let elem_tys: Vec<syn::Type> = match rty.map(strip_ref) {
                    Some(syn::Type::Tuple(tt)) => tt.elems.iter().cloned().collect(),
                    _ => vec![],
                };
                for (i, e) in t.elems.iter().enumerate() {
                    let sub = elem_tys.get(i);
                    match e {
                        syn::Pat::Ident(_) | syn::Pat::Wild(_) | syn::Pat::Reference(_) => self.for_targets(e, sub, targets)?,
                        _ => return unsup("nested destructuring in for-loop pattern", tokens_str(p)),
                    }
                }
                Ok(())
            }
            _ => unsup("complex for-loop pattern", tokens_str(p)),
        }
    }

    pub fn lower_for_iter_pub(&mut self, e: &Expr) -> LResult<(MExpr, bool, Option<syn::Type>, bool)> {
        self.lower_for_iter(e)
    }

    /// Returns (iterable, is_ref, element type, iterates map entries)
    fn lower_for_iter(&mut self, e: &Expr) -> LResult<(MExpr, bool, Option<syn::Type>, bool)> {
        match e {
            Expr::Paren(p) => self.lower_for_iter(&p.expr),
            Expr::Range(r) => {
                let (Some(s), Some(en)) = (&r.start, &r.end) else {
                    return unsup("open-ended range in for loop", tokens_str(e));
                };
                let rty = self.type_of(s).or_else(|| self.type_of(en));
                let s = self.lower_expr(s)?;
                let mut en = self.lower_expr(en)?;
                if matches!(r.limits, syn::RangeLimits::Closed(_)) {
                    en = MExpr::bin(en, BinOp::Add, MExpr::Int("1".into()));
                }
                let args = if matches!(&s, MExpr::Int(v) if v == "0") { vec![en] } else { vec![s, en] };
                Ok((MExpr::call_named("range", args), false, rty, false))
            }
            Expr::Reference(r) => {
                let (it, _, t, me) = self.lower_for_iter(&r.expr)?;
                Ok((it, r.mutability.is_some(), t, me))
            }
            Expr::MethodCall(mc) => {
                let m = mc.method.to_string();
                match m.as_str() {
                    "iter" | "into_iter" | "iter_mut" if mc.args.is_empty() => {
                        let kind = self.recv_kind(&mc.receiver);
                        if kind == RecvKind::Map {
                            let d = self.lower_expr(&mc.receiver)?;
                            return Ok((MExpr::method(d, "items", vec![]), m == "iter_mut", None, true));
                        }
                        let (it, _, t, me) = self.lower_for_iter(&mc.receiver)?;
                        Ok((it, m == "iter_mut", t, me))
                    }
                    "keys" | "values" | "values_mut" if mc.args.is_empty() && self.recv_kind(&mc.receiver) == RecvKind::Map => {
                        let d = self.lower_expr(&mc.receiver)?;
                        Ok((MExpr::method(d, if m == "keys" { "keys" } else { "values" }, vec![]), m == "values_mut", None, false))
                    }
                    "chars" if mc.args.is_empty() => {
                        let s = self.lower_expr(&mc.receiver)?;
                        Ok((MExpr::method(s, "codepoints", vec![]), false, parse_rty("char"), false))
                    }
                    "enumerate" if mc.args.is_empty() => {
                        let (it, r, t, me) = self.lower_for_iter(&mc.receiver)?;
                        if me {
                            return unsup("enumerate over map entries", tokens_str(e));
                        }
                        let elem = t.map(|t| quote::quote!((usize, #t))).and_then(|ts| syn::parse2::<syn::Type>(ts).ok());
                        Ok((MExpr::call_named("enumerate", vec![it]), r, elem, false))
                    }
                    "rev" if mc.args.is_empty() => {
                        let (it, r, t, me) = self.lower_for_iter(&mc.receiver)?;
                        if me {
                            return unsup("reversed map iteration", tokens_str(e));
                        }
                        Ok((MExpr::call_named("reversed", vec![it]), r, t, false))
                    }
                    "step_by" if mc.args.len() == 1 => {
                        if let Expr::Range(_) | Expr::Paren(_) = &*mc.receiver {
                            let (it, _, t, _) = self.lower_for_iter(&mc.receiver)?;
                            if let MExpr::Call(f, mut args) = it {
                                if args.len() == 1 {
                                    args.insert(0, MArgVal::pos(MExpr::Int("0".into())));
                                }
                                let step = self.lower_expr(&mc.args[0])?;
                                args.push(MArgVal::pos(step));
                                return Ok((MExpr::Call(f, args), false, t, false));
                            }
                        }
                        unsup("iterator adapter `.step_by()` on non-range", tokens_str(e))
                    }
                    "zip" if mc.args.len() == 1 => {
                        let (a, _, ta, m1) = self.lower_for_iter(&mc.receiver)?;
                        let (b, _, tb, m2) = self.lower_for_iter(&mc.args[0])?;
                        if m1 || m2 {
                            return unsup("zip over map entries", tokens_str(e));
                        }
                        let elem = match (ta, tb) {
                            (Some(x), Some(y)) => syn::parse2::<syn::Type>(quote::quote!((#x, #y))).ok(),
                            _ => None,
                        };
                        Ok((MExpr::call_named("zip", vec![a, b]), false, elem, false))
                    }
                    _ => unsup(format!("iterator chain `.{}()` in for loop", m), tokens_str(e)),
                }
            }
            _ => {
                let kind = self.recv_kind(e);
                let rty = self.type_of(e);
                let elem = rty.as_ref().and_then(|t| rty_args(t).into_iter().next());
                let v = self.lower_expr(e)?;
                match kind {
                    RecvKind::Map => Ok((MExpr::method(v, "items", vec![]), false, None, true)),
                    RecvKind::Vec | RecvKind::Set | RecvKind::Unknown | RecvKind::User(_) | RecvKind::Param => Ok((v, false, elem, false)),
                    RecvKind::Str => unsup("iterating a string directly", tokens_str(e)),
                    RecvKind::Opt => unsup("iterating an Option", tokens_str(e)),
                    _ => unsup("for loop over unsupported iterable", tokens_str(e)),
                }
            }
        }
    }

    // ======================================================================
    // Value expressions
    // ======================================================================

    pub fn lower_expr(&mut self, e: &Expr) -> LResult<MExpr> {
        self.lower_expr_exp(e, None)
    }

    /// Wrap a lowered expression in `^` if it names a `var` binding whose
    /// only use is here (a Rust move of a non-Copy value).
    pub fn maybe_transfer(&self, e: &Expr, v: MExpr) -> MExpr {
        if let Some(n) = single_ident(e) {
            if let Some(b) = self.lookup(&n) {
                if b.transferable && b.loop_depth == self.f.loop_depth && self.f.use_counts.get(&n).copied().unwrap_or(0) == 1 {
                    if let MExpr::Name(_) = &v {
                        return MExpr::Transfer(Box::new(v));
                    }
                }
            }
        }
        v
    }

    fn lower_place(&mut self, e: &Expr) -> LResult<MExpr> {
        match e {
            Expr::Unary(u) if matches!(u.op, RU::Deref(_)) => self.lower_deref(&u.expr),
            _ => self.lower_expr(e),
        }
    }

    fn lower_deref(&mut self, inner: &Expr) -> LResult<MExpr> {
        let kind = self.recv_kind(inner);
        let v = self.lower_expr(inner)?;
        match kind {
            RecvKind::Rc | RecvKind::Boxed => Ok(MExpr::Index(Box::new(v), vec![])),
            RecvKind::RefCell | RecvKind::Cell => unsup("deref of RefCell/Cell", tokens_str(inner)),
            _ => Ok(v), // references are implicit in Mojo
        }
    }

    pub fn lower_expr_exp(&mut self, e: &Expr, exp: Option<&MType>) -> LResult<MExpr> {
        match e {
            Expr::Paren(p) => self.lower_expr_exp(&p.expr, exp),
            Expr::Group(g) => self.lower_expr_exp(&g.expr, exp),
            Expr::Lit(l) => self.lower_lit(&l.lit, exp),
            Expr::Path(p) => self.lower_path_expr(p),
            Expr::Reference(r) => self.lower_expr_exp(&r.expr, exp),
            Expr::Unary(u) => match u.op {
                RU::Neg(_) => {
                    let v = self.lower_expr_exp(&u.expr, exp)?;
                    Ok(match v {
                        MExpr::Int(s) if !s.starts_with('-') => MExpr::Int(format!("-{}", s)),
                        MExpr::Float(s) if !s.starts_with('-') => MExpr::Float(format!("-{}", s)),
                        v => MExpr::Unary(UnOp::Neg, Box::new(v)),
                    })
                }
                RU::Not(_) => {
                    let t = self.type_of(&u.expr);
                    let v = self.lower_expr(&u.expr)?;
                    if t.as_ref().map(is_int_rty).unwrap_or(false) {
                        Ok(MExpr::Unary(UnOp::Invert, Box::new(v)))
                    } else if t.is_some() && !t.as_ref().map(|t| rty_last(t).as_deref() == Some("bool")).unwrap_or(false) {
                        // user type implementing Not
                        Ok(MExpr::Unary(UnOp::Invert, Box::new(v)))
                    } else {
                        Ok(MExpr::not(v))
                    }
                }
                RU::Deref(_) => self.lower_deref(&u.expr),
                _ => unsup("unknown unary operator", tokens_str(e)),
            },
            Expr::Binary(b) => self.lower_binary(b),
            Expr::Cast(c) => self.lower_cast(c),
            Expr::Call(c) => self.lower_call(c, exp),
            Expr::MethodCall(mc) => self.lower_method_call(mc, exp),
            Expr::Field(f) => {
                let base_t = self.type_of(&f.base);
                let base = self.lower_expr(&f.base)?;
                match &f.member {
                    syn::Member::Named(n) => Ok(MExpr::attr(base, &names::ident(&n.to_string()))),
                    syn::Member::Unnamed(i) => {
                        let is_tuple = matches!(base_t.as_ref().map(strip_ref), Some(syn::Type::Tuple(_)));
                        if is_tuple {
                            Ok(MExpr::Index(Box::new(base), vec![MExpr::Int(i.index.to_string())]))
                        } else {
                            Ok(MExpr::attr(base, &format!("_{}", i.index)))
                        }
                    }
                }
            }
            Expr::Index(ix) => {
                let base = self.lower_expr(&ix.expr)?;
                if let Expr::Range(r) = &*ix.index {
                    if r.start.is_none() && r.end.is_none() {
                        return Ok(base);
                    }
                    let s = match &r.start {
                        Some(s) => Some(Box::new(self.lower_expr(s)?)),
                        None => None,
                    };
                    let en = match &r.end {
                        Some(x) => {
                            let mut v = self.lower_expr(x)?;
                            if matches!(r.limits, syn::RangeLimits::Closed(_)) {
                                v = MExpr::bin(v, BinOp::Add, MExpr::Int("1".into()));
                            }
                            Some(Box::new(v))
                        }
                        None => None,
                    };
                    self.todo("slice indexing yields a Span (not an owned List)", tokens_str(e));
                    return Ok(MExpr::Index(Box::new(base), vec![MExpr::Slice(s, en)]));
                }
                if self.recv_kind(&ix.expr) == RecvKind::Map {
                    return unsup("HashMap indexing (Mojo Dict[] raises on missing key)", tokens_str(e));
                }
                let i = self.lower_expr(&ix.index)?;
                Ok(MExpr::Index(Box::new(base), vec![i]))
            }
            Expr::Tuple(t) => {
                if t.elems.is_empty() {
                    return Ok(MExpr::NoneLit);
                }
                let mut v = vec![];
                for x in &t.elems {
                    let lx = self.lower_expr(x)?;
                    v.push(self.maybe_transfer(x, lx));
                }
                Ok(MExpr::Tuple(v))
            }
            Expr::Array(a) => {
                let mut v = vec![];
                for x in &a.elems {
                    v.push(self.lower_expr(x)?);
                }
                Ok(MExpr::List(v))
            }
            Expr::Repeat(r) => {
                let elem_ty = match exp {
                    Some(MType::Named(n, args)) if n == "Array" => args.first().cloned(),
                    _ => self.type_of(&r.expr).filter(is_prim_rty).and_then(|t| self.lower_type(&t).ok()).map(MTypeArg::Type),
                };
                let Some(et) = elem_ty else {
                    return unsup("array repeat `[x; N]` without a known element type", tokens_str(e));
                };
                let n = self.lower_const_expr(&r.len)?;
                let fill = self.lower_expr(&r.expr)?;
                Ok(MExpr::Call(
                    Box::new(MExpr::Type(MType::Named("Array".into(), vec![et, MTypeArg::Expr(n)]))),
                    vec![MArgVal::kw("fill", fill)],
                ))
            }
            Expr::Struct(s) => self.lower_struct_lit(s),
            Expr::Range(r) => {
                let (Some(s), Some(en)) = (&r.start, &r.end) else {
                    return unsup("open-ended range value", tokens_str(e));
                };
                let s = self.lower_expr(s)?;
                let mut en = self.lower_expr(en)?;
                if matches!(r.limits, syn::RangeLimits::Closed(_)) {
                    en = MExpr::bin(en, BinOp::Add, MExpr::Int("1".into()));
                }
                let f = self.rt("range", "RustRange");
                Ok(MExpr::call(f, vec![s, en]))
            }
            Expr::If(i) => self.lower_if_value(i, exp),
            Expr::Match(m) => self.lower_match_value(m, exp),
            Expr::Block(b) => {
                if b.label.is_none() && b.block.stmts.len() == 1 {
                    if let Stmt::Expr(inner, None) = &b.block.stmts[0] {
                        return self.lower_expr_exp(inner, exp);
                    }
                }
                // `{ stmts; tail }`: hoist the statements (names are already
                // unique per function), keep the tail as the value.
                if let (None, Some(Stmt::Expr(tail, None))) = (&b.label, b.block.stmts.last()) {
                    if !needs_stmt_lowering(tail) {
                        let saved = self.take_pending();
                        let mut stmts = vec![];
                        self.push_scope();
                        let r = (|| -> LResult<MExpr> {
                            let n = b.block.stmts.len();
                            self.lower_stmts(&b.block.stmts[..n - 1], &Sink::Discard, &mut stmts)?;
                            let v = self.lower_expr_exp(tail, exp)?;
                            stmts.extend(self.take_pending());
                            Ok(v)
                        })();
                        self.pop_scope();
                        let mut p = saved;
                        p.extend(stmts);
                        self.f.pending = p;
                        return r;
                    }
                }
                self.hoist_with_type(e, exp)
            }
            Expr::Try(t) => self.lower_try(t),
            Expr::Macro(m) => self.lower_macro_expr(&m.mac, exp),
            Expr::Closure(c) => self.lower_closure(c),
            Expr::Unsafe(_) => unsup("unsafe block", tokens_str(e)),
            Expr::Async(_) | Expr::Await(_) => unsup("async/await", tokens_str(e)),
            Expr::Let(_) => unsup("`let` expression outside if/while condition", tokens_str(e)),
            Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) => unsup("control flow in expression position", tokens_str(e)),
            Expr::Assign(_) => unsup("assignment in expression position", tokens_str(e)),
            Expr::Loop(_) | Expr::While(_) | Expr::ForLoop(_) => unsup("loop in expression position", tokens_str(e)),
            Expr::Const(_) => unsup("inline const block", tokens_str(e)),
            Expr::RawAddr(_) => unsup("raw address-of", tokens_str(e)),
            Expr::Yield(_) => unsup("yield", tokens_str(e)),
            Expr::TryBlock(_) => unsup("try block", tokens_str(e)),
            _ => unsup("unsupported expression syntax", tokens_str(e)),
        }
    }

    /// Value-position if/match/block whose arms need statements: if the
    /// expected type is known and we are not under a short-circuit, hoist into
    /// a typed temporary.
    fn hoist_with_type(&mut self, e: &Expr, exp: Option<&MType>) -> LResult<MExpr> {
        let Some(t) = exp.filter(|t| !contains_infer(t)) else {
            return unsup("block/if/match with statements in expression position (type unknown)", tokens_str(e));
        };
        let tmp = self.fresh("val");
        let mut stmts = vec![MStmt::Var { kind: BindKind::Var, name: tmp.clone(), ty: Some(t.clone()), init: None }];
        let saved_pending = self.take_pending();
        self.lower_expr_sink(e, &Sink::Assign(MExpr::name(&tmp)), &mut stmts)?;
        let mut p = saved_pending;
        p.extend(stmts);
        self.f.pending = p;
        Ok(MExpr::name(tmp))
    }

    fn lower_if_value(&mut self, i: &syn::ExprIf, exp: Option<&MType>) -> LResult<MExpr> {
        // `a if c else b` when both branches are single expressions without lets.
        let simple = |b: &syn::Block| b.stmts.len() == 1 && matches!(&b.stmts[0], Stmt::Expr(_, None));
        let has_let = {
            let mut parts = vec![];
            flatten_and(&i.cond, &mut parts);
            parts.iter().any(|p| matches!(p, Expr::Let(_)))
        };
        if !has_let && simple(&i.then_branch) {
            if let Some((_, eb)) = &i.else_branch {
                let else_ok = match &**eb {
                    Expr::Block(b) => simple(&b.block),
                    Expr::If(_) => true,
                    _ => false,
                };
                if else_ok {
                    let before = self.f.pending.len();
                    let c = self.lower_expr(&i.cond)?;
                    let Stmt::Expr(te, None) = &i.then_branch.stmts[0] else { unreachable!() };
                    let a = self.lower_expr_exp(te, exp)?;
                    let b = match &**eb {
                        Expr::Block(b) => {
                            let Stmt::Expr(ee, None) = &b.block.stmts[0] else { unreachable!() };
                            self.lower_expr_exp(ee, exp)?
                        }
                        Expr::If(ei) => self.lower_if_value(ei, exp)?,
                        _ => unreachable!(),
                    };
                    if self.f.pending.len() != before {
                        // a branch hoisted statements: that would run them unconditionally
                        self.f.pending.truncate(before);
                        return self.hoist_with_type(&Expr::If(i.clone()), exp);
                    }
                    return Ok(MExpr::Ternary(Box::new(c), Box::new(a), Box::new(b)));
                }
            }
        }
        self.hoist_with_type(&Expr::If(i.clone()), exp)
    }

    fn lower_match_value(&mut self, m: &syn::ExprMatch, exp: Option<&MType>) -> LResult<MExpr> {
        let simple_arms = m.arms.iter().all(|a| match &*a.body {
            Expr::Block(b) => b.block.stmts.len() == 1 && matches!(&b.block.stmts[0], Stmt::Expr(_, None)),
            Expr::Return(_) | Expr::Break(_) | Expr::Continue(_) | Expr::Macro(_) => false,
            _ => true,
        });
        if !simple_arms {
            return self.hoist_with_type(&Expr::Match(m.clone()), exp);
        }
        let rty = self.type_of(&m.expr);
        let place = if is_place(&m.expr) {
            self.lower_expr(&m.expr)?
        } else {
            let v = self.lower_expr(&m.expr)?;
            let t = self.fresh("match");
            self.f.pending.push(MStmt::Var { kind: BindKind::Var, name: t.clone(), ty: None, init: Some(v) });
            MExpr::name(t)
        };
        let before = self.f.pending.len();
        let mut chain: Vec<(Option<MExpr>, MExpr)> = vec![];
        let n = m.arms.len();
        for (i, arm) in m.arms.iter().enumerate() {
            self.push_scope();
            let r = (|| -> LResult<bool> {
                let po = self.lower_pat(&arm.pat, &place, rty.as_ref())?;
                for b in &po.binds {
                    self.bind_expr(&b.rust, b.expr.clone(), b.rty.clone());
                }
                let mut conds = po.conds;
                if let Some((_, g)) = &arm.guard {
                    conds.push(self.lower_expr(g)?);
                }
                let body = match &*arm.body {
                    Expr::Block(b) => {
                        let Stmt::Expr(x, None) = &b.block.stmts[0] else { unreachable!() };
                        x.clone()
                    }
                    x => x.clone(),
                };
                let v = self.lower_expr_exp(&body, exp)?;
                let last = i + 1 == n;
                if conds.is_empty() || (last && arm.guard.is_none()) {
                    chain.push((None, v));
                    Ok(true)
                } else {
                    chain.push((Some(MExpr::and_all(conds)), v));
                    Ok(false)
                }
            })();
            self.pop_scope();
            if r? {
                break;
            }
        }
        if self.f.pending.len() != before {
            self.f.pending.truncate(before);
            return self.hoist_with_type(&Expr::Match(m.clone()), exp);
        }
        let mut acc: Option<MExpr> = None;
        for (c, v) in chain.into_iter().rev() {
            acc = Some(match (c, acc) {
                (None, _) => v,
                (Some(c), Some(rest)) => MExpr::Ternary(Box::new(c), Box::new(v), Box::new(rest)),
                (Some(_), None) => return unsup("non-exhaustive match lowering in expression position", tokens_str(m)),
            });
        }
        acc.ok_or_else(|| Unsupported { key: "empty match".into(), detail: tokens_str(m) })
    }

    fn lower_try(&mut self, t: &syn::ExprTry) -> LResult<MExpr> {
        if self.f.raises {
            if matches!(&*t.expr, Expr::Call(_) | Expr::MethodCall(_)) && self.is_result_call(&t.expr) {
                return self.lower_expr(&t.expr);
            }
            return unsup("`?` on a Result that does not come from a translated raising call", tokens_str(t));
        }
        if self.f.returns_option {
            let v = self.lower_expr(&t.expr)?;
            let tmp = self.fresh("opt");
            self.f.pending.push(MStmt::Var { kind: BindKind::Var, name: tmp.clone(), ty: None, init: Some(v) });
            self.f.pending.push(MStmt::If {
                arms: vec![IfArm { cond: MExpr::bin(MExpr::name(&tmp), BinOp::Is, MExpr::NoneLit), body: vec![MStmt::Return(Some(MExpr::NoneLit))] }],
                else_body: None,
            });
            return Ok(MExpr::method(MExpr::name(tmp), "value", vec![]));
        }
        unsup("`?` operator outside a Result/Option function", tokens_str(t))
    }

    fn lower_closure(&mut self, c: &syn::ExprClosure) -> LResult<MExpr> {
        let mut args = vec![];
        for (i, p) in c.inputs.iter().enumerate() {
            match p {
                syn::Pat::Type(pt) => {
                    let syn::Pat::Ident(pi) = &*pt.pat else {
                        return unsup("closure with destructuring parameters", tokens_str(c));
                    };
                    let (conv, ty) = self.lower_arg_type(&pt.ty, pi.mutability.is_some())?;
                    args.push((pi.ident.to_string(), conv, ty, (*pt.ty).clone()));
                }
                _ => {
                    let _ = i;
                    return unsup("closure with untyped parameters (needs type inference)", tokens_str(c));
                }
            }
        }
        let syn::ReturnType::Type(_, rt) = &c.output else {
            return unsup("closure without a return type annotation (needs type inference)", tokens_str(c));
        };
        if c.asyncness.is_some() {
            return unsup("async closure", tokens_str(c));
        }
        let ret = self.lower_type(rt)?;
        // captures: free identifiers that resolve to locals of the enclosing fn
        let mut captured: Vec<String> = vec![];
        {
            use syn::visit::Visit;
            struct V<'x>(&'x mut Vec<String>);
            impl<'x, 'ast> Visit<'ast> for V<'x> {
                fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
                    if p.path.segments.len() == 1 {
                        let n = p.path.segments[0].ident.to_string();
                        if !self.0.contains(&n) {
                            self.0.push(n);
                        }
                    }
                }
            }
            V(&mut captured).visit_expr(&c.body);
        }
        let param_names: Vec<String> = args.iter().map(|a| a.0.clone()).collect();
        let mut caps = vec![];
        for n in captured {
            if param_names.contains(&n) {
                continue;
            }
            if let Some(b) = self.lookup(&n) {
                if let MExpr::Name(m) = &b.expr {
                    caps.push(if c.capture.is_some() { format!("var {}", m) } else { format!("imm {}", m) });
                } else {
                    return unsup("closure capturing a pattern binding", tokens_str(c));
                }
            }
        }
        let name = self.fresh("closure");
        let outer = std::mem::take(&mut self.f.scopes);
        let saved_ret = (self.f.ret_mty.clone(), self.f.raises, self.f.returns_option, self.f.ret_rty.clone());
        self.f.scopes = outer.clone();
        self.push_scope();
        let mut margs = vec![];
        for (n, conv, ty, rty) in &args {
            let mn = self.declare(n, Some(strip_ref(rty).clone()), matches!(rty, syn::Type::Reference(_)), false);
            margs.push(MArg { conv: *conv, name: mn, ty: Some(ty.clone()), kw_only: false });
        }
        self.f.ret_mty = Some(ret.clone());
        self.f.raises = false;
        self.f.returns_option = ret.is_named("Optional");
        let saved_pending = self.take_pending();
        let mut body = vec![];
        let r = self.lower_expr_sink(&c.body, &Sink::Return, &mut body);
        self.f.pending = saved_pending;
        self.f.scopes = outer;
        self.f.ret_mty = saved_ret.0;
        self.f.raises = saved_ret.1;
        self.f.returns_option = saved_ret.2;
        self.f.ret_rty = saved_ret.3;
        r?;
        let f = MFunction {
            name: name.clone(),
            args: margs,
            ret: RetType::Value(ret),
            body: Some(body),
            captures: caps,
            ..Default::default()
        };
        self.f.pending.push(MStmt::NestedFn(Box::new(f)));
        Ok(MExpr::name(name))
    }

    // ---- literals -------------------------------------------------------------------

    fn lower_lit(&mut self, l: &Lit, exp: Option<&MType>) -> LResult<MExpr> {
        match l {
            Lit::Int(i) => {
                let raw = i.token().to_string();
                let digits = raw.trim_end_matches(i.suffix()).replace('_', "");
                let v = MExpr::Int(digits);
                match i.suffix() {
                    "" => {
                        if let Some(MType::Named(n, a)) = exp {
                            if a.is_empty() && (n.starts_with("Float")) {
                                return Ok(MExpr::Float(format!("{}.0", i.base10_digits())));
                            }
                        }
                        Ok(v)
                    }
                    "usize" | "isize" => Ok(v),
                    s => match names::prim(s) {
                        Some(m) => {
                            if s.starts_with('f') {
                                Ok(MExpr::call_named(m, vec![MExpr::Float(format!("{}.0", i.base10_digits()))]))
                            } else {
                                Ok(MExpr::call_named(m, vec![v]))
                            }
                        }
                        None => unsup("unknown integer literal suffix", raw),
                    },
                }
            }
            Lit::Float(f) => {
                let raw = f.token().to_string();
                let mut digits = raw.trim_end_matches(f.suffix()).replace('_', "");
                if digits.ends_with('.') {
                    digits.push('0');
                }
                if !digits.contains('.') && !digits.contains('e') && !digits.contains('E') {
                    digits.push_str(".0");
                }
                let v = MExpr::Float(digits);
                match f.suffix() {
                    "" => Ok(v),
                    s => match names::prim(s) {
                        Some(m) => Ok(MExpr::call_named(m, vec![v])),
                        None => unsup("unknown float literal suffix", raw),
                    },
                }
            }
            Lit::Str(s) => Ok(MExpr::Str(s.value())),
            Lit::Bool(b) => Ok(MExpr::Bool(b.value)),
            Lit::Char(c) => {
                let s = c.value().to_string();
                Ok(MExpr::method(MExpr::name("Codepoint"), "ord", vec![MExpr::Str(s)]))
            }
            Lit::Byte(b) => Ok(MExpr::call_named("UInt8", vec![MExpr::Int(b.value().to_string())])),
            _ => unsup("byte-string / C-string literal", tokens_str(l)),
        }
    }

    // ---- paths ----------------------------------------------------------------------

    fn lower_path_expr(&mut self, p: &syn::ExprPath) -> LResult<MExpr> {
        if p.qself.is_some() {
            return unsup("qualified path expression `<T as Trait>::x`", tokens_str(p));
        }
        let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
        if segs.len() == 1 {
            let n = &segs[0];
            if let Some(b) = self.lookup(n) {
                return Ok(b.expr.clone());
            }
            match n.as_str() {
                "None" => return Ok(MExpr::NoneLit),
                "Self" => return Ok(MExpr::call_named("Self", vec![])),
                _ => {}
            }
            if let Some(en) = self.variant_imports.get(n).cloned() {
                return self.enum_variant_value(&en, n);
            }
            if let Some(info) = self.syms.structs.get(n) {
                if info.fields.is_empty() && !info.tuple {
                    return Ok(MExpr::call_named(n, vec![])); // unit struct value
                }
            }
            if let Some(m) = self.ts.params.get(n) {
                return Ok(MExpr::name(m.clone()));
            }
            let args = self.lower_seg_args(&p.path.segments[0])?;
            let base = MExpr::name(names::ident(n));
            return Ok(if args.is_empty() { base } else { MExpr::Param(Box::new(base), args) });
        }
        // Enum::Variant / Type::CONST / Self::X / module::item
        let n = segs.len();
        let last = &segs[n - 1];
        let prev = &segs[n - 2];
        if let Some(c) = self.std_const(prev, last, &segs) {
            return Ok(c);
        }
        let owner = if prev == "Self" { self.ts.self_name.clone().unwrap_or_else(|| "Self".into()) } else { prev.clone() };
        if let Some(info) = self.syms.enums.get(&owner) {
            if info.variant(last).is_some() {
                return self.enum_variant_value(&owner, last);
            }
        }
        if self.syms.structs.contains_key(&owner) || self.syms.enums.contains_key(&owner) || prev == "Self" || self.ts.params.contains_key(prev) {
            let base = if prev == "Self" {
                MExpr::name("Self")
            } else if let Some(m) = self.ts.params.get(prev) {
                MExpr::name(m.clone())
            } else {
                let a = self.lower_seg_args(&p.path.segments[n - 2])?;
                if a.is_empty() { MExpr::name(prev) } else { MExpr::Param(Box::new(MExpr::name(prev)), a) }
            };
            return Ok(MExpr::attr(base, &names::ident(last)));
        }
        if ["std", "core", "alloc"].contains(&segs[0].as_str()) || names::prim(prev).is_some() {
            return unsup("std path with no Mojo mapping", tokens_str(p));
        }
        // module-qualified item: import it and refer by name
        if let Some(module) = self.resolve_module_path(&segs[..n - 1]) {
            self.import(&module, last);
            return Ok(MExpr::name(names::ident(last)));
        }
        if prev.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            // associated item on an external type
            self.todo(format!("associated item on external type `{}`", prev), tokens_str(p));
            return Ok(MExpr::attr(MExpr::name(prev), &names::ident(last)));
        }
        unsup("path into an external crate/module", tokens_str(p))
    }

    fn lower_seg_args(&mut self, seg: &syn::PathSegment) -> LResult<Vec<MTypeArg>> {
        let mut out = vec![];
        if let syn::PathArguments::AngleBracketed(a) = &seg.arguments {
            for g in &a.args {
                match g {
                    syn::GenericArgument::Type(t) => out.push(MTypeArg::Type(self.lower_type(t)?)),
                    syn::GenericArgument::Const(e) => out.push(MTypeArg::Expr(self.lower_const_expr(e)?)),
                    syn::GenericArgument::Lifetime(_) => {}
                    _ => return unsup("unsupported turbofish argument", tokens_str(g)),
                }
            }
        }
        Ok(out)
    }

    pub fn enum_variant_value(&mut self, enum_name: &str, variant: &str) -> LResult<MExpr> {
        let info = self.syms.enums.get(enum_name).cloned();
        let owner = if self.ts.self_name.as_deref() == Some(enum_name) { "Self".to_string() } else { enum_name.to_string() };
        match info {
            Some(info) => {
                let v = MExpr::attr(MExpr::name(&owner), &names::ident(variant));
                if info.c_like {
                    Ok(v)
                } else {
                    match info.variant(variant) {
                        Some(crate::symbols::VariantShape::Unit) => Ok(MExpr::call(v, vec![])),
                        _ => Ok(v), // constructor function value
                    }
                }
            }
            None => unsup("unknown enum", enum_name.to_string()),
        }
    }

    fn std_const(&mut self, prev: &str, last: &str, segs: &[String]) -> Option<MExpr> {
        let fl = if segs.iter().any(|s| s == "f64") { "F64" } else if segs.iter().any(|s| s == "f32") { "F32" } else { "" };
        if !fl.is_empty() {
            let m = match last {
                "PI" | "TAU" | "E" | "SQRT_2" | "FRAC_PI_2" | "FRAC_PI_3" | "FRAC_PI_4" | "FRAC_PI_6" | "FRAC_PI_8" | "FRAC_1_PI" | "FRAC_2_PI" | "LN_2" | "LN_10" | "FRAC_1_SQRT_2" | "LOG2_E" | "LOG10_E" => last,
                "EPSILON" | "MAX" | "MIN" | "MIN_POSITIVE" | "INFINITY" | "NEG_INFINITY" | "NAN" => last,
                _ => return None,
            };
            return Some(self.rt("num", &format!("{}_{}", fl, m)));
        }
        if let Some(t) = names::prim(prev) {
            if names::is_int(prev) && (last == "MAX" || last == "MIN") {
                return Some(MExpr::attr(MExpr::name(t), last));
            }
        }
        None
    }

    /// Resolve `a::b` (as used in an expression path) to a Mojo module path.
    pub fn resolve_module_path(&self, segs: &[String]) -> Option<String> {
        if segs.is_empty() {
            return None;
        }
        let mut base: Vec<String> = match segs[0].as_str() {
            "crate" => vec![],
            "self" => self.module_path.clone(),
            "super" => {
                let mut m = self.module_path.clone();
                m.pop();
                m
            }
            _ => {
                // relative child module?
                let mut m = self.module_path.clone();
                m.push(segs[0].clone());
                if self.syms_has_module(&m) {
                    m.pop();
                    m
                } else {
                    return None;
                }
            }
        };
        let mut rest = &segs[1..];
        if !["crate", "self", "super"].contains(&segs[0].as_str()) {
            rest = segs;
        }
        for s in rest {
            if s == "super" {
                base.pop();
            } else if s != "self" {
                base.push(s.clone());
            }
        }
        let mut parts = vec![self.crate_pkg.clone()];
        parts.extend(base);
        Some(parts.join("."))
    }

    fn syms_has_module(&self, m: &[String]) -> bool {
        self.syms.modules.contains(&m.join("::"))
    }

    // ---- operators ------------------------------------------------------------------

    fn lower_binary(&mut self, b: &syn::ExprBinary) -> LResult<MExpr> {
        if compound_op(&b.op).is_some() {
            return unsup("compound assignment in expression position", tokens_str(b));
        }
        let op = match b.op {
            RB::Add(_) => BinOp::Add,
            RB::Sub(_) => BinOp::Sub,
            RB::Mul(_) => BinOp::Mul,
            RB::Div(_) => BinOp::Div,
            RB::Rem(_) => {
                let l = self.lower_expr(&b.left)?;
                let r = self.lower_expr(&b.right)?;
                return Ok(self.lower_rem(&b.left, &b.right, l, r));
            }
            RB::And(_) => BinOp::And,
            RB::Or(_) => BinOp::Or,
            RB::BitXor(_) => BinOp::BitXor,
            RB::BitAnd(_) => BinOp::BitAnd,
            RB::BitOr(_) => BinOp::BitOr,
            RB::Shl(_) => BinOp::Shl,
            RB::Shr(_) => BinOp::Shr,
            RB::Eq(_) => BinOp::Eq,
            RB::Lt(_) => BinOp::Lt,
            RB::Le(_) => BinOp::Le,
            RB::Ne(_) => BinOp::Ne,
            RB::Ge(_) => BinOp::Ge,
            RB::Gt(_) => BinOp::Gt,
            _ => return unsup("unknown binary operator", tokens_str(b)),
        };
        let l = self.lower_expr(&b.left)?;
        let r = if matches!(op, BinOp::And | BinOp::Or) {
            // right side is conditionally evaluated: forbid hoisting
            let before = self.f.pending.len();
            let r = self.lower_expr(&b.right)?;
            if self.f.pending.len() != before {
                return unsup("short-circuit operand needing temporaries", tokens_str(b));
            }
            r
        } else {
            self.lower_expr(&b.right)?
        };
        Ok(MExpr::bin(l, op, r))
    }

    fn lower_rem(&mut self, le: &Expr, re: &Expr, l: MExpr, r: MExpr) -> MExpr {
        let lt = self.type_of(le);
        let user = lt.as_ref().map(|t| !is_prim_rty(t) && rty_last(t).map(|n| self.syms.structs.contains_key(&n)).unwrap_or(false)).unwrap_or(false);
        let _ = re;
        if user {
            MExpr::bin(l, BinOp::Mod, r)
        } else {
            // Rust `%` truncates toward zero; Mojo `%` follows the divisor's sign.
            let f = self.rt("num", "rem");
            MExpr::call(f, vec![l, r])
        }
    }

    fn lower_cast(&mut self, c: &syn::ExprCast) -> LResult<MExpr> {
        let target = rty_last(&c.ty);
        let src_t = self.type_of(&c.expr);
        let v = self.lower_expr(&c.expr)?;
        let Some(t) = target else {
            return unsup("cast to non-primitive type", tokens_str(c));
        };
        let Some(m) = names::prim(&t) else {
            return unsup("cast to non-primitive type", tokens_str(c));
        };
        if t == "char" {
            return unsup("integer to char cast", tokens_str(c));
        }
        if names::is_int(&t) && src_t.as_ref().map(is_float_rty).unwrap_or(false) {
            self.todo(
                "float->int cast: Rust saturates, Mojo conversion of out-of-range values differs",
                tokens_str(c),
            );
        }
        if let Some(st) = &src_t {
            if rty_last(st).as_deref() == Some(t.as_str()) {
                return Ok(v);
            }
        }
        Ok(MExpr::call(MExpr::name(m), vec![v]))
    }

    // ---- calls ----------------------------------------------------------------------

    fn lower_args(&mut self, args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>) -> LResult<Vec<MExpr>> {
        let mut v = vec![];
        for a in args {
            let x = self.lower_expr(a)?;
            v.push(self.maybe_transfer(a, x));
        }
        Ok(v)
    }

    fn lower_call(&mut self, c: &syn::ExprCall, exp: Option<&MType>) -> LResult<MExpr> {
        let Expr::Path(p) = &*c.func else {
            let f = self.lower_expr(&c.func)?;
            let args = self.lower_args(&c.args)?;
            return Ok(MExpr::call(f, args));
        };
        if p.qself.is_some() {
            return unsup("qualified call `<T as Trait>::f()`", tokens_str(c));
        }
        let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
        let n = segs.len();
        let last = segs[n - 1].as_str();
        if n == 1 {
            if let Some(b) = self.lookup(last) {
                let f = b.expr.clone();
                let args = self.lower_args(&c.args)?;
                return Ok(MExpr::call(f, args));
            }
            match last {
                "Some" if c.args.len() == 1 => {
                    let v = self.lower_expr(&c.args[0])?;
                    let v = self.maybe_transfer(&c.args[0], v);
                    if matches!(exp, Some(MType::Named(n, _)) if n == "Optional") {
                        return Ok(v);
                    }
                    return Ok(MExpr::call_named("Optional", vec![v]));
                }
                "Ok" | "Err" => return unsup("Result constructor outside return position", tokens_str(c)),
                "drop" if c.args.len() == 1 => {
                    let v = self.lower_expr(&c.args[0])?;
                    return Ok(MExpr::Transfer(Box::new(v)));
                }
                "min" | "max" if c.args.len() == 2 => {
                    let args = self.lower_args(&c.args)?;
                    return Ok(MExpr::call_named(last, args));
                }
                _ => {}
            }
            if let Some(en) = self.variant_imports.get(last).cloned() {
                let f = self.enum_variant_value(&en, last)?;
                let args = self.lower_args(&c.args)?;
                return Ok(MExpr::call(f, args));
            }
            let args = self.lower_args(&c.args)?;
            let ta = self.lower_seg_args(&p.path.segments[0])?;
            let base = MExpr::name(names::ident(last));
            let f = if ta.is_empty() { base } else { MExpr::Param(Box::new(base), ta) };
            return Ok(MExpr::call(f, args));
        }
        let prev = segs[n - 2].as_str();
        if let (Some(w), "fmt", 2) = (self.f.fmt_writer.clone(), last, c.args.len()) {
            if single_ident(&c.args[1]).as_deref() == Some(w.as_str()) && (prev == "Display" || prev == "Debug") {
                let recv = self.lower_expr(&c.args[0])?;
                let m = if prev == "Display" { "write_to" } else { "write_repr_to" };
                return Ok(MExpr::method(recv, m, vec![MExpr::name("writer")]));
            }
        }
        // std constructors and UFCS
        match (prev, last) {
            ("Box", "new") | ("Rc", "new") | ("Arc", "new") if c.args.len() == 1 => {
                let v = self.lower_expr(&c.args[0])?;
                let v = self.maybe_transfer(&c.args[0], v);
                let t = if prev == "Box" { "OwnedPointer" } else { "ArcPointer" };
                self.import("std.memory", t);
                return Ok(MExpr::call_named(t, vec![v]));
            }
            ("Rc", "clone") | ("Arc", "clone") if c.args.len() == 1 => {
                let v = self.lower_expr(&c.args[0])?;
                return Ok(MExpr::method(v, "copy", vec![]));
            }
            ("RefCell", "new") | ("Cell", "new") if c.args.len() == 1 => {
                let v = self.lower_expr(&c.args[0])?;
                let v = self.maybe_transfer(&c.args[0], v);
                self.import("mojo_rt.cell", prev);
                if prev == "RefCell" {
                    self.todo("RefCell lowered to mojo_rt.cell.RefCell (no dynamic borrow checking)", tokens_str(c));
                }
                return Ok(MExpr::call_named(prev, vec![v]));
            }
            ("String", "new") if c.args.is_empty() => return Ok(MExpr::call_named("String", vec![])),
            ("String", "from") | ("String", "from_str") if c.args.len() == 1 => {
                let v = self.lower_expr(&c.args[0])?;
                return Ok(MExpr::call_named("String", vec![v]));
            }
            ("String", "with_capacity") if c.args.len() == 1 => {
                let v = self.lower_expr(&c.args[0])?;
                return Ok(MExpr::Call(Box::new(MExpr::name("String")), vec![MArgVal::kw("capacity", v)]));
            }
            ("Vec", "new") | ("Vec", "with_capacity") | ("HashMap", "new") | ("HashMap", "default") | ("HashSet", "new") | ("HashMap", "with_capacity") | ("HashSet", "with_capacity") | ("Vec", "default") => {
                let explicit = self.lower_seg_args(&p.path.segments[n - 2])?;
                let ty = if !explicit.is_empty() {
                    let mapped = match prev {
                        "Vec" => "List",
                        "HashMap" => "Dict",
                        _ => {
                            self.import("std.collections", "Set");
                            "Set"
                        }
                    };
                    Some(MType::Named(mapped.into(), explicit))
                } else {
                    exp.filter(|t| !contains_infer(t)).cloned()
                };
                let Some(ty) = ty else {
                    return unsup(format!("`{}::{}()` without a known element type", prev, last), tokens_str(c));
                };
                if last == "with_capacity" && prev == "Vec" {
                    let cap = self.lower_expr(&c.args[0])?;
                    return Ok(MExpr::Call(Box::new(MExpr::Type(ty)), vec![MArgVal::kw("capacity", cap)]));
                }
                return Ok(MExpr::call(MExpr::Type(ty), vec![]));
            }
            ("Default", "default") if c.args.is_empty() => {
                return match exp.filter(|t| !contains_infer(t)) {
                    Some(t) => Ok(self.default_value(t)),
                    None => unsup("`Default::default()` without a known target type", tokens_str(c)),
                };
            }
            ("cmp", "max") | ("cmp", "min") | ("f32", "max") | ("f32", "min") | ("f64", "max") | ("f64", "min") if c.args.len() == 2 => {
                let args = self.lower_args(&c.args)?;
                return Ok(MExpr::call_named(last, args));
            }
            ("mem", "swap") | ("mem", "replace") | ("mem", "take") => {
                return unsup(format!("`std::mem::{}`", last), tokens_str(c));
            }
            _ => {}
        }
        if names::prim(prev).is_some() && !["Self"].contains(&prev) {
            // f32::sqrt(x) style UFCS on primitives -> treat like method call
            if let Some(first) = c.args.first() {
                let recv_t = parse_rty(prev);
                let recv = self.lower_expr(first)?;
                let rest: Vec<Expr> = c.args.iter().skip(1).cloned().collect();
                let mut rest_l = vec![];
                for a in &rest {
                    rest_l.push(self.lower_expr(a)?);
                }
                if let Some(v) = self.numeric_method(last, recv, rest_l, recv_t.as_ref())? {
                    return Ok(v);
                }
            }
            return unsup(format!("primitive associated function `{}::{}`", prev, last), tokens_str(c));
        }
        let owner = if prev == "Self" { self.ts.self_name.clone().unwrap_or_else(|| "Self".into()) } else { prev.to_string() };
        if let Some(info) = self.syms.enums.get(&owner) {
            if info.variant(last).is_some() {
                let f = self.enum_variant_value(&owner, last)?;
                let args = self.lower_args(&c.args)?;
                return Ok(MExpr::call(f, args));
            }
        }
        if let Some(m) = self.ts.params.get(prev).cloned() {
            if last == "default" && c.args.is_empty() {
                return Ok(MExpr::call_named(&m, vec![]));
            }
            let args = self.lower_args(&c.args)?;
            return Ok(MExpr::call(MExpr::attr(MExpr::name(m), &names::ident(last)), args));
        }
        if last == "from" && c.args.len() == 1 && (self.syms.structs.contains_key(&owner) || self.syms.enums.contains_key(&owner) || prev == "Self") {
            let base = if prev == "Self" { MExpr::name("Self") } else { MExpr::name(prev) };
            let args = self.lower_args(&c.args)?;
            return Ok(MExpr::call(MExpr::attr(base, "from_"), args));
        }
        if self.syms.structs.contains_key(&owner) || self.syms.enums.contains_key(&owner) || prev == "Self" {
            let base = if prev == "Self" {
                MExpr::name("Self")
            } else {
                let a = self.lower_seg_args(&p.path.segments[n - 2])?;
                if a.is_empty() { MExpr::name(prev) } else { MExpr::Param(Box::new(MExpr::name(prev)), a) }
            };
            let args = self.lower_args(&c.args)?;
            return Ok(MExpr::call(MExpr::attr(base, &names::ident(last)), args));
        }
        if ["std", "core", "alloc"].contains(&segs[0].as_str()) {
            return unsup(format!("std function `{}`", segs.join("::")), tokens_str(c));
        }
        if let Some(module) = self.resolve_module_path(&segs[..n - 1]) {
            self.import(&module, last);
            let args = self.lower_args(&c.args)?;
            return Ok(MExpr::call(MExpr::name(names::ident(last)), args));
        }
        if prev.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            // associated fn of an external type (e.g. a type from another crate)
            self.todo(format!("associated function on external type `{}`", prev), tokens_str(&c.func));
            let args = self.lower_args(&c.args)?;
            return Ok(MExpr::call(MExpr::attr(MExpr::name(prev), &names::ident(last)), args));
        }
        unsup("call into an external crate/module", tokens_str(c))
    }

    pub fn default_value(&mut self, t: &MType) -> MExpr {
        match t {
            MType::Named(n, _) if n == "Optional" => MExpr::NoneLit,
            _ => MExpr::call(MExpr::Type(t.clone()), vec![]),
        }
    }

    fn lower_struct_lit(&mut self, s: &syn::ExprStruct) -> LResult<MExpr> {
        if s.qself.is_some() {
            return unsup("qualified struct literal", tokens_str(s));
        }
        let segs: Vec<String> = s.path.segments.iter().map(|x| x.ident.to_string()).collect();
        let n = segs.len();
        let last = segs[n - 1].clone();
        // enum struct-variant?
        let (callee, info_name, variant_fields): (MExpr, Option<String>, Option<Vec<String>>) = {
            let maybe_enum = if n >= 2 {
                let prev = &segs[n - 2];
                Some(if prev == "Self" { self.ts.self_name.clone().unwrap_or_default() } else { prev.clone() })
            } else {
                self.variant_imports.get(&last).cloned()
            };
            let mut res = None;
            if let Some(en) = &maybe_enum {
                if let Some(info) = self.syms.enums.get(en) {
                    if let Some(crate::symbols::VariantShape::Struct(fs)) = info.variant(&last) {
                        let fs = fs.clone();
                        res = Some((self.enum_variant_value(en, &last)?, None, Some(fs)));
                    }
                }
            }
            match res {
                Some(r) => r,
                None => {
                    let name = if last == "Self" { self.ts.self_name.clone().unwrap_or_else(|| "Self".into()) } else { last.clone() };
                    let a = self.lower_seg_args(s.path.segments.last().unwrap())?;
                    let base = if last == "Self" { MExpr::name("Self") } else { MExpr::name(&last) };
                    let callee = if a.is_empty() { base } else { MExpr::Param(Box::new(base), a) };
                    (callee, Some(name), None)
                }
            }
        };
        let info = info_name.as_ref().and_then(|n| self.syms.structs.get(n)).cloned();
        let mut args = vec![];
        let mut seen = vec![];
        for fv in &s.fields {
            let (fname, positional) = match &fv.member {
                syn::Member::Named(i) => (i.to_string(), false),
                syn::Member::Unnamed(i) => (i.index.to_string(), true),
            };
            seen.push(fname.clone());
            let exp_ty = info.as_ref().and_then(|si| {
                let ft = si.fields.iter().find(|f| f.0 == fname).map(|f| f.1.clone())?;
                if si.generics.iter().any(|g| tokens_str(&ft).split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == g)) {
                    None
                } else {
                    Some(ft)
                }
            });
            let exp_m = match exp_ty {
                Some(t) => self.lower_type(&t).ok(),
                None => None,
            };
            let v = self.lower_expr_exp(&fv.expr, exp_m.as_ref())?;
            let v = self.maybe_transfer(&fv.expr, v);
            if positional {
                args.push(MArgVal::pos(v));
            } else {
                args.push(MArgVal::kw(names::ident(&fname), v));
            }
        }
        if let Some(rest) = &s.rest {
            let field_names: Vec<String> = match (&info, &variant_fields) {
                (Some(si), _) => si.fields.iter().map(|f| f.0.clone()).collect(),
                (None, Some(vf)) => vf.clone(),
                _ => return unsup("struct update syntax `..base` on a type with unknown fields", tokens_str(s)),
            };
            let base_ty = info.as_ref().filter(|si| si.generics.is_empty()).map(|si| MType::named(&si.name));
            let base = if is_place(rest) {
                self.lower_expr(rest)?
            } else {
                let v = self.lower_expr_exp(rest, base_ty.as_ref())?;
                let t = self.fresh("base");
                self.f.pending.push(MStmt::Var { kind: BindKind::Var, name: t.clone(), ty: None, init: Some(v) });
                MExpr::name(t)
            };
            for f in field_names {
                if !seen.contains(&f) {
                    let fe = MExpr::attr(base.clone(), &names::ident(&f));
                    args.push(MArgVal::kw(names::ident(&f), fe));
                }
            }
        }
        Ok(MExpr::Call(Box::new(callee), args))
    }

    // ---- method calls ---------------------------------------------------------------

    fn recv_kind(&self, e: &Expr) -> RecvKind {
        let Some(t) = self.type_of(e) else {
            return RecvKind::Unknown;
        };
        self.kind_of_rty(&t)
    }

    fn kind_of_rty(&self, t: &syn::Type) -> RecvKind {
        if let syn::Type::Tuple(_) = strip_ref(t) {
            return RecvKind::Tuple;
        }
        let Some(n) = rty_last(t) else {
            return RecvKind::Unknown;
        };
        match n.as_str() {
            "Vec" | "SmallVec" | "VecDeque" => RecvKind::Vec,
            "HashMap" | "FxHashMap" | "IndexMap" => RecvKind::Map,
            "HashSet" | "FxHashSet" => RecvKind::Set,
            "Option" => RecvKind::Opt,
            "String" | "str" | "SharedString" => RecvKind::Str,
            "f32" | "f64" => RecvKind::Float,
            "bool" => RecvKind::Bool,
            "Rc" | "Arc" => RecvKind::Rc,
            "Box" => RecvKind::Boxed,
            "RefCell" => RecvKind::RefCell,
            "Cell" => RecvKind::Cell,
            x if names::is_int(x) => RecvKind::Int,
            x if self.ts.params.contains_key(x) => RecvKind::Param,
            x if self.syms.structs.contains_key(x) || self.syms.enums.contains_key(x) => RecvKind::User(x.to_string()),
            "Self" => match &self.ts.self_name {
                Some(s) => RecvKind::User(s.clone()),
                None => RecvKind::Param,
            },
            _ => RecvKind::Unknown,
        }
    }

    fn lower_method_call(&mut self, mc: &syn::ExprMethodCall, exp: Option<&MType>) -> LResult<MExpr> {
        let name = mc.method.to_string();
        if let Some(v) = self.try_iter_chain(mc, exp)? {
            return Ok(v);
        }
        let kind = self.recv_kind(&mc.receiver);
        let recv_t = self.type_of(&mc.receiver);
        let turbofish = match &mc.turbofish {
            Some(t) => {
                let mut v = vec![];
                for g in &t.args {
                    match g {
                        syn::GenericArgument::Type(ty) => v.push(MTypeArg::Type(self.lower_type(ty)?)),
                        _ => return unsup("unsupported method turbofish", tokens_str(mc)),
                    }
                }
                v
            }
            None => vec![],
        };

        // Formatter writes inside a Display/Debug impl
        // Universal trait methods first.
        match name.as_str() {
            "clone" if mc.args.is_empty() => {
                let r = self.lower_expr(&mc.receiver)?;
                return Ok(MExpr::method(r, "copy", vec![]));
            }
            "to_string" if mc.args.is_empty() => {
                let r = self.lower_expr(&mc.receiver)?;
                return Ok(MExpr::call_named("String", vec![r]));
            }
            "to_owned" if mc.args.is_empty() => {
                let r = self.lower_expr(&mc.receiver)?;
                if kind == RecvKind::Str || matches!(&*mc.receiver, Expr::Lit(_)) {
                    return Ok(MExpr::call_named("String", vec![r]));
                }
                return Ok(MExpr::method(r, "copy", vec![]));
            }
            "into" if mc.args.is_empty() => {
                let r = self.lower_expr(&mc.receiver)?;
                let r = self.maybe_transfer(&mc.receiver, r);
                return match exp.filter(|t| !contains_infer(t)) {
                    Some(MType::Named(n, a)) if a.is_empty() && (self.syms.structs.contains_key(n) || self.syms.enums.contains_key(n)) => {
                        Ok(MExpr::method(MExpr::name(n), "from_", vec![r]))
                    }
                    Some(t) => Ok(MExpr::call(MExpr::Type(t.clone()), vec![r])),
                    None => unsup("`.into()` without a known target type", tokens_str(mc)),
                };
            }
            "eq" | "ne" | "lt" | "le" | "gt" | "ge" if mc.args.len() == 1 => {
                let op = match name.as_str() {
                    "eq" => BinOp::Eq,
                    "ne" => BinOp::Ne,
                    "lt" => BinOp::Lt,
                    "le" => BinOp::Le,
                    "gt" => BinOp::Gt,
                    _ => BinOp::Ge,
                };
                let r = self.lower_expr(&mc.receiver)?;
                let a = self.lower_expr(&mc.args[0])?;
                return Ok(MExpr::bin(r, op, a));
            }
            _ => {}
        }

        let user_defined = self.syms.user_methods.contains(&name);
        let passthrough = |this: &mut Self| -> LResult<MExpr> {
            let r = this.lower_expr(&mc.receiver)?;
            let args = this.lower_args(&mc.args)?;
            let r = match this.recv_kind(&mc.receiver) {
                RecvKind::Rc | RecvKind::Boxed => MExpr::Index(Box::new(r), vec![]),
                _ => r,
            };
            let m = MExpr::attr(r, &names::ident(&name));
            let m = if turbofish.is_empty() { m } else { MExpr::Param(Box::new(m), turbofish.clone()) };
            Ok(MExpr::call(m, args))
        };

        match &kind {
            RecvKind::User(_) | RecvKind::Param => return passthrough(self),
            RecvKind::Rc | RecvKind::Boxed => {
                // auto-deref to the pointee
                if let Some(inner) = recv_t.as_ref().and_then(|t| rty_args(t).into_iter().next()) {
                    let ik = self.kind_of_rty(&inner);
                    if ik == RecvKind::RefCell {
                        return self.refcell_method(&name, mc, true);
                    }
                }
                return passthrough(self);
            }
            RecvKind::RefCell => return self.refcell_method(&name, mc, false),
            RecvKind::Cell => {
                let r = self.lower_expr(&mc.receiver)?;
                let args = self.lower_args(&mc.args)?;
                if name != "get" {
                    self.todo("Cell mutation needs `mut` access in Mojo (no interior mutability)", tokens_str(mc));
                }
                return match name.as_str() {
                    "get" | "set" | "replace" | "take" => Ok(MExpr::method(r, &name, args)),
                    _ => unsup(format!("Cell method `.{}()`", name), tokens_str(mc)),
                };
            }
            _ => {}
        }

        let recv_is_unknown = kind == RecvKind::Unknown;
        if recv_is_unknown && user_defined {
            return passthrough(self);
        }

        let r = self.lower_expr(&mc.receiver)?;
        let mut args = vec![];
        for a in &mc.args {
            let x = self.lower_expr(a)?;
            args.push(self.maybe_transfer(a, x));
        }

        // numeric
        if matches!(kind, RecvKind::Float | RecvKind::Int | RecvKind::Unknown) {
            if let Some(v) = self.numeric_method(&name, r.clone(), args.clone(), recv_t.as_ref())? {
                return Ok(v);
            }
        }
        // Option
        if matches!(kind, RecvKind::Opt | RecvKind::Unknown) {
            if let Some(v) = self.option_method(&name, r.clone(), args.clone())? {
                return Ok(v);
            }
        }
        // collections / strings
        if matches!(kind, RecvKind::Vec | RecvKind::Map | RecvKind::Set | RecvKind::Str | RecvKind::Unknown) {
            if let Some(v) = self.collection_method(&name, &kind, r.clone(), args.clone(), mc)? {
                return Ok(v);
            }
        }
        if ITER_METHODS.contains(&name.as_str()) {
            return unsup(format!("iterator/closure method `.{}()`", name), tokens_str(mc));
        }
        if KNOWN_STD_UNMAPPED.contains(&name.as_str()) || !recv_is_unknown {
            return unsup(format!("std method `.{}()` not mapped", name), tokens_str(mc));
        }
        // Method on a value of unknown type from outside the crate: pass through.
        self.todo(format!("method `.{}()` on a value of unknown type passed through", name), tokens_str(mc));
        let m = MExpr::attr(r, &names::ident(&name));
        let m = if turbofish.is_empty() { m } else { MExpr::Param(Box::new(m), turbofish) };
        Ok(MExpr::call(m, args))
    }

    fn refcell_method(&mut self, name: &str, mc: &syn::ExprMethodCall, via_rc: bool) -> LResult<MExpr> {
        let mut r = self.lower_expr(&mc.receiver)?;
        if via_rc {
            r = MExpr::Index(Box::new(r), vec![]);
        }
        let args = self.lower_args(&mc.args)?;
        match name {
            "borrow" | "borrow_mut" | "replace" | "take" | "set" => {
                self.todo("RefCell lowered to mojo_rt.cell.RefCell (no dynamic borrow checking)", tokens_str(mc));
                Ok(MExpr::method(r, name, args))
            }
            "clone" => Ok(MExpr::method(r, "copy", args)),
            _ => unsup(format!("RefCell method `.{}()`", name), tokens_str(mc)),
        }
    }

    pub fn numeric_method(&mut self, name: &str, r: MExpr, args: Vec<MExpr>, recv_t: Option<&syn::Type>) -> LResult<Option<MExpr>> {
        let a0 = args.first().cloned();
        let math = |this: &mut Self, f: &str, mut v: Vec<MExpr>| -> MExpr {
            this.import("std.math", f);
            v.insert(0, r.clone());
            MExpr::call_named(f, v)
        };
        let rt = |this: &mut Self, f: &str, mut v: Vec<MExpr>| -> MExpr {
            let fe = this.rt("num", f);
            v.insert(0, r.clone());
            MExpr::call(fe, v)
        };
        let is_float = recv_t.map(is_float_rty).unwrap_or(false);
        Ok(Some(match (name, args.len()) {
            ("abs", 0) => MExpr::call_named("abs", vec![r.clone()]),
            ("min", 1) | ("max", 1) => {
                if is_float {
                    // Rust f32::max/min ignore NaN; route through the shim.
                    rt(self, &format!("f{}", name), args)
                } else {
                    MExpr::call_named(name, vec![r.clone(), a0.unwrap()])
                }
            }
            ("sqrt" | "floor" | "ceil" | "trunc" | "exp" | "exp2" | "log2" | "log10" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "cbrt", 0) => math(self, name, vec![]),
            ("ln", 0) => math(self, "log", vec![]),
            ("round", 0) => rt(self, "round_half_away", vec![]),
            ("powi", 1) => MExpr::bin(r.clone(), BinOp::Pow, a0.unwrap()),
            ("pow", 1) => MExpr::bin(r.clone(), BinOp::Pow, a0.unwrap()),
            ("powf", 1) => math(self, "pow", args),
            ("clamp", 2) => math(self, "clamp", args),
            ("mul_add", 2) => math(self, "fma", args),
            ("hypot", 1) | ("atan2", 1) | ("copysign", 1) => math(self, name, args),
            ("signum" | "to_radians" | "to_degrees" | "is_nan" | "is_finite" | "is_infinite" | "fract" | "recip", 0) => rt(self, name, vec![]),
            ("rem_euclid" | "div_euclid" | "abs_diff", 1) => rt(self, name, args),
            ("wrapping_add", 1) => MExpr::bin(r.clone(), BinOp::Add, a0.unwrap()),
            ("wrapping_sub", 1) => MExpr::bin(r.clone(), BinOp::Sub, a0.unwrap()),
            ("wrapping_mul", 1) => MExpr::bin(r.clone(), BinOp::Mul, a0.unwrap()),
            ("wrapping_neg", 0) => MExpr::Unary(UnOp::Neg, Box::new(r.clone())),
            ("checked_add" | "checked_sub" | "checked_mul" | "checked_div" | "checked_rem" | "saturating_add" | "saturating_sub" | "saturating_mul", 1) => rt(self, name, args),
            ("count_ones" | "leading_zeros" | "trailing_zeros", 0) => rt(self, name, vec![]),
            _ => return Ok(None),
        }))
    }

    fn option_method(&mut self, name: &str, r: MExpr, args: Vec<MExpr>) -> LResult<Option<MExpr>> {
        Ok(Some(match (name, args.len()) {
            ("is_some", 0) => MExpr::bin(r, BinOp::IsNot, MExpr::NoneLit),
            ("is_none", 0) => MExpr::bin(r, BinOp::Is, MExpr::NoneLit),
            ("unwrap", 0) => MExpr::method(r, "value", vec![]),
            ("expect", 1) => MExpr::method(r, "value", vec![]),
            ("unwrap_or", 1) => {
                let f = self.rt("option", "unwrap_or");
                MExpr::call(f, vec![r, args[0].clone()])
            }
            ("unwrap_or_default", 0) => {
                let f = self.rt("option", "unwrap_or_default");
                MExpr::call(f, vec![r])
            }
            ("take", 0) => {
                let f = self.rt("option", "take");
                MExpr::call(f, vec![r])
            }
            ("replace", 1) => {
                let f = self.rt("option", "replace");
                MExpr::call(f, vec![r, args[0].clone()])
            }
            ("as_ref", 0) | ("copied", 0) | ("cloned", 0) => r,
            _ => return Ok(None),
        }))
    }

    fn collection_method(&mut self, name: &str, kind: &RecvKind, r: MExpr, args: Vec<MExpr>, mc: &syn::ExprMethodCall) -> LResult<Option<MExpr>> {
        let a = |i: usize| args[i].clone();
        Ok(Some(match (name, args.len()) {
            ("len", 0) => {
                if *kind == RecvKind::Str {
                    self.todo("String len(): Rust counts UTF-8 bytes; verify Mojo String len semantics", tokens_str(mc));
                }
                MExpr::call_named("len", vec![r])
            }
            ("is_empty", 0) => MExpr::bin(MExpr::call_named("len", vec![r]), BinOp::Eq, MExpr::Int("0".into())),
            ("push", 1) if *kind == RecvKind::Str => MExpr::method(r, "__iadd__", vec![MExpr::call_named("String", vec![a(0)])]),
            ("push", 1) => MExpr::method(r, "append", vec![a(0)]),
            ("push_str", 1) => MExpr::method(r, "__iadd__", vec![a(0)]),
            ("pop", 0) if *kind == RecvKind::Vec => {
                let f = self.rt("vec", "pop");
                MExpr::call(f, vec![r])
            }
            ("insert", 2) if *kind == RecvKind::Vec => MExpr::method(r, "insert", vec![a(0), a(1)]),
            ("remove", 1) if *kind == RecvKind::Vec => MExpr::method(r, "pop", vec![a(0)]),
            ("clear", 0) => MExpr::method(r, "clear", vec![]),
            ("reverse", 0) if *kind == RecvKind::Vec => MExpr::method(r, "reverse", vec![]),
            ("reserve", 1) if *kind == RecvKind::Vec => MExpr::method(r, "reserve", vec![a(0)]),
            ("truncate", 1) if *kind == RecvKind::Vec => {
                let f = self.rt("vec", "truncate");
                MExpr::call(f, vec![r, a(0)])
            }
            ("swap", 2) if *kind == RecvKind::Vec => {
                let f = self.rt("vec", "swap");
                MExpr::call(f, vec![r, a(0), a(1)])
            }
            ("first", 0) | ("last", 0) if *kind == RecvKind::Vec => {
                let f = self.rt("vec", name);
                MExpr::call(f, vec![r])
            }
            ("get", 1) if *kind == RecvKind::Vec => {
                let f = self.rt("vec", "get");
                MExpr::call(f, vec![r, a(0)])
            }
            ("get", 1) if *kind == RecvKind::Map => MExpr::method(r, "get", vec![a(0)]),
            ("contains", 1) if matches!(kind, RecvKind::Vec | RecvKind::Set | RecvKind::Str) => MExpr::bin(a(0), BinOp::In, r),
            ("contains_key", 1) => MExpr::bin(a(0), BinOp::In, r),
            ("remove", 1) if *kind == RecvKind::Map => {
                let f = self.rt("dict", "remove");
                MExpr::call(f, vec![r, a(0)])
            }
            ("insert", 1) if *kind == RecvKind::Set => MExpr::method(r, "add", vec![a(0)]),
            ("extend", 1) if *kind == RecvKind::Vec => {
                if let Expr::MethodCall(_) = &mc.args[0] {
                    return unsup("`.extend()` from an iterator chain", tokens_str(mc));
                }
                MExpr::method(r, "extend", vec![a(0)])
            }
            ("to_vec", 0) => MExpr::method(r, "copy", vec![]),
            ("as_str", 0) | ("as_slice", 0) => r,
            ("trim", 0) if *kind == RecvKind::Str => MExpr::method(r, "strip", vec![]),
            ("trim_start", 0) if *kind == RecvKind::Str => MExpr::method(r, "lstrip", vec![]),
            ("trim_end", 0) if *kind == RecvKind::Str => MExpr::method(r, "rstrip", vec![]),
            ("to_lowercase", 0) if *kind == RecvKind::Str => MExpr::method(r, "lower", vec![]),
            ("to_uppercase", 0) if *kind == RecvKind::Str => MExpr::method(r, "upper", vec![]),
            ("replace", 2) if *kind == RecvKind::Str => MExpr::method(r, "replace", vec![a(0), a(1)]),
            _ => return Ok(None),
        }))
    }

    // ======================================================================
    // Light type inference
    // ======================================================================

    pub fn type_of(&self, e: &Expr) -> Option<syn::Type> {
        match e {
            Expr::Paren(p) => self.type_of(&p.expr),
            Expr::Group(g) => self.type_of(&g.expr),
            Expr::Reference(r) => self.type_of(&r.expr),
            Expr::Unary(u) => match u.op {
                RU::Deref(_) => {
                    let t = self.type_of(&u.expr)?;
                    match rty_last(&t).as_deref() {
                        Some("Box") | Some("Rc") | Some("Arc") => rty_args(&t).into_iter().next(),
                        _ => Some(t),
                    }
                }
                RU::Not(_) => self.type_of(&u.expr),
                RU::Neg(_) => self.type_of(&u.expr),
                _ => None,
            },
            Expr::Lit(l) => match &l.lit {
                Lit::Int(i) => parse_rty(if i.suffix().is_empty() { "i32" } else { i.suffix() }).filter(|_| !i.suffix().is_empty()),
                Lit::Float(f) => parse_rty(if f.suffix().is_empty() { "f64" } else { f.suffix() }).filter(|_| !f.suffix().is_empty()),
                Lit::Bool(_) => parse_rty("bool"),
                Lit::Str(_) => parse_rty("str"),
                Lit::Char(_) => parse_rty("char"),
                _ => None,
            },
            Expr::Path(p) if p.path.segments.len() == 1 => {
                let n = p.path.segments[0].ident.to_string();
                if let Some(b) = self.lookup(&n) {
                    return b.rty.clone();
                }
                None
            }
            Expr::Field(f) => {
                let bt = self.type_of(&f.base)?;
                let bt = strip_ref(&bt).clone();
                if let syn::Type::Tuple(t) = &bt {
                    if let syn::Member::Unnamed(i) = &f.member {
                        return t.elems.iter().nth(i.index as usize).cloned();
                    }
                }
                let sname = rty_last(&bt)?;
                let sname = if sname == "Self" { self.ts.self_name.clone()? } else { sname };
                let si = self.syms.structs.get(&sname)?;
                let fname = match &f.member {
                    syn::Member::Named(n) => n.to_string(),
                    syn::Member::Unnamed(i) => i.index.to_string(),
                };
                let ft = si.fields.iter().find(|x| x.0 == fname)?.1.clone();
                // substitute generic args if known
                let args = rty_args(&bt);
                if !si.generics.is_empty() {
                    if args.len() == si.generics.len() {
                        let map: HashMap<String, syn::Type> = si.generics.iter().cloned().zip(args).collect();
                        return Some(strip_ref(&subst_rty(&ft, &map)).clone());
                    }
                    // inside the struct's own impl: keep param names
                    if self.ts.self_name.as_deref() == Some(sname.as_str()) {
                        return Some(strip_ref(&ft).clone());
                    }
                    if si.generics.iter().any(|g| tokens_str(&ft).contains(g.as_str())) {
                        return None;
                    }
                }
                Some(strip_ref(&ft).clone())
            }
            Expr::Cast(c) => Some((*c.ty).clone()),
            Expr::Binary(b) => match b.op {
                RB::Eq(_) | RB::Ne(_) | RB::Lt(_) | RB::Le(_) | RB::Gt(_) | RB::Ge(_) | RB::And(_) | RB::Or(_) => parse_rty("bool"),
                RB::Add(_) | RB::Sub(_) | RB::Mul(_) | RB::Div(_) | RB::Rem(_) => {
                    let l = self.type_of(&b.left);
                    match &l {
                        Some(t) if is_prim_rty(t) => l,
                        _ => {
                            let r = self.type_of(&b.right);
                            if l.is_none() && r.as_ref().map(is_prim_rty).unwrap_or(false) {
                                r
                            } else {
                                None
                            }
                        }
                    }
                }
                _ => None,
            },
            Expr::MethodCall(mc) => {
                let m = mc.method.to_string();
                match m.as_str() {
                    "clone" | "abs" | "min" | "max" | "sqrt" | "floor" | "ceil" | "round" | "clamp" | "powi" | "powf" | "trunc" | "signum" => self.type_of(&mc.receiver),
                    "len" => parse_rty("usize"),
                    "is_empty" | "is_some" | "is_none" | "contains" | "contains_key" | "starts_with" | "ends_with" | "is_nan" => parse_rty("bool"),
                    "to_string" => parse_rty("String"),
                    "unwrap" | "expect" | "unwrap_or" | "unwrap_or_default" => {
                        let t = self.type_of(&mc.receiver)?;
                        if rty_last(&t).as_deref() == Some("Option") {
                            rty_args(&t).into_iter().next()
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            Expr::Call(c) => {
                if let Expr::Path(p) = &*c.func {
                    let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
                    let last = segs.last()?;
                    if segs.len() >= 2 {
                        let prev = segs[segs.len() - 2].as_str();
                        let arg_t = c.args.first().and_then(|a| self.type_of(a));
                        match (prev, last.as_str()) {
                            ("Rc", "clone") | ("Arc", "clone") => return arg_t,
                            ("Rc" | "Arc" | "Box" | "RefCell" | "Cell", "new") => {
                                let w = syn::Ident::new(prev, proc_macro2::Span::call_site());
                                return match arg_t {
                                    Some(t) => syn::parse2::<syn::Type>(quote::quote!(#w<#t>)).ok(),
                                    None => syn::parse2::<syn::Type>(quote::quote!(#w<_>)).ok(),
                                };
                            }
                            _ => {}
                        }
                    }
                    if self.syms.structs.get(last).map(|s| s.tuple).unwrap_or(false) {
                        return parse_rty(last);
                    }
                    if segs.len() >= 2 {
                        let prev = &segs[segs.len() - 2];
                        let owner = if prev == "Self" { self.ts.self_name.clone()? } else { prev.clone() };
                        if self.syms.enums.contains_key(&owner) || ((last == "new" || last == "from") && self.syms.structs.contains_key(&owner)) {
                            if self.syms.structs.get(&owner).map(|s| s.generics.is_empty()).unwrap_or(true) && self.syms.enums.get(&owner).map(|s| s.generics.is_empty()).unwrap_or(true) {
                                return parse_rty(&owner);
                            }
                        }
                    }
                }
                None
            }
            Expr::Struct(s) => {
                let last = s.path.segments.last()?.ident.to_string();
                let name = if last == "Self" { self.ts.self_name.clone()? } else { last };
                if self.syms.structs.get(&name).map(|s| s.generics.is_empty()).unwrap_or(false) {
                    parse_rty(&name)
                } else {
                    None
                }
            }
            Expr::Tuple(t) => {
                let mut tys = vec![];
                for e in &t.elems {
                    tys.push(self.type_of(e)?);
                }
                syn::parse2::<syn::Type>(quote::quote!((#(#tys),*))).ok()
            }
            Expr::Index(i) => {
                let t = self.type_of(&i.expr)?;
                if matches!(&*i.index, Expr::Range(_)) {
                    return Some(t);
                }
                match rty_last(&t).as_deref() {
                    Some("Vec") | Some("SmallVec") => rty_args(&t).into_iter().next(),
                    _ => {
                        if let syn::Type::Array(a) = strip_ref(&t) {
                            return Some((*a.elem).clone());
                        }
                        None
                    }
                }
            }
            _ => None,
        }
    }
}

fn is_pure(e: &MExpr) -> bool {
    matches!(e, MExpr::Name(_) | MExpr::Int(_) | MExpr::Float(_) | MExpr::Str(_) | MExpr::Bool(_) | MExpr::NoneLit)
}

pub fn contains_infer(t: &MType) -> bool {
    match t {
        MType::Infer => true,
        MType::Named(_, a) => a.iter().any(|x| matches!(x, MTypeArg::Type(t) if contains_infer(t))),
        MType::Tuple(ts) => ts.iter().any(contains_infer),
        MType::Func(a, r) => a.iter().any(contains_infer) || r.as_ref().map(|r| contains_infer(r)).unwrap_or(false),
        MType::Some(_) => false,
    }
}

fn needs_stmt_lowering(e: &Expr) -> bool {
    match e {
        Expr::If(_) | Expr::Match(_) | Expr::Loop(_) => true,
        Expr::Block(b) => !(b.block.stmts.len() == 1 && matches!(&b.block.stmts[0], Stmt::Expr(_, None))),
        Expr::Paren(p) => needs_stmt_lowering(&p.expr),
        _ => false,
    }
}

fn flatten_and<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    match e {
        Expr::Binary(b) if matches!(b.op, RB::And(_)) => {
            flatten_and(&b.left, out);
            flatten_and(&b.right, out);
        }
        Expr::Paren(p) if matches!(&*p.expr, Expr::Let(_)) => out.push(&p.expr),
        _ => out.push(e),
    }
}

fn compound_op(op: &RB) -> Option<BinOp> {
    Some(match op {
        RB::AddAssign(_) => BinOp::Add,
        RB::SubAssign(_) => BinOp::Sub,
        RB::MulAssign(_) => BinOp::Mul,
        RB::DivAssign(_) => BinOp::Div,
        RB::RemAssign(_) => BinOp::Mod,
        RB::BitXorAssign(_) => BinOp::BitXor,
        RB::BitAndAssign(_) => BinOp::BitAnd,
        RB::BitOrAssign(_) => BinOp::BitOr,
        RB::ShlAssign(_) => BinOp::Shl,
        RB::ShrAssign(_) => BinOp::Shr,
        _ => return None,
    })
}

//! Iterator-chain lowering: `src.iter().filter(..).map(..).collect()` and
//! friends become an explicit loop hoisted in front of the current statement.
//!
//! Closure parameter types are inferred from the element type of the source
//! (`Vec<T>` / `&[T]` / ranges), which is what makes untyped closures like
//! `|x| x * 2` translatable here. Supported:
//!   sources:    `v.iter()`, `v.into_iter()`, `v.iter().copied()/cloned()`, `(a..b)`
//!   adapters:   `map`, `filter`, `copied`, `cloned`
//!   terminals:  `collect` (to Vec), `any`, `all`, `count`, `sum`, `for_each`
//! Anything else returns `None` so the caller reports it as unsupported.

use super::expr::Sink;
use super::types::*;
use super::*;
use syn::Expr;

enum Stage<'e> {
    Map(&'e syn::ExprClosure),
    Filter(&'e syn::ExprClosure),
}

fn closure_param(c: &syn::ExprClosure) -> Option<String> {
    if c.inputs.len() != 1 {
        return None;
    }
    let mut p = &c.inputs[0];
    loop {
        match p {
            syn::Pat::Reference(r) => p = &r.pat,
            syn::Pat::Type(t) => p = &t.pat,
            syn::Pat::Ident(pi) if pi.subpat.is_none() => return Some(pi.ident.to_string()),
            syn::Pat::Wild(_) => return Some("_".into()),
            _ => return None,
        }
    }
}

/// Closure bodies may not contain control flow that escapes the closure.
fn body_is_local(e: &Expr) -> bool {
    use syn::visit::Visit;
    struct V(bool);
    impl<'ast> Visit<'ast> for V {
        fn visit_expr_return(&mut self, _: &'ast syn::ExprReturn) {
            self.0 = false;
        }
        fn visit_expr_try(&mut self, _: &'ast syn::ExprTry) {
            self.0 = false;
        }
        fn visit_expr_break(&mut self, _: &'ast syn::ExprBreak) {
            self.0 = false;
        }
        fn visit_expr_continue(&mut self, _: &'ast syn::ExprContinue) {
            self.0 = false;
        }
        fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
    }
    let mut v = V(true);
    v.visit_expr(e);
    v.0
}

impl<'a> FileCtx<'a> {
    /// Try to lower an iterator chain ending in `mc`. `Ok(None)` = not a
    /// chain we handle (caller falls back to its normal reporting).
    pub fn try_iter_chain(&mut self, mc: &syn::ExprMethodCall, exp: Option<&MType>) -> LResult<Option<MExpr>> {
        let terminal = mc.method.to_string();
        if !["collect", "any", "all", "count", "sum", "for_each"].contains(&terminal.as_str()) {
            return Ok(None);
        }
        // Walk adapters down to the source.
        let mut stages: Vec<Stage> = vec![];
        let mut cur: &Expr = &mc.receiver;
        let source: &Expr;
        let mut range_src = false;
        loop {
            match cur {
                Expr::MethodCall(m) => {
                    let n = m.method.to_string();
                    match n.as_str() {
                        "map" | "filter" if m.args.len() == 1 => {
                            let Expr::Closure(c) = &m.args[0] else { return Ok(None) };
                            stages.push(if n == "map" { Stage::Map(c) } else { Stage::Filter(c) });
                            cur = &m.receiver;
                        }
                        "copied" | "cloned" if m.args.is_empty() => cur = &m.receiver,
                        "iter" | "into_iter" if m.args.is_empty() => {
                            source = &m.receiver;
                            break;
                        }
                        _ => return Ok(None),
                    }
                }
                Expr::Paren(p) if matches!(&*p.expr, Expr::Range(_)) => {
                    source = &p.expr;
                    range_src = true;
                    break;
                }
                _ => return Ok(None),
            }
        }
        stages.reverse();
        // Element type of the source.
        let mut elem: Option<syn::Type> = if range_src {
            let Expr::Range(r) = source else { return Ok(None) };
            r.start.as_ref().and_then(|s| self.type_of(s)).or_else(|| r.end.as_ref().and_then(|e| self.type_of(e)))
        } else {
            let t = self.type_of(source);
            match t.as_ref().map(strip_ref) {
                Some(syn::Type::Slice(s)) => Some((*s.elem).clone()),
                Some(syn::Type::Array(a)) => Some((*a.elem).clone()),
                Some(t) if matches!(rty_last(t).as_deref(), Some("Vec") | Some("SmallVec") | Some("VecDeque") | Some("HashSet")) => rty_args(t).into_iter().next(),
                _ => None,
            }
        };
        if elem.is_none() && !range_src {
            return Ok(None); // need the element type to type closure params
        }
        for s in &stages {
            let c = match s {
                Stage::Map(c) | Stage::Filter(c) => c,
            };
            if closure_param(c).is_none() || !body_is_local(&c.body) || c.asyncness.is_some() {
                return Ok(None);
            }
        }
        let terminal_closure = if ["any", "all", "for_each"].contains(&terminal.as_str()) {
            let Some(Expr::Closure(c)) = mc.args.first() else { return Ok(None) };
            if closure_param(c).is_none() || !body_is_local(&c.body) {
                return Ok(None);
            }
            Some(c)
        } else {
            None
        };

        // Build the loop.
        let iterable = if range_src {
            let (it, _, _, _) = self.lower_for_iter_pub(source)?;
            it
        } else {
            self.lower_expr(source)?
        };
        let mut pre = self.take_pending();
        self.push_scope();
        let r = (|| -> LResult<(Vec<MStmt>, MStmt, MExpr)> {
            let loop_var = self.declare("__it", elem.clone(), true, false);
            let mut cur_expr = MExpr::name(&loop_var);
            let mut body: Vec<MStmt> = vec![];
            self.f.loop_depth += 1;
            for s in &stages {
                match s {
                    Stage::Filter(c) => {
                        let p = closure_param(c).unwrap();
                        self.bind_expr(&p, cur_expr.clone(), elem.clone());
                        let cond = self.lower_expr(&c.body)?;
                        body.extend(self.take_pending());
                        body.push(MStmt::If { arms: vec![IfArm { cond: MExpr::not(cond), body: vec![MStmt::Continue] }], else_body: None });
                    }
                    Stage::Map(c) => {
                        let p = closure_param(c).unwrap();
                        self.bind_expr(&p, cur_expr.clone(), elem.clone());
                        let new_elem = self.type_of(&c.body);
                        let v = self.lower_expr(&c.body)?;
                        body.extend(self.take_pending());
                        let n = self.declare("__mapped", new_elem.clone(), false, false);
                        body.push(MStmt::Var { kind: BindKind::Var, name: n.clone(), ty: None, init: Some(v) });
                        cur_expr = MExpr::name(n);
                        elem = new_elem;
                    }
                }
            }
            // terminal
            let acc = self.fresh("acc");
            let acc_e = MExpr::name(&acc);
            let (init, step, result): (MStmt, Vec<MStmt>, MExpr) = match terminal.as_str() {
                "collect" => {
                    let target = match exp.filter(|t| t.is_named("List") && !super::expr::contains_infer(t)) {
                        Some(t) => t.clone(),
                        None => {
                            let Some(et) = &elem else {
                                return unsup("`.collect()` with unknown element type", tokens_str(mc));
                            };
                            let turbofish_ok = match &mc.turbofish {
                                None => exp.map(|t| t.is_named("List")).unwrap_or(false) || exp.is_none(),
                                Some(tf) => tokens_str(tf).contains("Vec"),
                            };
                            if !turbofish_ok {
                                return unsup("`.collect()` into a non-Vec collection", tokens_str(mc));
                            }
                            if exp.is_none() && mc.turbofish.is_none() {
                                return unsup("`.collect()` without a known target collection", tokens_str(mc));
                            }
                            MType::generic("List", vec![self.lower_type(et)?])
                        }
                    };
                    (
                        MStmt::Var { kind: BindKind::Var, name: acc.clone(), ty: Some(target.clone()), init: Some(MExpr::call(MExpr::Type(target), vec![])) },
                        vec![MStmt::Expr(MExpr::method(acc_e.clone(), "append", vec![cur_expr.clone()]))],
                        MExpr::Transfer(Box::new(acc_e.clone())),
                    )
                }
                "count" => (
                    MStmt::Var { kind: BindKind::Var, name: acc.clone(), ty: Some(MType::named("Int")), init: Some(MExpr::Int("0".into())) },
                    vec![MStmt::AugAssign(acc_e.clone(), BinOp::Add, MExpr::Int("1".into()))],
                    acc_e.clone(),
                ),
                "sum" => {
                    let t = match (&mc.turbofish, &elem) {
                        (Some(tf), _) => tf.args.first().and_then(|g| if let syn::GenericArgument::Type(t) = g { Some(t.clone()) } else { None }),
                        (None, Some(e)) => Some(strip_ref(e).clone()),
                        _ => None,
                    };
                    let Some(t) = t.filter(is_prim_rty) else {
                        return unsup("`.sum()` over a non-primitive or unknown element type", tokens_str(mc));
                    };
                    let mt = self.lower_type(&t)?;
                    (
                        MStmt::Var { kind: BindKind::Var, name: acc.clone(), ty: Some(mt.clone()), init: Some(MExpr::call(MExpr::Type(mt), vec![MExpr::Int("0".into())])) },
                        vec![MStmt::AugAssign(acc_e.clone(), BinOp::Add, cur_expr.clone())],
                        acc_e.clone(),
                    )
                }
                "any" | "all" | "for_each" => {
                    let c = terminal_closure.unwrap();
                    let p = closure_param(c).unwrap();
                    self.bind_expr(&p, cur_expr.clone(), elem.clone());
                    if terminal == "for_each" {
                        let mut stmts = vec![];
                        self.lower_expr_sink(&c.body, &Sink::Discard, &mut stmts)?;
                        (MStmt::Pass, stmts, MExpr::NoneLit)
                    } else {
                        let cond = self.lower_expr(&c.body)?;
                        let mut stmts = self.take_pending();
                        let is_any = terminal == "any";
                        let test = if is_any { cond } else { MExpr::not(cond) };
                        stmts.push(MStmt::If {
                            arms: vec![IfArm { cond: test, body: vec![MStmt::Assign(acc_e.clone(), MExpr::Bool(is_any)), MStmt::Break] }],
                            else_body: None,
                        });
                        (MStmt::Var { kind: BindKind::Var, name: acc.clone(), ty: Some(MType::named("Bool")), init: Some(MExpr::Bool(!is_any)) }, stmts, acc_e.clone())
                    }
                }
                _ => unreachable!(),
            };
            body.extend(step);
            self.f.loop_depth -= 1;
            let for_stmt = MStmt::For { is_ref: false, targets: vec![loop_var], iter: iterable.clone(), body };
            Ok((vec![init], for_stmt, result))
        })();
        self.pop_scope();
        let (inits, for_stmt, result) = r?;
        for i in inits {
            if i != MStmt::Pass {
                pre.push(i);
            }
        }
        pre.push(for_stmt);
        self.f.pending = pre;
        Ok(Some(result))
    }
}

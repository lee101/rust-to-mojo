//! Pattern lowering. A pattern against a place expression becomes a list of
//! boolean conditions plus a list of bindings (name -> place sub-expression).
//! `match`, `if let`, `while let`, `matches!` and destructuring `let` all go
//! through here.

use super::names;
use super::types::*;
use super::*;
use crate::symbols::VariantShape;
use syn::Pat;

#[derive(Clone, Debug)]
pub struct PatBind {
    pub rust: String,
    pub expr: MExpr,
    pub mutable: bool,
    pub rty: Option<syn::Type>,
}

#[derive(Clone, Debug, Default)]
pub struct PatOut {
    pub conds: Vec<MExpr>,
    pub binds: Vec<PatBind>,
}

impl PatOut {
    fn extend(&mut self, o: PatOut) {
        self.conds.extend(o.conds);
        self.binds.extend(o.binds);
    }
}

impl<'a> FileCtx<'a> {
    pub fn lower_pat(&mut self, p: &Pat, place: &MExpr, rty: Option<&syn::Type>) -> LResult<PatOut> {
        let mut out = PatOut::default();
        match p {
            Pat::Wild(_) => {}
            Pat::Paren(pp) => return self.lower_pat(&pp.pat, place, rty),
            Pat::Reference(r) => return self.lower_pat(&r.pat, place, rty),
            Pat::Type(pt) => return self.lower_pat(&pt.pat, place, Some(&pt.ty)),
            Pat::Ident(pi) => {
                let n = pi.ident.to_string();
                if pi.subpat.is_none() && n.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    if let Some(en) = self.variant_imports.get(&n).cloned() {
                        return self.variant_pat(&en, &n, None, place);
                    }
                    if self.syms.consts.contains(&n) {
                        out.conds.push(MExpr::bin(place.clone(), BinOp::Eq, MExpr::name(&n)));
                        return Ok(out);
                    }
                    if n == "None" {
                        out.conds.push(MExpr::bin(place.clone(), BinOp::Is, MExpr::NoneLit));
                        return Ok(out);
                    }
                }
                out.binds.push(PatBind {
                    rust: n,
                    expr: place.clone(),
                    mutable: pi.mutability.is_some() && pi.by_ref.is_none(),
                    rty: rty.map(|t| strip_ref(t).clone()),
                });
                if let Some((_, sub)) = &pi.subpat {
                    out.extend(self.lower_pat(sub, place, rty)?);
                }
            }
            Pat::Lit(l) => {
                let v = self.lower_expr(&syn::Expr::Lit(l.clone()))?;
                out.conds.push(MExpr::bin(place.clone(), BinOp::Eq, v));
            }
            Pat::Range(r) => {
                if let Some(s) = &r.start {
                    let v = self.lower_expr(s)?;
                    out.conds.push(MExpr::bin(place.clone(), BinOp::Ge, v));
                }
                if let Some(e) = &r.end {
                    let v = self.lower_expr(e)?;
                    let op = if matches!(r.limits, syn::RangeLimits::Closed(_)) { BinOp::Le } else { BinOp::Lt };
                    out.conds.push(MExpr::bin(place.clone(), op, v));
                }
            }
            Pat::Or(o) => {
                let mut alts = vec![];
                for c in &o.cases {
                    let po = self.lower_pat(c, place, rty)?;
                    if !po.binds.is_empty() {
                        return unsup("or-pattern with bindings", tokens_str(p));
                    }
                    alts.push(MExpr::and_all(po.conds));
                }
                let mut it = alts.into_iter();
                let first = it.next().unwrap_or(MExpr::Bool(false));
                out.conds.push(it.fold(first, |a, b| MExpr::bin(a, BinOp::Or, b)));
            }
            Pat::Path(pp) => {
                let segs: Vec<String> = pp.path.segments.iter().map(|s| s.ident.to_string()).collect();
                let last = segs.last().cloned().unwrap_or_default();
                if segs.len() == 1 && last == "None" {
                    out.conds.push(MExpr::bin(place.clone(), BinOp::Is, MExpr::NoneLit));
                    return Ok(out);
                }
                if let Some(en) = self.enum_of_path(&segs) {
                    return self.variant_pat(&en, &last, None, place);
                }
                if segs.len() == 2 && (self.syms.structs.contains_key(&segs[0]) || segs[0] == "Self") {
                    // associated const
                    let v = self.lower_expr(&syn::Expr::Path(pp.clone()))?;
                    out.conds.push(MExpr::bin(place.clone(), BinOp::Eq, v));
                    return Ok(out);
                }
                if segs.len() == 1 && self.syms.consts.contains(&last) {
                    out.conds.push(MExpr::bin(place.clone(), BinOp::Eq, MExpr::name(&last)));
                    return Ok(out);
                }
                return unsup("pattern path to unknown enum/const", tokens_str(p));
            }
            Pat::TupleStruct(ts) => {
                let segs: Vec<String> = ts.path.segments.iter().map(|s| s.ident.to_string()).collect();
                let last = segs.last().cloned().unwrap_or_default();
                if segs.len() == 1 && last == "Some" {
                    out.conds.push(MExpr::bin(place.clone(), BinOp::IsNot, MExpr::NoneLit));
                    let inner_t = rty.and_then(|t| if rty_last(t).as_deref() == Some("Option") { rty_args(t).into_iter().next() } else { None });
                    let inner = MExpr::method(place.clone(), "value", vec![]);
                    if ts.elems.len() != 1 {
                        return unsup("Some(..) pattern arity", tokens_str(p));
                    }
                    out.extend(self.lower_pat(&ts.elems[0], &inner, inner_t.as_ref())?);
                    return Ok(out);
                }
                if segs.len() == 1 && (last == "Ok" || last == "Err") {
                    return unsup("Ok/Err pattern (Result is lowered to `raises`)", tokens_str(p));
                }
                if let Some(en) = self.enum_of_path(&segs) {
                    let elems: Vec<&Pat> = ts.elems.iter().collect();
                    return self.variant_pat(&en, &last, Some(VariantPats::Tuple(elems)), place);
                }
                // tuple struct destructuring
                let sname = if last == "Self" { self.ts.self_name.clone().unwrap_or_default() } else { last.clone() };
                if let Some(si) = self.syms.structs.get(&sname).cloned() {
                    for (i, e) in ts.elems.iter().enumerate() {
                        if matches!(e, Pat::Rest(_)) {
                            break;
                        }
                        let ft = si.fields.get(i).map(|f| f.1.clone());
                        let sub = MExpr::attr(place.clone(), &format!("_{}", i));
                        out.extend(self.lower_pat(e, &sub, ft.as_ref())?);
                    }
                    return Ok(out);
                }
                return unsup("tuple-struct pattern on unknown type", tokens_str(p));
            }
            Pat::Struct(sp) => {
                let segs: Vec<String> = sp.path.segments.iter().map(|s| s.ident.to_string()).collect();
                let last = segs.last().cloned().unwrap_or_default();
                if let Some(en) = self.enum_of_path(&segs) {
                    let fields: Vec<(String, &Pat)> = sp
                        .fields
                        .iter()
                        .map(|f| {
                            let n = match &f.member {
                                syn::Member::Named(i) => i.to_string(),
                                syn::Member::Unnamed(i) => i.index.to_string(),
                            };
                            (n, &*f.pat)
                        })
                        .collect();
                    return self.variant_pat(&en, &last, Some(VariantPats::Named(fields)), place);
                }
                let sname = if last == "Self" { self.ts.self_name.clone().unwrap_or_default() } else { last.clone() };
                let si = self.syms.structs.get(&sname).cloned();
                for f in &sp.fields {
                    let n = match &f.member {
                        syn::Member::Named(i) => i.to_string(),
                        syn::Member::Unnamed(i) => i.index.to_string(),
                    };
                    let ft = si.as_ref().and_then(|s| s.fields.iter().find(|x| x.0 == n).map(|x| x.1.clone()));
                    let sub = match &f.member {
                        syn::Member::Named(_) => MExpr::attr(place.clone(), &names::ident(&n)),
                        syn::Member::Unnamed(_) => MExpr::attr(place.clone(), &format!("_{}", n)),
                    };
                    out.extend(self.lower_pat(&f.pat, &sub, ft.as_ref())?);
                }
            }
            Pat::Tuple(t) => {
                let elem_tys: Vec<syn::Type> = match rty.map(strip_ref) {
                    Some(syn::Type::Tuple(tt)) => tt.elems.iter().cloned().collect(),
                    _ => vec![],
                };
                for (i, e) in t.elems.iter().enumerate() {
                    if matches!(e, Pat::Rest(_)) {
                        if i + 1 != t.elems.len() {
                            return unsup("tuple pattern with `..` in the middle", tokens_str(p));
                        }
                        break;
                    }
                    let sub = MExpr::Index(Box::new(place.clone()), vec![MExpr::Int(i.to_string())]);
                    out.extend(self.lower_pat(e, &sub, elem_tys.get(i))?);
                }
            }
            Pat::Slice(_) => return unsup("slice pattern", tokens_str(p)),
            Pat::Rest(_) => {}
            Pat::Const(_) => return unsup("inline const pattern", tokens_str(p)),
            Pat::Macro(_) => return unsup("macro in pattern position", tokens_str(p)),
            _ => return unsup("unsupported pattern syntax", tokens_str(p)),
        }
        Ok(out)
    }

    /// `[Enum, Variant]`, `[Self, Variant]`, `[.., Enum, Variant]` or an
    /// imported variant name -> enum name.
    fn enum_of_path(&self, segs: &[String]) -> Option<String> {
        let last = segs.last()?;
        if segs.len() == 1 {
            return self.variant_imports.get(last).cloned();
        }
        let prev = &segs[segs.len() - 2];
        let owner = if prev == "Self" { self.ts.self_name.clone()? } else { prev.clone() };
        let info = self.syms.enums.get(&owner)?;
        info.variant(last)?;
        Some(owner)
    }

    fn variant_pat(&mut self, en: &str, variant: &str, sub: Option<VariantPats>, place: &MExpr) -> LResult<PatOut> {
        let info = self.syms.enums.get(en).cloned().ok_or_else(|| Unsupported { key: "unknown enum".into(), detail: en.into() })?;
        let shape = info.variant(variant).cloned().ok_or_else(|| Unsupported { key: "unknown enum variant".into(), detail: format!("{}::{}", en, variant) })?;
        let mut out = PatOut::default();
        let vid = names::ident(variant);
        if info.c_like {
            let owner = if self.ts.self_name.as_deref() == Some(en) { "Self" } else { en };
            out.conds.push(MExpr::bin(place.clone(), BinOp::Eq, MExpr::attr(MExpr::name(owner), &vid)));
            return Ok(out);
        }
        out.conds.push(MExpr::method(place.clone(), &format!("isa_{}", variant), vec![]));
        let payload = MExpr::method(place.clone(), &format!("as_{}", variant), vec![]);
        let field_ty = |idx: usize| -> Option<syn::Type> {
            let _ = idx;
            None
        };
        match (sub, shape) {
            (None, _) => {}
            (Some(VariantPats::Tuple(elems)), VariantShape::Tuple(_)) => {
                for (i, e) in elems.iter().enumerate() {
                    if matches!(e, Pat::Rest(_)) {
                        break;
                    }
                    let place = MExpr::attr(payload.clone(), &format!("_{}", i));
                    out.extend(self.lower_pat(e, &place, field_ty(i).as_ref())?);
                }
            }
            (Some(VariantPats::Named(fields)), VariantShape::Struct(_)) => {
                for (n, p) in fields {
                    let place = MExpr::attr(payload.clone(), &names::ident(&n));
                    out.extend(self.lower_pat(p, &place, None)?);
                }
            }
            (Some(VariantPats::Tuple(elems)), VariantShape::Unit) if elems.is_empty() => {}
            _ => return unsup("pattern shape does not match enum variant", format!("{}::{}", en, variant)),
        }
        Ok(out)
    }
}

pub enum VariantPats<'p> {
    Tuple(Vec<&'p Pat>),
    Named(Vec<(String, &'p Pat)>),
}

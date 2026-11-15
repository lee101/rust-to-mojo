//! Macro lowering. Only well-known std macros are understood; everything else
//! (including all `macro_rules!` invocations) is reported as unsupported.

use super::expr::Sink;
use super::types::*;
use super::*;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, Token};

enum FmtPiece {
    Lit(String),
    Arg { arg: ArgRef, debug: bool },
}

enum ArgRef {
    Next,
    Index(usize),
    Name(String),
}

fn parse_format(s: &str) -> Result<Vec<FmtPiece>, String> {
    let mut out = vec![];
    let mut lit = String::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '{' {
            if i + 1 < cs.len() && cs[i + 1] == '{' {
                lit.push('{');
                i += 2;
                continue;
            }
            let mut j = i + 1;
            while j < cs.len() && cs[j] != '}' {
                j += 1;
            }
            if j >= cs.len() {
                return Err("unterminated `{` in format string".into());
            }
            let inner: String = cs[i + 1..j].iter().collect();
            let (name, spec) = match inner.find(':') {
                Some(k) => (inner[..k].to_string(), inner[k + 1..].to_string()),
                None => (inner.clone(), String::new()),
            };
            let debug = match spec.as_str() {
                "" => false,
                "?" => true,
                other => return Err(format!("format spec `{{:{}}}`", other)),
            };
            let arg = if name.is_empty() {
                ArgRef::Next
            } else if let Ok(n) = name.parse::<usize>() {
                ArgRef::Index(n)
            } else {
                ArgRef::Name(name)
            };
            if !lit.is_empty() {
                out.push(FmtPiece::Lit(std::mem::take(&mut lit)));
            }
            out.push(FmtPiece::Arg { arg, debug });
            i = j + 1;
            continue;
        }
        if c == '}' {
            if i + 1 < cs.len() && cs[i + 1] == '}' {
                lit.push('}');
                i += 2;
                continue;
            }
            return Err("unmatched `}` in format string".into());
        }
        lit.push(c);
        i += 1;
    }
    if !lit.is_empty() {
        out.push(FmtPiece::Lit(lit));
    }
    Ok(out)
}

struct MatchesArgs {
    e: Expr,
    pat: syn::Pat,
    guard: Option<Expr>,
}

impl Parse for MatchesArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let e: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
        let guard = if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(MatchesArgs { e, pat, guard })
    }
}

struct VecRepeat {
    elem: Expr,
    n: Expr,
}

impl Parse for VecRepeat {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let elem: Expr = input.parse()?;
        input.parse::<Token![;]>()?;
        let n: Expr = input.parse()?;
        Ok(VecRepeat { elem, n })
    }
}

fn macro_args(mac: &syn::Macro) -> LResult<Vec<Expr>> {
    mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
        .map(|p| p.into_iter().collect())
        .map_err(|e| Unsupported { key: format!("unparseable `{}!` arguments", path_last(&mac.path)), detail: e.to_string() })
}

const LOG_MACROS: &[&str] = &["trace", "debug", "info", "warn", "error", "log"];

impl<'a> FileCtx<'a> {
    /// Build a `String` expression from `format!`-style arguments, skipping
    /// the first `skip` arguments (e.g. the writer in `write!`).
    pub fn lower_format_macro(&mut self, mac: &syn::Macro, skip: usize) -> LResult<MExpr> {
        let pieces = self.format_pieces(mac, skip)?;
        Ok(concat_pieces(pieces))
    }

    /// Lower format args into a list of Writable pieces (string literals and
    /// values).
    fn format_pieces(&mut self, mac: &syn::Macro, skip: usize) -> LResult<Vec<MExpr>> {
        let args = macro_args(mac)?;
        if args.len() <= skip {
            return Ok(vec![]);
        }
        let fmt = match &args[skip] {
            Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => s.value(),
            other => return unsup("format string is not a literal", tokens_str(other)),
        };
        let pieces = parse_format(&fmt).map_err(|e| Unsupported { key: "unsupported format spec".into(), detail: e })?;
        let mut positional = vec![];
        let mut named: HashMap<String, Expr> = HashMap::new();
        for a in &args[skip + 1..] {
            if let Expr::Assign(asg) = a {
                if let Some(n) = super::expr::single_ident(&asg.left) {
                    named.insert(n, (*asg.right).clone());
                    continue;
                }
            }
            positional.push(a.clone());
        }
        let mut next = 0;
        let mut out = vec![];
        for p in pieces {
            match p {
                FmtPiece::Lit(s) => out.push(MExpr::Str(s)),
                FmtPiece::Arg { arg, debug } => {
                    let e: Expr = match arg {
                        ArgRef::Next => {
                            let e = positional.get(next).cloned();
                            next += 1;
                            e.ok_or_else(|| Unsupported { key: "format argument count mismatch".into(), detail: fmt.clone() })?
                        }
                        ArgRef::Index(i) => positional.get(i).cloned().ok_or_else(|| Unsupported { key: "format argument count mismatch".into(), detail: fmt.clone() })?,
                        ArgRef::Name(n) => match named.get(&n) {
                            Some(e) => e.clone(),
                            None => syn::parse_str::<Expr>(&n).map_err(|_| Unsupported { key: "bad inline format argument".into(), detail: n.clone() })?,
                        },
                    };
                    let t = self.type_of(&e);
                    let v = self.lower_expr(&e)?;
                    if debug && !t.as_ref().map(|t| is_prim_rty(t)).unwrap_or(false) {
                        self.todo("`{:?}` Debug formatting lowered to repr() (output format differs)", fmt.clone());
                        out.push(MExpr::call_named("repr", vec![v]));
                    } else {
                        out.push(v);
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn vec_elems(&mut self, mac: &syn::Macro) -> LResult<Option<Vec<MExpr>>> {
        if mac.parse_body::<VecRepeat>().is_ok() {
            return Ok(None);
        }
        let args = macro_args(mac)?;
        let mut v = vec![];
        for a in &args {
            let x = self.lower_expr(a)?;
            v.push(self.maybe_transfer(a, x));
        }
        Ok(Some(v))
    }

    pub fn lower_macro_expr(&mut self, mac: &syn::Macro, exp: Option<&MType>) -> LResult<MExpr> {
        let name = path_last(&mac.path);
        match name.as_str() {
            "vec" => {
                if let Ok(r) = mac.parse_body::<VecRepeat>() {
                    let e = self.lower_expr(&r.elem)?;
                    let n = self.lower_expr(&r.n)?;
                    let f = self.rt("vec", "vec_repeat");
                    return Ok(MExpr::call(f, vec![e, n]));
                }
                let elems = self.vec_elems(mac)?.unwrap_or_default();
                let list_exp = exp.filter(|t| t.is_named("List") && !super::expr::contains_infer(t));
                if elems.is_empty() {
                    return match list_exp {
                        Some(t) => Ok(MExpr::call(MExpr::Type(t.clone()), vec![])),
                        None => unsup("empty `vec![]` without element type", tokens_str(mac)),
                    };
                }
                if list_exp.is_some() {
                    return Ok(MExpr::List(elems));
                }
                // Hoist into a typed list literal: `var __v: List[_] = [...]`
                let t = self.fresh("vec");
                self.f.pending.push(MStmt::Var {
                    kind: BindKind::Var,
                    name: t.clone(),
                    ty: Some(MType::generic("List", vec![MType::Infer])),
                    init: Some(MExpr::List(elems)),
                });
                Ok(MExpr::Transfer(Box::new(MExpr::name(t))))
            }
            "format" => self.lower_format_macro(mac, 0),
            "anyhow" | "format_err" => {
                let s = self.lower_format_macro(mac, 0)?;
                Ok(MExpr::call_named("Error", vec![s]))
            }
            "matches" => {
                let m: MatchesArgs = mac.parse_body().map_err(|e| Unsupported { key: "unparseable `matches!`".into(), detail: e.to_string() })?;
                let rty = self.type_of(&m.e);
                let place = {
                    let v = self.lower_expr(&m.e)?;
                    match &m.e {
                        Expr::Path(_) | Expr::Field(_) => v,
                        _ => {
                            let t = self.fresh("m");
                            self.f.pending.push(MStmt::Var { kind: BindKind::Var, name: t.clone(), ty: None, init: Some(v) });
                            MExpr::name(t)
                        }
                    }
                };
                self.push_scope();
                let r = (|| -> LResult<MExpr> {
                    let po = self.lower_pat(&m.pat, &place, rty.as_ref())?;
                    for b in &po.binds {
                        self.bind_expr(&b.rust, b.expr.clone(), b.rty.clone());
                    }
                    let mut conds = po.conds;
                    if let Some(g) = &m.guard {
                        conds.push(self.lower_expr(g)?);
                    }
                    Ok(MExpr::and_all(conds))
                })();
                self.pop_scope();
                r
            }
            "panic" | "unreachable" | "todo" | "unimplemented" => {
                let msg = self.panic_message(mac, &name)?;
                self.import("std.os", "abort");
                Ok(MExpr::call_named("abort", vec![msg]))
            }
            "write" | "writeln" => {
                let w = self.fmt_writer_arg(mac)?;
                let mut pieces = self.format_pieces(mac, 1)?;
                if name == "writeln" {
                    pieces.push(MExpr::Str("\n".into()));
                }
                Ok(MExpr::method(w, "write", pieces))
            }
            other => unsup(format!("macro `{}!`", other), tokens_str(mac)),
        }
    }

    fn fmt_writer_arg(&mut self, mac: &syn::Macro) -> LResult<MExpr> {
        let args = macro_args(mac)?;
        let Some(first) = args.first() else {
            return unsup("`write!` without arguments", tokens_str(mac));
        };
        let Some(n) = super::expr::single_ident(first) else {
            return unsup("`write!` into a non-formatter target", tokens_str(mac));
        };
        if self.f.fmt_writer.as_deref() != Some(n.as_str()) {
            return unsup("`write!` outside a Display/Debug impl", tokens_str(mac));
        }
        Ok(MExpr::name("writer"))
    }

    fn panic_message(&mut self, mac: &syn::Macro, name: &str) -> LResult<MExpr> {
        let args = macro_args(mac)?;
        if args.is_empty() {
            let m = match name {
                "unreachable" => "internal error: entered unreachable code",
                "todo" => "not yet implemented",
                "unimplemented" => "not implemented",
                _ => "explicit panic",
            };
            return Ok(MExpr::Str(m.to_string()));
        }
        self.lower_format_macro(mac, 0)
    }

    pub fn lower_macro_stmt(&mut self, mac: &syn::Macro, sink: &Sink, out: &mut Vec<MStmt>) -> LResult<()> {
        let name = path_last(&mac.path);
        let segs: Vec<String> = mac.path.segments.iter().map(|s| s.ident.to_string()).collect();
        match name.as_str() {
            "println" | "print" => {
                let pieces = self.format_pieces(mac, 0)?;
                out.extend(self.take_pending());
                let mut args: Vec<MArgVal> = if pieces.is_empty() { vec![] } else { vec![MArgVal::pos(concat_pieces(pieces))] };
                if name == "print" {
                    args.push(MArgVal::kw("end", MExpr::Str(String::new())));
                }
                out.push(MStmt::Expr(MExpr::Call(Box::new(MExpr::name("print")), args)));
                Ok(())
            }
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" | "debug_assert_eq" | "debug_assert_ne" => {
                let args = macro_args(mac)?;
                let (cond, n_used, text) = match name.trim_start_matches("debug_") {
                    "assert" => {
                        let c = args.first().ok_or_else(|| Unsupported { key: "empty assert!".into(), detail: String::new() })?;
                        (self.lower_expr(c)?, 1, tokens_str(c))
                    }
                    k => {
                        if args.len() < 2 {
                            return unsup("malformed assert_eq!", tokens_str(mac));
                        }
                        let a = self.lower_expr(&args[0])?;
                        let b = self.lower_expr(&args[1])?;
                        let op = if k == "assert_eq" { BinOp::Eq } else { BinOp::Ne };
                        (MExpr::bin(a, op, b), 2, format!("{} {} {}", tokens_str(&args[0]), if k == "assert_eq" { "==" } else { "!=" }, tokens_str(&args[1])))
                    }
                };
                let msg = if args.len() > n_used {
                    // custom message: format!-style tail
                    let tail: Vec<String> = args[n_used..].iter().map(|a| quote::quote!(#a).to_string()).collect();
                    let m: syn::Macro = syn::parse_str(&format!("format!({})", tail.join(", "))).map_err(|e| Unsupported { key: "assert message".into(), detail: e.to_string() })?;
                    self.lower_format_macro(&m, 0)?
                } else {
                    MExpr::Str(format!("assertion failed: {}", text))
                };
                out.extend(self.take_pending());
                if name.starts_with("debug_") {
                    out.push(MStmt::Expr(MExpr::call_named("debug_assert", vec![cond, msg])));
                } else {
                    self.import("std.os", "abort");
                    out.push(MStmt::If {
                        arms: vec![IfArm { cond: MExpr::not(cond), body: vec![MStmt::Expr(MExpr::call_named("abort", vec![msg]))] }],
                        else_body: None,
                    });
                }
                Ok(())
            }
            "bail" if self.f.raises => {
                let s = self.lower_format_macro(mac, 0)?;
                out.extend(self.take_pending());
                out.push(MStmt::Raise(MExpr::call_named("Error", vec![s])));
                Ok(())
            }
            "write" | "writeln" if matches!(sink, Sink::Discard | Sink::Return) => {
                let v = self.lower_macro_expr(mac, None)?;
                out.extend(self.take_pending());
                out.push(MStmt::Expr(v));
                Ok(())
            }
            n if LOG_MACROS.contains(&n) && (segs.len() == 1 || segs[0] == "log") => {
                self.todo("logging macro dropped (no Mojo logging facade chosen)", format!("{}!", n));
                out.push(MStmt::Todo { reason: format!("dropped `{}!` logging call", n), rust: tokens_str(mac) });
                Ok(())
            }
            _ => {
                let v = self.lower_macro_expr(mac, None)?;
                let e = syn::Expr::Verbatim(proc_macro2::TokenStream::new());
                let _ = e;
                out.extend(self.take_pending());
                match sink {
                    Sink::Discard => {
                        if !matches!(v, MExpr::Name(_) | MExpr::Str(_)) {
                            out.push(MStmt::Expr(v));
                        }
                    }
                    Sink::Return => {
                        if self.f.ret_mty.is_some() {
                            out.push(MStmt::Return(Some(v)));
                        } else {
                            out.push(MStmt::Expr(v));
                        }
                    }
                    Sink::Assign(t) => out.push(MStmt::Assign(t.clone(), v)),
                }
                Ok(())
            }
        }
    }
}

/// Join Writable pieces into one `String` expression.
fn concat_pieces(pieces: Vec<MExpr>) -> MExpr {
    if pieces.is_empty() {
        return MExpr::call_named("String", vec![]);
    }
    let mut parts: Vec<MExpr> = vec![];
    for p in pieces {
        match (parts.last_mut(), p) {
            (Some(MExpr::Str(a)), MExpr::Str(b)) => a.push_str(&b),
            (_, MExpr::Str(s)) => parts.push(MExpr::Str(s)),
            (_, v) => parts.push(MExpr::call_named("String", vec![v])),
        }
    }
    if parts.len() == 1 {
        return match parts.pop().unwrap() {
            MExpr::Str(s) => MExpr::call_named("String", vec![MExpr::Str(s)]),
            v => v,
        };
    }
    // Ensure the chain starts with a String so `+` is String concatenation.
    let mut it = parts.into_iter();
    let first = match it.next().unwrap() {
        MExpr::Str(s) => MExpr::call_named("String", vec![MExpr::Str(s)]),
        v => v,
    };
    it.fold(first, |a, b| MExpr::bin(a, BinOp::Add, b))
}

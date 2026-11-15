//! `#[derive(...)]` lowering for structs.
//!
//! Mojo 1.0's `Equatable`, `Hashable` and `Writable` have reflection-based
//! default implementations, so `PartialEq`/`Eq`/`Hash`/`Debug` become plain
//! conformances. `Default`, `PartialOrd`/`Ord` and the derive_more operator
//! derives generate explicit methods.

use super::items::self_arg;
use super::*;

impl<'a> FileCtx<'a> {
    /// Conform `st` to `tr`, conditionally on generic params that lack it.
    fn conform(&mut self, st: &mut MStruct, tr: &str, generics: &[String], param_traits: &HashMap<String, Vec<String>>) {
        let implied = |have: &Vec<String>, t: &str| -> bool {
            have.iter().any(|h| h == t || (t == "Copyable" && h == "ImplicitlyCopyable"))
        };
        let missing: Vec<&String> = generics.iter().filter(|g| !param_traits.get(*g).map(|h| implied(h, tr)).unwrap_or(false)).collect();
        if missing.is_empty() {
            st.add_trait(tr);
        } else {
            let conds: Vec<String> = missing.iter().map(|g| format!("conforms_to({}, {})", g, tr)).collect();
            st.add_trait(&format!("{} where {}", tr, conds.join(" and ")));
        }
    }

    pub fn apply_struct_derives(
        &mut self,
        st: &mut MStruct,
        derives: &[String],
        fields: &[(String, MType)],
        generics: &[String],
        param_traits: &HashMap<String, Vec<String>>,
    ) -> LResult<()> {
        let has = |d: &str| derives.iter().any(|x| x == d);
        if has("Copy") {
            self.conform(st, "ImplicitlyCopyable", generics, param_traits);
        } else if has("Clone") {
            self.conform(st, "Copyable", generics, param_traits);
        }
        if has("PartialEq") || has("Eq") {
            self.conform(st, "Equatable", generics, param_traits);
        }
        if has("Hash") {
            self.conform(st, "Hashable", generics, param_traits);
        }
        if has("Debug") {
            self.conform(st, "Writable", generics, param_traits);
            st.comments.push("NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)".into());
        }
        let generic_field = |t: &MType| crate::printer::type_str(t).contains("Self.");
        for d in derives {
            match d.as_str() {
                "Clone" | "Copy" | "PartialEq" | "Eq" | "Hash" | "Debug" => {}
                "Default" => {
                    if fields.is_empty() {
                        st.add_trait("Defaultable");
                        continue; // @fieldwise_init already provides `__init__(out self)`
                    }
                    let mut body = vec![];
                    for (n, t) in fields {
                        if generic_field(t) {
                            for g in generics {
                                if crate::printer::type_str(t).contains(&format!("Self.{}", g)) && !param_traits.get(g).map(|h| h.iter().any(|x| x == "Defaultable")).unwrap_or(false) {
                                    self.todo("derive(Default) on a generic field whose parameter is not Defaultable", format!("field `{}`", n));
                                }
                            }
                        }
                        let v = self.default_value(t);
                        body.push(MStmt::Assign(MExpr::attr(MExpr::name("self"), n), v));
                    }
                    st.add_trait("Defaultable");
                    st.members.push(MMember::Method(MFunction { name: "__init__".into(), args: vec![self_arg(Conv::Out)], body: Some(body), ..Default::default() }));
                }
                "PartialOrd" | "Ord" => {
                    if st.has_method("__lt__") {
                        continue;
                    }
                    if d == "PartialOrd" && fields.iter().any(|(_, t)| t.is_named("Float32") || t.is_named("Float64")) {
                        self.todo("derive(PartialOrd) over float fields: NaN ordering follows Mojo comparisons", st.name.clone());
                    }
                    st.add_trait("Comparable");
                    // lexicographic __lt__, the rest derived from it
                    let mut lt_body = vec![];
                    for (n, _) in fields {
                        let a = MExpr::attr(MExpr::name("self"), n);
                        let b = MExpr::attr(MExpr::name("other"), n);
                        lt_body.push(MStmt::If {
                            arms: vec![IfArm { cond: MExpr::bin(a.clone(), BinOp::Ne, b.clone()), body: vec![MStmt::Return(Some(MExpr::bin(a, BinOp::Lt, b)))] }],
                            else_body: None,
                        });
                    }
                    lt_body.push(MStmt::Return(Some(MExpr::Bool(false))));
                    let other = MArg { conv: Conv::Default, name: "other".into(), ty: Some(MType::named("Self")), kw_only: false };
                    st.members.push(MMember::Method(MFunction { name: "__lt__".into(), args: vec![self_arg(Conv::Default), other.clone()], ret: RetType::Value(MType::named("Bool")), body: Some(lt_body), ..Default::default() }));
                    let lt = |a: &str, b: &str| MExpr::bin(MExpr::name(a), BinOp::Lt, MExpr::name(b));
                    for (name, e) in [("__le__", MExpr::not(lt("other", "self"))), ("__gt__", lt("other", "self")), ("__ge__", MExpr::not(lt("self", "other")))] {
                        st.members.push(MMember::Method(MFunction { name: name.into(), args: vec![self_arg(Conv::Default), other.clone()], ret: RetType::Value(MType::named("Bool")), body: Some(vec![MStmt::Return(Some(e))]), ..Default::default() }));
                    }
                }
                // derive_more operator derives
                "Add" | "Sub" | "Neg" | "AddAssign" | "SubAssign" | "Mul" | "Div" | "MulAssign" | "DivAssign" => {
                    if fields.is_empty() {
                        self.unsupported_derive(d);
                        continue;
                    }
                    if fields.iter().any(|(_, t)| generic_field(t)) {
                        self.todo(
                            format!("derive({}) on generic struct needs the parameter to support the operator", d),
                            "no Mojo std trait for arithmetic; consider DType parameterisation",
                        );
                    }
                    let (dunder, op, scalar, inplace) = match d.as_str() {
                        "Add" => ("__add__", BinOp::Add, false, false),
                        "Sub" => ("__sub__", BinOp::Sub, false, false),
                        "Mul" => ("__mul__", BinOp::Mul, true, false),
                        "Div" => ("__truediv__", BinOp::Div, true, false),
                        "AddAssign" => ("__iadd__", BinOp::Add, false, true),
                        "SubAssign" => ("__isub__", BinOp::Sub, false, true),
                        "MulAssign" => ("__imul__", BinOp::Mul, true, true),
                        "DivAssign" => ("__itruediv__", BinOp::Div, true, true),
                        _ => ("__neg__", BinOp::Sub, false, false),
                    };
                    let rhs_ty = if scalar {
                        // derive_more Mul/Div: multiply every field by a scalar of the field type
                        let first = &fields[0].1;
                        if fields.iter().any(|(_, t)| t != first) {
                            self.todo(format!("derive({}) with mixed field types", d), st.name.clone());
                            continue;
                        }
                        first.clone()
                    } else {
                        MType::named("Self")
                    };
                    let rhs = |n: &str| if scalar { MExpr::name("rhs") } else { MExpr::attr(MExpr::name("rhs"), n) };
                    if d == "Neg" {
                        let args: Vec<MArgVal> = fields.iter().map(|(n, _)| MArgVal::kw(n.clone(), MExpr::Unary(UnOp::Neg, Box::new(MExpr::attr(MExpr::name("self"), n))))).collect();
                        st.members.push(MMember::Method(MFunction {
                            name: dunder.into(),
                            args: vec![self_arg(Conv::Default)],
                            ret: RetType::Value(MType::named("Self")),
                            body: Some(vec![MStmt::Return(Some(MExpr::Call(Box::new(MExpr::name("Self")), args)))]),
                            ..Default::default()
                        }));
                    } else if inplace {
                        let body: Vec<MStmt> = fields.iter().map(|(n, _)| MStmt::AugAssign(MExpr::attr(MExpr::name("self"), n), op, rhs(n))).collect();
                        st.members.push(MMember::Method(MFunction {
                            name: dunder.into(),
                            args: vec![self_arg(Conv::Mut), MArg { conv: Conv::Default, name: "rhs".into(), ty: Some(rhs_ty), kw_only: false }],
                            body: Some(body),
                            ..Default::default()
                        }));
                    } else {
                        let args: Vec<MArgVal> = fields.iter().map(|(n, _)| MArgVal::kw(n.clone(), MExpr::bin(MExpr::attr(MExpr::name("self"), n), op, rhs(n)))).collect();
                        st.members.push(MMember::Method(MFunction {
                            name: dunder.into(),
                            args: vec![self_arg(Conv::Default), MArg { conv: Conv::Default, name: "rhs".into(), ty: Some(rhs_ty), kw_only: false }],
                            ret: RetType::Value(MType::named("Self")),
                            body: Some(vec![MStmt::Return(Some(MExpr::Call(Box::new(MExpr::name("Self")), args)))]),
                            ..Default::default()
                        }));
                    }
                }
                other => self.unsupported_derive(other),
            }
        }
        Ok(())
    }
}

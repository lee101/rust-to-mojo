//! Mojo pretty-printer for the IR in `ir.rs`.

use crate::ir::*;
use std::fmt::Write;

const INDENT: &str = "    ";

pub fn print_module(m: &MModule) -> String {
    let mut p = Printer::default();
    p.module(m);
    p.out
}

pub fn type_str(t: &MType) -> String {
    let mut s = String::new();
    write_type(&mut s, t);
    s
}

pub fn expr_str(e: &MExpr) -> String {
    let mut s = String::new();
    write_expr(&mut s, e, 0);
    s
}

pub fn signature_str(f: &MFunction) -> String {
    let mut s = String::new();
    write_signature(&mut s, f);
    s
}

#[derive(Default)]
struct Printer {
    out: String,
    depth: usize,
}

impl Printer {
    fn line(&mut self, text: &str) {
        if text.is_empty() {
            self.out.push('\n');
            return;
        }
        for _ in 0..self.depth {
            self.out.push_str(INDENT);
        }
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn blank(&mut self) {
        if self.out.is_empty() || self.out.ends_with("\n\n") {
            return;
        }
        let last = self.out.trim_end().lines().last().unwrap_or("");
        if last.ends_with(':') && !last.trim_start().starts_with('#') {
            return;
        }
        self.out.push('\n');
    }

    fn docstring(&mut self, doc: &Option<String>) {
        if let Some(d) = doc {
            let d = d.replace("\"\"\"", "\\\"\\\"\\\"");
            let d = d.trim_end();
            if !d.contains('\n') {
                self.line(&format!("\"\"\"{}\"\"\"", d.trim()));
            } else {
                let mut lines = d.lines();
                let first = lines.next().unwrap_or("");
                self.line(&format!("\"\"\"{}", first.trim_end()));
                for l in lines {
                    self.line(l.trim_end());
                }
                self.line("\"\"\"");
            }
        }
    }

    fn comments(&mut self, c: &[String]) {
        for l in c {
            for sub in l.lines() {
                self.line(&format!("# {}", sub));
            }
        }
    }

    fn module(&mut self, m: &MModule) {
        if let Some(d) = &m.doc {
            self.docstring(&Some(d.clone()));
            self.blank();
        }
        self.comments(&m.header_comments);
        if !m.header_comments.is_empty() {
            self.blank();
        }
        // Group `from X import a, b` lines.
        let mut grouped: Vec<(String, Vec<String>)> = vec![];
        let mut plain: Vec<String> = vec![];
        for imp in &m.imports {
            match &imp.name {
                None => plain.push(match &imp.alias {
                    Some(a) => format!("import {} as {}", imp.module, a),
                    None => format!("import {}", imp.module),
                }),
                Some(n) => {
                    let entry = match &imp.alias {
                        Some(a) => format!("{} as {}", n, a),
                        None => n.clone(),
                    };
                    if let Some(g) = grouped.iter_mut().find(|g| g.0 == imp.module) {
                        if !g.1.contains(&entry) {
                            g.1.push(entry);
                        }
                    } else {
                        grouped.push((imp.module.clone(), vec![entry]));
                    }
                }
            }
        }
        for l in &plain {
            self.line(l);
        }
        for (module, names) in &grouped {
            // `*` must stand alone
            if names.iter().any(|n| n == "*") {
                self.line(&format!("from {} import *", module));
                let rest: Vec<_> = names.iter().filter(|n| *n != "*").cloned().collect();
                if !rest.is_empty() {
                    self.line(&format!("from {} import {}", module, rest.join(", ")));
                }
            } else {
                self.line(&format!("from {} import {}", module, names.join(", ")));
            }
        }
        if !m.imports.is_empty() {
            self.blank();
        }
        for item in &m.items {
            self.blank();
            self.item(item);
        }
        if !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        while self.out.ends_with("\n\n") {
            self.out.pop();
        }
    }

    fn item(&mut self, item: &MItem) {
        match item {
            MItem::Struct(s) => self.strukt(s),
            MItem::Trait(t) => self.trait_(t),
            MItem::Function(f) => self.function(f),
            MItem::Comptime(c) => self.comptime(c),
            MItem::Stub(s) => self.stub(s),
            MItem::Comment(lines) => self.comments(lines),
        }
    }

    fn stub(&mut self, s: &MStub) {
        self.line(&format!(
            "# TODO(rust-to-mojo): {} `{}` not translated: {}",
            s.kind, s.name, s.reason
        ));
        if let Some(sig) = &s.signature {
            self.line(&format!("# signature hint: {}", sig));
        }
        self.line("# --- original Rust ---");
        for l in s.rust.lines() {
            if l.trim().is_empty() {
                self.line("#");
            } else {
                self.line(&format!("# {}", l));
            }
        }
        self.line("# --- end original Rust ---");
    }

    fn comptime(&mut self, c: &MComptime) {
        self.comments(&c.comments);
        let mut s = String::from("comptime ");
        s.push_str(&c.name);
        if !c.params.is_empty() {
            s.push('[');
            s.push_str(&params_str(&c.params));
            s.push(']');
        }
        if let Some(t) = &c.ty {
            s.push_str(": ");
            write_type(&mut s, t);
        }
        if let Some(v) = &c.value {
            s.push_str(" = ");
            write_expr(&mut s, v, 0);
        }
        self.line(&s);
    }

    fn strukt(&mut self, st: &MStruct) {
        self.comments(&st.comments);
        for d in &st.decorators {
            self.line(&format!("@{}", d));
        }
        let mut head = format!("struct {}", st.name);
        if !st.params.is_empty() {
            head.push('[');
            head.push_str(&params_str(&st.params));
            head.push(']');
        }
        if !st.traits.is_empty() {
            head.push('(');
            head.push_str(&st.traits.join(", "));
            head.push(')');
        }
        head.push(':');
        self.line(&head);
        self.depth += 1;
        self.docstring(&st.doc);
        self.members(&st.members, st.doc.is_some());
        self.depth -= 1;
    }

    fn members(&mut self, members: &[MMember], has_doc: bool) {
        let mut wrote_code = false;
        let mut prev_was_method = false;
        for m in members {
            match m {
                MMember::Field(f) => {
                    if let Some(d) = &f.doc {
                        for l in d.lines() {
                            self.line(&format!("# {}", l.trim_end()).trim_end().to_string());
                        }
                    }
                    let mut s = format!("var {}: ", f.name);
                    write_type(&mut s, &f.ty);
                    self.line(&s);
                    wrote_code = true;
                    prev_was_method = false;
                }
                MMember::Comptime(c) => {
                    if prev_was_method {
                        self.blank();
                    }
                    self.comptime(c);
                    wrote_code = true;
                    prev_was_method = false;
                }
                MMember::AssocType { name, bound } => {
                    self.line(&format!("comptime {}: {}", name, bound));
                    wrote_code = true;
                }
                MMember::Method(f) => {
                    self.blank();
                    self.function(f);
                    wrote_code = true;
                    prev_was_method = true;
                }
                MMember::Stub(s) => {
                    self.blank();
                    self.stub(s);
                    prev_was_method = true;
                }
                MMember::Comment(c) => {
                    self.line(&format!("# {}", c));
                }
            }
        }
        if !wrote_code {
            // Mojo 1.x rejects a body that is only a docstring.
            let _ = has_doc;
            self.line("pass");
        }
    }

    fn trait_(&mut self, t: &MTrait) {
        self.comments(&t.comments);
        let mut head = format!("trait {}", t.name);
        if !t.parents.is_empty() {
            head.push('(');
            head.push_str(&t.parents.join(", "));
            head.push(')');
        }
        head.push(':');
        self.line(&head);
        self.depth += 1;
        self.docstring(&t.doc);
        self.members(&t.members, t.doc.is_some());
        self.depth -= 1;
    }

    fn function(&mut self, f: &MFunction) {
        self.comments(&f.comments);
        for d in &f.decorators {
            self.line(&format!("@{}", d));
        }
        let mut s = String::new();
        write_signature(&mut s, f);
        s.push(':');
        self.line(&s);
        self.depth += 1;
        self.docstring(&f.doc);
        match &f.body {
            None => self.line("..."),
            Some(body) => {
                if body.is_empty() && f.doc.is_none() {
                    self.line("pass");
                } else {
                    self.stmts(body);
                }
            }
        }
        self.depth -= 1;
    }

    fn block(&mut self, body: &[MStmt]) {
        self.depth += 1;
        if body.iter().all(|s| matches!(s, MStmt::Comment(_) | MStmt::Todo { .. })) {
            self.stmts(body);
            self.line("pass");
        } else {
            self.stmts(body);
        }
        self.depth -= 1;
    }

    fn stmts(&mut self, body: &[MStmt]) {
        for s in body {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &MStmt) {
        match s {
            MStmt::Var { kind, name, ty, init } => {
                let mut l = match kind {
                    BindKind::Var => format!("var {}", name),
                    BindKind::Ref => format!("ref {}", name),
                };
                if let Some(t) = ty {
                    l.push_str(": ");
                    write_type(&mut l, t);
                }
                if let Some(e) = init {
                    l.push_str(" = ");
                    write_expr(&mut l, e, 0);
                }
                self.line(&l);
            }
            MStmt::Assign(a, b) => {
                let mut l = String::new();
                write_expr(&mut l, a, 0);
                l.push_str(" = ");
                write_expr(&mut l, b, 0);
                self.line(&l);
            }
            MStmt::AugAssign(a, op, b) => {
                let mut l = String::new();
                write_expr(&mut l, a, 0);
                let _ = write!(l, " {}= ", op.text());
                write_expr(&mut l, b, 0);
                self.line(&l);
            }
            MStmt::Expr(e) => {
                let mut l = String::new();
                write_expr(&mut l, e, 0);
                self.line(&l);
            }
            MStmt::Return(e) => match e {
                None => self.line("return"),
                // `abort` returns `Never`, which does not convert to the
                // function's return type: emit it as a statement.
                Some(e @ MExpr::Call(f, _)) if matches!(&**f, MExpr::Name(n) if n == "abort") => {
                    let mut l = String::new();
                    write_expr(&mut l, e, 0);
                    self.line(&l);
                }
                Some(e) => {
                    let mut l = String::from("return ");
                    write_expr(&mut l, e, 0);
                    self.line(&l);
                }
            },
            MStmt::Raise(e) => {
                let mut l = String::from("raise ");
                write_expr(&mut l, e, 0);
                self.line(&l);
            }
            MStmt::If { arms, else_body } => {
                for (i, arm) in arms.iter().enumerate() {
                    let mut l = String::from(if i == 0 { "if " } else { "elif " });
                    write_expr(&mut l, &arm.cond, 0);
                    l.push(':');
                    self.line(&l);
                    self.block(&arm.body);
                }
                if let Some(e) = else_body {
                    self.line("else:");
                    self.block(e);
                }
            }
            MStmt::While(c, body) => {
                let mut l = String::from("while ");
                write_expr(&mut l, c, 0);
                l.push(':');
                self.line(&l);
                self.block(body);
            }
            MStmt::For { is_ref, targets, iter, body } => {
                let mut l = String::from("for ");
                if *is_ref {
                    l.push_str("ref ");
                }
                l.push_str(&targets.join(", "));
                l.push_str(" in ");
                write_expr(&mut l, iter, 0);
                l.push(':');
                self.line(&l);
                self.block(body);
            }
            MStmt::Break => self.line("break"),
            MStmt::Continue => self.line("continue"),
            MStmt::Pass => self.line("pass"),
            MStmt::Comment(c) => {
                for l in c.lines() {
                    self.line(&format!("# {}", l));
                }
            }
            MStmt::NestedFn(f) => self.function(f),
            MStmt::Todo { reason, rust } => {
                self.line(&format!("# TODO(rust-to-mojo): {}", reason));
                for l in rust.lines() {
                    self.line(&format!("# {}", l));
                }
            }
        }
    }
}

fn params_str(ps: &[MParam]) -> String {
    ps.iter()
        .map(|p| match &p.bound {
            Some(b) => format!("{}: {}", p.name, b),
            None => p.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_signature(s: &mut String, f: &MFunction) {
    s.push_str("def ");
    s.push_str(&f.name);
    if !f.params.is_empty() {
        s.push('[');
        s.push_str(&params_str(&f.params));
        s.push(']');
    }
    s.push('(');
    let mut parts = vec![];
    for a in &f.args {
        if a.kw_only {
            parts.push("*".to_string());
        }
        let mut p = String::new();
        match a.conv {
            Conv::Default => {}
            Conv::Mut => p.push_str("mut "),
            Conv::Var => p.push_str("var "),
            Conv::Out => p.push_str("out "),
            Conv::Deinit => p.push_str("deinit "),
            Conv::Ref => p.push_str("ref "),
        }
        p.push_str(&a.name);
        if let Some(t) = &a.ty {
            p.push_str(": ");
            write_type(&mut p, t);
        }
        parts.push(p);
    }
    s.push_str(&parts.join(", "));
    s.push(')');
    if !f.captures.is_empty() {
        let _ = write!(s, " {{{}}}", f.captures.join(", "));
    }
    if f.raises {
        s.push_str(" raises");
    }
    match &f.ret {
        RetType::None => {}
        RetType::Value(t) => {
            s.push_str(" -> ");
            write_type(s, t);
        }
        RetType::Ref(origin, t) => {
            let _ = write!(s, " -> ref [{}] ", origin);
            write_type(s, t);
        }
    }
    for w in &f.where_clauses {
        let _ = write!(s, " where {}", w);
    }
}

pub fn write_type(s: &mut String, t: &MType) {
    match t {
        MType::Named(n, args) => {
            s.push_str(n);
            if !args.is_empty() {
                s.push('[');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    match a {
                        MTypeArg::Type(t) => write_type(s, t),
                        MTypeArg::Expr(e) => write_expr(s, e, 0),
                    }
                }
                s.push(']');
            }
        }
        MType::Tuple(ts) => {
            s.push_str("Tuple[");
            for (i, t) in ts.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                write_type(s, t);
            }
            s.push(']');
        }
        MType::Func(args, ret) => {
            s.push_str("def(");
            for (i, t) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                write_type(s, t);
            }
            s.push(')');
            if let Some(r) = ret {
                s.push_str(" -> ");
                write_type(s, r);
            }
        }
        MType::Some(traits) => {
            let _ = write!(s, "Some[{}]", traits.join(" & "));
        }
        MType::Infer => s.push('_'),
    }
}

fn expr_prec(e: &MExpr) -> u8 {
    match e {
        MExpr::Ternary(..) => 1,
        MExpr::Bin(_, op, _) => op.prec(),
        MExpr::Unary(UnOp::Not, _) => 4,
        MExpr::Unary(_, _) => 12,
        MExpr::Transfer(_) => 14,
        MExpr::Int(v) | MExpr::Float(v) if v.starts_with('-') => 12,
        _ => 15,
    }
}

fn escape_str(v: &str) -> String {
    let mut o = String::with_capacity(v.len() + 2);
    o.push('"');
    for c in v.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\0' => o.push_str("\\x00"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\x{:02x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

pub fn write_expr(s: &mut String, e: &MExpr, min_prec: u8) {
    let p = expr_prec(e);
    let paren = p < min_prec;
    if paren {
        s.push('(');
    }
    match e {
        MExpr::Name(n) => s.push_str(n),
        MExpr::Int(v) | MExpr::Float(v) => s.push_str(v),
        MExpr::Str(v) => s.push_str(&escape_str(v)),
        MExpr::Bool(b) => s.push_str(if *b { "True" } else { "False" }),
        MExpr::NoneLit => s.push_str("None"),
        MExpr::Bin(l, op, r) => {
            let op_p = op.prec();
            if op.is_comparison() {
                // never let comparisons chain
                write_expr(s, l, op_p + 1);
                let _ = write!(s, " {} ", op.text());
                write_expr(s, r, op_p + 1);
            } else if *op == BinOp::Pow {
                write_expr(s, l, op_p + 1);
                let _ = write!(s, " {} ", op.text());
                write_expr(s, r, op_p);
            } else {
                write_expr(s, l, op_p);
                let _ = write!(s, " {} ", op.text());
                write_expr(s, r, op_p + 1);
            }
        }
        MExpr::Unary(op, inner) => {
            match op {
                UnOp::Neg => s.push('-'),
                UnOp::Not => s.push_str("not "),
                UnOp::Invert => s.push('~'),
            }
            let ip = if *op == UnOp::Not { 4 } else { 12 };
            write_expr(s, inner, ip);
        }
        MExpr::Call(f, args) => {
            write_expr(s, f, 15);
            s.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                if let Some(n) = &a.name {
                    s.push_str(n);
                    s.push('=');
                }
                write_expr(s, &a.value, 0);
            }
            s.push(')');
        }
        MExpr::Attr(r, a) => {
            write_expr(s, r, 15);
            s.push('.');
            s.push_str(a);
        }
        MExpr::Index(r, idx) => {
            write_expr(s, r, 15);
            s.push('[');
            for (i, x) in idx.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                write_expr(s, x, 0);
            }
            s.push(']');
        }
        MExpr::Slice(a, b) => {
            if let Some(a) = a {
                write_expr(s, a, 0);
            }
            s.push(':');
            if let Some(b) = b {
                write_expr(s, b, 0);
            }
        }
        MExpr::Param(r, args) => {
            write_expr(s, r, 15);
            s.push('[');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                match a {
                    MTypeArg::Type(t) => write_type(s, t),
                    MTypeArg::Expr(e) => write_expr(s, e, 0),
                }
            }
            s.push(']');
        }
        MExpr::Type(t) => write_type(s, t),
        MExpr::Tuple(items) => {
            s.push('(');
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                write_expr(s, x, 0);
            }
            if items.len() == 1 {
                s.push(',');
            }
            s.push(')');
        }
        MExpr::List(items) => {
            s.push('[');
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                write_expr(s, x, 0);
            }
            s.push(']');
        }
        MExpr::Ternary(c, a, b) => {
            write_expr(s, a, 2);
            s.push_str(" if ");
            write_expr(s, c, 2);
            s.push_str(" else ");
            write_expr(s, b, 1);
        }
        MExpr::Transfer(inner) => {
            write_expr(s, inner, 15);
            s.push('^');
        }
    }
    if paren {
        s.push(')');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparisons_do_not_chain() {
        let e = MExpr::bin(
            MExpr::bin(MExpr::name("a"), BinOp::Lt, MExpr::name("b")),
            BinOp::Eq,
            MExpr::name("c"),
        );
        assert_eq!(expr_str(&e), "(a < b) == c");
    }

    #[test]
    fn subtraction_is_left_assoc() {
        let e = MExpr::bin(
            MExpr::name("a"),
            BinOp::Sub,
            MExpr::bin(MExpr::name("b"), BinOp::Sub, MExpr::name("c")),
        );
        assert_eq!(expr_str(&e), "a - (b - c)");
    }

    #[test]
    fn not_binds_loosely() {
        let e = MExpr::not(MExpr::bin(MExpr::name("a"), BinOp::And, MExpr::name("b")));
        assert_eq!(expr_str(&e), "not (a and b)");
        let e2 = MExpr::bin(MExpr::not(MExpr::name("a")), BinOp::And, MExpr::name("b"));
        assert_eq!(expr_str(&e2), "not a and b");
    }

    #[test]
    fn neg_literal_in_mul() {
        let e = MExpr::bin(MExpr::Int("-1".into()), BinOp::Mul, MExpr::name("x"));
        assert_eq!(expr_str(&e), "-1 * x");
        let e = MExpr::Attr(Box::new(MExpr::Int("-1".into())), "x".into());
        assert_eq!(expr_str(&e), "(-1).x");
    }
}

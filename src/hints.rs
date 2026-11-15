//! `--optimize-hints`: a cheap, syntactic pass that marks code where Mojo can
//! plausibly beat the Rust original (SIMD packing, `vectorize`, `parallelize`,
//! `comptime` parameters). Hints are advisory comments plus `hints.json`
//! entries; they never change the translated code. A later agent pass is
//! expected to act on them and benchmark (see README "Pipeline").

use crate::ir::*;
use crate::lower::FileCtx;
use serde::Serialize;
use syn::visit::Visit;

#[derive(Clone, Debug, Serialize)]
pub struct Hint {
    pub file: String,
    pub item: String,
    pub kind: String,
    pub message: String,
}

#[derive(Default)]
struct LoopStats {
    loops: usize,
    max_depth: usize,
    depth: usize,
    range_loops: usize,
    float_ops: usize,
    index_ops: usize,
    float_ops_in_loops: usize,
    index_in_loops: usize,
    calls_in_loops: usize,
}

impl<'ast> Visit<'ast> for LoopStats {
    fn visit_expr_for_loop(&mut self, f: &'ast syn::ExprForLoop) {
        self.loops += 1;
        if matches!(&*f.expr, syn::Expr::Range(_)) {
            self.range_loops += 1;
        }
        self.depth += 1;
        self.max_depth = self.max_depth.max(self.depth);
        syn::visit::visit_expr_for_loop(self, f);
        self.depth -= 1;
    }
    fn visit_expr_while(&mut self, w: &'ast syn::ExprWhile) {
        self.loops += 1;
        self.depth += 1;
        self.max_depth = self.max_depth.max(self.depth);
        syn::visit::visit_expr_while(self, w);
        self.depth -= 1;
    }
    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        use syn::BinOp::*;
        if matches!(b.op, Add(_) | Sub(_) | Mul(_) | Div(_) | AddAssign(_) | SubAssign(_) | MulAssign(_) | DivAssign(_)) {
            self.float_ops += 1;
            if self.depth > 0 {
                self.float_ops_in_loops += 1;
            }
        }
        syn::visit::visit_expr_binary(self, b);
    }
    fn visit_expr_index(&mut self, i: &'ast syn::ExprIndex) {
        self.index_ops += 1;
        if self.depth > 0 {
            self.index_in_loops += 1;
        }
        syn::visit::visit_expr_index(self, i);
    }
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if self.depth > 0 {
            self.calls_in_loops += 1;
        }
        syn::visit::visit_expr_call(self, c);
    }
}

impl<'a> FileCtx<'a> {
    pub fn hint_fn(&mut self, block: &syn::Block, f: &mut MFunction) {
        if !self.opts.optimize_hints {
            return;
        }
        let mut s = LoopStats::default();
        s.visit_block(block);
        let mut hints = vec![];
        if s.range_loops > 0 && s.float_ops_in_loops >= 2 && s.index_in_loops >= 1 {
            hints.push(("vectorize", "range loop with indexed element-wise arithmetic: candidate for `vectorize[simd_width]` over `SIMD[dtype, simd_width_of[dtype]()]` loads/stores"));
        }
        if s.max_depth >= 2 {
            hints.push(("parallelize", "nested loops: if outer iterations are independent, `parallelize[worker](n)` over the outer loop"));
        }
        if s.loops == 0 && s.float_ops >= 8 {
            hints.push(("simd-pack", "straight-line numeric code (>=8 arithmetic ops): pack components into `SIMD[DType.float32, 4]` and operate lane-wise"));
        }
        if s.loops > 0 && s.calls_in_loops == 0 && s.float_ops_in_loops >= 4 {
            hints.push(("autotune", "tight arithmetic loop with no calls: candidate for `comptime` unrolling / autotuned tile sizes"));
        }
        for (k, m) in hints {
            f.comments.push(format!("HINT(rust-to-mojo/opt:{}): {}", k, m));
            self.hints.push(format!("{}\t{}\t{}", f.name, k, m));
        }
    }

    pub fn hint_struct(&mut self, st: &mut MStruct, fields: &[(String, MType)]) {
        if !self.opts.optimize_hints || fields.is_empty() {
            return;
        }
        let first = &fields[0].1;
        let float = first.is_named("Float32") || first.is_named("Float64");
        if float && (2..=4).contains(&fields.len()) && fields.iter().all(|(_, t)| t == first) {
            let n = if fields.len() == 3 { 4 } else { fields.len() };
            let dt = if first.is_named("Float32") { "float32" } else { "float64" };
            let m = format!(
                "{} homogeneous float fields: candidate for `SIMD[DType.{}, {}]` storage so arithmetic dunders become single vector ops",
                fields.len(),
                dt,
                n
            );
            st.comments.push(format!("HINT(rust-to-mojo/opt:simd-struct): {}", m));
            self.hints.push(format!("{}\tsimd-struct\t{}", st.name, m));
        }
        let generic = st.params.iter().any(|p| p.bound.as_deref().map(|b| !b.starts_with("def(")).unwrap_or(false));
        if generic && fields.iter().all(|(_, t)| crate::printer::type_str(t).starts_with("Self.")) {
            let m = "numeric generic over a type parameter: Mojo idiom is `[dtype: DType]` with `Scalar[dtype]` fields, which gives arithmetic without operator traits";
            st.comments.push(format!("HINT(rust-to-mojo/opt:dtype-param): {}", m));
            self.hints.push(format!("{}\tdtype-param\t{}", st.name, m));
        }
    }
}

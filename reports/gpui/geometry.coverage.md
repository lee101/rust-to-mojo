# rust-to-mojo coverage: `geometry`

Target: Mojo 1.0 (Modular 26.5, Aug 2026): def-only functions, comptime, Deinitable, Array, std.* imports  
Tool: rust-to-mojo 0.1.0

| metric | value |
| --- | ---: |
| files | 1 |
| items (total) | 286 |
| translated (clean) | 127 |
| partial (code + TODO notes) | 101 |
| stubbed (TODO + original Rust) | 57 |
| skipped (tests) | 1 |
| emitted % (translated+partial / non-skipped) | 80.0% |
| clean % (translated / non-skipped) | 44.6% |
| internal errors (panics caught) | 0 |
| files with parse errors | 0 |

## Top reasons items were stubbed

| # | reason | items | example |
| ---: | --- | ---: | --- |
| 1 | impl on a std/primitive type (Mojo has no extensions) | 14 | From for f32 |
| 2 | `.into()` without a known target type | 12 | size.width.into() |
| 3 | impl of external trait `JsonSchema` | 6 | JsonSchema for AbsoluteLength |
| 4 | impl of std trait `TryFrom` (no Mojo mapping yet) | 5 | TryFrom for Pixels |
| 5 | impl of std trait `Sum` (no Mojo mapping yet) | 4 | Sum for Pixels |
| 6 | impl of external trait `Deserialize` | 3 | Deserialize for AbsoluteLength |
| 7 | impl of external trait `Serialize` | 3 | Serialize for AbsoluteLength |
| 8 | closure with untyped parameters (needs type inference) | 2 | \| id \| cx.find_display(id) |
| 9 | impl of std trait `Ord` (no Mojo mapping yet) | 2 | Ord for Pixels |
| 10 | impl of std trait `PartialOrd` (no Mojo mapping yet) | 2 | PartialOrd for Pixels |
| 11 | closure without a return type annotation (needs type inference) | 1 | \| \| point.relative_to(&self.origin) |
| 12 | impl of std trait `Hash` (no Mojo mapping yet) | 1 | Hash for Pixels |
| 13 | method signature collides with another impl's method (Mojo needs one definition) | 1 | AbsoluteLength.is_zero |
| 14 | std path with no Mojo mapping | 1 | std::f32::consts::GOLDEN_RATIO |

## Top TODO notes on partially translated items

| # | note | occurrences | example |
| ---: | --- | ---: | --- |
| 1 | dropped bound `Add` (no Mojo std trait) | 34 | `T: Add<T, Output = T>`: operators on generic params need a user trait or DType paramet... |
| 2 | method gated on an extra trait bound (`where conforms_to`) | 31 | Self.T: Comparable |
| 3 | impl specialised on a concrete type argument | 20 | method gated with `where Self.T == Pixels` |
| 4 | dropped bound `Sub` (no Mojo std trait) | 12 | `T: Sub<T, Output = T>`: operators on generic params need a user trait or DType paramet... |
| 5 | derive `Deserialize` has no Mojo equivalent | 10 | #[derive(Deserialize)] dropped |
| 6 | derive `Serialize` has no Mojo equivalent | 10 | #[derive(Serialize)] dropped |
| 7 | dropped bound `Mul` (no Mojo std trait) | 10 | `T: Mul<Rhs, Output = T>`: operators on generic params need a user trait or DType param... |
| 8 | float->int cast: Rust saturates, Mojo conversion of out-of-range values differs | 6 | (self.width.0 * scale_factor).round() as i32 |
| 9 | derive `Refineable` has no Mojo equivalent | 5 | #[derive(Refineable)] dropped |
| 10 | derive `JsonSchema` has no Mojo equivalent | 4 | #[derive(JsonSchema)] dropped |
| 11 | dropped bound `Div` (no Mojo std trait) | 3 | `T: Div<S, Output = T>`: operators on generic params need a user trait or DType paramet... |
| 12 | derive `Neg` has no Mojo equivalent | 2 | #[derive(Neg)] dropped |
| 13 | derive(Add) on generic struct needs the parameter to support the operator | 2 | no Mojo std trait for arithmetic; consider DType parameterisation |
| 14 | derive(Neg) on generic struct needs the parameter to support the operator | 2 | no Mojo std trait for arithmetic; consider DType parameterisation |
| 15 | derive(Sub) on generic struct needs the parameter to support the operator | 2 | no Mojo std trait for arithmetic; consider DType parameterisation |
| 16 | Debug on data enum maps to Variant's Writable (format differs from Rust) | 1 | GridPlacement |
| 17 | `{:?}` Debug formatting lowered to repr() (output format differs) | 1 | Size {{ {:?} × {:?} }} |
| 18 | derive(AddAssign) on generic struct needs the parameter to support the operator | 1 | no Mojo std trait for arithmetic; consider DType parameterisation |
| 19 | derive(Div) on generic struct needs the parameter to support the operator | 1 | no Mojo std trait for arithmetic; consider DType parameterisation |
| 20 | derive(SubAssign) on generic struct needs the parameter to support the operator | 1 | no Mojo std trait for arithmetic; consider DType parameterisation |
| 21 | dropped bound `Neg` (no Mojo std trait) | 1 | `T: Neg<Output = T>`: operators on generic params need a user trait or DType parameteri... |
| 22 | method `.rem_size()` on a value of unknown type passed through | 1 | window.rem_size() |

## File-level notes

| note | files |
| --- | ---: |
| external crate `anyhow` | 1 |
| external crate `derive_more` | 1 |
| external crate `refineable` | 1 |
| external crate `schemars` | 1 |
| external crate `serde` | 1 |
| external crate `taffy` | 1 |

## Per file

| source | items | translated | partial | stubbed | skipped | emitted % |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| geometry.rs | 286 | 127 | 101 | 57 | 1 | 80.0% |

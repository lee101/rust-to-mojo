# rust-to-mojo coverage: `geometry`

Target: Mojo 1.0 (Modular 26.5, Aug 2026): def-only functions, comptime, Deinitable, Array, std.* imports  
Tool: rust-to-mojo 0.1.0

| metric | value |
| --- | ---: |
| files | 1 |
| items (total) | 291 |
| translated (clean) | 196 |
| partial (code + TODO notes) | 44 |
| stubbed (TODO + original Rust) | 50 |
| skipped (tests) | 1 |
| emitted % (translated+partial / non-skipped) | 82.8% |
| clean % (translated / non-skipped) | 67.6% |
| internal errors (panics caught) | 0 |
| files with parse errors | 0 |

## Top reasons items were stubbed

| # | reason | items | example |
| ---: | --- | ---: | --- |
| 1 | impl on a std/primitive type (Mojo has no extensions) | 12 | From for f32 |
| 2 | `.into()` without a known target type | 7 | size.width.into() |
| 3 | impl of external trait `JsonSchema` | 6 | JsonSchema for AbsoluteLength |
| 4 | impl of std trait `TryFrom` (no Mojo mapping yet) | 5 | TryFrom for Pixels |
| 5 | impl of std trait `Sum` (no Mojo mapping yet) | 4 | Sum for Pixels |
| 6 | closure without a return type annotation (needs type inference) | 3 | \| id \| cx.find_display(id) |
| 7 | impl of external trait `Deserialize` | 3 | Deserialize for AbsoluteLength |
| 8 | impl of external trait `Serialize` | 3 | Serialize for AbsoluteLength |
| 9 | impl of std trait `Ord` (no Mojo mapping yet) | 2 | Ord for Pixels |
| 10 | impl of std trait `PartialOrd` (no Mojo mapping yet) | 2 | PartialOrd for Pixels |
| 11 | impl of std trait `Hash` (no Mojo mapping yet) | 1 | Hash for Pixels |
| 12 | method signature collides with another impl's method (Mojo needs one definition) | 1 | AbsoluteLength.is_zero |
| 13 | std path with no Mojo mapping | 1 | std::f32::consts::GOLDEN_RATIO |

## Top TODO notes on partially translated items

| # | note | occurrences | example |
| ---: | --- | ---: | --- |
| 1 | impl specialised on a concrete type argument | 20 | method gated with `where Self.T == Pixels` |
| 2 | derive `Deserialize` has no Mojo equivalent | 10 | #[derive(Deserialize)] dropped |
| 3 | derive `Serialize` has no Mojo equivalent | 10 | #[derive(Serialize)] dropped |
| 4 | float->int cast: Rust saturates, Mojo conversion of out-of-range values differs | 6 | (self.width.0 * scale_factor).round() as i32 |
| 5 | operator on a generic parameter with a different operand type | 6 | `self.origin * rhs.clone()`: mojo_rt.ops only covers `Output = Self` operators |
| 6 | derive `Refineable` has no Mojo equivalent | 5 | #[derive(Refineable)] dropped |
| 7 | derive `JsonSchema` has no Mojo equivalent | 4 | #[derive(JsonSchema)] dropped |
| 8 | derive `Neg` has no Mojo equivalent | 2 | #[derive(Neg)] dropped |
| 9 | dropped bound `Mul` (no Mojo std trait) | 2 | `T: Mul<Rhs, Output = Rhs>`: operators on generic params need a user trait or DType par... |
| 10 | extension impl lowered to a free function (Mojo has no extensions) | 2 | impl Half for i32 -> Half_i32_half(self, ...); method-syntax call sites are not rewritten |
| 11 | Debug on data enum maps to Variant's Writable (format differs from Rust) | 1 | GridPlacement |
| 12 | `{:?}` Debug formatting lowered to repr() (output format differs) | 1 | Size {{ {:?} × {:?} }} |
| 13 | method `.rem_size()` on a value of unknown type passed through | 1 | window.rem_size() |

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
| geometry.rs | 291 | 196 | 44 | 50 | 1 | 82.8% |

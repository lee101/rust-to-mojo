# Hand review: `crates/gpui/src/geometry.rs` → `geometry.mojo`

Reviewer: the author of rust-to-mojo, reading the generated file
([geometry.mojo](geometry.mojo)) against the Rust source, at the zed revision in
[SOURCE.txt](SOURCE.txt). Nothing here has been compiled (no Mojo toolchain was
available), so "right" means *matches the Mojo 1.0 docs and keeps the Rust
semantics*, and "wrong" lists the defects found by reading.

Coverage numbers for this file are in [geometry.coverage.json](geometry.coverage.json)
and the summary table in [crate.coverage.md](crate.coverage.md).

## What is right

**Struct shapes and derives.**
- `Point<T>`, `Size<T>`, `Bounds<T>`, `Edges<T>`, `Corners<T>` become `@fieldwise_init`
  structs with `Self.T` fields, as Mojo 1.0 requires.
- The Rust bound `T: Clone + Debug + Default + PartialEq` maps to
  `Copyable & Writable & Defaultable & Equatable & Deinitable`.
- `derive(Copy)` on a generic struct becomes the *conditional* conformance
  `ImplicitlyCopyable where conforms_to(T, ImplicitlyCopyable)`. That is exactly the
  Rust semantics, since the derive adds `T: Copy`.
- `derive(Default)` produces `__init__(out self)` that default-constructs each field.
- `derive_more`'s `Add/Sub/Neg/AddAssign/SubAssign` become field-wise
  dunders, each with a TODO saying the generic parameter must support the operator.

**Newtypes.**
- `Pixels(pub f32)`, `ScaledPixels`, `DevicePixels` and `Rems` become single-field
  structs (`_0`).
- The many operator impls merge into one struct body as overloaded dunders:
  `__mul__(Float32)` / `__mul__(Int)`, `__truediv__(Float32)` / `__truediv__(Self)`.
- The reflected impls (`impl Mul<Pixels> for f32`, `impl Mul<Pixels> for usize`)
  become `__rmul__(self, lhs: Float32)`. The bodies are rewritten correctly:
  Rust's `self` (the f32) becomes `lhs`, and Rust's `rhs` (the Pixels) becomes Mojo `self`.
- `Rem for Pixels` goes through `mojo_rt.num.rem`, so the sign of `%` follows Rust
  (dividend), not Mojo (divisor).
- `Pixels::round` uses `round_half_away` (Rust's ties-away-from-zero), and
  `f32::max` uses the NaN-ignoring `fmax`.

**Specialised impls.** `impl Point<Pixels> { fn scale(...) -> Point<ScaledPixels> }`
becomes a method on the generic struct gated with `where Self.T == Pixels`, which is
the 1.0 replacement for `self: Point[Pixels]`. Every such method carries a TODO.

**Enums.**
- `Axis`, `Anchor`, `Corner` and `GridLocation`-style fieldless enums become `_tag`
  structs with `comptime` variants, `__eq__`/`__ne__`, `__int__`, and a
  `write_to` that prints the variant name, matching Rust's `Debug` for these.
- `AbsoluteLength`, `DefiniteLength` and `Length` become payload structs plus a
  `Variant` wrapper. `match` on them becomes `isa_X()` / `as_X()` chains with correct
  bindings, and the last unguarded arm becomes `else`, which is sound because Rust
  matches are exhaustive.

**Traits.**
- `Along` (associated type `Unit`, closure-taking `apply_along`) becomes a Mojo
  trait with `associatedtype Unit` and a generic closure parameter
  `F1: def(Self.Unit) -> Self.Unit`.
- The implementations declare `comptime Unit = Self.T`.
- `Half` and `IsZero` are translated and conformed to.
- `Display` impls become `write_to` (`write!(f, "{}px", self.0)` becomes
  `writer.write(self._0, "px")`).

**Expressions.**
- `match` in expression position with simple arms becomes a conditional-expression
  chain (`Bounds::from_anchor_and_size`).
- `..Default::default()` and `Hsla { s: 0., ..*self }` style updates are expanded
  field by field.
- `x as f64` becomes `Float64(x)`. A float→int cast carries a TODO about saturation.
- `f32::consts::PI` becomes `mojo_rt.num.F32_PI`.

**Honesty.** Every untranslated item (serde/JsonSchema impls, `TryFrom`, `Sum`,
`PartialOrd`/`Ord`/`Hash` impls, `impl From<Pixels> for f32`, closures with untyped
parameters, `.into()` with an unknown target) is a commented stub with its original
Rust. I found no silently wrong code among the stubs.

## What is wrong or doubtful (fix-up list)

1. **Generic arithmetic will not compile.** The most important finding.
   `Bounds<T>` methods do `self.origin.x.copy() + self.size.width.copy()` on a
   `Self.T` that has no arithmetic trait, because Mojo has no std `Add` trait. Every
   such method has a TODO, but there are 30+ of them. The right fix is a design
   decision, not translation: either
   (a) a `mojo_rt` `Arithmetic` trait that `Pixels`/`ScaledPixels`/`DevicePixels`/`Rems`
   conform to (built-in scalars cannot, because Mojo has no retroactive conformance), or
   (b) the `--optimize-hints` `dtype-param` suggestion: `Point[dtype: DType]` over
   `Scalar[dtype]`, with unit newtypes kept as thin wrappers.
   (a) matches GPUI's API, and I recommend it for the first fix-up pass.
2. **`where conforms_to(Self.T, Half)` on methods** (31 methods): the syntax is
   plausible, but calling `.half()` on `Self.T` inside the method may still need
   an explicit trait downcast in 1.0. This is unverified.
3. **Inherent and trait methods with the same name.** `AbsoluteLength` has both
   `impl AbsoluteLength { fn is_zero }` and `impl IsZero for AbsoluteLength { fn is_zero }`.
   Rust allows this; Mojo needs one definition. The translator now keeps the first
   definition and stubs the second with "method signature collides with another impl's
   method". A human must pick one. I checked the specialised impls
   (`impl Size<Pixels>` vs `impl Size<DevicePixels>` and so on): none of them produce two
   methods with identical arguments. They differ in arity or argument types, and are
   additionally gated by different `where Self.T == ...` clauses.
4. **`Debug for Pixels` delegates to `Display::fmt(self, f)`.** This is now lowered to
   `self.write_to(writer)`. The first corpus run produced `Display.fmt(self, f)` with
   a TODO, and was fixed during this review.
5. **Copy-vs-borrow of generic args.** `bounds(origin: Point[T], size: Size[T])`
   borrows its arguments (Point is `Copy` in Rust), then passes them into
   `Bounds(origin=origin, size=size)`. That needs an implicit copy, which only works when
   `T` is `ImplicitlyCopyable`. This is fine for every GPUI instantiation, but it is not
   enforced by the signature.
6. **`Self.Output` in reflected operator return types.** This is now resolved through
   the impl's associated types. The first run emitted `-> Self.Output`, which was fixed
   during this review.
7. **`Point(x=..., y=...)` without explicit parameters** inside generic methods
   relies on Mojo inferring `T` from the arguments. That is documented for fieldwise
   init (`Box(42)`), but not for every case here, such as `map[U]` returning `Point[U]`.
8. **Docstrings** keep Rust-specific prose ("`Point<T>`", "implements `Default`").
   This is harmless, but an agent pass should rewrite it.
9. **Untranslated but important:**
   - `PartialOrd`/`Ord` for `Pixels` (stubbed). Comparisons like `a < b` on `Pixels` will
     fail to compile until `__lt__` and friends are generated from `partial_cmp`.
     This is next on the translator roadmap (Ordering support).
   - `Sum for Pixels` and `impl From<Pixels> for f32`. Mojo cannot add a
     constructor to `Float32`. The idiom is `Float32(p)` via an `__float__`-style
     conversion on `Pixels`, or `p._0`. This needs a human decision.
   - `.into()` with a generic target (`size.width.into()`). This needs the type
     inference planned for v0.2.

## Verdict

Most of the file's *data model* is translated well: structs, newtypes, enums, operator
impls, and specialisation. The emitted bodies keep Rust semantics where it matters,
including `%`, rounding, NaN handling, and float casts. What blocks compilation is
item 1 (generic arithmetic), plus the unverified 1.0 syntax points listed in
MAPPING.md. Both are concentrated, well-labelled fix-ups rather than scattered errors.

# Rust → Mojo concept mapping

This is the specification `rust-to-mojo` implements, and the reference for the
agent passes that come after it. Every entry has a status:

| status | meaning |
| --- | --- |
| **direct** | one-to-one construct; translation keeps the semantics |
| **lowered** | different Mojo construct with the same observable behaviour (any caveat is spelled out) |
| **shim** | needs a helper from `mojo_rt/` |
| **unsupported** | no faithful translation yet; the translator emits `# TODO(rust-to-mojo): ...` plus the original Rust and counts the item as *stubbed* |

**Target language: Mojo 1.0** (shipped 11 Aug 2026 as part of Modular 26.5).
This matters because 1.0 broke several things that older guides, and the
25.x syntax most people remember, still show:

* `fn` is now a **compile error**. Only `def` remains. Since 0.26.2, `def`
  is *non-raising by default*, and raising functions say `raises`.
  ([1.0.0b1 notes](https://mojolang.org/releases/v1.0.0b1/), [1.0.0b2](https://mojolang.org/releases/v1.0.0b2/), [0.26.2](https://mojolang.org/releases/v0.26.2/))
* `owned` has been removed (0.26.2). Owned arguments are `var`, and
  destructor/move arguments are `deinit`.
* `alias` is deprecated in favour of `comptime` (0.25.7 added it, 0.26.1 warns).
* `read` has been renamed to `imm`. Immutable borrowing is still the default with no keyword.
* `@register_passable("trivial")` is deprecated (0.26.2) and the effect keyword
  has been removed (1.0b2). Passability is now inferred from the fields.
* `ImplicitlyDestructible` is now `Deinitable`, `__del__` is now `__deinit__`, `InlineArray` is now `Array`,
  `StringSlice` is now `StringSpan`, and `UnsafePointer` is now `Pointer` (with `unsafe_*` operations).
* A list literal `[1, 2, 3]` now builds an `Array`. A `List` needs
  `var x: List[_] = [...]`.
* The std library is imported as `std.*` (`from std.math import sqrt`).
* Inside a struct body, struct parameters must be written `Self.T`. A bare `T` is rejected.
* Structs are `Movable` by default.

The emitted code has **not yet been compiled** (no Mojo toolchain was
available). `scripts/check_mojo.sh` runs `mojo build` over the golden outputs,
and the "unverified syntax" section at the end lists the points most likely to
need a fix.

---

## 1. Items and modules

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| crate | package directory with `__init__.mojo` | direct | crate root → `<crate>/__init__.mojo` |
| `mod foo;` (file) | `foo.mojo`, or `foo/__init__.mojo` if it has children | direct | `#[path]` honoured |
| `mod foo { ... }` (inline) | separate `foo.mojo` | lowered | |
| `pub mod foo` | `from . import foo` in the parent `__init__.mojo` | lowered | 1.0 needs an explicit submodule import for `pkg.foo` access |
| `use crate::a::B` | `from <crate>.a import B` | direct | always absolute imports; compile with `-I <out>` |
| `use super::*` / `self::` | resolved to an absolute module path | lowered | |
| `pub use x::Y` (re-export) | `from <crate>.x import Y` | direct | Mojo re-exports imported names |
| `use std::...` | dropped; mapped names imported where used | lowered | e.g. `Rc` adds `from std.memory import ArcPointer` |
| `use some_crate::...` | `# TODO` header comment, file note | unsupported | the dependency must be ported first |
| `use Enum::*` | none: variants are rewritten as `Enum.V` | lowered | Mojo cannot import struct members |
| visibility (`pub`, `pub(crate)`) | dropped | lowered | Mojo has no visibility. Generated helpers use `_` names |
| `#[cfg(test)]` / `#[test]` | skipped (counted as *skipped*) | n/a | |
| other `#[cfg(...)]` | translated unconditionally, with a TODO | lowered | platform code needs `comptime if` on `std.sys` queries |
| `const X: T = e` | `comptime X: T = e` | direct | |
| `static X: T = e` | `comptime X: T = e` + TODO | lowered | loses address identity |
| `static mut` | none | unsupported | no safe global mutable state |
| `type A<T> = B<T>` | `comptime A[T: ...] = B[T]` | direct | |
| doc comments `///`, `//!` | docstrings | direct | fenced Rust examples are stripped |

## 2. Types

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| `i8..i128`, `u8..u128` | `Int8..Int128`, `UInt8..UInt128` | direct | |
| `isize`, `usize` | `Int` | lowered | Mojo's index type is signed `Int`, which `len()` returns. Unsigned wrap-around at 0 is not reproduced |
| `f32`, `f64` | `Float32`, `Float64` | direct | |
| `bool` | `Bool` | direct | |
| `char` | `Codepoint` | direct | literals become `Codepoint.ord("c")` |
| `String` | `String` | direct | |
| `&str` | `String` (borrowed by default convention) | lowered | `StringSpan[origin]` would avoid a copy (optimisation pass) |
| `(A, B)` | `Tuple[A, B]` / `(a, b)` | direct | `t.0` becomes `t[0]` |
| `()` | no return type / `None` | direct | |
| `[T; N]` | `Array[T, N]` | direct | |
| `&[T]` argument | `List[T]` borrow + TODO | lowered | sub-slices would need `Span[T, origin]` |
| `Vec<T>` | `List[T]` | direct | |
| `HashMap<K,V>` | `Dict[K, V]` | direct | Mojo `Dict` preserves insertion order (Rust's does not, so this is a refinement) |
| `HashSet<T>` | `Set[T]` | direct | |
| `VecDeque<T>` | `Deque[T]` | direct | |
| `BTreeMap`/`BTreeSet` | none | unsupported | no ordered map in std |
| `Option<T>` | `Optional[T]` | direct | see section 8 |
| `Result<T, E>` (return) | `raises` + return type `T` | lowered | see section 9 |
| `Result<T, E>` elsewhere | none | unsupported | a value-level Result shim is planned |
| `Box<T>` | `OwnedPointer[T]` | lowered | |
| `Rc<T>` / `Arc<T>` | `ArcPointer[T]` (+ TODO for `Rc`) | lowered | atomic refcount for `Rc` |
| `RefCell<T>` / `Cell<T>` | `mojo_rt.cell.RefCell` / `Cell` | shim | see section 11 |
| `Range<T>` (as a value) | `mojo_rt.range.RustRange[T]` | shim | |
| `Cow<str>` | `String` + TODO | lowered | |
| `PhantomData<T>` field | dropped + TODO | lowered | |
| `&'a T` struct field (struct has lifetime `'a`) | `var f: Pointer[T, Self.origin_a]` with struct parameter `origin_a: ImmOrigin` (`MutOrigin` for `&'a mut`) | lowered | reads become `self.f[]`; a struct literal stores `Pointer(to=x)` (+TODO: the origin must be inferable at the call site); reassigning the field is stubbed. Other modules write `Foo[_]` for the origin. This is the 1.0 rule: "struct fields can no longer hide `UnsafeAnyOrigin`; add an `Origin` parameter instead" |
| `&'static T` / `&'static [T]` field | `T` / `List[T]` by value | lowered | + TODO: a copy of immutable static data (no address identity) |
| field whose type has no Mojo form (`Rc<dyn Trait>`, `Box<dyn Fn>`, unmapped std types, `&'a [T]`, ...) | `var f: Erased` (`mojo_rt.erased`) | shim | **the struct and every method that does not touch `f` still translate.** Code that reads, writes, destructures or constructs `f` is stubbed with "uses a field erased to `Erased` (cause: ...)". When the receiver's type is unknown, any erased field name counts (conservative) |
| raw pointers | none | unsupported | |
| `dyn Trait` (value, return, generic argument) | none | unsupported | Mojo 1.0 has no existential types |
| `&dyn Trait` / `&mut dyn Trait` argument (crate trait or `Fn*`) | generic parameter (`Some[Trait]` / `F: def(..)`) | lowered | static dispatch is observably the same for the callee; `dyn Any` stays unsupported |
| `impl Trait` (argument) | `Some[Trait]`, or a generic closure parameter | lowered | |
| `impl Trait` (return) | the concrete type of the body's tail | lowered | only when the tail is a non-generic crate struct/enum (literal, constructor, or a crate fn returning one) and there is no early `return`; otherwise unsupported |
| `fn(A) -> B` | `def(A) -> B` | direct | |

## 3. Ownership, borrowing and argument conventions

Rust's ownership model and Mojo's are close relatives. Both have move
semantics, compile-time borrow checking, and deterministic destruction. The
spellings differ.

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| `x: &T` | `x: T` (default immutable borrow) | direct | |
| `x: &mut T` | `mut x: T` | direct | caller passes `x`, no `&mut` |
| `x: T` (Copy type) | `x: T` | lowered | borrow instead of copy; same observable result |
| `x: T` (non-Copy) | `var x: T` | direct | ownership transfer |
| `mut x: T` | `var x: T` | direct | |
| `&self` / `&mut self` | `self` / `mut self` | direct | |
| `self` (by value) | `self` | lowered | borrowed; a body that moves out of `self` fails to compile and becomes a fix-up task |
| `mut self` | `var self` | direct | |
| move of a local on last use | `x^` | direct | inserted when a `var` binding has exactly one use, outside loops |
| `&expr`, `&mut expr` at use sites | `expr` | direct | references are implicit |
| `*r` on a reference | `r` | direct | |
| `*b` on `Box`/`Rc` | `b[]` | direct | |
| `x.clone()` | `x.copy()` | direct | `Copyable.copy()` |
| `Drop::drop` | `def __deinit__(deinit self)` | direct | |
| `Clone::clone` (custom) | `def __init__(out self, *, copy: Self)` | direct | `clone(){ *self }` becomes just a `Copyable` conformance |
| `drop(x)` | `x^` | direct | |
| `std::mem::{swap,replace,take}` | none | unsupported | |

## 4. Lifetimes and origins

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| lifetime parameters on fns | dropped | direct | Mojo infers origins for arguments |
| `fn f(&self) -> &T` | `def f(self) -> ref [self] T` | direct | |
| `fn f(x: &A) -> &B` (one ref arg) | `-> ref [x] B` | direct | |
| several ref args, elided return lifetime | none | unsupported | needs an explicit `origin_of(a, b)` union |
| `'static` bounds | dropped | direct | |
| structs with lifetime parameters | none | unsupported | need origin parameters plus `Pointer` |
| `for<'a>` (HRTB) | none | unsupported | |

## 5. Structs

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point<T: Clone + Debug + Default + PartialEq> { pub x: T, pub y: T }
```
becomes
```mojo
@fieldwise_init
struct Point[T: Copyable & Writable & Defaultable & Equatable & Deinitable](ImplicitlyCopyable where conforms_to(T, ImplicitlyCopyable), Equatable, Writable, Defaultable):
    var x: Self.T
    var y: Self.T

    def __init__(out self):
        self.x = Self.T()
        self.y = Self.T()
```

| Rust | Mojo | status |
| --- | --- | --- |
| named struct | `@fieldwise_init struct` with `var` fields | direct |
| tuple struct `S(A, B)` | fields `_0`, `_1`; `s.0` becomes `s._0` | direct |
| unit struct | empty struct (`pass`) | direct |
| struct literal `S { a, b: e }` | `S(a=a, b=e)` | direct |
| `S { a, ..base }` | `S(a=a, b=base.b, ...)` (fields expanded) | lowered |
| `impl S { ... }` | methods merged into the struct body | lowered (Mojo needs one definition) |
| `fn new(..) -> Self` | `@staticmethod def new(..) -> Self` | direct |
| `impl S<f32> { ... }` | methods with `where Self.T == Float32` | lowered (+TODO; 1.0 type equality is `==`) |
| `impl<T: Extra> S<T>` | methods with `where conforms_to(Self.T, Extra)` | lowered |
| impl in another module of the crate | merged into the struct in the defining module | lowered: lowered with the impl module's `use` context; names the impl module defines are imported; the original module keeps a NOTE. Only for type names unique in the crate |
| recursive struct (even via `Option`/`Box`) | none | unsupported (Mojo rejects it; needs pointers) |

## 6. Traits, generics and derives

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| `trait T { fn f(&self); }` | `trait T: def f(self): ...` | direct | |
| default method body | trait-provided method | direct | |
| `trait A: B + C` | `trait A(B, C)` | direct | |
| associated type `type U;` | `associatedtype U: Movable & Deinitable` | direct | impl side: `comptime U = X` |
| associated const | `comptime X: T` (+ default) | direct | |
| generic trait `trait T<X>` | none | unsupported | Mojo traits take no parameters |
| `fn f<T: Bound>(x: T)` | `def f[T: Bound & Deinitable](x: T)` | direct | `Movable` is added when no copy bound exists |
| `where` clauses on params | merged into the parameter bounds | direct | |
| `const N: usize` | `N: Int` parameter | direct | |
| `F: Fn(A) -> B` | `F: def(A) -> B` | direct | |
| `Clone` / `Copy` bound | `Copyable` / `ImplicitlyCopyable` | direct | |
| `Debug` / `Display` bound | `Writable` | lowered | |
| `PartialEq`/`Eq`, `PartialOrd`/`Ord`, `Hash`, `Default` | `Equatable`, `Comparable`, `Hashable`, `Defaultable` | direct | |
| `Send`, `Sync`, `Sized`, `Unpin` | dropped | direct | |
| `T: Add<Output = T>` (also `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `*Assign`; `Mul<S, Output = T>`) | bound not written; `a + b` on `T` becomes `ops.add(a, b)` | shim | see 6.1. Other heterogeneous bounds (`Add<u8, Output = T>`) stay dropped + TODO |
| blanket `impl<T: X> Y for T` | none | unsupported | |
| `impl LocalTrait for f32` / for a type from another crate | free functions `Trait_f32_method(self: Float32, ...)` | lowered | + TODO: Mojo has no extensions, so method-syntax call sites are not rewritten and the type does not conform to the trait |
| `impl ForeignTrait for Local` | none | unsupported | the trait must be ported |
| `#[derive(Clone)]` / `Copy` | `Copyable` / `ImplicitlyCopyable` | direct | conditional on generic params via `where conforms_to(...)` |
| `#[derive(PartialEq, Eq, Hash)]` | `Equatable`, `Hashable` | direct | 1.0 provides reflection-based defaults |
| `#[derive(Debug)]` | `Writable` | lowered | the reflection-based output format differs from Rust's Debug output |
| `#[derive(Default)]` | `Defaultable` + `__init__(out self)` | direct | |
| `#[derive(PartialOrd, Ord)]` | `Comparable` + lexicographic `__lt__/__le__/__gt__/__ge__` | direct | |
| derive_more `Add, Sub, Neg, Mul, Div, *Assign` | field-wise dunders | direct | |
| `Serialize`, `Deserialize`, `JsonSchema`, `Refineable`, ... | dropped + TODO | unsupported | |

### 6.1 Generic arithmetic (`mojo_rt.ops`)

GPUI's geometry is generic over the unit: `Point<T>`, `Size<T>`, `Bounds<T>` are
instantiated with `Pixels`, `ScaledPixels`, `DevicePixels`, `Rems`, and with `f32`
(54 uses of `Point<f32>` in the crate), and their methods do `self.origin.x + self.size.width`
under `T: Add<Output = T>`. Mojo 1.0 has no std arithmetic trait, and the built-in
scalars cannot be made to conform to a user trait: retroactive conformance
("struct extensions") is only a forum proposal, not documented for 1.0.

| option | how | verdict |
| --- | --- | --- |
| (a) user trait bound | `mojo_rt.ops.RAdd`, ... with `T: RAdd` on the Mojo parameter | rejected as the *only* mechanism: `Point[Float32]` would not instantiate, because `Float32` cannot conform |
| (b) DType parameter | `Point[dtype: DType]` with `Scalar[dtype]` fields | rejected for translation: it changes GPUI's API (`Point<Pixels>` has no DType) and loses the unit newtypes' type safety. Kept as the `--optimize-hints` `dtype-param` suggestion for hand-optimised kernels |
| (c) **compile-time dispatch** (chosen) | `ops.add[T: Copyable](a, b) -> T`: `comptime if conforms_to(T, RAdd)` calls the operator through the trait; `elif T == Float32` (and the other built-in numeric types, `Int` included) rebinds to the scalar and uses the built-in operator; `else: comptime assert False, ...` | works for both user newtypes and built-ins, keeps the generated signatures identical to Rust's, and still rejects a missing operator at compile time (at instantiation instead of declaration) |

What the translator does:

* A bound `T: Add<Output = T>` (or `Add<T, Output = T>`, `AddAssign`, `Neg<Output = T>`, ...)
  is accepted without a TODO and not written on the Mojo parameter.
* `a + b`, `a += b` and `-a` whose operand's Rust type is a generic parameter become
  `ops.add(a, b)`, `a = ops.add(a, b)` and `ops.neg(a)` (`from mojo_rt import ops`).
  `%` uses `ops.rem`, which keeps Rust's truncated remainder for built-in scalars.
* `impl Add for X` with `Rhs = Self` and `Output = Self` (or `derive_more` `Add`/`Sub`/`Neg`)
  adds the conformance `RAdd` (`RSub`, `RMul`, `RDiv`, `RRem`, `RNeg`) to `X`, so a
  translated `Pixels` takes the trait branch of the dispatch. The conformance is only
  added when the generated dunder takes `rhs: Self` by the default convention.
* `T * S` / `T / S` under `T: Mul<S, Output = T>` (S a type parameter or `f32`) becomes
  `ops.mul_rhs(a, b)` / `ops.div_rhs(a, b)`, which handles `S == T` and `S == Float32` with
  `T: RMulF32` / `RDivF32` (added for `impl Mul<f32, Output = Self> for X`). This is GPUI's
  "scale a unit type by f32" pattern.
* Any other heterogeneous operator (`T: Add<u8, Output = T>`) stays a plain operator with a
  TODO: Mojo traits take no type parameters, so `Add<Rhs>` has no general dispatch trait.

### Trait impls that become dunders

| Rust impl | Mojo | status |
| --- | --- | --- |
| `Add`, `Sub`, `Mul`, `Div`, `Rem`, `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr` | `__add__` `__sub__` `__mul__` `__truediv__` `__mod__` `__and__` `__or__` `__xor__` `__lshift__` `__rshift__` | direct |
| `*Assign` variants | `__iadd__` ... | direct |
| `Neg`, `Not` | `__neg__`, `__invert__` | direct |
| `impl Mul<Local> for f32` | `Local.__rmul__(self, lhs: Float32)` | lowered |
| `PartialEq::eq` / `ne` | `__eq__` / `__ne__` (`__ne__` generated if missing) | direct |
| `Display::fmt` | `def write_to[W: Writer](self, mut writer: W)`; `write!(f, ..)` becomes `writer.write(..)` | direct |
| `Debug::fmt` | `write_repr_to` | lowered |
| `Default::default` | `@staticmethod default()` + `__init__(out self)` | direct |
| `From<A>::from` | `@staticmethod from_(value: A) -> Self`; `B::from(a)` becomes `B.from_(a)` | lowered |
| `.into()` with a known target type | `Target.from_(x)` / `Target(x)` | lowered | target from the `let` annotation, the return type, a struct-literal field, an enum-variant payload type, or the parameter type of a crate function/method being called |
| `Index::index` | `__getitem__(self, i) -> ref [self] T` | direct |
| `Hash`, `PartialOrd::partial_cmp`, `Ord::cmp`, `Iterator`, `Deref`, `IndexMut`, `IntoIterator`, `FromStr`, ... | none | unsupported (planned) |

## 7. Enums

**Fieldless enums** become a struct wrapping an `Int` tag, with one `comptime`
member per variant:

```mojo
@fieldwise_init
struct Axis(ImplicitlyCopyable, Equatable, Intable, Writable):
    var _tag: Int
    comptime Vertical = Self(0)
    comptime Horizontal = Self(1)
    def __eq__(self, other: Self) -> Bool: ...
    def __int__(self) -> Int: ...      # makes `x as i32` work through Int(x)
```
Explicit discriminants are preserved, and `derive(Debug)` prints the variant name.
Status: **lowered**.

**Enums with data** become one payload struct per variant plus a wrapper that holds a
`Variant`:

```mojo
struct Shape_Circle(...): var _0: Float32
struct Shape_Rect(...):   var w: Float32; var h: Float32
struct Shape(...):
    var _v: Variant[Shape_Circle, Shape_Rect, Shape_Empty]
    @staticmethod
    def Circle(_0: Float32) -> Self: return Self(Shape_Circle(_0))
    def isa_Circle(self) -> Bool: return self._v.isa[Shape_Circle]()
    def as_Circle(ref self) -> ref [self] Shape_Circle: return self._v[Shape_Circle]
```
`Shape::Circle(r)` becomes `Shape.Circle(r)`, and the unit variant `Shape::Empty` becomes `Shape.Empty()`.
The `isa_V`/`as_V` accessors keep match lowering independent of the enum's type
arguments. Status: **lowered**. Recursive enums are **unsupported**, because Mojo
forbids self-referential structs.

## 8. Pattern matching

Mojo 1.0 has no `match` statement (`match` is a reserved word), so every
pattern is lowered to conditions plus bindings against a *place* expression:

| pattern | condition | bindings |
| --- | --- | --- |
| `_`, `x` | none | `x` → place |
| literal `3` | `place == 3` | |
| `1..=9` | `place >= 1 and place <= 9` | |
| `A \| B` | `cond_A or cond_B` | (unsupported with bindings) |
| `Enum::V` (fieldless) | `place == Enum.V` | |
| `Enum::V(p)` (data) | `place.isa_V()` | sub-pattern against `place.as_V()._0` |
| `S { a, b: p }` | none | `a` → `place.a` |
| `(p, q)` | none | sub-patterns on `place[0]`, `place[1]` |
| `Some(p)` / `None` | `place is not None` / `place is None` | `place.value()` |
| `Ok(p)` / `Err(p)` | none | **unsupported** (Result is `raises`) |
| slice patterns, `box`, `@` with refutable sub-patterns | none | **unsupported** |

`match` becomes an `if`/`elif`/`else` chain. A non-place scrutinee is first bound to
a temporary. The **last unguarded arm becomes `else`**, which is sound
because Rust matches are exhaustive. Guards can use bindings: bindings are
substituted into the guard as their place expressions. A match without a final unguarded arm gets
`else: abort("unreachable")`. In expression position, simple arms become
a chain of conditional expressions (`a if c else b`). When the arms need statements and the expected
type is known, the match is hoisted into a typed temporary. `if let`, `while let`, let-chains
(`if let A = x && c`) and `matches!` use the same machinery. `let`-`else`
is **unsupported**.

Bindings are declared as `ref x = place`, or as `var x = place` for primitives and `mut`
bindings.

## 9. Errors: `Result`, `?`, panics

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| `fn f() -> Result<T, E>` | `def f() raises -> T` | lowered | **error type erased** (TODO on every such fn). Mojo 1.0 has typed `raises E`, which is planned |
| `Ok(v)` in return position | `return v` | direct | |
| `Err(e)` in return position | `raise Error(String(e))` | lowered | |
| `anyhow!(..)` / `bail!(..)` | `Error(<formatted>)` / `raise Error(..)` | lowered | |
| `call()?` in a raising fn | `call()` | direct | only when `call` is itself a translated Result fn |
| `opt?` in an Option fn | `var t = opt; if t is None: return None; ... t.value()` | lowered | |
| `match`/`if let` on `Ok`/`Err` | none | unsupported | the planned lowering is `try`/`except` |
| `panic!`, `unreachable!`, `todo!`, `unimplemented!` | `abort(msg)` (`std.os`) | direct | |
| `assert!`, `assert_eq!` | `if not c: abort(msg)` | direct | always on, like Rust |
| `debug_assert!` | `debug_assert(c, msg)` | direct | |
| `.unwrap()` / `.expect()` on Option | `.value()` | lowered | aborts instead of panicking. The `expect` message is dropped |

## 10. Closures and iterators

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| closure with typed params and return type | nested `def` with a capture list `{imm k}` (`{var k}` for `move`), hoisted before the statement | direct | |
| closure with untyped params | parameter types from the call site | lowered | the callee's parameter is `impl Fn(A) -> R`, `F` with `F: Fn(A) -> R` (inline or `where`), `&dyn Fn(A)` or `Box<dyn Fn(A)>`, in a crate fn/method found by name (+ receiver type, + arity); impl-level parameters are substituted from the receiver's type. `Option<T>` combinators give `T`. `|&x|` binds the pointee. Otherwise unsupported |
| closure without a return annotation | from the callee signature, else the type of a single-expression body or block tail, else `()` when the body is a block with no tail and no `return value` | lowered | |
| `F: Fn(A) -> B` parameter | `F: def(A) -> B` | direct | |
| `F: Fn(&mut A)` | `F: def(mut A)` | direct | the closure gets a `mut a: A` parameter (form as in std's `def(mut Bencher)`) |
| `opt.map(f)`, `and_then`, `unwrap_or_else`, `or_else`, `map_or`, `filter`, `is_some_and`, `is_none_or`, `inspect` | `rt_option.map(opt, f)` ... (`from mojo_rt import option as rt_option`) | shim | |
| `FnMut` | `def(...)` + TODO | lowered | captures need `{mut x}` |
| `Box<dyn Fn>` | none | unsupported | |
| `for x in v` / `v.iter()` / `&v` | `for x in v` | direct | |
| `for x in v.iter_mut()` / `&mut v` | `for ref x in v` | direct | |
| `for i in a..b` / `a..=b` | `for i in range(a, b)` / `range(a, b + 1)` | direct | |
| `.rev()`, `.step_by(n)`, `.enumerate()`, `.zip()` in `for` | `reversed()`, `range(a, b, n)`, `enumerate()`, `zip()` | direct | |
| `for (k, v) in &map` | `for e in d.items(): ref k = e.key; ref v = e.value` | direct | |
| `.chars()` in `for` | `.codepoints()` | direct | 1.0 iterates strings by grapheme by default |
| adapter chains (`map/filter/fold/collect/any/sum/...`) | none | unsupported | the planned lowering is to explicit loops |

## 11. Shared ownership and interior mutability

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| `Rc::new(x)` / `Arc::new(x)` | `ArcPointer(x)` | lowered | |
| `Rc::clone(&r)` / `r.clone()` | `r.copy()` | direct | refcount increment |
| method call through `Rc`/`Box` | `r[].method()` | direct | explicit deref |
| `Weak<T>` | none | unsupported | 1.0 added `WeakPointer` (unverified) |
| `RefCell::new`, `.borrow()`, `.borrow_mut()` | `mojo_rt.cell.RefCell` | shim | **no dynamic borrow flags**: code that would panic with BorrowMutError keeps running (TODO on every use). Mutation through `&self` needs restructuring |
| `Cell::get/set/replace` | `mojo_rt.cell.Cell` | shim | `set` needs `mut` access in Mojo |
| `Mutex<T>`, `RwLock<T>` (parking_lot, std, spin) | `mojo_rt.sync.Mutex[T]` / `RwLock[T]` | shim | **no mutual exclusion** (single-threaded; TODO on every use). `lock()/read()/write()` return a reference; `let g = m.lock();` becomes `ref g = m.lock()`, and std's `.lock().unwrap()` drops the `unwrap`. `try_*` stays unsupported |
| atomics | none | unsupported | |

## 12. Integer and float semantics

| Rust | Mojo | status | notes |
| --- | --- | --- | --- |
| `+ - *` overflow | wraps | lowered | Rust debug builds panic and release builds wrap. Mojo matches release |
| `wrapping_add/sub/mul/neg` | `+ - * -x` | direct | |
| `checked_*` | `mojo_rt.num.checked_*` → `Optional` | shim | |
| `saturating_*` | `mojo_rt.num.saturating_*` | shim | |
| `a / b` (int) | `a / b` | direct | 1.0: integer `/` truncates, like Rust |
| `a % b` | `mojo_rt.num.rem(a, b)` | shim | Mojo `%` takes the divisor's sign. Rust takes the dividend's. User types keep `%` |
| `rem_euclid` / `div_euclid` / `abs_diff` | `mojo_rt.num.*` | shim | |
| `x as T` (numeric) | `T(x)` | direct | |
| float → int `as` | `Int32(x)` + TODO | lowered | Rust saturates and maps NaN to 0. Mojo differs for out-of-range values |
| `!x` (int) / `!b` (bool) | `~x` / `not b` | direct | chosen by the inferred type |
| `f32::max/min` | `mojo_rt.num.fmax/fmin` | shim | Rust ignores NaN |
| `.round()` | `mojo_rt.num.round_half_away` | shim | Rust rounds halfway cases away from zero |
| `.sqrt() .floor() .ceil() .trunc() .sin() ...` | `std.math` functions | direct | |
| `.powi(n)` / `.powf(y)` | `x ** n` / `pow(x, y)` | direct | |
| `.clamp(a, b)`, `.mul_add`, `.hypot`, `.atan2`, `.copysign` | `std.math.clamp/fma/hypot/atan2/copysign` | direct | |
| `.signum()`, `.fract()`, `.recip()`, `.to_radians()`, `.is_nan()` | `mojo_rt.num.*` | shim | |
| `i32::MAX` | `Int32.MAX` | direct | |
| `f32::MAX`, `f32::consts::PI`, ... | `mojo_rt.num.F32_MAX`, `F32_PI`, ... | shim | |

## 13. Macros

| Rust | Mojo | status |
| --- | --- | --- |
| `#[derive(...)]` | see section 6 | direct/lowered |
| `format!` | `String(..) + ".." + String(..)` concatenation | direct (`{}` `{0}` `{name}` `{:?}`) |
| format specs (`{:.2}`, `{:>8}`, `{:x}`, `{:#?}`) | none | unsupported |
| `println!` / `print!` | `print(..)` / `print(.., end="")` | direct |
| `eprintln!`, `dbg!` | none | unsupported |
| `vec![a, b]` | `[a, b]` with a `List` type context, else `var __v: List[_] = [..]` | direct |
| `vec![x; n]` | `mojo_rt.vec.vec_repeat(x, n)` | shim |
| `matches!` | pattern condition | direct |
| `write!`/`writeln!` in `fmt` | `writer.write(..)` | direct |
| `log::debug!` and friends | dropped (inert TODO comment) | lowered |
| `macro_rules!` definitions and their invocations | none | unsupported (expand with `cargo expand` first, or port by hand) |
| proc-macro attributes (other than derive) | ignored | unsupported |

## 14. Unsafe and FFI

| Rust | Mojo | status |
| --- | --- | --- |
| `unsafe fn` / `unsafe {}` | none | unsupported |
| raw pointers | (`Pointer` with `unsafe_load`/`unsafe_store` in 1.0) | unsupported |
| `extern "C" { fn ... }` | (`external_call["name", Ret](args)`) | unsupported |
| `#[no_mangle] extern "C" fn` | (`@export("name") def f(...) abi("C")`) | unsupported |
| `#[repr(C)]` | NOTE comment | lowered (verify layout) |
| `union` | none | unsupported |

GPUI's platform layers (Metal, DirectX, Wayland/X11, CoreText) live almost
entirely in this row. The plan is to port them by hand against Mojo's FFI
(`external_call`, `@export ... abi("C")`, `Pointer`) rather than translate them.

## 15. Async

`async fn`, `.await`, futures and executors are **unsupported**. GPUI's
executor (`executor.rs`, `platform_scheduler.rs`) needs a Mojo design (Mojo
has `async` support, but its runtime differs). That is a manual port.

---

## Unverified Mojo 1.0 syntax (check these first on a real toolchain)

1. `comptime Vertical = Self(0)`: a `comptime` member of struct type inside
   its own struct. The stdlib does this for `DType`, but it is not verified for user structs.
2. `where Self.T == Float32` on methods for specialised impls: the
   [1.0 notes](https://mojolang.org/releases/v1.0.0/) show `def foo(self) where Self.T == Int:`.
3. Conditional conformance syntax on structs: `Equatable where conforms_to(T, Equatable)`.
   This is documented, but combining two conditions with `and` is not.
4. `def as_V(ref self) -> ref [self] Payload`: `ref self` plus a return origin.
5. Closure capture lists `def f(x: Int) {imm k} -> Int:` ([closures](https://mojolang.org/docs/manual/functions/closures/))
   combined with `raises`.
6. `associatedtype U: ...` in traits, and `comptime U = X` to satisfy it in a struct.
7. `for ref x in list:`
8. `Codepoint.ord("c")`, and `std.utils.numerics` as the location of `isnan`/`inf`.
9. `self = Self(...)` inside `__init__(out self)` (used for `Default`/`Clone` bodies).
10. Passing a `Shape_Circle` where a `Variant[...]` field is expected (relies on
    `Variant`'s `@implicit` constructor through `@fieldwise_init`).
11. `@staticmethod` calls on an unbound generic struct (`Point.new(x, y)`) and whether
    the parameters are inferred.
12. `from . import foo` in a package `__init__.mojo` when modules import each other
    (circular imports). Cross-module impl merging adds more of these.
13. `mojo_rt.ops`: `T == Float32` as a compile-time type comparison inside `comptime if`
    (the 1.0 notes show `where Self.T == Int`), refinement of `T` by `conforms_to(T, RAdd)`
    so that `a + b` resolves through the trait (the manual's `represent` example does this
    for `Writable`), `rebind[Float32](a)` on a `T` that is `Float32` only in that branch,
    and `comptime assert False, "..."` in a discarded branch.
14. Calling `Self.T` methods inside a method gated with `where conforms_to(Self.T, X)`.
    The manual documents the clause (`def sort() where conforms_to(Self.T, Comparable):`)
    but not the refinement inside the body.
15. Origin parameter spellings `ImmOrigin` / `MutOrigin` on a struct, `Pointer(to=x)` as the
    constructor after the `UnsafePointer`/`Pointer` merge, and `Foo[_]` to leave a struct's
    origin parameter unbound in a signature.
16. `def(mut T) -> R` as a function type with an argument convention.
17. `self` as an ordinary parameter name of a free function (extension impls).
18. Inference of `U` from the closure's return type in `rt_option.map(opt, f)`.
19. A struct field and a method with the same name (Rust getters such as `fn id(&self)`
    next to `id: u32`) are emitted as is; Mojo may reject the clash.

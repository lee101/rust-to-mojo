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
| `&'a T` struct field | none | unsupported | needs `Pointer[T, origin]` and origin parameters |
| raw pointers | none | unsupported | |
| `dyn Trait` | none | unsupported | Mojo 1.0 has no existential types |
| `impl Trait` (argument) | `Some[Trait]`, or a generic closure parameter | lowered | |
| `impl Trait` (return) | none | unsupported | |
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
| impl in another module | none | unsupported (cross-file merge is planned) |
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
| `Add<Output = T>` and other operator bounds | dropped + TODO | lowered | **no std arithmetic trait in Mojo**. The Mojo idiom is `[dtype: DType]` with `Scalar[dtype]` fields; the optimisation pass flags these |
| blanket `impl<T: X> Y for T` | none | unsupported | |
| `impl LocalTrait for f32` / foreign types | none | unsupported | Mojo has no extensions |
| `impl ForeignTrait for Local` | none | unsupported | the trait must be ported |
| `#[derive(Clone)]` / `Copy` | `Copyable` / `ImplicitlyCopyable` | direct | conditional on generic params via `where conforms_to(...)` |
| `#[derive(PartialEq, Eq, Hash)]` | `Equatable`, `Hashable` | direct | 1.0 provides reflection-based defaults |
| `#[derive(Debug)]` | `Writable` | lowered | the reflection-based output format differs from Rust's Debug output |
| `#[derive(Default)]` | `Defaultable` + `__init__(out self)` | direct | |
| `#[derive(PartialOrd, Ord)]` | `Comparable` + lexicographic `__lt__/__le__/__gt__/__ge__` | direct | |
| derive_more `Add, Sub, Neg, Mul, Div, *Assign` | field-wise dunders | direct | |
| `Serialize`, `Deserialize`, `JsonSchema`, `Refineable`, ... | dropped + TODO | unsupported | |

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
| `.into()` with a known target type | `Target.from_(x)` / `Target(x)` | lowered |
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
| closure with untyped params | none | unsupported | needs type inference (the #1 planned improvement) |
| `F: Fn(A) -> B` parameter | `F: def(A) -> B` | direct | |
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
| `Mutex`, `RwLock`, atomics | none | unsupported | |

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
    (circular imports).

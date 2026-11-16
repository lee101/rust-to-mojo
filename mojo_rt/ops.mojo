"""Generic arithmetic for Rust operator-trait bounds.

Rust code such as

    impl<T: Add<Output = T> + Copy> Bounds<T> {
        fn right(&self) -> T { self.origin.x + self.size.width }
    }

needs `+` on a type parameter. Mojo 1.0 has no std arithmetic trait, and the
built-in scalars (`Float32`, `Int`, ...) cannot be made to conform to a user
trait (there is no documented retroactive conformance / extension mechanism).
So rust-to-mojo lowers an operator on a value whose type is a generic
parameter to a call into this module:

    mojo_rt.ops.add(self.origin.x, self.size.width)

Each function dispatches at compile time:

  1. if `T` conforms to the matching `R*` trait (translated user types such as
     GPUI's `Pixels`, which get the conformance from their `impl Add for Pixels`),
     the operator is called through the trait (`conforms_to` refines `T` inside
     a `comptime if`);
  2. if `T` is one of the built-in numeric types, the value is rebound to that
     type and the built-in operator is used (with Rust semantics for `%`);
  3. otherwise compilation stops with a `comptime assert` naming the operator,
     which is where Rust would have rejected the missing bound.

The Rust bound itself (`T: Add<Output = T>`) is therefore not written on the
Mojo parameter: it is checked when the generic is instantiated, not when it is
declared. Only `Output = Self` operators are covered; heterogeneous operators
(`T: Mul<f32, Output = T>`) stay TODOs in the generated code.

UNVERIFIED Mojo 1.0 points (see MAPPING.md): `T == Float32` as a compile-time
type comparison inside `comptime if`, and refinement of `T` by
`conforms_to(T, RAdd)` so that `a + b` resolves through the trait.
"""

from mojo_rt.num import rem as _scalar_rem


trait RAdd:
    """Rust `Add<Output = Self>`."""

    def __add__(self, rhs: Self) -> Self:
        ...


trait RSub:
    """Rust `Sub<Output = Self>`."""

    def __sub__(self, rhs: Self) -> Self:
        ...


trait RMul:
    """Rust `Mul<Output = Self>`."""

    def __mul__(self, rhs: Self) -> Self:
        ...


trait RDiv:
    """Rust `Div<Output = Self>`."""

    def __truediv__(self, rhs: Self) -> Self:
        ...


trait RRem:
    """Rust `Rem<Output = Self>`."""

    def __mod__(self, rhs: Self) -> Self:
        ...


trait RNeg:
    """Rust `Neg<Output = Self>`."""

    def __neg__(self) -> Self:
        ...


trait RArith(RAdd, RSub, RMul, RDiv, RNeg):
    """The four operators plus negation (a GPUI unit type such as `Pixels`)."""

    pass


@always_inline
def add[T: Copyable & Deinitable](a: T, b: T) -> T:
    """Rust `a + b` where `T: Add<Output = T>`."""
    comptime if conforms_to(T, RAdd):
        return a + b
    elif T == Int:
        var r = rebind[Int](a) + rebind[Int](b)
        return rebind[T](Int(r)).copy()
    elif T == Float32:
        var r = rebind[Float32](a) + rebind[Float32](b)
        return rebind[T](r).copy()
    elif T == Float64:
        var r = rebind[Float64](a) + rebind[Float64](b)
        return rebind[T](r).copy()
    elif T == Float16:
        var r = rebind[Float16](a) + rebind[Float16](b)
        return rebind[T](r).copy()
    elif T == Int8:
        var r = rebind[Int8](a) + rebind[Int8](b)
        return rebind[T](r).copy()
    elif T == Int16:
        var r = rebind[Int16](a) + rebind[Int16](b)
        return rebind[T](r).copy()
    elif T == Int32:
        var r = rebind[Int32](a) + rebind[Int32](b)
        return rebind[T](r).copy()
    elif T == Int64:
        var r = rebind[Int64](a) + rebind[Int64](b)
        return rebind[T](r).copy()
    elif T == UInt8:
        var r = rebind[UInt8](a) + rebind[UInt8](b)
        return rebind[T](r).copy()
    elif T == UInt16:
        var r = rebind[UInt16](a) + rebind[UInt16](b)
        return rebind[T](r).copy()
    elif T == UInt32:
        var r = rebind[UInt32](a) + rebind[UInt32](b)
        return rebind[T](r).copy()
    elif T == UInt64:
        var r = rebind[UInt64](a) + rebind[UInt64](b)
        return rebind[T](r).copy()
    else:
        comptime assert False, "rust-to-mojo: type parameter has no `+` (Rust bound `Add<Output = T>`); conform it to mojo_rt.ops.RAdd"

@always_inline
def sub[T: Copyable & Deinitable](a: T, b: T) -> T:
    """Rust `a - b` where `T: Sub<Output = T>`."""
    comptime if conforms_to(T, RSub):
        return a - b
    elif T == Int:
        var r = rebind[Int](a) - rebind[Int](b)
        return rebind[T](Int(r)).copy()
    elif T == Float32:
        var r = rebind[Float32](a) - rebind[Float32](b)
        return rebind[T](r).copy()
    elif T == Float64:
        var r = rebind[Float64](a) - rebind[Float64](b)
        return rebind[T](r).copy()
    elif T == Float16:
        var r = rebind[Float16](a) - rebind[Float16](b)
        return rebind[T](r).copy()
    elif T == Int8:
        var r = rebind[Int8](a) - rebind[Int8](b)
        return rebind[T](r).copy()
    elif T == Int16:
        var r = rebind[Int16](a) - rebind[Int16](b)
        return rebind[T](r).copy()
    elif T == Int32:
        var r = rebind[Int32](a) - rebind[Int32](b)
        return rebind[T](r).copy()
    elif T == Int64:
        var r = rebind[Int64](a) - rebind[Int64](b)
        return rebind[T](r).copy()
    elif T == UInt8:
        var r = rebind[UInt8](a) - rebind[UInt8](b)
        return rebind[T](r).copy()
    elif T == UInt16:
        var r = rebind[UInt16](a) - rebind[UInt16](b)
        return rebind[T](r).copy()
    elif T == UInt32:
        var r = rebind[UInt32](a) - rebind[UInt32](b)
        return rebind[T](r).copy()
    elif T == UInt64:
        var r = rebind[UInt64](a) - rebind[UInt64](b)
        return rebind[T](r).copy()
    else:
        comptime assert False, "rust-to-mojo: type parameter has no `-` (Rust bound `Sub<Output = T>`); conform it to mojo_rt.ops.RSub"

@always_inline
def mul[T: Copyable & Deinitable](a: T, b: T) -> T:
    """Rust `a * b` where `T: Mul<Output = T>`."""
    comptime if conforms_to(T, RMul):
        return a * b
    elif T == Int:
        var r = rebind[Int](a) * rebind[Int](b)
        return rebind[T](Int(r)).copy()
    elif T == Float32:
        var r = rebind[Float32](a) * rebind[Float32](b)
        return rebind[T](r).copy()
    elif T == Float64:
        var r = rebind[Float64](a) * rebind[Float64](b)
        return rebind[T](r).copy()
    elif T == Float16:
        var r = rebind[Float16](a) * rebind[Float16](b)
        return rebind[T](r).copy()
    elif T == Int8:
        var r = rebind[Int8](a) * rebind[Int8](b)
        return rebind[T](r).copy()
    elif T == Int16:
        var r = rebind[Int16](a) * rebind[Int16](b)
        return rebind[T](r).copy()
    elif T == Int32:
        var r = rebind[Int32](a) * rebind[Int32](b)
        return rebind[T](r).copy()
    elif T == Int64:
        var r = rebind[Int64](a) * rebind[Int64](b)
        return rebind[T](r).copy()
    elif T == UInt8:
        var r = rebind[UInt8](a) * rebind[UInt8](b)
        return rebind[T](r).copy()
    elif T == UInt16:
        var r = rebind[UInt16](a) * rebind[UInt16](b)
        return rebind[T](r).copy()
    elif T == UInt32:
        var r = rebind[UInt32](a) * rebind[UInt32](b)
        return rebind[T](r).copy()
    elif T == UInt64:
        var r = rebind[UInt64](a) * rebind[UInt64](b)
        return rebind[T](r).copy()
    else:
        comptime assert False, "rust-to-mojo: type parameter has no `*` (Rust bound `Mul<Output = T>`); conform it to mojo_rt.ops.RMul"

@always_inline
def div[T: Copyable & Deinitable](a: T, b: T) -> T:
    """Rust `a / b` where `T: Div<Output = T>`."""
    comptime if conforms_to(T, RDiv):
        return a / b
    elif T == Int:
        var r = rebind[Int](a) / rebind[Int](b)
        return rebind[T](Int(r)).copy()
    elif T == Float32:
        var r = rebind[Float32](a) / rebind[Float32](b)
        return rebind[T](r).copy()
    elif T == Float64:
        var r = rebind[Float64](a) / rebind[Float64](b)
        return rebind[T](r).copy()
    elif T == Float16:
        var r = rebind[Float16](a) / rebind[Float16](b)
        return rebind[T](r).copy()
    elif T == Int8:
        var r = rebind[Int8](a) / rebind[Int8](b)
        return rebind[T](r).copy()
    elif T == Int16:
        var r = rebind[Int16](a) / rebind[Int16](b)
        return rebind[T](r).copy()
    elif T == Int32:
        var r = rebind[Int32](a) / rebind[Int32](b)
        return rebind[T](r).copy()
    elif T == Int64:
        var r = rebind[Int64](a) / rebind[Int64](b)
        return rebind[T](r).copy()
    elif T == UInt8:
        var r = rebind[UInt8](a) / rebind[UInt8](b)
        return rebind[T](r).copy()
    elif T == UInt16:
        var r = rebind[UInt16](a) / rebind[UInt16](b)
        return rebind[T](r).copy()
    elif T == UInt32:
        var r = rebind[UInt32](a) / rebind[UInt32](b)
        return rebind[T](r).copy()
    elif T == UInt64:
        var r = rebind[UInt64](a) / rebind[UInt64](b)
        return rebind[T](r).copy()
    else:
        comptime assert False, "rust-to-mojo: type parameter has no `/` (Rust bound `Div<Output = T>`); conform it to mojo_rt.ops.RDiv"

@always_inline
def rem[T: Copyable & Deinitable](a: T, b: T) -> T:
    """Rust `a % b` where `T: Rem<Output = T>`."""
    comptime if conforms_to(T, RRem):
        return a % b
    elif T == Int:
        var r = _scalar_rem(Int64(rebind[Int](a)), Int64(rebind[Int](b)))
        return rebind[T](Int(r)).copy()
    elif T == Float32:
        var r = _scalar_rem(rebind[Float32](a), rebind[Float32](b))
        return rebind[T](r).copy()
    elif T == Float64:
        var r = _scalar_rem(rebind[Float64](a), rebind[Float64](b))
        return rebind[T](r).copy()
    elif T == Float16:
        var r = _scalar_rem(rebind[Float16](a), rebind[Float16](b))
        return rebind[T](r).copy()
    elif T == Int8:
        var r = _scalar_rem(rebind[Int8](a), rebind[Int8](b))
        return rebind[T](r).copy()
    elif T == Int16:
        var r = _scalar_rem(rebind[Int16](a), rebind[Int16](b))
        return rebind[T](r).copy()
    elif T == Int32:
        var r = _scalar_rem(rebind[Int32](a), rebind[Int32](b))
        return rebind[T](r).copy()
    elif T == Int64:
        var r = _scalar_rem(rebind[Int64](a), rebind[Int64](b))
        return rebind[T](r).copy()
    elif T == UInt8:
        var r = _scalar_rem(rebind[UInt8](a), rebind[UInt8](b))
        return rebind[T](r).copy()
    elif T == UInt16:
        var r = _scalar_rem(rebind[UInt16](a), rebind[UInt16](b))
        return rebind[T](r).copy()
    elif T == UInt32:
        var r = _scalar_rem(rebind[UInt32](a), rebind[UInt32](b))
        return rebind[T](r).copy()
    elif T == UInt64:
        var r = _scalar_rem(rebind[UInt64](a), rebind[UInt64](b))
        return rebind[T](r).copy()
    else:
        comptime assert False, "rust-to-mojo: type parameter has no `%` (Rust bound `Rem<Output = T>`); conform it to mojo_rt.ops.RRem"


@always_inline
def neg[T: Copyable & Deinitable](a: T) -> T:
    """Rust `-a` where `T: Neg<Output = T>`."""
    comptime if conforms_to(T, RNeg):
        return -a
    elif T == Int:
        var r = -rebind[Int](a)
        return rebind[T](r).copy()
    elif T == Float32:
        var r = -rebind[Float32](a)
        return rebind[T](r).copy()
    elif T == Float64:
        var r = -rebind[Float64](a)
        return rebind[T](r).copy()
    elif T == Float16:
        var r = -rebind[Float16](a)
        return rebind[T](r).copy()
    elif T == Int8:
        var r = -rebind[Int8](a)
        return rebind[T](r).copy()
    elif T == Int16:
        var r = -rebind[Int16](a)
        return rebind[T](r).copy()
    elif T == Int32:
        var r = -rebind[Int32](a)
        return rebind[T](r).copy()
    elif T == Int64:
        var r = -rebind[Int64](a)
        return rebind[T](r).copy()
    elif T == UInt8:
        var r = -rebind[UInt8](a)
        return rebind[T](r).copy()
    elif T == UInt16:
        var r = -rebind[UInt16](a)
        return rebind[T](r).copy()
    elif T == UInt32:
        var r = -rebind[UInt32](a)
        return rebind[T](r).copy()
    elif T == UInt64:
        var r = -rebind[UInt64](a)
        return rebind[T](r).copy()
    else:
        comptime assert False, "rust-to-mojo: type parameter has no unary `-` (Rust bound `Neg<Output = T>`); conform it to mojo_rt.ops.RNeg"


# ---- heterogeneous `T * S` / `T / S` (Rust `T: Mul<S, Output = T>`) ---------
# GPUI scales unit types by `f32`: `Point<Pixels> * f32`, `Size<T> / S`. Mojo
# traits take no type parameters, so only two cases can dispatch: `S == T`
# (the same-type operator) and `S == Float32` with `T` conforming to
# `RMulF32` / `RDivF32` (from `impl Mul<f32, Output = Self> for X`), plus the
# built-in scalars multiplied by a value of their own type.


trait RMulF32:
    """Rust `Mul<f32, Output = Self>`."""

    def __mul__(self, rhs: Float32) -> Self:
        ...


trait RDivF32:
    """Rust `Div<f32, Output = Self>`."""

    def __truediv__(self, rhs: Float32) -> Self:
        ...


@always_inline
def mul_rhs[T: Copyable & Deinitable, S: Copyable & Deinitable](a: T, b: S) -> T:
    """Rust `a * b` where `T: Mul<S, Output = T>`."""
    comptime if T == S:
        return mul(a, rebind[T](b).copy())
    elif S == Float32 and conforms_to(T, RMulF32):
        return a * rebind[Float32](b)
    else:
        comptime assert False, "rust-to-mojo: no `*` between these types (Rust bound `Mul<S, Output = T>`); conform T to mojo_rt.ops.RMulF32 or port by hand"

@always_inline
def div_rhs[T: Copyable & Deinitable, S: Copyable & Deinitable](a: T, b: S) -> T:
    """Rust `a / b` where `T: Div<S, Output = T>`."""
    comptime if T == S:
        return div(a, rebind[T](b).copy())
    elif S == Float32 and conforms_to(T, RDivF32):
        return a / rebind[Float32](b)
    else:
        comptime assert False, "rust-to-mojo: no `/` between these types (Rust bound `Div<S, Output = T>`); conform T to mojo_rt.ops.RDivF32 or port by hand"

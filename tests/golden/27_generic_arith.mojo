"""Generic arithmetic: operators on a type parameter go through mojo_rt.ops."""

from mojo_rt import ops
from mojo_rt.ops import RAdd, RMulF32, RNeg, RSub

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Pixels(ImplicitlyCopyable, Equatable, Writable, Defaultable, RAdd, RSub, RMulF32):
    var _0: Float32

    def __init__(out self):
        self._0 = Float32()

    def __add__(self, rhs: Pixels) -> Pixels:
        return Pixels(self._0 + rhs._0)

    def __sub__(self, rhs: Self) -> Self:
        return Pixels(self._0 - rhs._0)

    def __mul__(self, rhs: Float32) -> Pixels:
        return Pixels(self._0 * rhs)

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Point[T: Copyable & Defaultable & Deinitable](ImplicitlyCopyable where conforms_to(T, ImplicitlyCopyable), Equatable where conforms_to(T, Equatable), Writable where conforms_to(T, Writable), Defaultable, RAdd, RNeg):
    var x: Self.T
    var y: Self.T

    def __init__(out self):
        self.x = Self.T()
        self.y = Self.T()

    def __add__(self, rhs: Self) -> Self:
        return Point(x=ops.add(self.x, rhs.x), y=ops.add(self.y, rhs.y))

    def __neg__(self) -> Self:
        return Point(x=ops.neg(self.x), y=ops.neg(self.y))

    def __iadd__(mut self, rhs: Self):
        self.x = ops.add(self.x, rhs.x)
        self.y = ops.add(self.y, rhs.y)

    def sum(self) -> Self.T:
        return ops.add(self.x.copy(), self.y.copy())

    def delta(self, other: Point[Self.T]) -> Point[Self.T]:
        return Point(x=ops.sub(other.x.copy(), self.x.copy()), y=ops.sub(other.y.copy(), self.y.copy()))

    def scale(self, factor: Float32) -> Point[Self.T]:
        """Scaling by `f32`: `ops.mul_rhs` (same type, or `T: RMulF32`)."""
        return Point(x=ops.mul_rhs(self.x.copy(), factor), y=ops.mul_rhs(self.y.copy(), factor))

def double[T: ImplicitlyCopyable & Deinitable](var v: T) -> T:
    return ops.add(v, v)

# TODO(rust-to-mojo): dropped bound `Add` (no Mojo std trait) -- `T: Add<u8, Output = T>`: operators on generic params need a user trait or DType parameterisation
# TODO(rust-to-mojo): operator on a generic parameter with a different operand type -- `v + 1u8`: mojo_rt.ops only covers `Output = Self` operators
def bump[T: Movable & Deinitable](var v: T) -> T:
    """Heterogeneous `Add<u8>`: no dispatch trait, stays a TODO."""
    return v + UInt8(1)

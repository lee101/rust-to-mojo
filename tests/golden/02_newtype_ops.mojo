from mojo_rt.num import fmax, round_half_away
from mojo_rt.ops import RAdd, RMulF32, RNeg, RSub
from std.math import floor

# TODO(rust-to-mojo): derive(PartialOrd) over float fields: NaN ordering follows Mojo comparisons -- Pixels
# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Pixels(ImplicitlyCopyable, Equatable, Writable, Defaultable, Comparable, RAdd, RSub, RMulF32, RNeg):
    """Logical pixels."""
    var _0: Float32

    def __init__(out self):
        self._0 = Float32()

    def __lt__(self, other: Self) -> Bool:
        if self._0 != other._0:
            return self._0 < other._0
        return False

    def __le__(self, other: Self) -> Bool:
        return not other < self

    def __gt__(self, other: Self) -> Bool:
        return other < self

    def __ge__(self, other: Self) -> Bool:
        return not self < other

    def __add__(self, rhs: Pixels) -> Pixels:
        return Pixels(self._0 + rhs._0)

    def __sub__(self, rhs: Self) -> Pixels:
        return Pixels(self._0 - rhs._0)

    def __mul__(self, rhs: Float32) -> Pixels:
        return Pixels(self._0 * rhs)

    # Rust: impl Mul<Pixels> for f32
    def __rmul__(self, lhs: Float32) -> Pixels:
        return Pixels(lhs * self._0)

    def __neg__(self) -> Pixels:
        return Pixels(-self._0)

    def __iadd__(mut self, rhs: Pixels):
        self._0 += rhs._0

    comptime ZERO: Pixels = Pixels(0.0)

    def floor(self) -> Self:
        return Self(floor(self._0))

    def round(self) -> Self:
        return Self(round_half_away(self._0))

    def abs(self) -> Self:
        return Self(abs(self._0))

    def max(self, other: Pixels) -> Pixels:
        return Pixels(fmax(self._0, other._0))

def px(value: Float32) -> Pixels:
    return Pixels(value)

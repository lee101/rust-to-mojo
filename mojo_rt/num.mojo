"""Rust numeric semantics on top of Mojo scalars.

Every function is generic over `Scalar[dtype]`; in Mojo 1.0 `Int` is
`Scalar[DType.int]`, so the same helpers cover `Int`, `Int32`, `Float32`, ...
"""

from std.bit import pop_count, count_leading_zeros, count_trailing_zeros
from std.math import trunc, copysign
# Location of the NaN/inf helpers is unverified for 1.0 (std.utils.numerics in 25.x).
from std.utils.numerics import isnan, isinf, inf, nan


# ---- float constants (std::f32::consts / std::f64::consts and limits) ------------

comptime F32_PI = Float32(3.14159265358979323846264338327950288)
comptime F32_TAU = Float32(6.28318530717958647692528676655900577)
comptime F32_E = Float32(2.71828182845904523536028747135266250)
comptime F32_SQRT_2 = Float32(1.41421356237309504880168872420969808)
comptime F32_FRAC_1_SQRT_2 = Float32(0.707106781186547524400844362104849039)
comptime F32_FRAC_PI_2 = Float32(1.57079632679489661923132169163975144)
comptime F32_FRAC_PI_3 = Float32(1.04719755119659774615421446109316763)
comptime F32_FRAC_PI_4 = Float32(0.785398163397448309615660845819875721)
comptime F32_FRAC_PI_6 = Float32(0.52359877559829887307710723054658381)
comptime F32_FRAC_PI_8 = Float32(0.39269908169872415480783042290993786)
comptime F32_FRAC_1_PI = Float32(0.318309886183790671537767526745028724)
comptime F32_FRAC_2_PI = Float32(0.636619772367581343075535053490057448)
comptime F32_LN_2 = Float32(0.693147180559945309417232121458176568)
comptime F32_LN_10 = Float32(2.30258509299404568401799145468436421)
comptime F32_LOG2_E = Float32(1.44269504088896340735992468100189214)
comptime F32_LOG10_E = Float32(0.434294481903251827651128918916605082)
comptime F32_EPSILON = Float32(1.19209290e-07)
comptime F32_MAX = Float32(3.40282347e38)
comptime F32_MIN = Float32(-3.40282347e38)
comptime F32_MIN_POSITIVE = Float32(1.17549435e-38)
comptime F32_INFINITY = inf[DType.float32]()
comptime F32_NEG_INFINITY = -inf[DType.float32]()
comptime F32_NAN = nan[DType.float32]()

comptime F64_PI = Float64(3.14159265358979323846264338327950288)
comptime F64_TAU = Float64(6.28318530717958647692528676655900577)
comptime F64_E = Float64(2.71828182845904523536028747135266250)
comptime F64_SQRT_2 = Float64(1.41421356237309504880168872420969808)
comptime F64_FRAC_1_SQRT_2 = Float64(0.707106781186547524400844362104849039)
comptime F64_FRAC_PI_2 = Float64(1.57079632679489661923132169163975144)
comptime F64_FRAC_PI_3 = Float64(1.04719755119659774615421446109316763)
comptime F64_FRAC_PI_4 = Float64(0.785398163397448309615660845819875721)
comptime F64_FRAC_PI_6 = Float64(0.52359877559829887307710723054658381)
comptime F64_FRAC_PI_8 = Float64(0.39269908169872415480783042290993786)
comptime F64_FRAC_1_PI = Float64(0.318309886183790671537767526745028724)
comptime F64_FRAC_2_PI = Float64(0.636619772367581343075535053490057448)
comptime F64_LN_2 = Float64(0.693147180559945309417232121458176568)
comptime F64_LN_10 = Float64(2.30258509299404568401799145468436421)
comptime F64_LOG2_E = Float64(1.44269504088896340735992468100189214)
comptime F64_LOG10_E = Float64(0.434294481903251827651128918916605082)
comptime F64_EPSILON = Float64(2.2204460492503131e-16)
comptime F64_MAX = Float64(1.7976931348623157e308)
comptime F64_MIN = Float64(-1.7976931348623157e308)
comptime F64_MIN_POSITIVE = Float64(2.2250738585072014e-308)
comptime F64_INFINITY = inf[DType.float64]()
comptime F64_NEG_INFINITY = -inf[DType.float64]()
comptime F64_NAN = nan[DType.float64]()


# ---- division / remainder ----------------------------------------------------------

def rem[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    """Rust `%`: the remainder truncated toward zero, so its sign follows the
    dividend. Mojo's `%` follows the divisor (floored, Python-style)."""
    comptime if dtype.is_floating_point():
        return a - trunc(a / b) * b
    else:
        var r = a % b
        if r != 0 and ((r < 0) != (a < 0)):
            r -= b
        return r


def rem_euclid[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    """Rust `rem_euclid`: always non-negative remainder."""
    var r = rem(a, b)
    if r < 0:
        r += abs(b)
    return r


def div_euclid[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    """Rust `div_euclid`."""
    comptime if dtype.is_floating_point():
        var q = trunc(a / b)
        if rem(a, b) < 0:
            return q - 1 if b > 0 else q + 1
        return q
    else:
        var q = (a - rem(a, b)) / b
        if rem(a, b) < 0:
            return q - 1 if b > 0 else q + 1
        return q


def abs_diff[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    return a - b if a > b else b - a


# ---- floats ---------------------------------------------------------------------

def fmax[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    """Rust `f32::max`: if one argument is NaN, return the other."""
    if isnan(a):
        return b
    if isnan(b):
        return a
    return a if a > b else b


def fmin[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    """Rust `f32::min`: if one argument is NaN, return the other."""
    if isnan(a):
        return b
    if isnan(b):
        return a
    return a if a < b else b


def round_half_away[dtype: DType](x: Scalar[dtype]) -> Scalar[dtype]:
    """Rust `f32::round`: halfway cases round away from zero."""
    var t = trunc(x)
    if abs(x - t) >= 0.5:
        t += copysign(Scalar[dtype](1), x)
    return t


def signum[dtype: DType](x: Scalar[dtype]) -> Scalar[dtype]:
    """Rust `f32::signum`: 1.0 for +0.0 and positives, -1.0 for -0.0 and
    negatives, NaN for NaN."""
    if isnan(x):
        return x
    return copysign(Scalar[dtype](1), x)


def fract[dtype: DType](x: Scalar[dtype]) -> Scalar[dtype]:
    return x - trunc(x)


def recip[dtype: DType](x: Scalar[dtype]) -> Scalar[dtype]:
    return Scalar[dtype](1) / x


def to_radians[dtype: DType](x: Scalar[dtype]) -> Scalar[dtype]:
    return x * (Scalar[dtype](3.14159265358979323846264338327950288) / Scalar[dtype](180))


def to_degrees[dtype: DType](x: Scalar[dtype]) -> Scalar[dtype]:
    return x * (Scalar[dtype](180) / Scalar[dtype](3.14159265358979323846264338327950288))


def is_nan[dtype: DType](x: Scalar[dtype]) -> Bool:
    return isnan(x)


def is_infinite[dtype: DType](x: Scalar[dtype]) -> Bool:
    return isinf(x)


def is_finite[dtype: DType](x: Scalar[dtype]) -> Bool:
    return not isnan(x) and not isinf(x)


# ---- checked / saturating integer arithmetic --------------------------------------
# Mojo fixed-width integers wrap on overflow (like Rust release builds).

def checked_add[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Optional[Scalar[dtype]]:
    var r = a + b
    comptime if dtype.is_signed():
        if (b > 0 and r < a) or (b < 0 and r > a):
            return None
    else:
        if r < a:
            return None
    return r


def checked_sub[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Optional[Scalar[dtype]]:
    var r = a - b
    comptime if dtype.is_signed():
        if (b > 0 and r > a) or (b < 0 and r < a):
            return None
    else:
        if b > a:
            return None
    return r


def checked_mul[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Optional[Scalar[dtype]]:
    if a == 0 or b == 0:
        return Scalar[dtype](0)
    comptime if dtype.is_signed():
        if (a == -1 and b == Scalar[dtype].MIN) or (b == -1 and a == Scalar[dtype].MIN):
            return None
    var r = a * b
    if r / b != a:
        return None
    return r


def checked_div[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Optional[Scalar[dtype]]:
    if b == 0:
        return None
    comptime if dtype.is_signed():
        if a == Scalar[dtype].MIN and b == -1:
            return None
    return a / b


def checked_rem[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Optional[Scalar[dtype]]:
    if b == 0:
        return None
    comptime if dtype.is_signed():
        if a == Scalar[dtype].MIN and b == -1:
            return None
    return rem(a, b)


def saturating_add[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    var r = checked_add(a, b)
    if r:
        return r.value()
    return Scalar[dtype].MAX if b > 0 else Scalar[dtype].MIN


def saturating_sub[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    var r = checked_sub(a, b)
    if r:
        return r.value()
    comptime if dtype.is_signed():
        return Scalar[dtype].MIN if b > 0 else Scalar[dtype].MAX
    else:
        return Scalar[dtype](0)


def saturating_mul[dtype: DType](a: Scalar[dtype], b: Scalar[dtype]) -> Scalar[dtype]:
    var r = checked_mul(a, b)
    if r:
        return r.value()
    return Scalar[dtype].MAX if (a < 0) == (b < 0) else Scalar[dtype].MIN


# ---- bit counting -------------------------------------------------------------------

def count_ones[dtype: DType](x: Scalar[dtype]) -> UInt32:
    return UInt32(Int(pop_count(x)))


def leading_zeros[dtype: DType](x: Scalar[dtype]) -> UInt32:
    return UInt32(Int(count_leading_zeros(x)))


def trailing_zeros[dtype: DType](x: Scalar[dtype]) -> UInt32:
    return UInt32(Int(count_trailing_zeros(x)))

from mojo_rt.num import F32_PI, checked_add, rem, saturating_sub

def rem_signed(a: Int32, b: Int32) -> Int32:
    return rem(a, b)

def rem_float(a: Float64, b: Float64) -> Float64:
    return rem(a, b)

def wrapping(a: UInt8, b: UInt8) -> UInt8:
    return a + b

def checked(a: UInt32, b: UInt32) -> Optional[UInt32]:
    return checked_add(a, b)

def saturating(a: Int16, b: Int16) -> Int16:
    return saturating_sub(a, b)

# TODO(rust-to-mojo): float->int cast: Rust saturates, Mojo conversion of out-of-range values differs -- x as i32
def casts(x: Float32) -> Int32:
    var y = Int32(x)
    var z = UInt8(y)
    return Int32(z) + Int32(Int64(y))

def bits(x: UInt32) -> UInt32:
    return (~x & 0xFF) << 2 | x >> 3 ^ 1

def bools(a: Bool, b: Bool) -> Bool:
    return not a or a and not b

def literals() -> Float64:
    var a = UInt64(1000)
    var b = Float64(2.5e3)
    var c = 0x10
    return Float64(a) + b + Float64(c)

def maxes() -> Int32:
    return Int32.MAX - max(abs(Int32.MIN), 1)

def pi_ish() -> Float32:
    return F32_PI * 2.0

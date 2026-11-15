# HINT(rust-to-mojo/opt:vectorize): range loop with indexed element-wise arithmetic: candidate for `vectorize[simd_width]` over `SIMD[dtype, simd_width_of[dtype]()]` loads/stores
def dot(a: List[Float32], b: List[Float32]) -> Float32:
    var s = Float32(0.0)
    for i in range(len(a)):
        s += a[i] * b[i]
    return s

# HINT(rust-to-mojo/opt:vectorize): range loop with indexed element-wise arithmetic: candidate for `vectorize[simd_width]` over `SIMD[dtype, simd_width_of[dtype]()]` loads/stores
# HINT(rust-to-mojo/opt:parallelize): nested loops: if outer iterations are independent, `parallelize[worker](n)` over the outer loop
# HINT(rust-to-mojo/opt:autotune): tight arithmetic loop with no calls: candidate for `comptime` unrolling / autotuned tile sizes
def matmul(a: List[Float32], b: List[Float32], mut out_: List[Float32], n: Int):
    for i in range(n):
        for j in range(n):
            var acc: Float32 = 0.0
            for k in range(n):
                acc += a[i * n + k] * b[k * n + j]
            out_[i * n + j] = acc

def lerp(a: Float32, b: Float32, t: Float32) -> Float32:
    return a + (b - a) * t

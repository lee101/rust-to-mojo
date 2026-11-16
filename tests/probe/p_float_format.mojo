from std.utils.numerics import inf, nan


def main():
    print(Float32(2.0), Float32(0.1), Float32(1.5), Float32(1e20), Float32(1.25e-7), Float32(-0.0), Float32(16777216.0), Float32(3.4028235e38))
    print(Float64(2.0), Float64(0.1), Float64(1e20), Float64(1e21), Float64(1.25e-7), Float64(123456789.125), Float64(1e16), Float64(1e15))
    print(nan[DType.float32](), inf[DType.float32](), -inf[DType.float64]())
    print(String(Float32(2.0)), repr(Float32(2.0)))

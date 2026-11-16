from g25_numeric_kernel import dot, matmul, lerp
from std.memory import bitcast


def main():
    var n = 4
    var a = List[Float32]()
    var b = List[Float32]()
    for i in range(n * n):
        a.append(Float32(i) * 0.5 - 3.0)
        b.append(1.0 / (Float32(i) + 1.0))
    print(bitcast[DType.uint32](dot(a, b)))
    var out = List[Float32]()
    for _ in range(n * n):
        out.append(0.0)
    matmul(a, b, out, n)
    for v in out:
        print(bitcast[DType.uint32](v), end=" ")
    print()
    print(bitcast[DType.uint32](lerp(1.0, 3.0, 0.25)), bitcast[DType.uint32](lerp(-1.0, 1.0, 0.75)))

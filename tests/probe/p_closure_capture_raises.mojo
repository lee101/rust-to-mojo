def apply[F: def(Int) raises -> Int](f: F, x: Int) raises -> Int:
    return f(x)


def main() raises:
    var k = 3
    def add_k(x: Int) raises {imm k} -> Int:
        return x + k
    print(apply(add_k, 4))

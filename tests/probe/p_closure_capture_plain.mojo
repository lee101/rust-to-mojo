def apply[F: def(Int) -> Int](f: F, x: Int) -> Int:
    return f(x)


def main():
    var k = 3
    def add_k(x: Int) {imm k} -> Int:
        return x + k
    print(apply(add_k, 4))

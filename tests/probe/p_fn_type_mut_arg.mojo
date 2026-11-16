def update[F: def(mut Int)](mut x: Int, f: F):
    f(x)


def main():
    var v = 1
    def bump(mut a: Int):
        a += 10
    update(v, bump)
    print(v)

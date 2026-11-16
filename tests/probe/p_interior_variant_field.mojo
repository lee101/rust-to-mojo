from std.utils import Variant

@fieldwise_init
struct A(Copyable):
    var x: Int

@fieldwise_init
struct B(Copyable):
    var y: Int

@fieldwise_init
struct E(Copyable):
    var _v: Variant[A, B]

    def as_A(ref self) -> ref [self._v] A:
        return self._v[A]


def main():
    var e = E(A(3))
    print(e.as_A().x)

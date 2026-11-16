from std.utils import Variant
from std.os import abort

@fieldwise_init
struct A(Copyable):
    var x: Int

@fieldwise_init
struct B(Copyable):
    var y: Int

@fieldwise_init
struct E(Copyable):
    var _v: Variant[A, B]

    def as_A(ref self) -> ref [self] A:
        if not self._v.isa[A]():
            abort("wrong variant")
        return self._v.unsafe_get[A]()


def main():
    var e = E(A(3))
    print(e.as_A().x)

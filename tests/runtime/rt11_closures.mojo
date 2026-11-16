from g11_closures import apply, apply_twice, typed_closure, capturing, untyped, any_negative
from std.memory import bitcast


def dbl(x: Int32) -> Int32:
    return x * 2


def halve(x: Float32) -> Float32:
    return x / 2.0


def main():
    print(apply(dbl, 21), bitcast[DType.uint32](apply_twice(halve, 10.0)))
    print(typed_closure(41), capturing(7))
    var a: List[Int32] = [1, -2, 3]
    var u = untyped(a)
    print(len(u), u[0], u[1], u[2])
    var p: List[Int32] = [1, 2]
    print(Int(any_negative(p)), Int(any_negative(a)))

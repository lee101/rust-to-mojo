from g02_newtype_ops import Pixels, px
from std.memory import bitcast
from std.utils.numerics import nan


def b(x: Pixels) -> UInt32:
    return bitcast[DType.uint32](x._0)


def main():
    var a = px(1.5)
    var c = px(-2.25)
    print(b(a + c), b(a - c), b(-a))
    print(b(a * 3.0), b(2.0 * c))
    var d = a
    d += c
    print(b(d))
    var vals: List[Float32] = [2.5, -2.5, 0.5, -0.5, 1.4999, 3.7, -3.7]
    for v in vals:
        var p = px(v)
        print(b(p.floor()), b(p.round()), b(p.abs()))
    print(b(a.max(c)), b(px(nan[DType.float32]()).max(a)))
    print(Int(a < c), Int(c < a), Int(a == px(1.5)))
    print(b(Pixels.ZERO))

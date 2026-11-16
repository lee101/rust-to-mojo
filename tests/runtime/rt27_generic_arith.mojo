from g27_generic_arith import Pixels, Point, double
from std.memory import bitcast


def u(x: Float32) -> UInt32:
    return bitcast[DType.uint32](x)


def main():
    var p = Point(x=Pixels(1.5), y=Pixels(-4.0))
    var q = Point(x=Pixels(0.25), y=Pixels(2.0))
    var r = p + q
    print(u(r.x._0), u(r.y._0))
    var f = Point(x=Float32(3.5), y=Float32(-1.25))
    var m = f
    m += Point(x=Float32(0.5), y=Float32(2.0))
    print(u(m.x), u(m.y))
    var nf = -f
    print(u(nf.x), u(nf.y))
    print(u(f.sum()), u(p.sum()._0))
    var d = p.delta(q)
    print(u(d.x._0), u(d.y._0))
    var s = p.scale(2.0)
    var sf = f.scale(0.5)
    print(u(s.x._0), u(s.y._0), u(sf.x), u(sf.y))
    var i = Point(x=Int32(7), y=Int32(-3))
    print(i.sum(), double(Int64(21)))
    print(bitcast[DType.uint64](double(Float64(1.25))))

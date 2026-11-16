from g21_bounds import Point, Size, Bounds
from std.memory import bitcast


def main():
    var b = Bounds(origin=Point(x=Float32(1.5), y=Float32(-2.0)), size=Size(width=Float32(3.0), height=Float32(4.5)))
    print(bitcast[DType.uint32](b.right()), bitcast[DType.uint32](b.bottom()))
    print(Int(b.contains(Point(x=Float32(2.0), y=Float32(0.0)))), Int(b.contains(Point(x=Float32(5.0), y=Float32(0.0)))))
    var bi = Bounds(origin=Point(x=Int32(10), y=Int32(20)), size=Size(width=Int32(5), height=Int32(7)))
    print(bi.right(), bi.bottom(), Int(bi.contains(Point(x=Int32(15), y=Int32(27)))))
    var s = Point(x=Int64(1), y=Int64(2)) + Point(x=Int64(10), y=Int64(20))
    print(s.x, s.y)
    print(Int(Point(x=Int64(1), y=Int64(2)) == Point[Int64]()))

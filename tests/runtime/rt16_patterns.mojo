from g16_patterns import Pair, swap, sum_pair, bucket, quadrant, nested


def main():
    var t = swap((Int32(1), Int32(2)))
    print(t[0], t[1])
    print(sum_pair(Pair(a=40, b=2)))
    var ns: List[UInt32] = [0, 1, 9, 10, 99, 100]
    for n in ns:
        print(bucket(n), end=" ")
    print()
    print(quadrant((Float32(1.0), Float32(1.0))), quadrant((Float32(-1.0), Float32(5.0))), quadrant((Float32(1.0), Float32(-1.0))))
    var s5 = Optional(Optional(Int32(5)))
    var sn = Optional(Optional[Int32](None))
    var nn = Optional[Optional[Int32]](None)
    print(nested(s5), nested(sn), nested(nn))

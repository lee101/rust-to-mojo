from g18_integers import rem_signed, rem_float, wrapping, checked, saturating, casts, bits, bools, literals, maxes, pi_ish
from std.memory import bitcast


def show(o: Optional[UInt32]) -> String:
    if o:
        return String("Some(") + String(o.value()) + ")"
    return String("None")


def main():
    print(rem_signed(7, 3), rem_signed(-7, 3), rem_signed(7, -3), rem_signed(-7, -3))
    print(bitcast[DType.uint64](rem_float(-7.5, 2.0)), bitcast[DType.uint64](rem_float(7.5, -2.0)))
    print(wrapping(250, 10), wrapping(1, 2))
    print(show(checked(UInt32.MAX, 1)), show(checked(1, 2)))
    print(saturating(-32000, 1000), saturating(5, 3))
    print(casts(3.9), casts(-1.5), casts(300.0))
    print(bits(0x12345678), bits(0))
    print(Int(bools(True, True)), Int(bools(True, False)), Int(bools(False, True)), Int(bools(False, False)))
    print(bitcast[DType.uint64](literals()))
    print(maxes())
    print(bitcast[DType.uint32](pi_ish()))

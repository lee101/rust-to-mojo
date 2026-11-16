from g06_option import first_positive, double_or_zero, add_opts, describe, has
from std.memory import bitcast


def show(o: Optional[Int32]) -> String:
    if o:
        return String("Some(") + String(o.value()) + ")"
    return String("None")


def main():
    var l1: List[Int32] = [-3, 0, 7, 9]
    var l2: List[Int32] = [-3, 0]
    print(show(first_positive(l1)))
    print(show(first_positive(l2)))
    print(double_or_zero(Int32(21)), double_or_zero(None))
    print(show(add_opts(Int32(2), Int32(3))), show(add_opts(None, Int32(3))), show(add_opts(Int32(2), None)))
    print(bitcast[DType.uint32](describe(Float32(1.5))), bitcast[DType.uint32](describe(None)))
    print(Int(has(Optional(String("a")))), Int(has(None)))

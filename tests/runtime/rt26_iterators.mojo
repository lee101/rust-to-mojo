from g26_iterators import evens_squared, total, count_big, all_positive, print_all, sum_of_range
from std.memory import bitcast


def main():
    var a: List[Int64] = [1, 2, 3, 4, -6]
    var e = evens_squared(a)
    print(len(e), e[0], e[1], e[2])
    var f: List[Float32] = [0.5, 0.25, 1.0]
    print(bitcast[DType.uint32](total(f)))
    var u: List[UInt32] = [1, 5, 10, 20]
    print(count_big(u, 5))
    var p: List[Float64] = [1.0, 2.0]
    var n: List[Float64] = [1.0, -2.0]
    print(Int(all_positive(p)), Int(all_positive(n)))
    var s: List[String] = ["p", "q"]
    print_all(s)
    print(sum_of_range(10))

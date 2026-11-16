from g10_traits import Square, total_area, print_it
from std.memory import bitcast


def main():
    var v: List[Square] = [Square(2.0), Square(0.5)]
    print(bitcast[DType.uint64](total_area(v)))
    print(print_it(v[0]), Square.SIDES)
    print(v[1].describe())

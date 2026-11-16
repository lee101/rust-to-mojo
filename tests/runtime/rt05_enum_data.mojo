from g05_enum_data import Shape, unit_square, nothing
from std.memory import bitcast


def main():
    var shapes: List[Shape] = [Shape.Circle(2.0), Shape.Circle(11.0), Shape.Rect(2.0, 3.5), unit_square(), nothing()]
    for s in shapes:
        print(bitcast[DType.uint32](s.area()), s.describe())
    print(Int(unit_square() == Shape.Rect(1.0, 1.0)), Int(nothing() == unit_square()))

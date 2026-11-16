trait Shape:
    comptime Unit: Movable & Deinitable
    comptime SIDES: UInt32

    def area(self) -> Float64:
        ...

    def describe(self) -> String:
        return String("shape with area ") + String(self.area())


@fieldwise_init
struct Square(ImplicitlyCopyable, Shape):
    var side: Float64
    comptime Unit = Float64
    comptime SIDES: UInt32 = 4

    def area(self) -> Float64:
        return self.side * self.side


def total_area[S: Copyable & Shape & Deinitable](shapes: List[S]) -> Float64:
    var t: Float64 = 0.0
    for s in shapes:
        t += s.area()
    return t


def main():
    var l: List[Square] = [Square(2.0), Square(3.0)]
    print(total_area(l), Square(1.0).describe(), Square.SIDES)

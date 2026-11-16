from std.math import sqrt

@fieldwise_init
struct Point[T: Copyable & Deinitable](Copyable):
    var x: Self.T
    var y: Self.T

    def length(self) -> Float32 where Self.T == Float32:
        var x = rebind[Float32](self.x)
        var y = rebind[Float32](self.y)
        return sqrt(x * x + y * y)

    def scale(self, factor: Float32) -> Self where Self.T == Float32:
        return rebind_var[Self](Point[Float32](x=rebind[Float32](self.x) * factor, y=rebind[Float32](self.y) * factor))


def main():
    var p = Point[Float32](3.0, 4.0)
    print(p.length(), p.scale(2.0).x)

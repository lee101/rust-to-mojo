@fieldwise_init
struct P[T: Copyable & Deinitable](Copyable):
    var x: Self.T

    @staticmethod
    def new(var x: Self.T) -> Self:
        return Self(x=x^)


def main():
    var p = P.new(Int32(3))
    print(p.x)

@fieldwise_init
struct C(ImplicitlyCopyable, Defaultable):
    var _tag: Int
    comptime A = Self(0)
    comptime B = Self(5)

    def __init__(out self):
        self = Self.B


def main():
    print(C()._tag, C.A._tag)

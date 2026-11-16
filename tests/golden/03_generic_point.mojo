from std.math import sqrt

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Point[T: Copyable & Writable & Defaultable & Equatable & Deinitable](ImplicitlyCopyable where conforms_to(T, ImplicitlyCopyable), Equatable, Hashable where conforms_to(T, Hashable), Writable, Defaultable):
    var x: Self.T
    var y: Self.T

    def __init__(out self):
        self.x = Self.T()
        self.y = Self.T()

    @staticmethod
    def new(var x: Self.T, var y: Self.T) -> Self:
        return Self(x=x^, y=y^)

    def swap(self) -> Self:
        return Point(x=self.y.copy(), y=self.x.copy())

    def max_coord(self) -> Self.T where conforms_to(Self.T, Comparable):
        if self.x > self.y:
            return self.x.copy()
        else:
            return self.y.copy()

    # TODO(rust-to-mojo): impl specialised on a concrete type argument -- method gated with `where Self.T == Float32`
    def length(self) -> Float32 where Self.T == Float32:
        return sqrt(self.x * self.x + self.y * self.y)

    # TODO(rust-to-mojo): impl specialised on a concrete type argument -- method gated with `where Self.T == Float32`
    def scale(self, factor: Float32) -> Point[Float32] where Self.T == Float32:
        return Point(x=self.x * factor, y=self.y * factor)

def point[T: Copyable & Writable & Defaultable & Equatable & Deinitable](var x: T, var y: T) -> Point[T]:
    return Point(x=x^, y=y^)

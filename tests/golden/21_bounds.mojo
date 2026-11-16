from mojo_rt import ops
from mojo_rt.ops import RAdd

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Point[T: Copyable & Writable & Defaultable & Equatable & Deinitable](ImplicitlyCopyable where conforms_to(T, ImplicitlyCopyable), Equatable, Writable, Defaultable, RAdd):
    var x: Self.T
    var y: Self.T

    def __init__(out self):
        self.x = Self.T()
        self.y = Self.T()

    def __add__(self, rhs: Self) -> Self:
        return Point(x=ops.add(self.x, rhs.x), y=ops.add(self.y, rhs.y))

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Size[T: Copyable & Writable & Defaultable & Equatable & Deinitable](ImplicitlyCopyable where conforms_to(T, ImplicitlyCopyable), Equatable, Writable, Defaultable):
    var width: Self.T
    var height: Self.T

    def __init__(out self):
        self.width = Self.T()
        self.height = Self.T()

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Bounds[T: Copyable & Writable & Defaultable & Equatable & Deinitable](ImplicitlyCopyable where conforms_to(T, ImplicitlyCopyable), Equatable, Writable, Defaultable):
    var origin: Point[Self.T]
    var size: Size[Self.T]

    def __init__(out self):
        self.origin = Point[Self.T]()
        self.size = Size[Self.T]()

    def right(self) -> Self.T:
        return ops.add(self.origin.x.copy(), self.size.width.copy())

    def bottom(self) -> Self.T:
        return ops.add(self.origin.y.copy(), self.size.height.copy())

    def contains(self, point: Point[Self.T]) -> Bool where conforms_to(Self.T, Comparable):
        return point.x >= self.origin.x and point.x <= ops.add(self.origin.x.copy(), self.size.width.copy()) and point.y >= self.origin.y and point.y <= ops.add(self.origin.y.copy(), self.size.height.copy())

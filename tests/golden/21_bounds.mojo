# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Point[T: Copyable & Writable & Defaultable & Equatable & Deinitable](ImplicitlyCopyable where conforms_to(T, ImplicitlyCopyable), Equatable, Writable, Defaultable):
    var x: Self.T
    var y: Self.T

    def __init__(out self):
        self.x = Self.T()
        self.y = Self.T()

    # TODO(rust-to-mojo): dropped bound `Add` (no Mojo std trait) -- `T: Add<Output = T>`: operators on generic params need a user trait or DType parameterisation
    def __add__(self, rhs: Self) -> Self:
        return Point(x=self.x + rhs.x, y=self.y + rhs.y)

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

    # TODO(rust-to-mojo): dropped bound `Add` (no Mojo std trait) -- `T: Add<T, Output = T>`: operators on generic params need a user trait or DType parameterisation
    # TODO(rust-to-mojo): dropped bound `Sub` (no Mojo std trait) -- `T: Sub<Output = T>`: operators on generic params need a user trait or DType parameterisation
    def right(self) -> Self.T:
        return self.origin.x.copy() + self.size.width.copy()

    # TODO(rust-to-mojo): dropped bound `Add` (no Mojo std trait) -- `T: Add<T, Output = T>`: operators on generic params need a user trait or DType parameterisation
    # TODO(rust-to-mojo): dropped bound `Sub` (no Mojo std trait) -- `T: Sub<Output = T>`: operators on generic params need a user trait or DType parameterisation
    def bottom(self) -> Self.T:
        return self.origin.y.copy() + self.size.height.copy()

    # TODO(rust-to-mojo): dropped bound `Add` (no Mojo std trait) -- `T: Add<T, Output = T>`: operators on generic params need a user trait or DType parameterisation
    # TODO(rust-to-mojo): method gated on an extra trait bound (`where conforms_to`) -- Self.T: Comparable
    def contains(self, point: Point[Self.T]) -> Bool where conforms_to(Self.T, Comparable):
        return point.x >= self.origin.x and point.x <= self.origin.x.copy() + self.size.width.copy() and point.y >= self.origin.y and point.y <= self.origin.y.copy() + self.size.height.copy()

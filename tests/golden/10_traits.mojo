trait Shape:
    comptime Unit: Movable & Deinitable
    comptime SIDES: UInt32

    def area(self) -> Float64:
        ...

    def describe(self) -> String:
        return String("shape with area ") + String(self.area())

trait Named(Shape):
    def name(self) -> String:
        ...

@fieldwise_init
struct Square(ImplicitlyCopyable, Shape, Named):
    var side: Float64
    comptime Unit = Float64
    comptime SIDES: UInt32 = 4

    def area(self) -> Float64:
        return self.side * self.side

    def name(self) -> String:
        return String("square")

def total_area[S: Movable & Shape & Deinitable](shapes: List[S]) -> Float64:
    var t: Float64 = 0.0
    for s in shapes:
        t += s.area()
    return t

def print_it(s: Some[Named]) -> String:
    return s.name()

# TODO(rust-to-mojo): fn `boxed` not translated: `&dyn Trait` argument (no existential types in Mojo) (&dyn Shape<Unit = f64>)
# --- original Rust ---
# pub fn boxed(s: &dyn Shape<Unit = f64>) -> f64 {
#     s.area()
# }
# --- end original Rust ---

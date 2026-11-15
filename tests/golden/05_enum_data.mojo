from std.utils import Variant

@fieldwise_init
struct Shape_Circle(Copyable, Writable, Equatable):
    """Payload of `Shape::Circle`."""
    var _0: Float32

@fieldwise_init
struct Shape_Rect(Copyable, Writable, Equatable):
    """Payload of `Shape::Rect`."""
    var w: Float32
    var h: Float32

@fieldwise_init
struct Shape_Empty(Copyable, Writable, Equatable):
    """Payload of `Shape::Empty`."""

# TODO(rust-to-mojo): Debug on data enum maps to Variant's Writable (format differs from Rust) -- Shape
# Rust enum `Shape` lowered to a tagged struct over Variant (see MAPPING.md#enums).
@fieldwise_init
struct Shape(Copyable, Writable, Equatable):
    var _v: Variant[Shape_Circle, Shape_Rect, Shape_Empty]

    @staticmethod
    def Circle(_0: Float32) -> Self:
        return Self(Shape_Circle(_0))

    @staticmethod
    def Rect(w: Float32, h: Float32) -> Self:
        return Self(Shape_Rect(w, h))

    @staticmethod
    def Empty() -> Self:
        return Self(Shape_Empty())

    def isa_Circle(self) -> Bool:
        return self._v.isa[Shape_Circle]()

    def as_Circle(ref self) -> ref [self] Shape_Circle:
        return self._v[Shape_Circle]

    def isa_Rect(self) -> Bool:
        return self._v.isa[Shape_Rect]()

    def as_Rect(ref self) -> ref [self] Shape_Rect:
        return self._v[Shape_Rect]

    def isa_Empty(self) -> Bool:
        return self._v.isa[Shape_Empty]()

    def as_Empty(ref self) -> ref [self] Shape_Empty:
        return self._v[Shape_Empty]

    def area(self) -> Float32:
        if self.isa_Circle():
            ref r = self.as_Circle()._0
            return 3.14159 * r * r
        elif self.isa_Rect():
            ref w = self.as_Rect().w
            ref h = self.as_Rect().h
            return w * h
        else:
            return 0.0

    # TODO(rust-to-mojo): `&str` return lowered to owned String -- &'static str
    def describe(self) -> String:
        if self.isa_Circle() and self.as_Circle()._0 > 10.0:
            ref r = self.as_Circle()._0
            return "big circle"
        elif self.isa_Circle():
            return "circle"
        elif self.isa_Rect() and self.as_Rect().w == self.as_Rect().h:
            ref w = self.as_Rect().w
            ref h = self.as_Rect().h
            return "square"
        else:
            return "other"

def unit_square() -> Shape:
    return Shape.Rect(w=1.0, h=1.0)

def nothing() -> Shape:
    return Shape.Empty()

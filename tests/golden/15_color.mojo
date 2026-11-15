from mojo_rt.num import rem
from std.math import clamp, floor

@fieldwise_init
struct Rgba(ImplicitlyCopyable, Equatable, Defaultable):
    """An RGBA color"""
    var r: Float32
    var g: Float32
    var b: Float32
    var a: Float32

    def __init__(out self):
        self.r = Float32()
        self.g = Float32()
        self.b = Float32()
        self.a = Float32()

    def blend(self, other: Rgba) -> Self:
        if other.a >= 1.0:
            return other
        elif other.a <= 0.0:
            return self
        else:
            var new_r = (1.0 - other.a) * self.r + other.a * other.r
            var new_g = (1.0 - other.a) * self.g + other.a * other.g
            var new_b = (1.0 - other.a) * self.b + other.a * other.b
            return Rgba(r=new_r, g=new_g, b=new_b, a=self.a)

    # TODO(rust-to-mojo): float->int cast: Rust saturates, Mojo conversion of out-of-range values differs -- (h * 6.0).floor() as i32
    @staticmethod
    def from_(color: Hsla) -> Self:
        var h = color.h
        var s = color.s
        var l = color.l
        var c = (1.0 - abs(2.0 * l - 1.0)) * s
        var x = c * (1.0 - abs(rem(h * 6.0, 2.0) - 1.0))
        var m = l - c / 2.0
        var cm = c + m
        var xm = x + m
        var __match1 = Int32(floor(h * 6.0))
        var __tmp2 = (cm, xm, m) if __match1 == 0 or __match1 == 6 else (xm, cm, m) if __match1 == 1 else (m, cm, xm) if __match1 == 2 else (m, xm, cm) if __match1 == 3 else (xm, m, cm) if __match1 == 4 else (cm, m, xm)
        ref r = __tmp2[0]
        ref g = __tmp2[1]
        ref b = __tmp2[2]
        return Rgba(r=clamp(r, 0.0, 1.0), g=clamp(g, 0.0, 1.0), b=clamp(b, 0.0, 1.0), a=color.a)

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Hsla(ImplicitlyCopyable, Writable, Defaultable):
    """An HSLA color"""
    var h: Float32
    var s: Float32
    var l: Float32
    var a: Float32

    def __init__(out self):
        self.h = Float32()
        self.s = Float32()
        self.l = Float32()
        self.a = Float32()

    def is_transparent(self) -> Bool:
        return self.a == 0.0

    def opacity(self, factor: Float32) -> Self:
        return Hsla(h=self.h, s=self.s, l=self.l, a=self.a * clamp(factor, 0.0, 1.0))

    def grayscale(self) -> Self:
        return Hsla(s=0.0, h=self.h, l=self.l, a=self.a)

def rgba(hex: UInt32) -> Rgba:
    var r = Float32(hex >> 24 & 0xFF) / 255.0
    var g = Float32(hex >> 16 & 0xFF) / 255.0
    var b = Float32(hex >> 8 & 0xFF) / 255.0
    var a = Float32(hex & 0xFF) / 255.0
    return Rgba(r=r, g=g, b=b, a=a)

def transparent_black() -> Hsla:
    return Hsla(h=0.0, s=0.0, l=0.0, a=0.0)

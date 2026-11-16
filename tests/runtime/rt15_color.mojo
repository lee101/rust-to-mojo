from g15_color import Rgba, Hsla, rgba, transparent_black
from std.memory import bitcast


def u(x: Float32) -> UInt32:
    return bitcast[DType.uint32](x)


def pr(c: Rgba):
    print(u(c.r), u(c.g), u(c.b), u(c.a))


def ph(c: Hsla):
    print(u(c.h), u(c.s), u(c.l), u(c.a))


def main():
    var a = rgba(0x336699FF)
    var b = rgba(0xFF000080)
    pr(a)
    pr(a.blend(b))
    pr(a.blend(rgba(0x00FF0000)))
    pr(a.blend(rgba(0x0000FFFF)))
    for i in range(13):
        var h = Hsla(h=Float32(i) / 12.0, s=0.75, l=0.4, a=0.9)
        pr(Rgba.from_(h))
    var h = Hsla(h=0.3, s=0.5, l=0.6, a=0.8)
    ph(h.opacity(0.5))
    ph(h.opacity(7.0))
    ph(h.grayscale())
    print(Int(h.is_transparent()), Int(transparent_black().is_transparent()))
    print(Int(a == rgba(0x336699FF)))

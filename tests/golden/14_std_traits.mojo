@fieldwise_init
struct Rgb(ImplicitlyCopyable, Equatable, Writable, Defaultable):
    var r: UInt8
    var g: UInt8
    var b: UInt8

    def write_to[W: Writer](self, mut writer: W):
        writer.write("rgb(", self.r, ", ", self.g, ", ", self.b, ")")

    def write_repr_to[W: Writer](self, mut writer: W):
        if self.r == self.g and self.g == self.b:
            writer.write("Gray(", self.r, ")")
        else:
            writer.write("Rgb(", self.r, ", ", self.g, ", ", self.b, ")")

    @staticmethod
    def default() -> Self:
        return Rgb(r=0, g=0, b=0)

    def __init__(out self):
        self = Self.default()

    @staticmethod
    def from_(hex: UInt32) -> Self:
        return Rgb(r=UInt8(hex >> 16 & 0xFF), g=UInt8(hex >> 8 & 0xFF), b=UInt8(hex & 0xFF))

def white() -> Rgb:
    return Rgb.from_(0xFFFFFF)

def black() -> Rgb:
    var c: Rgb = Rgb.from_(0x000000)
    return c^

def fallback() -> Rgb:
    return Rgb()

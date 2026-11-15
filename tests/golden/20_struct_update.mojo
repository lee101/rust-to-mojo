# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Style(ImplicitlyCopyable, Writable, Defaultable):
    var width: Float32
    var height: Float32
    var visible: Bool

    def __init__(out self):
        self.width = Float32()
        self.height = Float32()
        self.visible = Bool()

def wide() -> Style:
    var __base1 = Style()
    return Style(width=100.0, height=__base1.height, visible=__base1.visible)

def hidden(base: Style) -> Style:
    return Style(visible=False, width=base.width, height=base.height)

def from_fn() -> Style:
    var __base2 = make()
    return Style(height=2.0, width=__base2.width, visible=__base2.visible)

def make() -> Style:
    return Style(width=1.0, height=1.0, visible=True)

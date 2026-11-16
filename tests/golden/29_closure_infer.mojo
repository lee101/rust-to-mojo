"""Closure parameter types inferred from the call site."""

from mojo_rt import option as rt_option
from mojo_rt.option import unwrap_or

@fieldwise_init
struct Window:
    var width: Float32

@fieldwise_init
struct App:
    var windows: List[Window]
    var scale: Float32

    def update_scale[F1: def(Float32) -> Float32](mut self, f: F1):
        self.scale = f(self.scale)

    def demo(mut self) -> Float32:
        var k: Float32 = 2.0
        def __closure1(x: Float32) {imm k} -> Float32:
            return x * k
        var a = apply(3.0, __closure1)
        def __closure2(s: Float32) -> Float32:
            return s + 1.0
        self.update_scale(__closure2)
        return a

def apply[F: def(Float32) -> Float32](x: Float32, f: F) -> Float32:
    return f(x)

def with_window[F1: def(mut Window)](mut w: Window, f: F1):
    f(w)

# TODO(rust-to-mojo): slice argument `&[T]` lowered to List[T] borrow -- callers passing sub-slices need Span[T, origin]
def count_if[F: def(Int32) -> Bool](xs: List[Int32], pred: F) -> Int:
    var n: Int = 0
    for x in xs:
        if pred(x):
            n += 1
    return n

def grow(mut w: Window):
    def __closure3(mut win: Window):
        win.width += 10.0
    with_window(w, __closure3)

# TODO(rust-to-mojo): slice argument `&[T]` lowered to List[T] borrow -- callers passing sub-slices need Span[T, origin]
def positives(xs: List[Int32]) -> Int:
    def __closure4(x: Int32) -> Bool:
        return x > 0
    return count_if(xs, __closure4)

def first_len(var name: Optional[String]) -> Int:
    def __closure5(n: String) -> Int:
        return n.byte_length()
    return unwrap_or(rt_option.map(name, __closure5), 0)

def or_default(v: Optional[Int32]) -> Int32:
    def __closure6() -> Int32:
        return -1
    return rt_option.unwrap_or_else(v, __closure6)

def is_big(v: Optional[Int32]) -> Bool:
    def __closure7(x: Int32) -> Bool:
        return x > 100
    return rt_option.is_some_and(v, __closure7)

# TODO(rust-to-mojo): fn `opaque` not translated: closure with untyped parameters (needs type inference) (| x | x + 1)
# signature hint: def opaque() -> Int32
# --- original Rust ---
# /// No context to infer from: stays a stub.
# pub fn opaque() -> i32 {
#     let f = |x| x + 1;
#     f(1)
# }
# --- end original Rust ---

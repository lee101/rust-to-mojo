from mojo_rt.num import rem
from std.os import abort

def greet(name: String, n: Int32) -> String:
    return String("hello ") + String(name) + ", you are #" + String(n)

def named_args(x: Float32) -> String:
    return String("x=") + String(x) + " doubled=" + String(x * 2.0)

# TODO(rust-to-mojo): `{:?}` Debug formatting lowered to repr() (output format differs) -- {:?}
def debug_fmt(v: List[Int32]) -> String:
    return String(repr(v))

# TODO(rust-to-mojo): fn `precise` not translated: unsupported format spec (format spec `{:.2}`)
# signature hint: def precise(x: Float32) -> String
# --- original Rust ---
# pub fn precise(x: f32) -> String {
#     format!("{:.2}", x)
# }
# --- end original Rust ---

def shout(msg: String):
    print(String(msg) + "!")
    print(String("no newline"), end="")
    print()

def checked(x: Int32) -> Int32:
    if not x >= 0:
        abort(String("x must be non-negative, got ") + String(x))
    if rem(x, 1) != 0:
        abort("assertion failed: x % 1 == 0")
    debug_assert(x < 1000, "assertion failed: x<1000")
    if x > 100:
        abort(String("too big: ") + String(x))
    return x

def never(x: Int32) -> Int32:
    if x == 0:
        return 1
    else:
        return abort("internal error: entered unreachable code")

# TODO(rust-to-mojo): logging macro dropped (no Mojo logging facade chosen) -- debug!
def logs(x: Int32) -> Int32:
    # TODO(rust-to-mojo): dropped `debug!` logging call
    # log::debug !("x = {}", x)
    return x + 1

# TODO(rust-to-mojo): macro `square` not translated: `macro_rules!` definition (no Mojo macro system)
# --- original Rust ---
# macro_rules! square {
#     ($x:expr) => {
#         $x * $x
#     };
# }
# --- end original Rust ---

# TODO(rust-to-mojo): fn `uses_macro` not translated: macro `square!` (square !(x))
# signature hint: def uses_macro(x: Int32) -> Int32
# --- original Rust ---
# pub fn uses_macro(x: i32) -> i32 {
#     square!(x)
# }
# --- end original Rust ---

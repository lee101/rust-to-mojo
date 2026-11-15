@fieldwise_init
struct Pair:
    var a: Int32
    var b: Int32

def swap(p: Tuple[Int32, Int32]) -> Tuple[Int32, Int32]:
    var x = p[0]
    var y = p[1]
    return (y, x)

def sum_pair(p: Pair) -> Int32:
    var a = p.a
    var b = p.b
    return a + b

def bucket(n: UInt32) -> UInt32:
    if n == 0:
        return 0
    elif n >= 1 and n <= 9:
        return 1
    elif n >= 10 and n <= 99:
        return 2
    else:
        return 3

def quadrant(p: Tuple[Float32, Float32]) -> UInt8:
    if p[0] >= 0.0 and p[1] >= 0.0:
        var x = p[0]
        var y = p[1]
        return 1
    elif p[0] < 0.0:
        var x_1 = p[0]
        return 2
    else:
        return 3

def nested(o: Optional[Optional[Int32]]) -> Int32:
    if o is not None and o.value() is not None:
        var v = o.value().value()
        return v
    elif o is not None and o.value() is None:
        return -1
    else:
        return -2

# TODO(rust-to-mojo): fn `first_of` not translated: slice pattern ([first, ..])
# signature hint: def first_of(xs: List[Int32]) -> Int32
# --- original Rust ---
# pub fn first_of(xs: &[i32]) -> i32 {
#     match xs {
#         [first, ..] => *first,
#         [] => 0,
#     }
# }
# --- end original Rust ---

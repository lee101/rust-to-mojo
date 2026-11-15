from mojo_rt.vec import pop

def pick(a: Optional[Int32], b: Optional[Int32]) -> Int32:
    if a is not None:
        var x = a.value()
        return x
    elif b is not None:
        var y = b.value()
        return y * 10
    else:
        return 0

def drain(var v: List[Int32]) -> Int32:
    var total: Int32 = 0
    while True:
        var __tmp1 = pop(v)
        if __tmp1 is None:
            break
        ref x = __tmp1.value()
        total += x
    return total

def chained(a: Optional[Int32], limit: Int32) -> Bool:
    if a is not None and a.value() > limit:
        var x = a.value()
        return True
    return False

# TODO(rust-to-mojo): fn `let_else` not translated: let-else (let Some(x) = a else { return 0; };)
# signature hint: def let_else(a: Optional[Int32]) -> Int32
# --- original Rust ---
# pub fn let_else(a: Option<i32>) -> i32 {
#     let Some(x) = a else {
#         return 0;
#     };
#     x
# }
# --- end original Rust ---

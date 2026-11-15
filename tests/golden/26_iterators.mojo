from mojo_rt.num import rem
from std.collections import Set

def evens_squared(v: List[Int64]) -> List[Int64]:
    var __acc1: List[Int64] = List[Int64]()
    for __it in v:
        if rem(__it, 2) != 0:
            continue
        var __mapped = __it * __it
        __acc1.append(__mapped)
    return __acc1^

# TODO(rust-to-mojo): slice argument `&[T]` lowered to List[T] borrow -- callers passing sub-slices need Span[T, origin]
def total(v: List[Float32]) -> Float32:
    var __acc2: Float32 = Float32(0)
    for __it in v:
        __acc2 += __it
    return __acc2

def count_big(v: List[UInt32], limit: UInt32) -> Int:
    var __acc3: Int = 0
    for __it in v:
        if not __it > limit:
            continue
        __acc3 += 1
    return __acc3

def all_positive(v: List[Float64]) -> Bool:
    var __acc4: Bool = True
    for __it in v:
        if not __it > 0.0:
            __acc4 = False
            break
    return __acc4

def print_all(v: List[String]):
    for __it in v:
        print(String(__it))

def sum_of_range(n: UInt64) -> UInt64:
    var __acc6: UInt64 = UInt64(0)
    for __it in range(n):
        if rem(__it, 3) != 0:
            continue
        __acc6 += __it
    return __acc6

# TODO(rust-to-mojo): fn `unknown_source` not translated: bound on a generic trait (Iterator<Item = u8>)
# --- original Rust ---
# pub fn unknown_source(xs: impl Iterator<Item = u8>) -> usize {
#     xs.map(|x| x as usize).count()
# }
# --- end original Rust ---

# TODO(rust-to-mojo): fn `into_set` not translated: `.collect()` into a non-Vec collection (v.iter().copied().collect())
# signature hint: def into_set(v: List[Int32]) -> Set[Int32]
# --- original Rust ---
# pub fn into_set(v: &Vec<i32>) -> std::collections::HashSet<i32> {
#     v.iter().copied().collect()
# }
# --- end original Rust ---

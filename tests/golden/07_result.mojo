# TODO(rust-to-mojo): external crate `anyhow` has no Mojo port; dropped: use anyhow::anyhow, anyhow::Result

# TODO(rust-to-mojo): Result<T, E> lowered to `raises` (error type erased) -- E = anyhow::Error
def parse_digit(c: UInt8) raises -> UInt32:
    if c >= 48 and c <= 57:
        return UInt32(c - 48)
    else:
        raise Error(String("not a digit: ") + String(c))

# TODO(rust-to-mojo): Result<T, E> lowered to `raises` (error type erased) -- E = anyhow::Error
def sum_two(a: UInt8, b: UInt8) raises -> UInt32:
    var x = parse_digit(a)
    var y = parse_digit(b)
    return x + y

# TODO(rust-to-mojo): Result<T, E> lowered to `raises` (error type erased) -- E = anyhow::Error
def check(flag: Bool) raises:
    if not flag:
        raise Error(String("flag was false"))

from mojo_rt.num import rem

def sum_to(n: UInt32) -> UInt32:
    var total: UInt32 = 0
    for i in range(n):
        total += i
    return total

def sum_odd_inclusive(a: Int64, b: Int64) -> Int64:
    var s: Int64 = 0
    for i in range(a, b + 1):
        if rem(i, 2) == 0:
            continue
        s += i
    return s

def countdown(var n: Int32) -> Int32:
    var steps: Int32 = 0
    while n > 0:
        n -= 1
        steps += 1
    return steps

def find_first_over(limit: Int32) -> Int32:
    var x: Int32 = 1
    while True:
        x *= 2
        if x > limit:
            break
    return x

def evens(n: Int) -> Int:
    var c: Int = 0
    for _ in range(0, n, 2):
        c += 1
    for i in reversed(range(n)):
        c += i
    return c

def sign(x: Float64) -> Float64:
    if x > 0.0:
        return 1.0
    elif x < 0.0:
        return -1.0
    else:
        return 0.0

# TODO(rust-to-mojo): `&str` return lowered to owned String -- &'static str
def classify(x: Int32) -> String:
    var label = "big" if x > 100 else "small"
    return label

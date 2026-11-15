from mojo_rt.option import unwrap_or

def first_positive(values: List[Int32]) -> Optional[Int32]:
    for v in values:
        if v > 0:
            return v
    return None

def double_or_zero(x: Optional[Int32]) -> Int32:
    if x is not None:
        var v = x.value()
        return v * 2
    else:
        return 0

def add_opts(a: Optional[Int32], b: Optional[Int32]) -> Optional[Int32]:
    var __opt1 = a
    if __opt1 is None:
        return None
    var x = __opt1.value()
    var __opt2 = b
    if __opt2 is None:
        return None
    var y = __opt2.value()
    return x + y

def describe(x: Optional[Float32]) -> Float32:
    if x is not None:
        var v = x.value()
        return v + 1.0
    else:
        return unwrap_or(x, -1.0)

def has(x: Optional[String]) -> Bool:
    return x is not None

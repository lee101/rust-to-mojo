def apply[F: def(Int32) -> Int32](f: F, x: Int32) -> Int32:
    return f(x)

def apply_twice[F1: def(Float32) -> Float32](f: F1, x: Float32) -> Float32:
    return f(f(x))

def typed_closure(k: Int32) -> Int32:
    def __closure1(x: Int32) -> Int32:
        return x + 1
    var add = __closure1
    return add(k)

def capturing(k: Int32) -> Int32:
    def __closure2(x: Int32) {imm k} -> Int32:
        return x * k
    var scale = __closure2
    return scale(3)

def untyped(v: List[Int32]) -> List[Int32]:
    var __acc3: List[Int32] = List[Int32]()
    for __it in v:
        var __mapped = __it * 2
        __acc3.append(__mapped)
    return __acc3^

def any_negative(v: List[Int32]) -> Bool:
    var __acc4: Bool = False
    for __it in v:
        if __it < 0:
            __acc4 = True
            break
    return __acc4

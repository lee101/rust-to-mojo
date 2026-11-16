@fieldwise_init
struct Buffer:
    var data: List[UInt8]
    var name: String

def wrap(var data: List[UInt8], var name: String) -> Buffer:
    return Buffer(data=data^, name=name^)

def rename(var b: Buffer, var name: String) -> Buffer:
    return Buffer(data=b.data.copy(), name=name^)

def shadow(x: Int32) -> Int32:
    var x_1 = x + 1
    var x_2 = x_1 * 2
    var x_3 = x_2 - 1
    var y = x_3
    return x_2 + y

def takes_twice(var v: List[UInt8]) -> Int:
    var n = len(v)
    var w = v^
    return n + len(w)

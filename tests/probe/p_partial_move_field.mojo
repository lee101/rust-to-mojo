@fieldwise_init
struct Buffer(Movable):
    var data: List[UInt8]
    var name: String


def rename(var b: Buffer, var name: String) -> Buffer:
    return Buffer(data=b.data^, name=name^)


def main():
    var b = Buffer([UInt8(1), 2], "a")
    var c = rename(b^, "b")
    print(len(c.data), c.name)

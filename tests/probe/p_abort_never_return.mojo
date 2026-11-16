from std.os import abort

def never(x: Int32) -> Int32:
    if x == 0:
        return 1
    else:
        abort("unreachable")


def main():
    print(never(0))

from g12_macros import greet, named_args, debug_fmt, shout, checked, never, logs


def main():
    print(greet("bob", 3))
    print(named_args(1.5))
    print(named_args(2.0))
    print(named_args(0.1))
    var v: List[Int32] = [1, -2, 3]
    print(debug_fmt(v))
    shout("hey")
    print(checked(5), never(0), logs(9))

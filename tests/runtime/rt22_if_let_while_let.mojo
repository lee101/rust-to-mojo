from g22_if_let_while_let import pick, drain, chained


def main():
    print(pick(Int32(3), Int32(4)), pick(None, Int32(4)), pick(None, None))
    var v: List[Int32] = [1, 2, 3, 4]
    print(drain(v^), drain(List[Int32]()))
    print(Int(chained(Int32(5), 3)), Int(chained(Int32(2), 3)), Int(chained(None, 3)))

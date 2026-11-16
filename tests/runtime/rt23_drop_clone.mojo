from g23_drop_clone import Handle, Tiny


def main():
    var h = Handle(id=1, tag=String("t"))
    var c = h.copy()
    print("clone", c.id, c.tag)
    _ = c^
    print("after drop")
    var t = Tiny(7)
    var u = t
    print("tiny", u._0)
    _ = h^
    print("end")

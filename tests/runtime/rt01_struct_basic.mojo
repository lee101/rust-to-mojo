from g01_struct_basic import Counter, make_default


def main():
    var c = Counter.new(String("x"), 3)
    c.tick()
    c.tick()
    print(c.get(), Int(c.is_positive()))
    var old = c.reset()
    print(old, c.get(), Int(c.is_positive()))
    var d = make_default()
    print(d.label, d.step, d.count)

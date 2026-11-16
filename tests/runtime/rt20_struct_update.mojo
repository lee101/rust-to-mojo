from g20_struct_update import Style, wide, hidden, from_fn
from std.memory import bitcast


def p(s: Style):
    print(bitcast[DType.uint32](s.width), bitcast[DType.uint32](s.height), Int(s.visible))


def main():
    p(wide())
    p(hidden(Style(width=3.0, height=4.0, visible=True)))
    p(from_fn())

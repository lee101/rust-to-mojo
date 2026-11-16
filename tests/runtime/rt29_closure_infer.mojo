from g29_closure_infer import App, Window, grow, positives, first_len, or_default, is_big
from std.memory import bitcast


def main():
    var app = App(windows=List[Window](), scale=1.5)
    var a = app.demo()
    print(bitcast[DType.uint32](a), bitcast[DType.uint32](app.scale))
    var w = Window(width=5.0)
    grow(w)
    print(bitcast[DType.uint32](w.width))
    var xs: List[Int32] = [1, -2, 3, 0]
    print(positives(xs))
    print(first_len(Optional(String("héllo"))), first_len(None))
    print(or_default(Int32(4)), or_default(None))
    print(Int(is_big(Int32(101))), Int(is_big(Int32(1))), Int(is_big(None)))

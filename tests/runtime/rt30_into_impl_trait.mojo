from g30_into_impl_trait import Length, px_len, measure, unit_square, Half_f32_half, label_len
from std.memory import bitcast


def main():
    var l = px_len(2.5)
    if l.isa_Px():
        print("px", bitcast[DType.uint32](l.as_Px()._0._0))
    else:
        print("rem")
    print(bitcast[DType.uint32](measure(4.25)))
    print(bitcast[DType.uint32](unit_square().area()))
    print(bitcast[DType.uint32](Half_f32_half(3.0)))
    var s = String("abcd")
    print(label_len(Optional(s)), label_len(None))

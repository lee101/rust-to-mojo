from g04_enum_clike import Axis, Corner, corner_code


def main():
    var v = Axis.Vertical
    var h = v.invert()
    print(Int(v.is_vertical()), Int(h.is_vertical()), Int(h.invert() == v))
    print(Int(v < h), Int32(Int(v)) + Int32(Int(h)))
    var cs: List[Corner] = [Corner.TopLeft, Corner.TopRight, Corner.BottomLeft, Corner.BottomRight, Corner()]
    for c in cs:
        print(corner_code(c), Int32(Int(c)))

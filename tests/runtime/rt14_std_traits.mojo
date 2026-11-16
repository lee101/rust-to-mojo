from g14_std_traits import Rgb, white, black, fallback


def main():
    var w = white()
    print(w, repr(w))
    var c = Rgb(r=1, g=2, b=3)
    print(c, repr(c))
    print(black(), fallback())
    print(Int(black() == fallback()))
    print(Rgb.from_(0x123456))

from mojo_rt import option as rt_option

def main():
    var o = Optional(Int32(4))
    def dbl(x: Int32) -> Float64:
        return Float64(x) * 1.5
    var r = rt_option.map(o, dbl)
    print(r.value())

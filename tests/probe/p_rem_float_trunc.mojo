from std.math import trunc
def main():
    var a = Float64(-7.5)
    var b = Float64(2.0)
    print(a - trunc(a / b) * b, a % b)

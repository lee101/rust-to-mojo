from g08_control_flow import sum_to, sum_odd_inclusive, countdown, find_first_over, evens, sign, classify
from std.memory import bitcast


def main():
    print(sum_to(10), sum_to(0))
    print(sum_odd_inclusive(-5, 9), sum_odd_inclusive(3, 3))
    print(countdown(7), countdown(-2))
    print(find_first_over(100), find_first_over(0))
    print(evens(7), evens(0))
    print(bitcast[DType.uint64](sign(2.5)), bitcast[DType.uint64](sign(-0.1)), bitcast[DType.uint64](sign(0.0)))
    print(classify(101), classify(5))

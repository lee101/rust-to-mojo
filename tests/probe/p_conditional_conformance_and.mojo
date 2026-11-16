@fieldwise_init
struct W[T: Copyable & Deinitable](Copyable, Equatable where conforms_to(T, Equatable), Hashable where conforms_to(T, Hashable) and conforms_to(T, Equatable)):
    var v: Self.T


def main():
    print(W(1) == W(1))

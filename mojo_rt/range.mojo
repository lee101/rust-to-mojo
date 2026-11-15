"""A value-level `std::ops::Range<T>` (half-open)."""


@fieldwise_init
struct RustRange[T: ImplicitlyCopyable & Comparable & Deinitable](ImplicitlyCopyable):
    var start: Self.T
    var end: Self.T

    def contains(self, value: Self.T) -> Bool:
        return self.start <= value and value < self.end

    def is_empty(self) -> Bool:
        return not (self.start < self.end)

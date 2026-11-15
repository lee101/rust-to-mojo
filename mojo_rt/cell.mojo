"""Cell / RefCell stand-ins.

Rust's `Cell::set` and `RefCell::borrow_mut` mutate through a shared
reference. Mojo has no interior mutability through immutable references, so
these methods take `mut self`; call sites holding only an immutable borrow
will fail to compile and need restructuring (an agent fix-up task).

RefCell's dynamic borrow flags are NOT enforced: a program that would panic
with `BorrowMutError` in Rust will not panic here. This is recorded as a TODO
on every generated use.
"""


struct Cell[T: Copyable & Deinitable](Copyable):
    var _value: Self.T

    def __init__(out self, var value: Self.T):
        self._value = value^

    def get(self) -> Self.T:
        return self._value.copy()

    def set(mut self, var value: Self.T):
        self._value = value^

    def replace(mut self, var value: Self.T) -> Self.T:
        var old = self._value.copy()
        self._value = value^
        return old^


struct RefCell[T: Movable & Deinitable](Movable):
    var _value: Self.T

    def __init__(out self, var value: Self.T):
        self._value = value^

    def borrow(ref self) -> ref [self._value] Self.T:
        return self._value

    def borrow_mut(mut self) -> ref [self._value] Self.T:
        return self._value

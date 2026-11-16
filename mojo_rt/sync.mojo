"""Mutex / RwLock stand-ins.

GPUI keeps state behind `parking_lot::Mutex`/`RwLock` (and occasionally
`std::sync` or `spin` locks). Mojo 1.0's standard library has no mutex, and
the code rust-to-mojo emits is single-threaded, so these shims hold the value
directly:

  * `lock()`, `read()`, `write()` and `get_mut()` return a reference to the
    value instead of a guard. A Rust `let guard = m.lock();` is emitted as
    `ref guard = m.lock()`, and the guard's scope-based unlock has no
    counterpart, because nothing is ever locked.
  * There is NO mutual exclusion. Code that really shares the value across
    threads must be ported by hand (for example over `std.os.atomic`). Every
    generated use carries a TODO saying so.
  * Rust mutates through `&self` here (interior mutability). These methods take
    `ref self`, so a call site that only holds an immutable borrow gets an
    immutable reference and mutation through it fails to compile. That is a
    deliberate, loud fix-up point, as with `mojo_rt.cell.RefCell`.

Target: Mojo 1.0. Not compiled yet.
"""


struct Mutex[T: Movable & Deinitable](Movable):
    var _value: Self.T

    def __init__(out self, var value: Self.T):
        self._value = value^

    def lock(ref self) -> ref [self._value] Self.T:
        return self._value

    def try_lock(ref self) -> ref [self._value] Self.T:
        """Never fails: there is no contention in the shim."""
        return self._value

    def get_mut(mut self) -> ref [self._value] Self.T:
        return self._value

    def into_inner(deinit self) -> Self.T:
        return self._value^


struct RwLock[T: Movable & Deinitable](Movable):
    var _value: Self.T

    def __init__(out self, var value: Self.T):
        self._value = value^

    def read(ref self) -> ref [self._value] Self.T:
        return self._value

    def write(ref self) -> ref [self._value] Self.T:
        return self._value

    def get_mut(mut self) -> ref [self._value] Self.T:
        return self._value

    def into_inner(deinit self) -> Self.T:
        return self._value^

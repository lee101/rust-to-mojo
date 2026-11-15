"""Option helpers. Mojo's `Optional.take()` / `or_else()` consume or abort; Rust's
Option methods return Options or copies, so generated code goes through these."""


def unwrap_or[T: Copyable & Deinitable](opt: Optional[T], default: T) -> T:
    """Rust `opt.unwrap_or(default)` without consuming `opt`."""
    if opt:
        return opt.value().copy()
    return default.copy()


def unwrap_or_default[T: Copyable & Defaultable & Deinitable](opt: Optional[T]) -> T:
    """Rust `opt.unwrap_or_default()`."""
    if opt:
        return opt.value().copy()
    return T()


def take[T: Movable & Deinitable](mut opt: Optional[T]) -> Optional[T]:
    """Rust `opt.take()`: move the value out, leaving `None`; never aborts."""
    if opt:
        return Optional(opt.take())
    return None


def replace[T: Movable & Deinitable](mut opt: Optional[T], var value: T) -> Optional[T]:
    """Rust `opt.replace(value)`: store `value`, return the previous contents."""
    var old = take(opt)
    opt = Optional(value^)
    return old^

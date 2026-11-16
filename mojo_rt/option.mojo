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


# ---- combinators taking a closure (Option::map and friends) ----------------
# rust-to-mojo passes the closure as a nested `def` whose parameter types come
# from the Option's element type. UNVERIFIED: inference of `U` from the
# closure's return type when calling `map(opt, f)`.


def map[T: Copyable & Deinitable, U: Copyable & Deinitable, F: def(T) -> U](opt: Optional[T], f: F) -> Optional[U]:
    """Rust `opt.map(f)`."""
    if opt:
        return Optional[U](f(opt.value()))
    return None


def and_then[T: Copyable & Deinitable, U: Copyable & Deinitable, F: def(T) -> Optional[U]](opt: Optional[T], f: F) -> Optional[U]:
    """Rust `opt.and_then(f)`."""
    if opt:
        return f(opt.value())
    return None


def unwrap_or_else[T: Copyable & Deinitable, F: def() -> T](opt: Optional[T], f: F) -> T:
    """Rust `opt.unwrap_or_else(f)`: `f` runs only when `opt` is None."""
    if opt:
        return opt.value().copy()
    return f()


def or_else[T: Copyable & Deinitable, F: def() -> Optional[T]](opt: Optional[T], f: F) -> Optional[T]:
    """Rust `opt.or_else(f)`."""
    if opt:
        return opt.copy()
    return f()


def map_or[T: Copyable & Deinitable, U: Copyable & Deinitable, F: def(T) -> U](opt: Optional[T], var default: U, f: F) -> U:
    """Rust `opt.map_or(default, f)`."""
    if opt:
        return f(opt.value())
    return default^


def is_some_and[T: Copyable & Deinitable, F: def(T) -> Bool](opt: Optional[T], f: F) -> Bool:
    """Rust `opt.is_some_and(f)`."""
    if opt:
        return f(opt.value())
    return False


def is_none_or[T: Copyable & Deinitable, F: def(T) -> Bool](opt: Optional[T], f: F) -> Bool:
    """Rust `opt.is_none_or(f)`."""
    if opt:
        return f(opt.value())
    return True


def filter[T: Copyable & Deinitable, F: def(T) -> Bool](opt: Optional[T], f: F) -> Optional[T]:
    """Rust `opt.filter(f)`."""
    if opt and f(opt.value()):
        return opt.copy()
    return None


def inspect[T: Copyable & Deinitable, F: def(T) -> None](opt: Optional[T], f: F) -> Optional[T]:
    """Rust `opt.inspect(f)`."""
    if opt:
        f(opt.value())
    return opt.copy()

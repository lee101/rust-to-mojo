"""Vec helpers with Rust return types."""


def pop[T: Copyable & Deinitable](mut v: List[T]) -> Optional[T]:
    """Rust `v.pop()`: `None` on an empty vector (Mojo's `List.pop()` aborts)."""
    if len(v) == 0:
        return None
    return Optional(v.pop())


def get[T: Copyable & Deinitable](v: List[T], i: Int) -> Optional[T]:
    """Rust `v.get(i).cloned()`."""
    if i < 0 or i >= len(v):
        return None
    return Optional(v[i].copy())


def first[T: Copyable & Deinitable](v: List[T]) -> Optional[T]:
    return get(v, 0)


def last[T: Copyable & Deinitable](v: List[T]) -> Optional[T]:
    return get(v, len(v) - 1)


def truncate[T: Copyable & Deinitable](mut v: List[T], n: Int):
    while len(v) > n:
        _ = v.pop()


def swap[T: Copyable & Deinitable](mut v: List[T], i: Int, j: Int):
    if i == j:
        return
    var tmp = v[i].copy()
    v[i] = v[j].copy()
    v[j] = tmp^


def vec_repeat[T: Copyable & Deinitable](value: T, n: Int) -> List[T]:
    """Rust `vec![value; n]`."""
    var out = List[T](capacity=n)
    for _ in range(n):
        out.append(value.copy())
    return out^

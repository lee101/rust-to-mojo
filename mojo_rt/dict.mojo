"""HashMap helpers."""


def remove[K: KeyElement, V: Copyable & Deinitable](mut d: Dict[K, V], key: K) -> Optional[V]:
    """Rust `map.remove(&key)`: `None` when absent (Mojo's `Dict.pop` raises)."""
    try:
        return Optional(d.pop(key))
    except:
        return None

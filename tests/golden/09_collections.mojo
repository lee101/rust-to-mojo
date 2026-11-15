def squares(n: Int) -> List[Int]:
    var out_: List[Int] = List[Int]()
    for i in range(n):
        out_.append(i * i)
    return out_

# TODO(rust-to-mojo): slice argument `&[T]` lowered to List[T] borrow -- callers passing sub-slices need Span[T, origin]
def total(values: List[Float32]) -> Float32:
    var t: Float32 = 0.0
    for v in values:
        t += v
    return t

def indexed(values: List[Int32]) -> Int32:
    var acc: Int32 = 0
    for i, v in enumerate(values):
        acc += Int32(i) * v
    if len(values) == 0:
        return -1
    return acc + values[0] + Int32(len(values))

def word_counts(words: List[String]) -> Dict[String, Int]:
    var counts: Dict[String, Int] = Dict[String, Int]()
    for w in words:
        if w in counts:
            var c = counts.get(w).value()
            counts[w.copy()] = c + 1
        else:
            counts[w.copy()] = 1
    return counts

def sum_map(m: Dict[String, Int32]) -> Int32:
    var s: Int32 = 0
    for __entry1 in m.items():
        ref _k = __entry1.key
        ref v = __entry1.value
        s += v
    return s

def literal() -> List[Int32]:
    var v: List[_] = [1, 2, 3]
    return v^

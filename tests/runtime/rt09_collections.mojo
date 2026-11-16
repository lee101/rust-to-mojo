from g09_collections import squares, total, indexed, word_counts, sum_map, literal
from std.memory import bitcast


def main() raises:
    var s = squares(5)
    print(len(s), s[2], s[4])
    var f: List[Float32] = [1.5, 2.25, -0.5]
    print(bitcast[DType.uint32](total(f)))
    var a: List[Int32] = [3, 4, 5]
    print(indexed(a), indexed(List[Int32]()))
    var words: List[String] = ["a", "b", "a", "c", "a"]
    var c = word_counts(words)
    print(len(c), c["a"], c["b"], c["c"])
    var m = Dict[String, Int32]()
    m["x"] = 4
    m["y"] = -9
    print(sum_map(m))
    var l = literal()
    print(l[0], l[1], l[2])

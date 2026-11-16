@fieldwise_init
struct Span[origin_a: ImmOrigin]:
    var text: Pointer[String, Self.origin_a]
    var start: Int

    def len_(self) -> Int:
        return self.text[].byte_length() - self.start


def make(text: String) -> Span[origin_of(text)]:
    return Span(text=Pointer(to=text), start=1)


def use_any(s: Span[_]) -> Int:
    return s.len_()


def main():
    var t = String("hello")
    var s = Span(text=Pointer(to=t), start=1)
    print(s.len_(), use_any(s), make(t).len_())

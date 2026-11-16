@fieldwise_init
struct Span[origin_a: ImmOrigin]:
    var text: Pointer[String, Self.origin_a]
    var start: Int

    @staticmethod
    def new(ref [Self.origin_a] text: String, start: Int) -> Self:
        return Span(text=Pointer(to=text), start=start)

    def len_(self) -> Int:
        return self.text[].byte_length() - self.start


def main():
    var t = String("hello")
    var s = Span.new(t, 1)
    print(s.len_())

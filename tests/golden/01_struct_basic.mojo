# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Counter(Copyable, Equatable, Writable):
    """A 2D counter with a name."""
    # Current count.
    var count: Int64
    var step: Int64
    var label: String

    @staticmethod
    def new(var label: String, step: Int64) -> Self:
        """Creates a counter starting at zero."""
        return Counter(count=0, step=step, label=label^)

    def get(self) -> Int64:
        return self.count

    def tick(mut self):
        self.count += self.step

    def reset(mut self) -> Int64:
        var old = self.count
        self.count = 0
        return old

    def is_positive(self) -> Bool:
        return self.count > 0 and self.step != 0

def make_default() -> Counter:
    return Counter.new(String("default"), 1)

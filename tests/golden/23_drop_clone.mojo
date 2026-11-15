@fieldwise_init
struct Handle(Copyable):
    var id: UInt32
    var tag: String

    def __init__(out self, *, copy: Self):
        self = Handle(id=copy.id + 1000, tag=copy.tag.copy())

    def __deinit__(deinit self):
        print(String("dropping ") + String(self.id))

@fieldwise_init
struct Tiny(ImplicitlyCopyable):
    var _0: UInt8

@fieldwise_init
struct S:
    var id: UInt32

    def id(self) -> UInt32:
        return self.id


def main():
    print(S(3).id())

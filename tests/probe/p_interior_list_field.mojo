@fieldwise_init
struct Grid:
    var cells: List[Float32]

    def first(self) -> ref [self.cells] Float32:
        return self.cells[0]


def main():
    var g = Grid([Float32(1.5), 2.5])
    print(g.first())

@fieldwise_init
struct Grid:
    var cells: List[Float32]
    var width: Int

    def __getitem__(self, i: Int) -> ref [self] Float32:
        return self.cells[i]

    # TODO(rust-to-mojo): method `Grid.row` not translated: returning a slice / trait-object reference (&[f32])
    # --- original Rust ---
    # pub fn row(&self, r: usize) -> &[f32] {
    #     &self.cells[r * self.width..(r + 1) * self.width]
    # }
    # --- end original Rust ---

    def get(self, x: Int, y: Int) -> Float32:
        return self.cells[y * self.width + x]

    def first(self) -> ref [self] Float32:
        return self.cells[0]

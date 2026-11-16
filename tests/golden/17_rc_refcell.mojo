from mojo_rt.cell import Cell, RefCell
from std.memory import ArcPointer

@fieldwise_init
struct Node:
    var value: Int32
    var visits: Cell[UInt32]

# TODO(rust-to-mojo): Rc lowered to ArcPointer (atomic refcount) -- Rc<T> -> ArcPointer[T]
# TODO(rust-to-mojo): RefCell lowered to mojo_rt.cell.RefCell (no dynamic borrow checking) -- RefCell::new(vec ![v])
# TODO(rust-to-mojo): RefCell lowered to mojo_rt.cell.RefCell (no dynamic borrow checking) -- other.borrow_mut()
def shared(v: Int32) -> ArcPointer[RefCell[List[Int32]]]:
    var __vec1: List[_] = [v]
    var data = ArcPointer(RefCell(__vec1^))
    var other = data.copy()
    other[].borrow_mut().append(v + 1)
    return data^

# TODO(rust-to-mojo): fn `count` not translated: interior mutability through a shared reference (`Cell` mutated via `&`; Mojo has none) (n.visits.set(n.visits.get() + 1))
# signature hint: def count(n: Node) -> UInt32
# --- original Rust ---
# pub fn count(n: &Node) -> u32 {
#     n.visits.set(n.visits.get() + 1);
#     n.visits.get()
# }
# --- end original Rust ---

"""Erased: the placeholder type for a Rust struct field that Mojo 1.0 cannot
represent yet.

rust-to-mojo lowers a field such as

    platform: Rc<dyn Platform>,
    on_close: Option<Box<dyn FnOnce(&mut App)>>,

to `var platform: Erased`, so that the struct itself (its layout, its other
fields, and every method that does not touch the field) still translates.
Mojo 1.0 has no existential (`dyn Trait`) types, and a capturing closure has
no nameable function type, so there is nothing faithful to put there.

Every piece of generated code that reads, writes or constructs an erased
field is stubbed with a `TODO(rust-to-mojo)` and the original Rust, so an
`Erased` value is never used for anything except being carried around. Its
reflection-based `Equatable`/`Hashable`/`Writable` defaults exist only so that
derives on the enclosing struct still type-check; they compare and print
the placeholder, not the Rust value. See MAPPING.md section 2.

Target: Mojo 1.0. Not compiled yet.
"""


@fieldwise_init
struct Erased(ImplicitlyCopyable, Movable, Defaultable, Equatable, Hashable, Writable):
    """An opaque, always-empty stand-in for an untranslatable field value."""

    var _handle: Int

    def __init__(out self):
        self._handle = 0

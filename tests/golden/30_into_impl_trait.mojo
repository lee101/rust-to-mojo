"""`.into()` targets from argument/variant types, `-> impl Trait` with a
concrete tail, and impls of local traits for primitives.
"""

from std.os import abort
from std.utils import Variant

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Pixels(ImplicitlyCopyable, Equatable, Writable):
    var _0: Float32

    @staticmethod
    def from_(v: Float32) -> Self:
        return Pixels(v)

# NOTE(rust-to-mojo): derive(Debug) -> Writable (reflection-based output; format differs from Rust's Debug)
@fieldwise_init
struct Rems(ImplicitlyCopyable, Equatable, Writable):
    var _0: Float32

@fieldwise_init
struct Length_Px(Copyable, ImplicitlyCopyable, Writable, Equatable):
    """Payload of `Length::Px`."""
    var _0: Pixels

@fieldwise_init
struct Length_Rem(Copyable, ImplicitlyCopyable, Writable, Equatable):
    """Payload of `Length::Rem`."""
    var _0: Rems

# TODO(rust-to-mojo): Debug on data enum maps to Variant's Writable (format differs from Rust) -- Length
# Rust enum `Length` lowered to a tagged struct over Variant (see MAPPING.md#enums).
@fieldwise_init
struct Length(Copyable, ImplicitlyCopyable, Writable, Equatable):
    var _v: Variant[Length_Px, Length_Rem]

    @staticmethod
    def Px(var _0: Pixels) -> Self:
        return Self(Length_Px(_0^))

    @staticmethod
    def Rem(var _0: Rems) -> Self:
        return Self(Length_Rem(_0^))

    def isa_Px(self) -> Bool:
        return self._v.isa[Length_Px]()

    def as_Px(ref self) -> ref [self._v] Length_Px:
        if not self._v.isa[Length_Px]():
            abort("Length::Px: wrong enum variant")
        return self._v.unsafe_get[Length_Px]()

    def isa_Rem(self) -> Bool:
        return self._v.isa[Length_Rem]()

    def as_Rem(ref self) -> ref [self._v] Length_Rem:
        if not self._v.isa[Length_Rem]():
            abort("Length::Rem: wrong enum variant")
        return self._v.unsafe_get[Length_Rem]()

def px_len(v: Float32) -> Length:
    return Length.Px(Pixels.from_(v))

def width_of(p: Pixels) -> Float32:
    return p._0

def measure(v: Float32) -> Float32:
    return width_of(Pixels.from_(v))

trait Shape:
    def area(self) -> Float32:
        ...

@fieldwise_init
struct Square(Shape):
    var side: Float32

    def area(self) -> Float32:
        return self.side * self.side

def unit_square() -> Square:
    return Square(side=1.0)

trait Half:
    def half(self) -> Self:
        ...

# TODO(rust-to-mojo): extension impl lowered to a free function (Mojo has no extensions) -- impl Half for f32 -> Half_f32_half(self, ...); method-syntax call sites are not rewritten
def Half_f32_half(self: Float32) -> Float32:
    return self / 2.0

# TODO(rust-to-mojo): fn `evens` not translated: `impl Trait` return type (impl Iterator<Item = u32>)
# --- original Rust ---
# /// No concrete tail type: stays a stub.
# pub fn evens(n: u32) -> impl Iterator<Item = u32> {
#     (0..n).filter(|x| x % 2 == 0)
# }
# --- end original Rust ---

# TODO(rust-to-mojo): `Option<&T>` lowered to `Optional[T]` (copies the referent) -- Option<&String>
def label_len(label: Optional[String]) -> Int:
    """`Option<&T>` arguments become `Optional[T]` (a copy of the referent)."""
    if label is not None:
        ref l = label.value()
        return l.byte_length()
    else:
        return 0

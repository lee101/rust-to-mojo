"""One untranslatable field no longer stubs the whole type."""

# TODO(rust-to-mojo): external crate `parking_lot` has no Mojo port; dropped: use parking_lot::Mutex

from mojo_rt.erased import Erased
from mojo_rt.sync import Mutex
from std.memory import Pointer

trait Platform:
    def name(self) -> String:
        ...

# TODO(rust-to-mojo): field type erased to `Erased` (code using the field is stubbed) -- App.platform: `dyn Trait` (no existential types in Mojo) (dyn Platform)
# TODO(rust-to-mojo): field type erased to `Erased` (code using the field is stubbed) -- App.on_quit: `dyn Trait` (no existential types in Mojo) (dyn FnOnce())
@fieldwise_init
struct App:
    var platform: Erased
    var on_quit: Erased
    var frame_count: Int
    var title: String

    def next_frame(mut self) -> Int:
        """Does not touch the erased fields: translates."""
        self.frame_count += 1
        return self.frame_count

    # TODO(rust-to-mojo): String len(): Rust counts UTF-8 bytes; verify Mojo String len semantics -- self.title.len()
    def title_len(self) -> Int:
        return len(self.title)

    # TODO(rust-to-mojo): method `App.platform_name` not translated: uses a field erased to `Erased` (cause: `dyn Trait` (no existential types in Mojo)) (self.platform)
    # signature hint: def platform_name(self) -> String
    # --- original Rust ---
    # /// Reads an erased field: stubbed.
    # pub fn platform_name(&self) -> String {
    #     self.platform.name()
    # }
    # --- end original Rust ---

    # TODO(rust-to-mojo): method `App.new` not translated: `dyn Trait` (no existential types in Mojo) (dyn Platform)
    # --- original Rust ---
    # /// Constructs the erased field: stubbed.
    # pub fn new(platform: Rc<dyn Platform>) -> App {
    #     App { platform, on_quit: None, frame_count: 0, title: String::new() }
    # }
    # --- end original Rust ---

# TODO(rust-to-mojo): lock lowered to mojo_rt.sync shim (no mutual exclusion; guards are plain references) -- Mutex<u32> -> mojo_rt.sync.Mutex
@fieldwise_init
struct Counter:
    var hits: Mutex[UInt32]

    # TODO(rust-to-mojo): lock lowered to mojo_rt.sync shim (no mutual exclusion; guards are plain references) -- self.hits.lock()
    def bump(self) -> UInt32:
        ref hits = self.hits.lock()
        hits += 1
        return hits

    # TODO(rust-to-mojo): lock lowered to mojo_rt.sync shim (no mutual exclusion; guards are plain references) -- self.hits.lock()
    def read_(self) -> UInt32:
        return self.hits.lock()

# TODO(rust-to-mojo): borrowed-reference field lowered to `Pointer` with an origin parameter -- Span.text: &'a String
@fieldwise_init
struct Span[origin_a: ImmOrigin]:
    var text: Pointer[String, Self.origin_a]
    var start: Int

    # TODO(rust-to-mojo): borrowed-reference field initialised with `Pointer(to=...)` (the struct's origin parameter must be inferable at the call site) -- Span.text
    @staticmethod
    def new(text: String, start: Int) -> Self:
        return Span(text=Pointer(to=text), start=start)

    # TODO(rust-to-mojo): String len(): Rust counts UTF-8 bytes; verify Mojo String len semantics -- self.text.len()
    def len_(self) -> Int:
        return len(self.text[]) - self.start

# TODO(rust-to-mojo): `&'static` field stored by value (copy of immutable static data) -- Meta.names: &'static[&'static str]
@fieldwise_init
struct Meta:
    var names: List[String]
    var id: UInt32

    def get_id(self) -> UInt32:
        return self.id

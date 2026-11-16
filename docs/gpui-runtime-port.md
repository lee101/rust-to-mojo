# Design proposal: porting the GPUI runtime to Mojo 1.0

Status: **proposal**, not yet validated. No Mojo toolchain was available while writing it,
so none of the Mojo below has been compiled. Each Mojo feature this design relies on is
tagged either **[verified]** (stated in the Mojo 1.0 release notes or manual on
mojolang.org, checked November 2026) or **[unverified]** (believed to work, not
confirmed). Section 7 collects the unverified items.

Source of truth: zed `596dfdc`, `crates/gpui` (checked out under
`target/corpus/zed/crates/gpui`). Coverage figures come from
[reports/gpui/crate.coverage.md](../reports/gpui/crate.coverage.md).

---

## 1. Summary

* The translator already handles GPUI's **value layer** well (geometry 80%, scene 77%,
  style 77%, keystroke 85%, interactive 84% emitted). The **runtime layer** (App,
  Window, Context, element erasure, executor, platform) comes out at 0–20% and should not be
  forced through the translator. It needs a hand design, and this document proposes one.
* The core rule for working without `dyn`:
  **closed sets use `Variant`, and open sets use an erased box with a vtable built at comptime.**
  GPUI decides some sets itself: platforms, atlases, `PlatformInput`, `Effect`, scene
  primitives. Those become `Variant`s. Applications decide others: entity types, element
  types, event types, globals, actions, callbacks. Those become `(opaque pointer, vtable)`
  pairs, where the vtable is a struct of thin function pointers instantiated per type.
  GPUI already does this in two places: `ArenaElement { value: *mut u8, drop: unsafe fn(*mut u8) }`
  in `src/arena.rs`, and `AnyView { render: fn(&AnyView, &mut Window, &mut App) -> AnyElement }`
  in `src/view.rs`.
* Callbacks (`Box<dyn FnMut(..)>`, `Rc<dyn Fn(..)>`, about 30 distinct signatures) become
  **hand-rolled closure objects**: `state: OpaquePtr`, `call: def(OpaquePtr, ...) thin`,
  `drop: def(OpaquePtr) thin`. A generic constructor builds them from any `F` that conforms to
  the matching function trait.
* Platform recommendation: **SDL3 (windowing, input, clipboard, IME) plus wgpu-native
  (`webgpu.h`) for rendering**, with GPUI's WGSL shaders from `gpui_wgpu` reused as they are.
  SDL3 plus SDL_gpu is the fallback if `webgpu.h` churn or its callback-heavy API proves painful.
* Layout and text start out as **thin Rust `cdylib` shims with a C ABI** around the exact crates
  GPUI uses: taffy `=0.15.0`, and a cosmic-text-based text system. That gives behavioural
  parity early. Moving them to native Mojo comes later and is optional.
* The executor does not use Mojo `async`, which the stdlib describes as unfinished.
  Instead there is a main-thread run queue, a pthread pool, and continuation-style `Task[R]`.

## 2. Goals, non-goals, and strategy

### Goals

1. A Mojo package `gpui` whose public shape follows GPUI's (`App`, `Entity[T]`,
   `Context[T]`, `Window`, `Render`, `div()` builders, `px()`, `rgb()`), so that GPUI
   *application* code translates as mechanically as possible.
2. Linux (Wayland and X11) and macOS desktop windows that render GPUI's scene primitives at
   parity with Zed's renderer.
3. A deterministic headless test platform, so the runtime can be tested without a GPU.
4. The translator stays the main tool. Hand-written code is limited to the runtime core and
   backends, and goes into a recorded overlay (see section 6) so re-translation never
   overwrites it.

### Non-goals (for this proposal)

Windows and web backends, accessibility (`accesskit`, `src/window/a11y.rs`), the
inspector, the profiler (`src/profiler*`), hot reload, screen capture (`scap`), layer-shell,
system notifications, app menus, `serde`/`schemars` JSON schemas, and Zed itself. Images
(`img.rs`, `image` crate) and SVG (`svg_renderer.rs`, `resvg`/`usvg`) are deferred to after M5.

### Where the platform code lives

The platform backends are **not in `crates/gpui`**. The workspace `Cargo.toml` lists
`gpui_linux`, `gpui_macos`, `gpui_apple`, `gpui_windows`, `gpui_web`, `gpui_wgpu` and
`gpui_platform` as separate crates, and `gpui` only takes `gpui_platform` as a
dev-dependency. Only `gpui` is checked out in our corpus. `crates/gpui/src/platform/`
holds the traits and shared data (`platform.rs`, `keystroke.rs`, `app_menu.rs`), the
`test/` platform, and `threaded_dispatcher.rs`. So the trait surface is in hand, but the
real backends (including `gpui_wgpu`'s WGSL shaders) still need to be fetched and read. The
statements below about the wgpu renderer come from the workspace layout and should be checked
against that crate.

### Per-module plan

Legend: **M** = mechanical (translate, light review). **M+F** = mechanical plus fix-up
passes (stubs to finish by hand or by teaching the translator). **H** = hand design (the
translator output is reference material only). "emitted" is the current coverage figure.

| module (src/...) | lines | emitted | plan | reason |
| --- | ---: | ---: | --- | --- |
| `geometry.rs` | 4162 | 80% | M+F | pure values. Remaining stubs: generic arithmetic bounds (`Add<Output=T>`), `impl ... for f32` (no extensions) |
| `color.rs`, `colors.rs` | 1193 | 71–83% | M | pure values; `Serialize`/`JsonSchema` dropped (non-goal) |
| `scene.rs` | 1271 | 77% | M+F | `#[repr(C)]` primitives (`Quad`, `Shadow`, `Underline`, `MonochromeSprite`, `SubpixelSprite`, `PolychromeSprite`, `PathTile`...) need layout asserts, and `Primitive`/`PrimitiveBatch` are `Variant`s |
| `bounds_tree.rs` | 500 | 73% | M+F | draw-order spatial index. One `unsafe` |
| `style.rs` | 1529 | 77% | M+F | `#[derive(Refineable)]` produces `StyleRefinement`, which needs a generator |
| `styled.rs` | 904 | 50% | M+F (generator) | almost all methods come from `gpui_macros::style_helpers!()` and similar. Expand them with a script, not by hand |
| `path_builder.rs`, `path_winding.rs` | 1419 | 35–55% | M+F | uses `lyon_path` (external crate). Deferred to M3b |
| `platform/keystroke.rs`, `keyboard.rs` | 818 | 60–85% | M | data and parsing |
| `keymap/context.rs` | 889 | 34% | M+F | `KeyBindingContextPredicate` is a recursive enum, so it needs `OwnedPointer` boxing |
| `keymap.rs`, `keymap/binding.rs` | 1309 | 7–50% | M+F | `Box<dyn Action>` becomes an erased action (section 3.6) |
| `interactive.rs` | 998 | 84% | M | event structs (`MouseDownEvent`, `KeyDownEvent`...). `PlatformInput` becomes a `Variant` |
| `gestures.rs`, `spring.rs`, `tab_stop.rs` | 3662 | 57–78% | M+F | value logic |
| `text_system/line_wrapper.rs`, `line_layout.rs`, `line.rs` | 5017 | 50–65% | M+F | value logic over `ShapedRun`/`ShapedGlyph`. `LineLayoutCache` borrows (`CacheKeyRef<'a>`) need origins or copies |
| `text_system.rs` | 1400 | 58% | M+F / H | data types are M. `TextSystem` wraps `Arc<dyn PlatformTextSystem>`, which needs H (section 4.5) |
| `key_dispatch.rs` | 2200 | 69% | M+F | `DispatchTree` is arrays plus ids. Listener fields (`Rc<dyn Fn(&dyn Any, DispatchPhase, &mut Window, &mut App)>`) become erased callbacks |
| `queue.rs` | 541 | 43% | M+F | priority queue used by the scheduler |
| `elements/anchored.rs`, `deferred.rs`, `surface.rs` | 1008 | 56–71% | M+F | small elements once the `Element` trait exists |
| `elements/div.rs` | 5819 | 27% | H core + M+F rest | `Interactivity` holds about 20 listener kinds (`ClickListener = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>`, ...). Style plumbing is mechanical |
| `elements/text.rs`, `list.rs`, `uniform_list.rs` | 6124 | 33–58% | M+F | follow `div.rs` |
| `elements/canvas.rs`, `animation.rs`, `container_query.rs` | 1226 | 0–8% | H (small) | boxed closures are the whole point of these elements |
| `elements/img.rs`, `svg.rs`, `image_cache.rs` | 1961 | 22–63% | deferred | async asset loading, plus `image`/`resvg` |
| `element.rs` | 1000 | 14% | H | `Element` (associated `RequestLayoutState`/`PrepaintState`), `Drawable<E>`, `AnyElement(ArenaBox<dyn ElementObject>)` |
| `view.rs` | 667 | 11% | H | `AnyView` already uses thin fn pointers, so the port is direct but manual |
| `arena.rs` | 401 | 36% | H (small) | bump arena with `(ptr, drop fn)`. A short port onto `Pointer` |
| `subscription.rs` | 559 | 36% | H (small) | `SubscriberSet` / `Subscription { unsubscribe: Option<Box<dyn FnOnce()>> }` |
| `app/entity_map.rs` | 1421 | 23% | H | `SecondaryMap<EntityId, Box<dyn Any>>`, `Arc<RwLock<EntityRefCounts>>`, leases |
| `app.rs` | 4002 | 12% | H | `App` has more than 25 subscriber sets, the effects queue, globals and windows |
| `app/context.rs` | 938 | 0% | H + generator | `Context<'a, T> { app: &'a mut App, entity_state: WeakEntity<T> }`. Its ~60 methods follow one pattern |
| `app/async_context.rs` | — | 36% | H | depends on the executor design |
| `window.rs` | 10984 | 19% | H, split | `Window`, `Frame`, hitboxes, focus, draw, dispatch. The data parts (`Hitbox`, `ContentMask`, `ElementId`, `PaintQuad`) are M |
| `taffy.rs` | 787 | 32% | H | `TaffyLayoutEngine` over `TaffyTree<NodeContext>` with a boxed `MeasureFn` |
| `executor.rs`, `platform_scheduler.rs` | — | 0–2% | H | `async_task::Runnable`, futures, `scheduler` crate |
| `platform.rs` | 4063 | 58% | M data / H traits | `Platform` (81 methods), `PlatformWindow` (103), `PlatformDispatcher`, `PlatformTextSystem`, `PlatformAtlas`. Data types (`WindowOptions`, `CursorStyle`, `ClipboardItem`, `AtlasTile`) are M |
| `platform/test/*` | 1705 | 0–100% | M+F into H shape | `TestDispatcher` (87%) is the model for the deterministic executor |
| `action.rs` | 456 | 42% | H | `trait Action: Any`, `boxed_clone`, `partial_eq`, `inventory` registration |
| `global.rs`, `asset_cache.rs` | 433 | 33–54% | H (small) | `TypeId`-keyed `Box<dyn Any>` |
| profiler, inspector, a11y, hot_reload, bench, scap | — | — | skip | non-goals |

The value layer is roughly 35k of the crate's 82k lines. Getting it to compile is M0's
job, and nothing in it needs a new language feature beyond what MAPPING.md already uses.

## 3. Core runtime without `dyn`

### 3.1 Primitives: type keys, opaque pointers, vtables

Mojo 1.0 has no `TypeId`, and the release notes describe none. GPUI uses `TypeId` for
entity types, event types, globals, element state (`(GlobalElementId, TypeId)` in
`Frame::element_states`) and actions. Proposal:

```mojo
trait GpuiType(Movable, Deinitable):
    comptime TYPE_NAME: StaticString        # e.g. "my_app::Counter"

def type_key[T: GpuiType]() -> UInt64:
    comptime k = fnv1a_64(T.TYPE_NAME)       # evaluated at compile time  [unverified: comptime string hashing]
    return k
```

The translator emits `TYPE_NAME` from the Rust path for every struct used as an entity,
event, global or element state. If Mojo's reflection can produce a type name at
comptime (`reflect[T]` exists [verified], but a type-name query is **[unverified]**),
the trait member can get a default and stop being boilerplate. Collisions are checked
when a vtable is first registered: the registry maps key to name and aborts if the name
differs.

Opaque storage uses a single alias:

```mojo
comptime OpaquePtr = Pointer[UInt8, MutUntrackedOrigin]   # exact origin name [unverified];
                                                          # 1.0 notes say pointer fields need an
                                                          # Origin param or UntrackedOrigin [verified]
struct TypeVTable(ImplicitlyCopyable):
    var key: UInt64
    var name: StaticString
    var drop: def(OpaquePtr) thin -> None    # `thin` fn types exist [verified]

def _drop_as[T: GpuiType](p: OpaquePtr):
    var tp = p.bitcast[T]()
    tp.destroy_pointee()                     # name [unverified]
    tp.unsafe_free()                         # [verified rename]

def vtable_of[T: GpuiType]() -> TypeVTable:
    return TypeVTable(type_key[T](), T.TYPE_NAME, _drop_as[T])   # generic thin fn value [unverified]
```

`ErasedBox` (owning `OpaquePtr` + `TypeVTable`, frees on `__deinit__`) replaces `Box<dyn Any>`.
`ErasedRef` (non-owning, with `downcast[T]() -> Optional[Pointer[T, ...]]` that compares keys)
replaces `&dyn Any`. Both go in `mojo_rt/erased.mojo` so the translator can target them.

### 3.2 The entity map

GPUI (`app/entity_map.rs`) keeps:
* `entities: SecondaryMap<EntityId, Box<dyn Any>>`, the values;
* `ref_counts: Arc<RwLock<EntityRefCounts>>` with `counts: SlotMap<EntityId, AtomicUsize>` and
  `dropped_entity_ids`. `AnyEntity` holds a `Weak` to it and increments or decrements in
  `Clone`/`Drop`. When the count reaches 0, the id is pushed onto `dropped_entity_ids`, and
  `App::release_dropped_entities` later runs `release_listeners` and drops the box;
* **leases**: `update_entity` *removes* the `Box` from the map (`lease_inner`), passes
  `&mut T` plus `&mut App`, and reinserts it. A nested update of the same entity panics
  (`double_lease_panic`). This is how Rust gets `&mut T` and `&mut App` at the same time;
* access tracking (`AccessedEntities`) for view invalidation.

Proposed Mojo design: a **generational slot arena with per-entity heap boxes**.

```mojo
@fieldwise_init
struct EntityId(ImplicitlyCopyable, Equatable, Hashable):
    var index: UInt32
    var generation: UInt32

struct EntitySlot(Movable):
    var generation: UInt32
    var value: OpaquePtr          # null while reserved, or after release
    var vtable: TypeVTable
    var leased: Bool

struct RefCounts(Movable):        # shared by all handles
    var counts: List[Int]         # indexed by slot index
    var generations: List[UInt32]
    var dropped: List[EntityId]
    var free_list: List[UInt32]

struct EntityMap(Movable):
    var slots: List[EntitySlot]
    var ref_counts: ArcPointer[RefCell[RefCounts]]   # mojo_rt.cell.RefCell
    var accessed: AccessedEntities                   # M: plain sets
```

* Each entity value has its **own heap allocation**, so its address stays fixed while
  `slots` grows. That replaces the lease-by-removal. `update_entity[T]` checks the type key,
  sets `leased = True` (a nested update aborts with GPUI's message), builds `Context[T]`,
  calls the closure with `mut` access through `value.bitcast[T]()`, then clears `leased`.
  `read_entity` aborts if `leased`, which matches GPUI's `read` on a leased entity.
* `Entity[T]` holds `EntityId` and `ArcPointer[RefCell[RefCounts]]`. A custom copy
  initialiser (`__init__(out self, *, copy: Self)`) increments the count, and `__deinit__`
  decrements it and pushes onto `dropped`. `WeakEntity[T]` holds the same pair but does not
  count. `upgrade()` succeeds when `generations[index] == id.generation and counts[index] > 0`.
  Because of the generation check, `WeakPointer` (whose 1.0 status is **[unverified]**) is not needed.
* `AnyEntity` is `Entity` without `T` plus a `key: UInt64`, and `downcast[T]()` compares keys.
* **Threading narrowing:** GPUI uses `AtomicUsize` and a lock because `Entity<T>` is `Send`.
  M1 uses plain `Int` counts with a debug assertion that every count change happens on the
  main thread. Background work receives plain data, not handles. If a ported app really
  needs handles on other threads, switch to `std.atomic` [module verified, API unverified]
  plus a spin lock from `std.utils.lock` [module verified, type names unverified]. This
  narrowing is a semantic change and must be recorded in MAPPING.md.

Windows use the same lease pattern. `App.windows: SlotMap<WindowId, Option<Box<Window>>>`
takes the window out during `update_window`. In Mojo, `windows: List[WindowSlot]` with
`OwnedPointer[Window]` and a `leased` flag.

### 3.3 `App` and `Context[T]`

`App` stays one concrete struct, and **its platform-dependent members are not `dyn`**:

* `platform: Rc<dyn Platform>` → `PlatformImpl = Variant[TestPlatform, SdlPlatform]`.
  The set is closed and calls are infrequent, so `Variant` dispatch costs almost nothing.
  Hand-written `if p.isa[...]` forwarding is enough for the roughly 25 methods M1–M4 need.
* `text_system: Arc<TextSystem>` → concrete `TextSystem` over `Variant[NoopTextSystem, ShimTextSystem]`.
* `keyboard_layout: Box<dyn PlatformKeyboardLayout>`, `asset_source: Arc<dyn AssetSource>`,
  `http_client` → stubbed or a closed `Variant` (HTTP is a non-goal).
* `globals_by_type: TypeIdHashMap<Box<dyn Any>>` → `Dict[UInt64, ErasedBox]`.

`Context<'a, T>` holds `&'a mut App` and derefs to `App`. Mojo has no `Deref`, so:

```mojo
struct Context[T: GpuiType, o: MutOrigin]:
    var app: Pointer[App, Self.o]
    var entity_state: WeakEntity[Self.T]
    def notify(mut self): ...
    def emit[E: GpuiType](mut self, var event: E): ...
    # forwarding: generated, see below
```

A small generator (a translator feature: "Deref forwarding") emits a forwarding method on
`Context` for each `App` method used through `cx.` in the corpus. Until it exists, call
sites translate `cx.foo()` to `cx.app[].foo()`. Struct methods that take a `cx: &mut App`
alongside a `self` stored inside `App` alias in the same way Rust's lease avoids. Here the
untracked-origin pointer hides the alias from Mojo's exclusivity checker. That is sound
only because of the `leased` discipline, so it is listed as a risk (7.6).

### 3.4 Callbacks: hand-rolled closure objects

`app.rs`, `window.rs`, `key_dispatch.rs` and `div.rs` together contain about 140 `dyn`
occurrences, and most of them are callbacks. The aliases include `Handler = Box<dyn FnMut(&mut App) -> bool>`,
`Listener = Box<dyn FnMut(&dyn Any, &mut App) -> bool>`, `ReleaseListener = Box<dyn FnOnce(&mut dyn Any, &mut App)>`,
`NewEntityListener`, `AnyObserver = Box<dyn FnMut(&mut Window, &mut App) -> bool>`,
`FrameCallback = Box<dyn FnOnce(&mut Window, &mut App)>`, `KeyListener = Rc<dyn Fn(&dyn Any, DispatchPhase, &mut Window, &mut App)>`,
the `Mouse*Listener`/`ClickListener`/`ActionListener` family in `div.rs`, and `MeasureFn` in `taffy.rs`.

What Mojo 1.0 offers [verified]: nested `def`s with capture lists (`{imm k}`,
`{mut x}`, `{var x^}`); lambdas, where "a capture-free (thin) lambda is a function
value, while others are closure instances **with no function type**"; thin function
pointer types (`def(...) thin -> R`); and structs that declare a function trait
(`struct Double(def(Int) -> Int):`). The manual does not cover escaping closures, storing them
in fields, or putting them in `List`s. So **a capturing closure cannot be a field type**, and that
is the gap to work around.

Proposal: one erased callable struct per signature family, generated into `mojo_rt/callbacks.mojo`.

```mojo
struct AppHandler(Movable):                        # Box<dyn FnMut(&mut App) -> bool>
    var state: OpaquePtr
    var call: def(OpaquePtr, mut App) thin -> Bool
    var drop: def(OpaquePtr) thin -> None

    @staticmethod
    def new[F: def(mut App) -> Bool](var f: F) -> Self:   # F: closure instance or user struct
        var p = alloc_move[F](f^)                         # heap-move the closure [unverified]
        return Self(p, _call_as[F], _drop_as_fn[F])

    def __call__(mut self, mut cx: App) -> Bool:
        return self.call(self.state, cx)
    def __deinit__(deinit self):
        self.drop(self.state)

def _call_as[F: def(mut App) -> Bool](p: OpaquePtr, mut cx: App) -> Bool:
    return p.bitcast[F]()[](cx)
```

* `FnOnce` families get `call_once(deinit self, ...)`, which calls and then drops.
  `Rc<dyn Fn>` families wrap the struct in `ArcPointer` (copy means refcount increment).
* Whether a capturing closure (`{var x^}`) can be passed as a generic `F` and moved to the
  heap is the **single most important unverified point** in this design (7.1).
  The fallback needs no closures at all: translated code emits a named struct per closure
  (captures become fields) that declares `(def(mut App) -> Bool)`. That pattern is verified
  for structs, and it is a mechanical lowering the translator can do.
* Invocation follows GPUI's existing pattern: `SubscriberSet::retain` takes the
  subscriber map out (`mem::take`), invokes each callback with `&mut App`, then merges
  back any subscribers added meanwhile. The callback storage is therefore never borrowed
  while `App` is mutably passed. Keep that structure exactly.
* `subscription.rs` ports almost directly. `BTreeMap` (not in Mojo std) becomes
  `Dict` plus sorted ids, because subscriber order must stay insertion order. The
  `Subscription { unsubscribe: Option<Box<dyn FnOnce()>> }` becomes a `VoidOnce` erased callable.

### 3.5 The effects queue

`Effect` (in `app.rs`) is a closed enum: `Notify`, `Emit { event: ArenaBox<dyn Any> }`,
`RefreshWindows`, `NotifyGlobalObservers`, `Defer { callback: Box<dyn FnOnce(&mut App)> }`,
`EntityCreated`. Mojo: `Variant[...]` payload structs (the translator's normal data-enum
lowering), with `Emit` carrying an `ErasedBox` (event arena allocation is an optimisation
for later) and `Defer` carrying an `AppOnce` callable. `App::update` → `start_update` /
`finish_update` → `flush_effects` (the loop over `release_dropped_entities`,
`release_dropped_focus_handles`, pop effect, apply, and finally `schedule_frame` for dirty
windows) translates almost line by line once the types exist. Keep it identical. The ordering
semantics (observers activated via `defer`, `pending_notifications` dedupe) are subtle
and covered by GPUI's tests, which we should port to the test platform in M1.

### 3.6 Events, globals, actions

* `EventEmitter<E>` / `cx.emit(e)`: `Emit` stores `ErasedBox` plus the type key, and
  `subscribe[T, E]` wraps the user callback in a `Listener` that calls `downcast[E]` and
  skips the event on a key mismatch (GPUI `expect`s instead; keep the abort).
* Globals: `Dict[UInt64, ErasedBox]`. `update_global` uses the same lease trick as
  `GlobalLease` (take it out, call, put it back).
* `Action` (`trait Action: Any + Send` with `boxed_clone`, `partial_eq`, `name`, `build(serde_json::Value)`):
  `ActionVTable { clone, eq, name, build }` plus `ErasedBox`. GPUI registers actions
  through `inventory` (link-time). Mojo has no equivalent, so registration becomes an explicit
  `register_actions[A, B, C](app)` call that the translator emits for each `actions!(...)` invocation.
  Building actions from JSON keymaps is deferred, since there is no serde.

## 4. Window and the element tree

### 4.1 `Element` and `AnyElement`

```mojo
trait Element(Movable, Deinitable):
    associatedtype RequestLayoutState: Movable & Deinitable    # syntax per MAPPING #6 [unverified]
    associatedtype PrepaintState: Movable & Deinitable
    def id(self) -> Optional[ElementId]: ...
    def request_layout(mut self, id: Optional[GlobalElementId], mut window: Window,
                       mut cx: App) -> Tuple[LayoutId, Self.RequestLayoutState]: ...
    def prepaint(mut self, id: Optional[GlobalElementId], bounds: Bounds[Pixels],
                 mut rl: Self.RequestLayoutState, mut window: Window, mut cx: App) -> Self.PrepaintState: ...
    def paint(mut self, id: Optional[GlobalElementId], bounds: Bounds[Pixels],
              mut rl: Self.RequestLayoutState, mut pp: Self.PrepaintState,
              mut window: Window, mut cx: App): ...
```

(`inspector_id` and `source_location` are dropped because the inspector is a non-goal. The
a11y methods are dropped too.)

`Drawable<E>` with `ElementDrawPhase<RL, PP>` (`Start`, `RequestLayout{..}`, `LayoutComputed{..}`,
`Prepaint{..}`, `Painted`) becomes a generic struct holding
`Variant[PhaseStart, PhaseRequested[E.RequestLayoutState], PhaseComputed[...], PhasePrepainted[...], PhasePainted]`.
Generic payloads in `Variant` are fine. The phase logic itself is mechanical.

`AnyElement(ArenaBox<dyn ElementObject>)` becomes an arena allocation plus an `ElementVTable`:

```mojo
struct ElementVTable(ImplicitlyCopyable):
    var key: UInt64
    var request_layout: def(OpaquePtr, mut Window, mut App) thin -> LayoutId
    var prepaint:       def(OpaquePtr, mut Window, mut App) thin -> None
    var paint:          def(OpaquePtr, mut Window, mut App) thin -> None
    var layout_as_root: def(OpaquePtr, Size[AvailableSpace], mut Window, mut App) thin -> Size[Pixels]
    var drop:           def(OpaquePtr) thin -> None
```

`into_any[E: Element](var e: E) -> AnyElement` allocates `Drawable[E]` in the element arena
(`arena.rs` ported onto `Pointer`; it is already a `(value ptr, drop fn)` design) and fills
the vtable with `_request_layout_as[E]` and the others. `AnyView` (`view.rs`) keeps GPUI's existing
`render: fn(&AnyView, &mut Window, &mut App) -> AnyElement` thin-pointer design, with
`any_view::render::<V>` → `_render_view[V]`.

`IntoElement` conversions (`.child("text")`, `.child(format!(..))`, `.child(entity)`) rely
on Rust trait dispatch over `impl IntoElement`. In Mojo, `child` is overloaded for
`String`, `AnyElement`, `Entity[V]` and generic `E: Element`. `impl IntoElement` returns from
`render` become `-> AnyElement` (one allocation per render root, which GPUI already pays
in `AnyView`).

Builders such as `div().flex().bg(..)` move `self` by value. In Mojo that is `var self` and `return self^`.
`Div` is boxed internally (`InteractivityBox`) for the same reason as in Rust.

### 4.2 The three-phase frame, hitboxes, dispatch and focus

`Window::draw` (`window.rs:3783`) does the following: enter the element arena scope, `invalidate_entities`,
clear access tracking, `draw_roots` (request_layout → `compute_layout` → prepaint →
paint over the root view, deferred draws, tooltips, prompts), install the input handler,
`next_frame.finish`, swap `rendered_frame`/`next_frame`, dispatch focus-change listeners,
`record_entities_accessed`, mark `needs_present`. The structure ports as is. The pieces:

* `Frame` is mostly data: `hitboxes: Vec<Hitbox>`, `dispatch_tree: DispatchTree`,
  `scene: Scene`, `element_states: FxHashMap<(GlobalElementId, TypeId), ElementStateBox>`
  (→ `Dict[ElementStateKey, ErasedBox]`), `mouse_listeners: Vec<Option<AnyMouseListener>>`
  (→ `List[Optional[MouseListener]]`), `tab_stops`, `cursor_styles`.
* Hit testing (`insert_hitbox`, `HitTest`, `HitboxBehavior`, `captured_hitbox`) is plain
  data and arithmetic, so it is M.
* `DispatchTree` (`key_dispatch.rs`) is a node vector with `parent` indices, focus/view
  maps and listener lists. It is M once `KeyListener`/`DispatchActionListener` are erased
  callables. `PlatformInput` is a closed set, so mouse/key listeners receive the `Variant`
  and test `isa`. That replaces `&dyn Any` + `TypeId` for input events.
* Focus: `FocusMap` in `Arc<RwLock<..>>` (handles may drop off-thread) → main-thread
  `ArcPointer[RefCell[FocusMap]]`, with the same narrowing as entity refcounts.
* `with_element_state[S]` (type-keyed cross-frame state) uses `ErasedBox` and `type_key[S]()`.

### 4.3 Layout

`taffy.rs` wraps `TaffyTree<NodeContext>`, where `NodeContext { measure: Box<MeasureFn> }`
and `MeasureFn = dyn FnMut(Size<Option<Pixels>>, Size<AvailableSpace>, &mut Window, &mut App) -> Size<Pixels>`.
Measurement re-enters `Window`/`App` (text measurement) *during* `compute_layout_with_measure`.
GPUI enables flexbox, **grid**, block, float and content-size (`Cargo.toml`), and
`examples/grid_layout.rs` exercises grid.

| option | pros | cons |
| --- | --- | --- |
| Yoga (C API) | stable C ABI, measure callback with context pointer | no CSS grid; layout differences from taffy break pixel parity |
| port taffy with the translator | native Mojo, no C boundary, good stress test for the translator | taffy is generic-trait-heavy (`LayoutPartialTree`, `impl Trait`), so not feasible before translator milestones 1–4 |
| **Rust `cdylib` shim over taffy `=0.15.0` (recommended now)** | exact GPUI semantics including grid; the C API is ours, so we shape it for Mojo | needs a Rust toolchain in the build; measure callback crosses C → Mojo |

Recommendation: a crate `gpui_mojo_sys/layout` (about 300 lines) exposing what `taffy.rs`
uses: `layout_new`, `layout_clear`, `request_layout(style: *const CStyle, children, n) -> u64`,
`request_measured_layout(style, measure_ctx: u64) -> u64`,
`compute_layout(root, avail_w, avail_h, measure_cb, user_data)`, `layout_bounds(id) -> CBounds`.
`Style → taffy::Style` conversion (`ToTaffy` impls in `taffy.rs`) happens on the Rust side.
The `CStyle` is a `#[repr(C)]` flattening of GPUI's `Style` layout fields, so the Mojo side
only copies fields (M). The measure callback is `extern "C" fn(user_data, ctx, known_w, known_h, avail_w, avail_h) -> CSize`.
On the Mojo side `user_data` points at a struct holding `Pointer[Window]`, `Pointer[App]`, and
the `List[MeasureCallable]` indexed by `measure_ctx`. This requires a C-callable Mojo function (7.2).
**If C → Mojo callbacks turn out to be unusable**, the fallback is a two-pass protocol:
layout runs and returns "needs measurement for node X under constraints Y" as a
resumable state, Mojo measures, and layout resumes. Taffy does not support this, so the fallback is a
Mojo flexbox subset first, with grid later. Porting taffy to Mojo remains the long-term
plan once the translator handles its generics.

### 4.4 Scene and primitives

`Scene` (`scene.rs`) holds per-kind vectors (`shadows`, `quads`, `paths: Vec<PaintedPath>`,
`underlines`, `monochrome_sprites`, `subpixel_sprites`, `polychrome_sprites`, `surfaces`),
`paint_operations`, layers, and a `BoundsTree` for draw order. `finish()` sorts and
`batches()` yields `PrimitiveBatch` runs keyed by kind and atlas texture. All of this is
value code (77% emitted today). Two cautions:

1. **GPU layout.** `Quad`, `Shadow`, `Underline`, `MonochromeSprite`, `SubpixelSprite`,
   `PolychromeSprite`, `PathCurve`, `PathPaint` and `PathTile` are `#[repr(C)]` and are uploaded
   verbatim as storage-buffer arrays that the WGSL structs mirror (`PathCurve.pad` exists
   "to align to 8 bytes"). Mojo struct layout guarantees are **[unverified]**. The translator
   should emit, for every `#[repr(C)]` struct, a test asserting `size_of` and field offsets
   against values recorded from the Rust build (a small `cargo test` dumps them). The
   optimisation pass's `simd-struct` hint (turning `Hsla`/`Point<f32>` into `SIMD`) **must not
   apply** to types reachable from these structs, because SIMD alignment would change the layout.
2. **Paths.** `PaintedPath` carries `Rc<PathTiles>` produced by CPU tiling (`path_builder.rs`
   using `lyon_path`, `path_winding.rs`). Quads, shadows, underlines and sprites come first (M3).
   Paths are M3b, and need either a lyon port or a small flattening routine.

### 4.5 Text

`PlatformTextSystem` needs `font_id`, `font_metrics`, `advance`, `glyph_for_char`,
`layout_line(text, font_size, runs) -> LineLayout`, `glyph_raster_bounds`,
`rasterize_glyph`, and fallback/missing-glyph plumbing. Everything above it
(`LineLayoutCache`, `LineWrapper`, `ShapedLine`, `WrappedLine`) is in-crate value code, so it is M+F.

| option | notes |
| --- | --- |
| HarfBuzz + FreeType + fontconfig (Linux), CoreText (macOS) via C | all stable C ABIs. CoreText is a C API usable through `external_call`. Fallback, bidi, and run segmentation are ours to write, and there are two backends |
| **cosmic-text behind a Rust `cdylib` shim (recommended now)** | one cross-platform backend. Coarse-grained API (`layout_line` returns glyph arrays) with **no callbacks**. Zed's Linux backend is, to my knowledge, cosmic-text based (verify in `gpui_linux`), so Linux behaviour would match |

Recommendation: `gpui_mojo_sys/text` exposing about 8 functions with `#[repr(C)]` outputs
(glyph id, font id, x position, byte index). Rasterisation returns an A8 or BGRA buffer,
which the Mojo side inserts into the atlas. HarfBuzz/FreeType/CoreText become a later,
optional "no Rust at runtime" milestone.

## 5. Platform layer

### 5.1 Options

| | (a) SDL3 + wgpu-native | (b) SDL3 + SDL_gpu | (c) direct Vulkan / Metal | (d) GLFW + wgpu-native |
| --- | --- | --- | --- | --- |
| windowing/input | SDL3: Wayland, X11, Cocoa; IME events; clipboard with MIME types; cursors; HiDPI | same | write it ourselves (Wayland, X11, AppKit) | GLFW: no IME composition events, weak clipboard (text only), cursors limited |
| C ABI stability | SDL3 is ABI-stable across 3.x. `webgpu.h` is converging but not frozen, so pin a wgpu-native release | SDL only, stable | Vulkan C is stable. Metal is Objective-C, so it needs `objc_msgSend` through FFI | GLFW is stable. `webgpu.h` as in (a) |
| shaders | **WGSL as-is**: reuse `gpui_wgpu`'s shaders, and naga translates to SPIR-V/MSL internally | needs SPIR-V + MSL (+DXIL). WGSL would go through offline naga/tint, or HLSL through SDL_shadercross | SPIR-V and MSL, two renderers | WGSL as-is |
| FFI ergonomics | SDL is poll-based (no callbacks needed). webgpu.h adapter/device requests and buffer maps are **callback-based** | poll-based, synchronous API. Best fit for Mojo FFI | Metal needs Obj-C runtime calls, which are painful | as (a) |
| renderer template | `gpui_wgpu` maps nearly 1:1 from Rust `wgpu` calls to `webgpu.h` calls | rewrite pipelines against a different API | two renderers to write from scratch (Vulkan and Metal) | as (a) |
| future web target | yes (same WGSL) | no | no | yes |

### 5.2 Recommendation: (a) SDL3 + wgpu-native

Reasons: shader reuse plus a renderer that can be ported from `gpui_wgpu` with the same pipeline
structure gives the shortest path to pixel parity. SDL3 covers Wayland and X11 together
(Zed's `gpui_linux` implements both by hand), and it has IME and MIME clipboard support, which
GLFW lacks. SDL being poll-based keeps C → Mojo callbacks off the hot path.

Mitigations for (a)'s weak spots:
* `webgpu.h` callbacks (`wgpuInstanceRequestAdapter`, `RequestDevice`, `BufferMapAsync`,
  error callbacks): put them in a ~100-line C shim (`gpui_mojo_sys/gpu_shim.c`) exposing
  synchronous `shim_request_device(...)`, which spins `wgpuInstanceProcessEvents`. Mojo then
  calls only plain functions. Atlas uploads use `wgpuQueueWriteTexture`, which needs no mapping.
* Surface creation: SDL3 exposes native handles via window properties (X11 window number,
  Wayland `wl_surface`/`wl_display`). On macOS, `SDL_Metal_CreateView` / `SDL_Metal_GetLayer` give a
  `CAMetalLayer` for `wgpuInstanceCreateSurface`. Exact property names should be checked against SDL 3.2 headers.
* Keep (b) as the planned fallback. The scene and atlas layers sit behind a small
  `Renderer` interface (`draw(scene)`, `atlas()`), so switching only rewrites the renderer.

Binding mechanics: declare each C function once in `gpui/platform/sdl/ffi.mojo` as a
wrapper around `external_call["SDL_PollEvent", Bool](ptr)`, or resolve symbols at runtime with
`OwnedDLHandle(...).get_function[Ret]("name")` [verified: takes a return type and uses the C
ABI with by-value structs]. Prefer `OwnedDLHandle` so the Mojo build does not need link
flags. C structs (`SDL_Event` is a 128-byte union) are mirrored as Mojo structs with an
explicit byte array plus typed accessors, and checked with size asserts.

### 5.3 Event loop, dispatcher, executor

GPUI's executor (`executor.rs`) is futures-based: `ForegroundExecutor::spawn(impl Future)`,
`BackgroundExecutor::spawn(impl Future + Send)`, `timer(duration)`, `Task<R>` (cancel on
drop), on top of `PlatformDispatcher { dispatch, dispatch_on_main_thread, dispatch_after,
is_main_thread, ... }` and `async_task::Runnable`. The Mojo stdlib says its async support
is unfinished and should not be used to build async patterns yet [verified, `std.runtime`
docs]. `std.runtime` has no public thread-spawn API [verified]. Proposal:

* **Main loop (SDL):** `while running: SDL_WaitEventTimeout(min(next_timer_deadline, frame_deadline))`,
  translate `SDL_Event` → `PlatformInput` → `Window::dispatch_event`, drain the foreground
  queue, fire due timers (a min-heap: `dispatch_after`), and `draw` + `present` invalidated windows
  (FIFO present mode for vsync pacing). Other threads wake the loop with `SDL_PushEvent` (a user
  event), which SDL documents as thread-safe.
* **Foreground queue:** `Deque[Runnable]` per `Priority`, where `Runnable` is an erased `FnOnce(mut App)`.
* **Background pool:** N worker threads (N from `std.runtime.parallelism_level()` [verified]
  or `sysconf`) created with `pthread_create` through FFI, with a job queue guarded by
  `pthread_mutex`/`pthread_cond` (also FFI, because std has only spin locks). Worker threads need a
  C-callable entry point (7.2). The C shim can host the pool and call one exported Mojo function as the fallback.
* **Tasks:** `Task[R]` is `ArcPointer` to `{state, result: Optional[R], cancelled}`. App code
  uses continuation style: `cx.background_spawn(job).then(cx, on_done)`, where `on_done` runs on the
  main thread with `(mut T, R, mut Context[T])`. Rust `async move |cx| { ..await.. }` blocks
  are **not** mechanically translatable. The translator flags them, and they are hand-ported. Most
  GPUI app code awaits once or twice, so continuations stay readable. Revisit this when Mojo
  async stabilises.
* **Test dispatcher:** port `platform/test/dispatcher.rs` (87% emitted) as a
  deterministic single-threaded scheduler with simulated time. All M1/M2 tests run on it.

### 5.4 Clipboard, IME, cursors (later work)

Clipboard: `SDL_GetClipboardText`/`SDL_SetClipboardText` first, MIME data (`SDL_SetClipboardData`)
for `ClipboardEntry::Image` later. IME: SDL3 `SDL_StartTextInput`, `SDL_EVENT_TEXT_EDITING`
(composition) and `SDL_EVENT_TEXT_INPUT`, with `SDL_SetTextInputArea` fed from
`InputHandler::bounds_for_range`. Mapping onto `PlatformInputHandler`/`InputHandler`
(`platform.rs:2080`, `:2379`) is M5+ work. Cursors: `SDL_CreateSystemCursor` covers most of `CursorStyle`.
Known macOS issue: during live window resize, AppKit runs its own loop and `SDL_WaitEvent`
does not return, so smooth resizing needs `SDL_AddEventWatch` (a C → Mojo callback) or
SDL's main-callbacks mode. Accept stutter until 7.2 is resolved.

## 6. Milestones

| milestone | deliverable | exit criteria | translator work that helps |
| --- | --- | --- | --- |
| **M0: leaf modules compile** | `geometry`, `color(s)`, `style`, `scene` (minus paths), `bounds_tree`, `keystroke`, `interactive`, `keymap/context`, `text_system/line_*` data | `scripts/check_mojo.sh` passes. Ported unit tests from the Rust `#[cfg(test)]` blocks pass. `repr(C)` size/offset asserts pass | run the README's unverified-syntax list against a real toolchain. Generic arithmetic (done in pass 2: `mojo_rt.ops` dispatch, see MAPPING.md 6.1). Recursive enum → `OwnedPointer` auto-boxing. Cross-module impl merging (done in pass 2 for unique type names). `let-else`. Iterator chains → loops |
| **M1: headless App** | `mojo_rt/erased.mojo`, `callbacks.mojo`, `EntityMap`, `App` (entities, observe/subscribe/emit/notify/defer, globals, release listeners, effects), `Context[T]`, `TestPlatform` + `TestDispatcher` | GPUI's entity/observer/subscription tests (ported from `app.rs`/`entity_map.rs`/`subscription.rs` test modules) pass, including ordering and release semantics | **erased-field shims** (pass 2 erases such fields to `mojo_rt.erased.Erased` and stubs only the code that touches them; next: map `Box<dyn FnMut(A..)->R>` aliases to generated callable structs and `Box<dyn Any>` → `ErasedBox`). **Closure inference** from the expected `impl FnMut(..)` parameter type (done in pass 2 for crate signatures and `Option` combinators). Closure → named struct lowering as the fallback. `Deref` forwarding generator for `Context` |
| **M2: element tree, headless** | `Element`/`Drawable`/`AnyElement`, arena, `AnyView`/`Render`, `Window` with `draw` (no platform window), layout shim, `Div` (styles, children; no listeners yet), `Text` with the shim text system, `Scene` complete, a CPU reference rasteriser for quads (test-only) | `hello_world.rs` (examples) translated with only `render` adjustments. Its `Scene` matches a Rust-dumped scene for the same window size (compare primitive arrays) | **macro expansion input mode** (`cargo expand` output as translator input) for `style_helpers!` and the other style macros, and `actions!`. A `Refineable` derive generator. **Overlay mechanism**: `ports/gpui/*.mojo` hand files take precedence over translated ones, and the translator imports them and records which items they replace |
| **M3: on screen** | SDL3 window, wgpu-native surface, renderer for quads, shadows, underlines, mono/subpixel/poly sprites (atlas), WGSL from `gpui_wgpu`, SDL main loop, foreground queue | `hello_world` renders on Linux (Wayland and X11) and macOS. A screenshot diff against Rust GPUI is within tolerance | `repr(C)` layout test generation; FFI declaration generation from a list of C prototypes (simple, not bindgen) |
| **M3b: paths** | `path_builder`/`path_winding` + path pipeline | `examples/painting.rs`, `gradient.rs` | an external-crate substitution table (`lyon_path` → local port) |
| **M4: input, focus, actions** | `PlatformInput` from SDL events, hit testing, mouse listeners in `Interactivity`, `DispatchTree` key/action dispatch, keymap, focus handles, tab stops, background pool + `Task[R]` | `examples/input.rs` (without IME), `tab_stop.rs`, `focus_visible.rs`, `scrollable.rs` behave like the Rust versions | async block flagging plus continuation suggestions. Erased `Action` vtable generation per `actions!` |
| **M5: a small real app** | port an app that does not depend on Zed: `examples/list_example.rs` + `text_wrapper.rs` + `drag_drop.rs`, then a todo or markdown-viewer style app | runs on both OSes; app code is ≥80% translator output | whatever the M5 coverage report ranks highest |

Exit after M5: IME, clipboard images, `img`/`svg`, animation, native text backends,
taffy in Mojo, and a fully native Mojo build with no Rust shim.

Process rule for every milestone: hand-written runtime files live in an overlay directory
and are never regenerated. Translated files are regenerated from the corpus, and every
manual edit to a translated file is either upstreamed as a translator rule or saved as a
patch that the corpus script reapplies (README "Fix-up passes").

## 7. Risks and open questions

### 7.1 Storing capturing closures **[unverified, highest risk]**
The design needs `def new[F: def(mut App) -> Bool](var f: F)` to accept a capturing
closure and move it into heap memory. The 1.0 notes say capturing closures have "no function
type", and the manual says closures without `{var^}` "can't be assigned to new variables or copied".
If generic `F` cannot bind to a closure instance, the fallback is defunctionalisation:
every Rust closure that is boxed becomes a named struct declaring the function trait. That is
verified for structs, but verbose. The translator can generate these structs, but hand-written app code gets uglier.

### 7.2 C → Mojo callbacks **[unverified]**
Needed for: pthread entry points, the taffy measure callback, SDL event watches (macOS live
resize), and, unless shimmed, webgpu.h callbacks. `@export("name") def f(...) abi("C")` is in
our MAPPING notes, but the 1.0 release notes do not cover `@export`. The `def(...) abi("C")`
function-type syntax appears only in a "before" example, so it may have changed. Mitigation:
every callback-taking API goes through `gpui_mojo_sys`, so in the worst case only one
`@export`ed Mojo dispatcher function is needed. If even that fails, layout falls back to
a Mojo flexbox subset, and the C-hosted thread pool can run only C-side jobs (text shaping,
image decode), never Mojo code.

### 7.3 Other unverified Mojo points
* Taking a thin function value of a generic instantiation (`_drop_as[T]`). GPUI's
  `any_view::render::<V>` pattern depends on it. The 1.0 notes mention "dynamic function pointers with
  unbound type parameters", which suggests it works.
* Comptime string hashing for `type_key`, and a comptime type-name query in reflection.
* `associatedtype` syntax and `comptime X = T` satisfaction (MAPPING unverified #6).
* The name of the mutable untracked origin, `destroy_pointee`/allocation helper names on
  `Pointer`, and `ArcPointer`'s mutability through `[]`.
* Struct layout and alignment rules versus C, for GPU buffers and `SDL_Event`.
* Lock type names in `std.utils.lock`, and the `std.atomic` API.
* `WeakPointer` (not relied on), and `Variant` compile-time cost with many generic payloads.

### 7.4 No retroactive conformance
GPUI implements its traits for `f32`/`usize` (`Half`, `IsZero`, `Along`, `From<..> for u32`;
26 "impl on a std/primitive type" stubs). Mojo cannot add them. Wrap the values in newtypes,
use free functions, or use the `dtype-param` rewrite. This is decided per trait during M0.

### 7.5 Semantic narrowings to record
Main-thread-only entity and focus refcounts. `BTreeMap` replaced by `Dict` plus sorting.
`SharedString` → `String` (copies, which matters for render hot paths: consider an
`ArcPointer[String]` `SharedString`). `SmallVec` → `List` (allocation per element child list,
a measurable cost in `Div`). Async replaced by continuations.

### 7.6 Aliasing through untracked pointers
Passing `mut App` while the callee also holds a `Pointer` into an App-owned entity box
or window is invisible to Mojo's exclusivity checker. The `leased` flags give runtime
protection equal to GPUI's panics. Whether Mojo's optimiser assumes `mut` arguments are
not aliased (making this undefined behaviour rather than merely unchecked) is an **open
question** for the Mojo team, and the answer decides between this design and true
take-out leases (moving the entity into a local and back, like Rust does, at the cost of a move per update).

### 7.7 Dependencies and churn
`webgpu.h` may still change between wgpu-native releases (pin one version, with all calls in
one module). The Rust shims (`gpui_mojo_sys`) add a Rust toolchain to the build until they are replaced.
The `gpui_wgpu`/`gpui_linux`/`gpui_macos` sources are not yet in our corpus, so the
renderer and text-system claims above are unconfirmed until they are fetched at the same revision.

### 7.8 Open design questions
1. API fidelity versus Mojo idiom: this proposal keeps GPUI's names and shapes so
   translated app code fits. Is that the right priority for a Mojo-native UI library, or should
   M5+ move to a more Mojo-flavoured API (for example, comptime-typed element trees with no erasure)?
2. Should `App` become generic over the platform (`App[P: Platform]`) for zero-cost static
   dispatch? Rejected here because the parameter would spread to `Context`, `Window`, every
   element and every app type. `Variant` dispatch is used instead. Revisit if profiling disagrees.
3. Licensing: GPUI is Apache-2.0. Translated output is a derivative work, so the Mojo package must
   carry the Apache licence and NOTICE for translated files (the translator itself is MIT).
4. Compile time: every entity, element and callback type instantiates vtable functions. With
   Zed-scale apps this could be thousands of instantiations, and nobody has measured Mojo's behaviour at that scale.

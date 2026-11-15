# rust-to-mojo coverage: `gpui`

Target: Mojo 1.0 (Modular 26.5, Aug 2026): def-only functions, comptime, Deinitable, Array, std.* imports  
Tool: rust-to-mojo 0.1.0

| metric | value |
| --- | ---: |
| files | 103 |
| items (total) | 4637 |
| translated (clean) | 1278 |
| partial (code + TODO notes) | 595 |
| stubbed (TODO + original Rust) | 2681 |
| skipped (tests) | 83 |
| emitted % (translated+partial / non-skipped) | 41.1% |
| clean % (translated / non-skipped) | 28.1% |
| internal errors (panics caught) | 0 |
| files with parse errors | 0 |

## Top reasons items were stubbed

| # | reason | items | example |
| ---: | --- | ---: | --- |
| 1 | method of an untranslated type (cause: `dyn Trait` (no existential types in Mojo)) | 1040 | App |
| 2 | method of an untranslated type (cause: lock type (no Mojo std equivalent)) | 170 | HeadlessAtlas |
| 3 | method of an untranslated type (cause: reference type in field/value position (needs Pointer + origin)) | 149 | TaskTiming |
| 4 | closure with untyped parameters (needs type inference) | 131 | \| t \| Some((t.thread_id, t.timings.upgrade() ?)) |
| 5 | `dyn Trait` (no existential types in Mojo) | 121 | dyn Action + 'static |
| 6 | reference type in field/value position (needs Pointer + origin) | 99 | type `&'static core::panic::Location<'static>` |
| 7 | closure taking a `&mut` argument | 59 | FnOnce(&mut Context<T>) -> T |
| 8 | `.into()` without a known target type | 53 | value.into() |
| 9 | impl for a type defined in another module or not translated (Mojo needs methods inside the struct) | 53 | DefaultColors for App |
| 10 | `impl Trait` return type | 33 | impl Iterator<Item = MacroActionData> |
| 11 | method of an untranslated type (cause: std type with no Mojo mapping) | 30 | PlatformFrameSignal |
| 12 | bound on a generic trait | 28 | IntoIterator<Item = u64> |
| 13 | let-else | 28 | let Some((index, _)) = thread_timings.iter().enumerate().find(\|(_, t) \| t.thread_id =... |
| 14 | method of an untranslated type (cause: unsized slice type `[T]`) | 28 | GlobalElementId |
| 15 | impl for an external type (Mojo has no extensions) | 27 | TaskExt for Task |
| 16 | impl on a std/primitive type (Mojo has no extensions) | 26 | From for u32 |
| 17 | method of an untranslated type (cause: recursive struct (Mojo forbids self-referential structs; needs pointers)) | 24 | BackgroundExecutor |
| 18 | `&dyn Trait` argument (no existential types in Mojo) | 20 | &dyn Platform |
| 19 | closure without a return type annotation (needs type inference) | 18 | \| current : &Histogram<u64>, earlier : &Histogram<u64> \| { let mut delta = current.cl... |
| 20 | lock type (no Mojo std equivalent) | 16 | spin::Mutex<ThreadTimings> |
| 21 | std type with no Mojo mapping | 16 | core::panic::Location<'static> |
| 22 | impl for non-path type (reference, tuple, slice, dyn) | 15 | dyn Action |
| 23 | method of an untranslated type (cause: recursive enum (Mojo forbids self-referential structs; needs pointers)) | 15 | KeyBindingContextPredicate |
| 24 | blanket impl `impl<T> Trait for T` | 14 | BorrowAppContext |
| 25 | impl of external trait `JsonSchema` | 14 | JsonSchema for Rgba |

## Top TODO notes on partially translated items

| # | note | occurrences | example |
| ---: | --- | ---: | --- |
| 1 | RefCell lowered to mojo_rt.cell.RefCell (no dynamic borrow checking) | 67 | self.entry.borrow_mut() |
| 2 | shared/immutable string lowered to String | 53 | SharedString -> String |
| 3 | method gated on an extra trait bound (`where conforms_to`) | 49 | Self.T: Copyable |
| 4 | derive `Serialize` has no Mojo equivalent | 46 | #[derive(Serialize)] dropped |
| 5 | derive `Deserialize` has no Mojo equivalent | 45 | #[derive(Deserialize)] dropped |
| 6 | Debug on data enum maps to Variant's Writable (format differs from Rust) | 38 | FrameEvent |
| 7 | dropped bound `Add` (no Mojo std trait) | 36 | `U: Add<U, Output = U>`: operators on generic params need a user trait or DType paramet... |
| 8 | Rc lowered to ArcPointer (atomic refcount) | 35 | Rc<T> -> ArcPointer[T] |
| 9 | SmallVec lowered to List (no inline storage) | 33 | SmallVec<[T; N]> -> List[T] |
| 10 | cfg-gated item translated unconditionally | 32 | cfg(not (feature = "profiler")) |
| 11 | derive `JsonSchema` has no Mojo equivalent | 32 | #[derive(JsonSchema)] dropped |
| 12 | Result<T, E> lowered to `raises` (error type erased) | 31 | E = anyhow::Error |
| 13 | associated function on external type `Duration` | 28 | Duration::from_micros |
| 14 | method `.random_range()` on a value of unknown type passed through | 25 | rng.random_range(0..self.topologies.len()) |
| 15 | impl specialised on a concrete type argument | 24 | method gated with `where Self.T == Pixels` |
| 16 | dropped bound `Into` (no Mojo std trait) | 22 | `F1: Into<Hsla>`: operators on generic params need a user trait or DType parameterisation |
| 17 | slice argument `&[T]` lowered to List[T] borrow | 20 | callers passing sub-slices need Span[T, origin] |
| 18 | dropped bound `Sub` (no Mojo std trait) | 14 | `U: Sub<Output = U>`: operators on generic params need a user trait or DType parameteri... |
| 19 | float->int cast: Rust saturates, Mojo conversion of out-of-range values differs | 14 | (color[0] as f32 / a) as u8 |
| 20 | `&str` return lowered to owned String | 13 | &'static str |
| 21 | associated item on external type `Ordering` | 12 | Ordering::Relaxed |
| 22 | derive `Deref` has no Mojo equivalent | 12 | #[derive(Deref)] dropped |
| 23 | associated function on external type `Instant` | 11 | Instant::now |
| 24 | derive `DerefMut` has no Mojo equivalent | 10 | #[derive(DerefMut)] dropped |
| 25 | derive `Refineable` has no Mojo equivalent | 10 | #[derive(Refineable)] dropped |

## File-level notes

| note | files |
| --- | ---: |
| external crate `collections` | 24 |
| external crate `anyhow` | 22 |
| external crate `smallvec` | 22 |
| cfg-gated module translated unconditionally | 18 |
| external crate `futures` | 15 |
| external crate `scheduler` | 14 |
| external crate `serde` | 12 |
| external crate `gpui_util` | 11 |
| external crate `schemars` | 11 |
| external crate `parking_lot` | 9 |
| external crate `derive_more` | 8 |
| external crate `refineable` | 8 |
| external crate `itertools` | 7 |
| external crate `image` | 5 |
| external crate `accesskit` | 3 |
| external crate `gpui_macros` | 3 |
| external crate `rand` | 3 |
| external crate `raw_window_handle` | 3 |
| external crate `slotmap` | 3 |
| external crate `thiserror` | 3 |
| external crate `async_task` | 2 |
| external crate `bitflags` | 2 |
| external crate `hdrhistogram` | 2 |
| external crate `objc2_core_foundation` | 2 |
| external crate `objc2_core_video` | 2 |

## Per file

| source | items | translated | partial | stubbed | skipped | emitted % |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| src/_accessibility.rs | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/_ownership_and_data_flow.rs | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/action.rs | 26 | 9 | 2 | 15 | 0 | 42.3% |
| src/action.rs#mod no_action | 4 | 0 | 1 | 3 | 0 | 25.0% |
| src/app.rs | 309 | 24 | 13 | 267 | 5 | 12.2% |
| src/app/async_context.rs | 53 | 15 | 4 | 34 | 0 | 35.8% |
| src/app/bench_context.rs | 92 | 23 | 7 | 60 | 2 | 33.3% |
| src/app/context.rs | 65 | 0 | 0 | 65 | 0 | 0.0% |
| src/app/entity_map.rs | 111 | 16 | 9 | 85 | 1 | 22.7% |
| src/app/visual_test_context.rs | 44 | 0 | 0 | 44 | 0 | 0.0% |
| src/arena.rs | 23 | 5 | 3 | 14 | 1 | 36.4% |
| src/asset_cache.rs | 14 | 3 | 4 | 6 | 1 | 53.8% |
| src/assets.rs | 15 | 4 | 1 | 9 | 1 | 35.7% |
| src/bench.rs | 2 | 0 | 0 | 2 | 0 | 0.0% |
| src/bounds_tree.rs | 16 | 8 | 3 | 4 | 1 | 73.3% |
| src/color.rs | 73 | 40 | 11 | 21 | 1 | 70.8% |
| src/color.rs#mod property | 2 | 0 | 0 | 2 | 0 | 0.0% |
| src/colors.rs | 12 | 10 | 0 | 2 | 0 | 83.3% |
| src/debug_overlay.rs | 26 | 19 | 1 | 5 | 1 | 80.0% |
| src/element.rs | 57 | 7 | 1 | 48 | 1 | 14.3% |
| src/elements/anchored.rs | 22 | 11 | 4 | 6 | 1 | 71.4% |
| src/elements/animation.rs | 27 | 1 | 1 | 24 | 1 | 7.7% |
| src/elements/animation.rs#mod easing | 7 | 3 | 0 | 4 | 0 | 42.9% |
| src/elements/canvas.rs | 8 | 0 | 0 | 8 | 0 | 0.0% |
| src/elements/container_query.rs | 8 | 0 | 0 | 8 | 0 | 0.0% |
| src/elements/deferred.rs | 10 | 5 | 0 | 4 | 1 | 55.6% |
| src/elements/div.rs | 166 | 15 | 29 | 121 | 1 | 26.7% |
| src/elements/image_cache.rs | 32 | 5 | 2 | 25 | 0 | 21.9% |
| src/elements/image_cache.rs#mod any_image_cache | 1 | 0 | 0 | 1 | 0 | 0.0% |
| src/elements/img.rs | 48 | 11 | 3 | 33 | 1 | 29.8% |
| src/elements/list.rs | 98 | 32 | 24 | 41 | 1 | 57.7% |
| src/elements/mod.rs | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/elements/surface.rs | 11 | 5 | 2 | 4 | 0 | 63.6% |
| src/elements/svg.rs | 47 | 26 | 3 | 17 | 1 | 63.0% |
| src/elements/text.rs | 76 | 6 | 19 | 50 | 1 | 33.3% |
| src/elements/uniform_list.rs | 35 | 6 | 10 | 17 | 2 | 48.5% |
| src/executor.rs | 46 | 1 | 0 | 44 | 1 | 2.2% |
| src/geometry.rs | 286 | 128 | 100 | 57 | 1 | 80.0% |
| src/gestures.rs | 56 | 31 | 12 | 12 | 1 | 78.2% |
| src/gestures.rs#mod friction_spline | 13 | 9 | 1 | 1 | 2 | 90.9% |
| src/global.rs | 6 | 2 | 0 | 4 | 0 | 33.3% |
| src/gpui.rs | 12 | 2 | 1 | 7 | 2 | 30.0% |
| src/gpui.rs#mod private | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/gpui.rs#mod seal | 1 | 1 | 0 | 0 | 0 | 100.0% |
| src/hot_reload.rs | 5 | 1 | 0 | 3 | 1 | 25.0% |
| src/input.rs | 21 | 3 | 1 | 16 | 1 | 20.0% |
| src/inspector.rs | 2 | 0 | 1 | 1 | 0 | 50.0% |
| src/inspector.rs#mod conditional | 20 | 6 | 1 | 13 | 0 | 35.0% |
| src/inspector.rs#mod inspector_reflection | 2 | 0 | 0 | 2 | 0 | 0.0% |
| src/interactive.rs | 77 | 55 | 9 | 12 | 1 | 84.2% |
| src/key_dispatch.rs | 46 | 20 | 11 | 14 | 1 | 68.9% |
| src/keymap.rs | 17 | 6 | 2 | 8 | 1 | 50.0% |
| src/keymap/binding.rs | 15 | 1 | 0 | 14 | 0 | 6.7% |
| src/keymap/context.rs | 42 | 12 | 2 | 27 | 1 | 34.1% |
| src/path_builder.rs | 51 | 18 | 10 | 23 | 0 | 54.9% |
| src/path_winding.rs | 50 | 16 | 1 | 32 | 1 | 34.7% |
| src/platform.rs | 233 | 90 | 41 | 96 | 6 | 57.7% |
| src/platform/app_menu.rs | 26 | 3 | 7 | 15 | 1 | 40.0% |
| src/platform/keyboard.rs | 5 | 2 | 1 | 2 | 0 | 60.0% |
| src/platform/keystroke.rs | 58 | 42 | 7 | 9 | 0 | 84.5% |
| src/platform/layer_shell.rs | 5 | 3 | 1 | 1 | 0 | 80.0% |
| src/platform/platform_info.rs | 4 | 4 | 0 | 0 | 0 | 100.0% |
| src/platform/popup.rs | 5 | 3 | 1 | 1 | 0 | 80.0% |
| src/platform/scap_screen_capture.rs | 18 | 2 | 2 | 14 | 0 | 22.2% |
| src/platform/test.rs | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/platform/test/dispatcher.rs | 23 | 17 | 3 | 3 | 0 | 87.0% |
| src/platform/test/display.rs | 7 | 5 | 2 | 0 | 0 | 100.0% |
| src/platform/test/platform.rs | 108 | 3 | 4 | 98 | 3 | 6.7% |
| src/platform/test/window.rs | 90 | 0 | 0 | 90 | 0 | 0.0% |
| src/platform/threaded_dispatcher.rs | 35 | 4 | 0 | 30 | 1 | 11.8% |
| src/platform/visual_test.rs | 60 | 0 | 0 | 60 | 0 | 0.0% |
| src/platform_scheduler.rs | 17 | 0 | 0 | 16 | 1 | 0.0% |
| src/prelude.rs | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/profiler.rs | 121 | 30 | 37 | 53 | 1 | 55.8% |
| src/profiler/actions.rs | 21 | 6 | 6 | 9 | 0 | 57.1% |
| src/profiler/hang.rs | 25 | 9 | 8 | 7 | 1 | 70.8% |
| src/profiler/hang/monitor.rs | 8 | 5 | 0 | 3 | 0 | 62.5% |
| src/profiler/journal.rs | 132 | 51 | 16 | 59 | 6 | 53.2% |
| src/queue.rs | 36 | 11 | 4 | 20 | 1 | 42.9% |
| src/randomized_element_tree.rs | 103 | 45 | 20 | 35 | 3 | 65.0% |
| src/scene.rs | 74 | 51 | 5 | 17 | 1 | 76.7% |
| src/seeds.rs | 3 | 0 | 0 | 2 | 1 | 0.0% |
| src/shared_uri.rs | 4 | 0 | 1 | 3 | 0 | 25.0% |
| src/spring.rs | 48 | 21 | 6 | 20 | 1 | 57.4% |
| src/style.rs | 67 | 27 | 24 | 15 | 1 | 77.3% |
| src/styled.rs | 2 | 0 | 1 | 1 | 0 | 50.0% |
| src/subscription.rs | 26 | 4 | 5 | 16 | 1 | 36.0% |
| src/svg_renderer.rs | 33 | 9 | 1 | 22 | 1 | 31.2% |
| src/tab_stop.rs | 20 | 9 | 3 | 7 | 1 | 63.2% |
| src/taffy.rs | 48 | 11 | 4 | 32 | 1 | 31.9% |
| src/text_system.rs | 115 | 53 | 12 | 47 | 3 | 58.0% |
| src/text_system/font_fallbacks.rs | 3 | 1 | 1 | 1 | 0 | 66.7% |
| src/text_system/font_features.rs | 12 | 1 | 1 | 10 | 0 | 16.7% |
| src/text_system/line.rs | 25 | 6 | 6 | 12 | 1 | 50.0% |
| src/text_system/line_layout.rs | 50 | 23 | 4 | 22 | 1 | 55.1% |
| src/text_system/line_wrapper.rs | 27 | 16 | 1 | 9 | 1 | 65.4% |
| src/util.rs | 14 | 7 | 2 | 4 | 1 | 69.2% |
| src/view.rs | 47 | 4 | 1 | 42 | 0 | 10.6% |
| src/view.rs#mod any_view | 1 | 0 | 0 | 1 | 0 | 0.0% |
| src/window.rs | 506 | 59 | 38 | 405 | 4 | 19.3% |
| src/window/a11y.rs | 38 | 10 | 8 | 19 | 1 | 48.6% |
| src/window/a11y/debug.rs | 11 | 3 | 2 | 6 | 0 | 45.5% |
| src/window/prompts.rs | 15 | 7 | 0 | 8 | 0 | 46.7% |

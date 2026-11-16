# rust-to-mojo coverage: `gpui`

Target: Mojo 1.0 (Modular 26.5, Aug 2026): def-only functions, comptime, Deinitable, Array, std.* imports  
Tool: rust-to-mojo 0.1.0

| metric | value |
| --- | ---: |
| files | 103 |
| items (total) | 4700 |
| translated (clean) | 1802 |
| partial (code + TODO notes) | 849 |
| stubbed (TODO + original Rust) | 1923 |
| skipped (tests) | 126 |
| emitted % (translated+partial / non-skipped) | 58.0% |
| clean % (translated / non-skipped) | 39.4% |
| internal errors (panics caught) | 0 |
| files with parse errors | 0 |

## Top reasons items were stubbed

| # | reason | items | example |
| ---: | --- | ---: | --- |
| 1 | closure with untyped parameters (needs type inference) | 189 | \| t \| Some((t.thread_id, t.timings.upgrade() ?)) |
| 2 | uses a field erased to `Erased` (cause: `dyn Trait` (no existential types in Mojo)) | 188 | self.platform |
| 3 | `dyn Trait` (no existential types in Mojo) | 176 | dyn Action + 'static |
| 4 | closure without a return type annotation (needs type inference) | 124 | \| dirty_at \| self.draw_end.duration_since(dirty_at) |
| 5 | bound on a generic trait | 65 | Future<Output = R> |
| 6 | let-else | 50 | let Some((index, _)) = thread_timings.iter().enumerate().find(\|(_, t) \| t.thread_id =... |
| 7 | `.into()` without a known target type | 48 | from.into() |
| 8 | `impl Trait` return type | 43 | impl Iterator<Item = MacroActionData> |
| 9 | std type with no Mojo mapping | 42 | core::panic::Location<'static> |
| 10 | Result type outside a function return position | 34 | anyhow::Result<()> |
| 11 | reference type in field/value position (needs Pointer + origin) | 32 | type `&mut Window` |
| 12 | method of an untranslated type (cause: `dyn Trait` (no existential types in Mojo)) | 27 | Effect |
| 13 | method of an untranslated type (cause: std type with no Mojo mapping) | 25 | ElementId |
| 14 | method of an untranslated type (cause: recursive struct (Mojo forbids self-referential structs; needs pointers)) | 24 | BackgroundExecutor |
| 15 | returning a slice / trait-object reference | 21 | &[&'static str] |
| 16 | impl for an external type (Mojo has no extensions) | 20 | TaskExt for Task |
| 17 | `?` on a Result that does not come from a translated raising call | 19 | Histogram::new(3).map_err(\| error \| { anyhow::anyhow !("Failed to create dirty-to-pre... |
| 18 | block/if/match with statements in expression position (type unknown) | 19 | match self.by_refresh_interval.iter().position(\| frames \| frames.refresh_interval == ... |
| 19 | constructs a value with a field erased to `Erased` (cause: `dyn Trait` (no existential types in Mojo)) | 17 | Self { _release : gpui_util::defer(Box::new(release)), } |
| 20 | method signature collides with another impl's method (Mojo needs one definition) | 17 | App.leak_detector_snapshot |
| 21 | impl of std trait `Deref` (no Mojo mapping yet) | 16 | Deref for GlobalColors |
| 22 | std method `.debug_struct()` not mapped | 16 | f.debug_struct("TaskTiming") |
| 23 | call into an external crate/module | 15 | gpui::profiler::get_current_thread_task_timings(included) |
| 24 | impl for non-path type (reference, tuple, slice, dyn) | 15 | dyn Action |
| 25 | impl on a std/primitive type (Mojo has no extensions) | 15 | From for u32 |

## Top TODO notes on partially translated items

| # | note | occurrences | example |
| ---: | --- | ---: | --- |
| 1 | field type erased to `Erased` (code using the field is stubbed) | 130 | TaskTiming.location: std type with no Mojo mapping (core::panic::Location<'static>) |
| 2 | lock lowered to mojo_rt.sync shim (no mutual exclusion; guards are plain references) | 83 | spin::Mutex<ThreadTimings> -> mojo_rt.sync.Mutex |
| 3 | `Option<&T>` lowered to `Optional[T]` (copies the referent) | 78 | Option<&'static str> |
| 4 | RefCell lowered to mojo_rt.cell.RefCell (no dynamic borrow checking) | 74 | self.entry.borrow_mut() |
| 5 | cfg-gated item translated unconditionally | 68 | cfg(not (feature = "profiler")) |
| 6 | shared/immutable string lowered to String | 62 | SharedString -> String |
| 7 | Result<T, E> lowered to `raises` (error type erased) | 60 | E = anyhow::Error |
| 8 | derive `Serialize` has no Mojo equivalent | 46 | #[derive(Serialize)] dropped |
| 9 | Rc lowered to ArcPointer (atomic refcount) | 45 | Rc<T> -> ArcPointer[T] |
| 10 | derive `Deserialize` has no Mojo equivalent | 45 | #[derive(Deserialize)] dropped |
| 11 | SmallVec lowered to List (no inline storage) | 39 | SmallVec<[T; N]> -> List[T] |
| 12 | Debug on data enum maps to Variant's Writable (format differs from Rust) | 38 | FrameEvent |
| 13 | derive `JsonSchema` has no Mojo equivalent | 32 | #[derive(JsonSchema)] dropped |
| 14 | dropped bound `Into` (no Mojo std trait) | 31 | `F1: Into<Hsla>`: operators on generic params need a user trait or DType parameterisation |
| 15 | associated function on external type `Duration` | 30 | Duration::from_micros |
| 16 | impl specialised on a concrete type argument | 27 | method gated with `where Self.T == Pixels` |
| 17 | method `.random_range()` on a value of unknown type passed through | 25 | rng.random_range(0..self.topologies.len()) |
| 18 | slice argument `&[T]` lowered to List[T] borrow | 25 | callers passing sub-slices need Span[T, origin] |
| 19 | associated function on external type `Instant` | 18 | Instant::now |
| 20 | Weak<T> lowered to WeakPointer[T] (1.0 API unverified; upgrade() calls are not mapped) | 15 | std::sync::Weak<GuardedTaskTimings> |
| 21 | associated item on external type `Ordering` | 15 | Ordering::Relaxed |
| 22 | extension impl lowered to a free function (Mojo has no extensions) | 14 | impl Half for i32 -> Half_i32_half(self, ...); method-syntax call sites are not rewritten |
| 23 | float->int cast: Rust saturates, Mojo conversion of out-of-range values differs | 14 | (color[0] as f32 / a) as u8 |
| 24 | `&str` return lowered to owned String | 13 | &'static str |
| 25 | derive `Deref` has no Mojo equivalent | 12 | #[derive(Deref)] dropped |

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
| src/action.rs | 26 | 7 | 4 | 15 | 0 | 42.3% |
| src/action.rs#mod no_action | 4 | 0 | 3 | 1 | 0 | 75.0% |
| src/app.rs | 311 | 70 | 32 | 202 | 7 | 33.6% |
| src/app/async_context.rs | 53 | 22 | 8 | 23 | 0 | 56.6% |
| src/app/bench_context.rs | 92 | 39 | 15 | 36 | 2 | 60.0% |
| src/app/context.rs | 65 | 14 | 5 | 46 | 0 | 29.2% |
| src/app/entity_map.rs | 123 | 39 | 24 | 59 | 1 | 51.6% |
| src/app/visual_test_context.rs | 44 | 21 | 6 | 16 | 1 | 62.8% |
| src/arena.rs | 23 | 6 | 6 | 10 | 1 | 54.5% |
| src/asset_cache.rs | 14 | 4 | 4 | 5 | 1 | 61.5% |
| src/assets.rs | 16 | 5 | 1 | 9 | 1 | 40.0% |
| src/bench.rs | 2 | 0 | 0 | 2 | 0 | 0.0% |
| src/bounds_tree.rs | 16 | 10 | 1 | 4 | 1 | 73.3% |
| src/color.rs | 79 | 42 | 11 | 25 | 1 | 67.9% |
| src/color.rs#mod property | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/colors.rs | 12 | 11 | 0 | 1 | 0 | 91.7% |
| src/debug_overlay.rs | 26 | 19 | 1 | 5 | 1 | 80.0% |
| src/element.rs | 57 | 14 | 17 | 25 | 1 | 55.4% |
| src/elements/anchored.rs | 22 | 11 | 6 | 4 | 1 | 81.0% |
| src/elements/animation.rs | 27 | 6 | 8 | 12 | 1 | 53.8% |
| src/elements/animation.rs#mod easing | 7 | 4 | 0 | 3 | 0 | 57.1% |
| src/elements/canvas.rs | 8 | 2 | 2 | 4 | 0 | 50.0% |
| src/elements/container_query.rs | 8 | 2 | 3 | 3 | 0 | 62.5% |
| src/elements/deferred.rs | 10 | 5 | 2 | 2 | 1 | 77.8% |
| src/elements/div.rs | 171 | 34 | 36 | 100 | 1 | 41.2% |
| src/elements/image_cache.rs | 31 | 8 | 5 | 18 | 0 | 41.9% |
| src/elements/image_cache.rs#mod any_image_cache | 1 | 0 | 0 | 1 | 0 | 0.0% |
| src/elements/img.rs | 46 | 11 | 5 | 29 | 1 | 35.6% |
| src/elements/list.rs | 98 | 35 | 26 | 36 | 1 | 62.9% |
| src/elements/mod.rs | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/elements/surface.rs | 11 | 5 | 4 | 2 | 0 | 81.8% |
| src/elements/svg.rs | 47 | 30 | 6 | 10 | 1 | 78.3% |
| src/elements/text.rs | 76 | 7 | 26 | 42 | 1 | 44.0% |
| src/elements/uniform_list.rs | 34 | 12 | 13 | 7 | 2 | 78.1% |
| src/executor.rs | 46 | 4 | 2 | 39 | 1 | 13.3% |
| src/geometry.rs | 313 | 204 | 47 | 61 | 1 | 80.4% |
| src/gestures.rs | 66 | 41 | 12 | 12 | 1 | 81.5% |
| src/gestures.rs#mod friction_spline | 13 | 9 | 1 | 1 | 2 | 90.9% |
| src/global.rs | 6 | 3 | 0 | 3 | 0 | 50.0% |
| src/gpui.rs | 12 | 3 | 2 | 5 | 2 | 50.0% |
| src/gpui.rs#mod private | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/gpui.rs#mod seal | 1 | 1 | 0 | 0 | 0 | 100.0% |
| src/hot_reload.rs | 6 | 2 | 0 | 3 | 1 | 40.0% |
| src/input.rs | 21 | 4 | 0 | 16 | 1 | 20.0% |
| src/inspector.rs | 2 | 0 | 1 | 1 | 0 | 50.0% |
| src/inspector.rs#mod conditional | 20 | 6 | 4 | 10 | 0 | 50.0% |
| src/inspector.rs#mod inspector_reflection | 2 | 0 | 1 | 1 | 0 | 50.0% |
| src/interactive.rs | 101 | 79 | 10 | 11 | 1 | 89.0% |
| src/key_dispatch.rs | 46 | 20 | 10 | 15 | 1 | 66.7% |
| src/keymap.rs | 17 | 6 | 2 | 8 | 1 | 50.0% |
| src/keymap/binding.rs | 15 | 4 | 2 | 9 | 0 | 40.0% |
| src/keymap/context.rs | 42 | 12 | 2 | 27 | 1 | 34.1% |
| src/path_builder.rs | 46 | 20 | 11 | 15 | 0 | 67.4% |
| src/path_winding.rs | 42 | 16 | 4 | 21 | 1 | 48.8% |
| src/platform.rs | 234 | 99 | 51 | 78 | 6 | 65.8% |
| src/platform/app_menu.rs | 26 | 3 | 7 | 15 | 1 | 40.0% |
| src/platform/keyboard.rs | 5 | 2 | 3 | 0 | 0 | 100.0% |
| src/platform/keystroke.rs | 59 | 42 | 10 | 7 | 0 | 88.1% |
| src/platform/layer_shell.rs | 5 | 3 | 1 | 1 | 0 | 80.0% |
| src/platform/platform_info.rs | 4 | 4 | 0 | 0 | 0 | 100.0% |
| src/platform/popup.rs | 5 | 3 | 1 | 1 | 0 | 80.0% |
| src/platform/scap_screen_capture.rs | 18 | 2 | 4 | 12 | 0 | 33.3% |
| src/platform/test.rs | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/platform/test/dispatcher.rs | 23 | 17 | 4 | 2 | 0 | 91.3% |
| src/platform/test/display.rs | 7 | 5 | 2 | 0 | 0 | 100.0% |
| src/platform/test/platform.rs | 108 | 22 | 15 | 41 | 30 | 47.4% |
| src/platform/test/window.rs | 90 | 20 | 32 | 37 | 1 | 58.4% |
| src/platform/threaded_dispatcher.rs | 36 | 8 | 15 | 11 | 2 | 67.6% |
| src/platform/visual_test.rs | 60 | 13 | 5 | 42 | 0 | 30.0% |
| src/platform_scheduler.rs | 17 | 0 | 4 | 12 | 1 | 25.0% |
| src/prelude.rs | 0 | 0 | 0 | 0 | 0 | 100.0% |
| src/profiler.rs | 121 | 34 | 40 | 46 | 1 | 61.7% |
| src/profiler/actions.rs | 21 | 6 | 6 | 9 | 0 | 57.1% |
| src/profiler/hang.rs | 25 | 9 | 8 | 7 | 1 | 70.8% |
| src/profiler/hang/monitor.rs | 8 | 5 | 0 | 3 | 0 | 62.5% |
| src/profiler/journal.rs | 133 | 60 | 26 | 41 | 6 | 67.7% |
| src/queue.rs | 36 | 11 | 5 | 19 | 1 | 45.7% |
| src/randomized_element_tree.rs | 103 | 45 | 22 | 33 | 3 | 67.0% |
| src/scene.rs | 86 | 57 | 6 | 22 | 1 | 74.1% |
| src/seeds.rs | 3 | 0 | 0 | 2 | 1 | 0.0% |
| src/shared_uri.rs | 4 | 0 | 1 | 3 | 0 | 25.0% |
| src/spring.rs | 40 | 22 | 11 | 6 | 1 | 84.6% |
| src/style.rs | 70 | 31 | 26 | 12 | 1 | 82.6% |
| src/styled.rs | 2 | 0 | 1 | 1 | 0 | 50.0% |
| src/subscription.rs | 26 | 5 | 7 | 13 | 1 | 48.0% |
| src/svg_renderer.rs | 33 | 9 | 3 | 20 | 1 | 37.5% |
| src/tab_stop.rs | 20 | 9 | 4 | 6 | 1 | 68.4% |
| src/taffy.rs | 33 | 11 | 4 | 17 | 1 | 46.9% |
| src/text_system.rs | 117 | 56 | 17 | 40 | 4 | 64.6% |
| src/text_system/font_fallbacks.rs | 3 | 1 | 1 | 1 | 0 | 66.7% |
| src/text_system/font_features.rs | 12 | 1 | 1 | 10 | 0 | 16.7% |
| src/text_system/line.rs | 23 | 6 | 10 | 6 | 1 | 72.7% |
| src/text_system/line_layout.rs | 52 | 23 | 11 | 17 | 1 | 66.7% |
| src/text_system/line_wrapper.rs | 27 | 16 | 1 | 9 | 1 | 65.4% |
| src/util.rs | 14 | 7 | 2 | 4 | 1 | 69.2% |
| src/view.rs | 42 | 13 | 6 | 23 | 0 | 45.2% |
| src/view.rs#mod any_view | 1 | 0 | 0 | 1 | 0 | 0.0% |
| src/window.rs | 507 | 171 | 68 | 254 | 14 | 48.5% |
| src/window/a11y.rs | 38 | 11 | 12 | 14 | 1 | 62.2% |
| src/window/a11y/debug.rs | 11 | 3 | 4 | 4 | 0 | 63.6% |
| src/window/prompts.rs | 15 | 8 | 1 | 6 | 0 | 60.0% |

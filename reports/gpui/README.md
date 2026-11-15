# GPUI corpus report

Output of `rust-to-mojo` over Zed's `crates/gpui` at the revision in
[SOURCE.txt](SOURCE.txt). Regenerate with `scripts/gpui_corpus.sh`.

| file | contents |
| --- | --- |
| [summary.txt](summary.txt) | CLI summary of the whole-crate run (top 15 stub reasons and TODO notes) |
| [crate.coverage.md](crate.coverage.md) | whole crate: totals, ranked reasons, per-file table |
| [crate.coverage.json](crate.coverage.json) | whole crate, machine-readable: every item with status and reasons (the agent work queue) |
| [crate.hints.json](crate.hints.json) | `--optimize-hints` candidates across the crate |
| [geometry.mojo](geometry.mojo) | translated `src/geometry.rs` (single-file mode) |
| [geometry.coverage.md](geometry.coverage.md) / [.json](geometry.coverage.json) | its coverage |
| [geometry.hints.json](geometry.hints.json) | its optimisation hints |
| [geometry-review.md](geometry-review.md) | **hand review** of `geometry.mojo`: what is right, and what is wrong |
| [color.mojo](color.mojo), [color.property.mojo](color.property.mojo) | translated `src/color.rs` (it has an inline `mod`, so it becomes a package) |
| [color.coverage.json](color.coverage.json) | its coverage |

## Reading the numbers

* **translated**: code emitted with no outstanding notes.
* **partial**: code emitted with `TODO(rust-to-mojo)` notes, such as a dropped bound, an
  unsupported derive, `Rc` → `ArcPointer`, or an erased error type.
* **stubbed**: no code. A TODO header plus the original Rust as comments.
* **emitted %** = (translated + partial) / non-skipped. **clean %** = translated / non-skipped.

The whole-crate figure is dominated by one cause. Types with `dyn Trait` fields (`Box<dyn PlatformWindow>`,
`Rc<dyn Fn>` callbacks), parking_lot locks, or borrowed fields are stubbed, and that
cascades to every method of those types (`App`, `Window`, `Context`...). This is reported as
`method of an untranslated type (cause: ...)`. Those types are GPUI's runtime core, and
porting them is a design task: Mojo 1.0 has no existential types. The leaf modules
(`geometry`, `color`, `style` value types) translate at 69–80% emitted.

Nothing here has been compiled. See MAPPING.md for the unverified syntax list.

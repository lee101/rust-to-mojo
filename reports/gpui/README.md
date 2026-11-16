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

Pass 1 was dominated by a cascade: a type with one `dyn Trait`, lock or borrowed field was
stubbed together with every method. Since pass 2 such fields are erased to
`mojo_rt.erased.Erased` (or lowered to a `mojo_rt.sync` lock or a `Pointer` with an origin
parameter), the type translates, and only code touching the erased field is stubbed
("uses a field erased to `Erased`"). The remaining top reasons are closures whose types
depend on the callee's own generic parameters, `dyn Trait` in signatures, and erased-field
uses. The runtime core (`App`, `Window`, `Context`) still needs a hand design; see
[docs/gpui-runtime-port.md](../../docs/gpui-runtime-port.md).

Nothing here has been compiled. See MAPPING.md for the unverified syntax list.

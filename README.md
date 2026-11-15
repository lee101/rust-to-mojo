# rust-to-mojo

An open-source **Rust → Mojo source translator** written in Rust. It aims to be honest:
anything it cannot translate faithfully becomes a visible
`# TODO(rust-to-mojo): <reason>` with the original Rust attached, and every item is
counted in a coverage report. That report then becomes the work queue for later
(agent) passes.

The long-term goal is to port Zed's **GPUI** UI framework, and the apps built on it,
to Mojo, so Mojo gets a good native desktop graphics/UI library. This first version
targets the Rust subset GPUI's leaf modules use: geometry newtypes and operator impls,
colour math, plain structs and enums, impls, traits and generics, `match`, `Option`, `Vec`/`HashMap`,
and `Rc`/`RefCell`.

* **Target language: Mojo 1.0** (Aug 2026): `def`-only functions, `comptime`,
  `var`/`mut`/`out`/`deinit`/`ref` conventions, `Deinitable`, `Array`, and `std.*`
  imports. See [MAPPING.md](MAPPING.md) for the full concept mapping and the list of
  unverified syntax points.
* **Not yet compiled against a Mojo toolchain.** The output was written without access to
  Mojo. `scripts/check_mojo.sh` is ready to run on a Linux/macOS box with Mojo installed.

## Quick start

```bash
cargo build --release
# one file
./target/release/rust-to-mojo path/to/geometry.rs -o out/geometry
# a whole crate (Cargo.toml [lib] path, or src/lib.rs) -> package tree
./target/release/rust-to-mojo path/to/crates/gpui -o out/gpui --optimize-hints
cargo test                       # golden + robustness tests
scripts/gpui_corpus.sh           # re-run the GPUI corpus and refresh reports/gpui/
scripts/check_mojo.sh            # on a machine with `mojo`: compile-check goldens + mojo_rt
```

The output directory contains:

| file | what |
| --- | --- |
| `<crate>/__init__.mojo`, `<crate>/<mod>.mojo` ... | the module tree (`mod` → package dirs) |
| `mojo_rt/` | the runtime shim (Option/Vec helpers, Rust integer semantics, Cell/RefCell); `--no-runtime` skips it |
| `coverage.json` | per-crate and per-file records: every item with status `translated` / `partial` / `stubbed` / `skipped` and its reasons |
| `coverage.md` | human summary: totals, top stub reasons, top TODO notes, per-file table |
| `hints.json` | with `--optimize-hints`: optimisation candidates (see below) |

Compile the output with the output directory on the import path, e.g.
`mojo build -I out ...`. Generated imports are absolute (`from gpui.geometry import Point`).

## Design

```
 .rs ──syn──▶ AST ──symbols pass (crate-wide)──▶ lowering ──▶ Mojo IR ──printer──▶ .mojo
                                                    │
                                                    └──▶ coverage records (per item)
```

* `src/symbols.rs`: pass 1 over the whole crate. It collects structs (fields, tuple-ness),
  enums (variant shapes), traits, user method names, `Result`-returning functions,
  `Copy` types and the module set.
* `src/lower/`: syn → IR. `items.rs` (modules, structs, enums, traits, impl merging,
  functions), `expr.rs` (statements, expressions, light type inference, method
  mapping), `pat.rs` (pattern → conditions + bindings), `macros.rs`, `derive.rs`,
  `types.rs`.
* `src/ir.rs` and `src/printer.rs`: a small Mojo IR and the only code that formats
  Mojo text (precedence, indentation, escaping).
* `src/report.rs`: coverage model and summaries. `src/driver.rs`: file/crate
  drivers and output layout. `src/hints.rs`: `--optimize-hints`.

### The honesty contract

1. Every lowering function either returns IR believed to be semantically faithful, or
   returns `Unsupported { key, detail }`.
2. The item driver turns an `Unsupported` into a **stub**: a TODO header, a best-effort
   translated signature (as a hint), and the original Rust in comments. A stub never
   leaves callable but wrong code behind.
3. Lossy-but-safe choices (a dropped trait bound, `Rc` → `ArcPointer`, an erased error
   type, a dropped `log::debug!`) produce code plus a `TODO(rust-to-mojo)` note. The
   item is then counted as **partial**.
4. Panics inside the translator are caught per item and reported as
   `internal error`. A run never aborts. The GPUI run has 0 internal errors.
5. Where Rust and Mojo semantics differ in ways that are easy to miss, the output
   goes through `mojo_rt` instead of the look-alike Mojo operator. Examples: `%` (sign
   of the result), `f32::max` (NaN), `round` (ties), `Vec::pop` (Option instead of abort).

Some known simplifications are documented in MAPPING.md, for example `usize` → `Int`,
borrowing Copy arguments instead of copying them, and Debug → Writable output format.

## Results

* **Golden tests:** 25 Rust inputs, each with its expected Mojo and expected per-item
  coverage (`tests/golden/*.rs`, `*.mojo`, `*.coverage.txt`). There is also a crate-mode
  layout test and a never-panic robustness test. See the
  [GPUI hand review](reports/gpui/geometry-review.md) for a critical read of real output.
* **GPUI corpus** (zed `596dfdc`, `crates/gpui`, 103 files, 0 panics, 0 parse errors):

  | scope | items | translated | partial | stubbed | emitted % | clean % |
  | --- | ---: | ---: | ---: | ---: | ---: | ---: |
  | `geometry.rs` | 286 | 127 | 101 | 57 | 80.0 | 44.6 |
  | `color.rs` | 75 | 40 | 11 | 23 | 68.9 | 54.1 |
  | whole crate | 4637 | 1278 | 595 | 2681 | 41.1 | 28.1 |

  The crate total is dominated by stubbed runtime types (`dyn Trait` fields, locks,
  borrowed fields) that cascade to their methods. See [reports/gpui/](reports/gpui/) for
  the ranked reasons, the per-file table and the hand review.

## Pipeline: translate → fix-up passes → optimisation passes

The translator is stage 1 of a three-stage pipeline. It is designed so that agents
can drive stages 2 and 3 from the structured output, in the same spirit as the
`lee101/mojo-factory` porting pipeline:

1. **Translate (this tool, deterministic).** Run over the crate and get `.mojo` plus
   `coverage.json`. Nothing is guessed. Anything unknown is a stub or a TODO.
2. **Fix-up passes (agents, iterative).** Work items come from two sources:
   * `coverage.json` → `stub_reasons` (ranked by frequency) and per-item `reasons`.
     A good loop: pick the top reason, either *teach the translator* (preferred when it
     recurs, e.g. "iterator/closure method `.map()`") or hand-port the item. Then
     re-run the translator and check that the stub count dropped.
   * `scripts/check_mojo.sh` compiler errors on the emitted code. They should be fed back
     the same way: fix the lowering rule if the error is systematic, otherwise patch the file and
     record the patch so a re-translation does not lose it.
   Goldens pin behaviour: every translator change runs `cargo test`, and intentional
   output changes are re-blessed with `UPDATE_GOLDEN=1 cargo test --test golden` and
   reviewed as a diff.
3. **Optimisation passes (agents + benchmarks).** `--optimize-hints` marks candidates
   (`# HINT(rust-to-mojo/opt:<kind>)` comments plus `hints.json`):
   * `simd-struct`: 2–4 homogeneous float fields (`Point<f32>`, `Hsla`, `Rgba`) →
     store as `SIMD[DType.float32, N]` so operator dunders become single vector ops;
   * `dtype-param`: numeric generics over `T` → `[dtype: DType]` with
     `Scalar[dtype]` fields (also fixes the missing arithmetic-trait problem);
   * `vectorize`: range loops with indexed element-wise arithmetic → `vectorize`;
   * `parallelize`: nested loops with independent outer iterations;
   * `simd-pack` / `autotune`: straight-line float math, and tight loops for `comptime`
     unrolling or autotuned tile sizes.
   Each hint is a hypothesis. An optimisation agent rewrites the hinted function,
   benchmarks it against the translated baseline (and the Rust original where possible),
   and keeps the change only if it is faster and its tests still pass.

## Status and roadmap

Done in v0.1: items/impl merging, structs, enums (fieldless → tagged struct; data →
`Variant` wrapper), traits with associated types/consts, generics with bound mapping, specialised
impls via `where`, operator impls (including reflected `impl Mul<P> for f32`), derive mapping
(including derive_more ops), pattern lowering (match/if-let/while-let/let-chains/matches!),
`Option` and `?`, `Result` → `raises`, loops and the common iterator forms in `for`, typed closures
with capture lists, `format!`/`println!`/`assert!`/`panic!`/`vec!`, `Rc`/`RefCell`/`Cell`,
Rust integer semantics via `mojo_rt`, crate mode with `mod` resolution, coverage reports and
optimisation hints.

Next, ranked by GPUI impact (see the corpus report):
1. local type inference for closures, and lowering iterator adapter chains to loops;
2. merging impls defined in other modules into the defining struct;
3. a value-level `Result` shim, plus `match` on `Ok`/`Err` → `try`/`except`;
4. `Ordering`/`partial_cmp`, `Hash` and `Iterator` impls;
5. typed `raises E`;
6. run `scripts/check_mojo.sh` on a real Mojo 1.0 toolchain and work through the
   unverified-syntax list in MAPPING.md.

## Licence

MIT. See [LICENSE](LICENSE).

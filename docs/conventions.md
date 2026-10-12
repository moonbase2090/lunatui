# lunatui conventions (PRD M16)

This document defines **one canonical way** for each recurring thing in lunatui. Anything it doesn't cover follows idiomatic Rust and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

- Code that does the same kind of thing a different way is a review blocker.
- Every rule is marked **CI** (checked mechanically) or **Review** (checked by Muse). Rules marked **CI (planned)** don't have a mechanical check yet, so Muse checks them by hand until one lands.
- If a convention blocks real work, change this document in its own PR with MB2090 sign-off. Never route around it.

## 1. Module and file layout

| Rule | Check |
|---|---|
| Crates live at the repository root as `lunatui-<area>/`, plus the `lunatui/` facade, `benches/` and `xtask/` (PRD §3.1). | Review |
| `src/lib.rs` contains only crate attributes, `mod` declarations and `pub use` re-exports. | CI (planned) |
| One concept per module file, `src/<concept>.rs`. A module with submodules is `src/<concept>.rs` plus `src/<concept>/`. Never `mod.rs`. | CI (planned): no `mod.rs` files |
| A source file is at most ~400 lines, and a type has at most ~7 fields and ~10 public methods. These are M9's soft limits: going over one needs a justification in the PR body. | CI (planned): reports every file and type over a limit |
| Unit tests go in a `#[cfg(test)] mod tests` at the bottom of the file they test. Integration tests go in `<crate>/tests/<behavior>.rs`. | Review |
| Crate dependencies point one way. `core` depends on no lunatui crate. `layout`, `text`, `theme` and `term` depend only on `core`. `runtime` depends on those. `widgets` depends on `runtime` and below. The `lunatui` facade re-exports them. `test`, `ratatui` and `prismattyc` may depend on any of these, but nothing depends on them except as a dev-dependency. `core`, `layout` and `text` stay `no_std` (D8). | CI (planned): architecture-boundary check over `cargo metadata` |

The dependency directions above are inferred from PRD §3.1 and must be confirmed by DD-3 (public API).

## 2. Naming

| Rule | Check |
|---|---|
| Casing follows RFC 430. Acronyms are capitalized as words (`Sgr`, `Osc`, not `SGR`). | CI (`clippy::upper_case_acronyms`, compiler lints) |
| Types are nouns, functions are verbs, and booleans read as predicates (`is_dirty`, `within`). | Review |
| Getters have no `get_` prefix. Conversions use `as_` (borrowed, cheap), `to_` (expensive or owned) and `into_` (consuming), per C-GETTER and C-CONV. | Review |
| Crate names are `lunatui-<area>`. Module names are singular nouns. | Review |
| No abbreviations except the terminal vocabulary used in the PRD (`fg`, `bg`, `attr`, `sgr`, `csi`, `osc`). | Review |

## 3. Error types and propagation

| Rule | Check |
|---|---|
| Each library crate has one error type, `pub enum Error`, in `src/error.rs`. It is `#[non_exhaustive]` and implements `Display`, `std::error::Error` and `From` for wrapped sources. | Review |
| Each library crate has `pub type Result<T, E = Error> = core::result::Result<T, E>`. | Review |
| Errors propagate with `?`. Library code never calls `unwrap`, `expect`, `panic!`, `todo!` or `unimplemented!` (M11). | CI: `clippy::unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, denied workspace-wide crates |
| Binaries (`xtask`, examples, benches) return `std::io::Result<()>` or `ExitCode` from `main`. | Review |
| Error impls are hand-written. Adding an error-derive crate is a dependency-allowlist change (M16). | CI (cargo-deny allowlist, planned) |

## 4. Constructors and builders

| Rule | Check |
|---|---|
| `new` is the primary constructor and takes only required values. A type with a natural empty value implements `Default`, and `new()` without arguments also implements `Default`. | CI (`clippy::new_without_default`) |
| Other constructors are named for what they build (`Options::fullscreen()`) or what they convert from (`from_source`), as in PRD §3.2. | Review |
| Optional settings use consuming builder methods named after the setting, with no `with_` or `set_` prefix: `fn fg(self, color: Color) -> Self`. They are `#[must_use]`. | CI (`clippy::must_use_candidate`, `return_self_not_must_use`) and Review |
| No public fields on types with invariants. Plain data records (`AllocStats`, `WriteStats`) may use public fields. | Review |

## 5. Widget structure

This section follows the PRD §3.2 sketch and is provisional until DD-3 (public API) is signed off.

| Rule | Check |
|---|---|
| One widget per file, `lunatui-widgets/src/<widget>.rs`. | Review |
| A widget is a struct that borrows its data (`'a`) and is built with `new` plus builder methods (§4). | Review |
| It implements `Widget` with `render(&self, area: Rect, cx: &mut RenderCx)` and `measure(&self, avail: Size) -> Size`. `render` takes `&self` and never consumes the widget. | Review |
| State that must survive frames (focus, hover, animation, scroll) lives in the app or the runtime, keyed by the widget's ID (PRD §2.2), not in the widget. | Review |
| Every visual state has a styled snapshot test (M6). | Review |

## 6. Tests and test names

| Rule | Check |
|---|---|
| Test names are snake_case sentences describing the behavior, with no `test_` prefix: `pads_shorter_lines_to_the_widest`. | CI (planned): no `fn test_` |
| One behavior per test. | Review |
| Fallible tests return `Result` and use `?`. They don't call `unwrap` or `expect`. | Review |
| A test whose assertions depend on process-wide state (the counting allocator) runs as a `harness = false` integration test. | Review |
| Property tests are named for the invariant they check (M3). | Review |
| A test fails when the behavior it names breaks. Muse asks of every test: would it fail if that behavior broke? If not, it is rejected (M17). | Review |
| No tautological tests (M17). A test never asserts a value against itself or a constant against the same constant, computes the expected value with the code under test or a copy of its logic, asserts what a mock or stub of the unit under test returns, checks only that code ran, was called or didn't panic, compares against a snapshot accepted without review (M6), or still passes when the implementation is deleted or replaced with a stub or default. | Review |

## 7. Benches

| Rule | Check |
|---|---|
| Benches live in `benches/benches/<name>.rs` with `harness = false` and are run through `cargo xtask bench`. | Review |
| There are three kinds, named by kind: `metrics` (allocation and write counts), `criterion_<area>` (wall clock) and `gungraun_<area>` (instruction counts, behind the `gungraun` feature). | Review |
| Every worst-case scene is registered in the `ceiling` bench (PRD §1.3). CI fails if any sample exceeds the 16 ms ceiling. The 8 ms target is measured on a dedicated machine and reported in every PR's Proof section. | CI (`cargo xtask bench`) and Review |
| Timed code wraps inputs and outputs in `std::hint::black_box`. Write targets are reused, pre-sized buffers, never `io::sink()`, which the optimizer removes. | Review |
| A workload's expected counts are documented on it and checked by the harness self-test. | Review |

## 8. Examples and captures

| Rule | Check |
|---|---|
| An example is `examples/<name>.rs`, registered as an `[[example]]` of the `lunatui` facade with `test = true`, and runs with one `cargo run --example <name>` command (M13). | CI (planned): every file in `examples/` is registered |
| Every example has a row in `examples/README.md`: command, what it shows, and the PR that last changed it. | Review |
| Every example has a capture in `examples/captures/`: `<name>.cast` recorded with asciinema, and `<name>.png`, a still rendered from it with agg. | CI (planned): every example has both files |
| Captures are re-recorded whenever the example's output changes. | Review |

## 9. Feature flags

| Rule | Check |
|---|---|
| Features are additive and named for the capability or dependency they enable, with no `use-` or `with-` prefix (C-FEATURE). | Review |
| Optional dependencies are enabled with `dep:` syntax and never exposed as implicit features. | Review |
| Default features are minimal. | Review |
| CI builds and lints with `--all-features`. | CI |

## 10. Lints, comments and formatting

| Rule | Check |
|---|---|
| Formatting is `rustfmt` with default settings. There is no `rustfmt.toml`. | CI (`cargo fmt --check`) |
| No `#[allow]`. Suppression is only `#[expect(lint, reason = "...")]`, listed in `docs/lints.md` (M16). | CI: `clippy::allow_attributes` and `allow_attributes_without_reason` denied; `cargo xtask check` fails on any unlisted `#[expect]`, a missing reason, or a stale listing |
| No comments except the approved forms below (M14). Anything else fails the comment check. | CI: `cargo xtask check` |
| `///` docs only on `pub` items (not `pub(crate)`), and on the fields, variants and trait items of public types. Never on `impl` blocks, trait-impl items, or items inside function bodies. | CI: `cargo xtask check` |
| Member manifests use `[lints] workspace = true` and never override workspace lints. | CI: `cargo xtask check` |
| Dependencies come only from the `deny.toml` allowlist. Adding a crate, or bumping any version in `Cargo.lock`, needs MB2090 sign-off (M16). | CI: `cargo deny` and `cargo xtask signoff` |
| Lint, CI and dependency configuration (the paths in `.github/CODEOWNERS`) changes only with MB2090 sign-off, in a PR without feature work (M16). | CI: `cargo xtask signoff` (needs the `mb2090-signoff` label) and CODEOWNERS review |
| Every `.rs` file starts with exactly this license header, identical everywhere, followed by one blank line. | CI: `cargo xtask check` |
| Every `unsafe` block and `unsafe impl` is preceded by a `// SAFETY:` comment explaining why its contract holds. | CI: `clippy::undocumented_unsafe_blocks` denied |
| `unsafe` code exists only in small modules that expose a safe API, and SIMD modules keep a scalar reference implementation alongside (M10). | Review |

### Approved comment forms (M14)

The default is no comments. These are the only exceptions MB2090 has approved. A new exception needs his explicit approval, in its own small PR or flagged in a PR body. Once approved, it is added here and to the comment check's allowlist.

| Form | Where | Approved |
|---|---|---|
| `///` rustdoc | On public API items only | M14 |
| `#![doc = include_str!("<path>/README.md")]` | Crate root of every target, for crate-level docs: libraries (`../README.md`), examples (`README.md`), benches, integration tests and `xtask` (`../README.md`) | MB2090, 2026-10-06 |
| License header (text below) | First two lines of every `.rs` file | MB2090, 2026-10-12 |
| `// SAFETY: <why the unsafe contract holds>`, optionally continued on the following `//` lines | Immediately before an `unsafe` block or `unsafe impl` | MB2090, 2026-10-06 |

The license header, exactly:

```rust
// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT
```

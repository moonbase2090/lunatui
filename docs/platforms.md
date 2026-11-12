# Supported platforms

Linux, macOS and Windows are all tier 1 at 1.0. This page says what tier 1 means and how each claim is checked.

## What tier 1 means

For a tier 1 platform:

- The whole workspace builds and passes `cargo test --workspace --all-features` with the stable toolchain.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean.
- The terminal session, input parser and encoder have platform tests that run in CI, not only on a developer machine.
- A bug that happens only on that platform is treated as a release blocker, not a known limitation.

A tier 2 platform builds and gets best-effort fixes, with no release gate.

## How each platform is covered

| Platform | Terminal input | Output |
|---|---|---|
| Linux, macOS | Own VT input parser on a raw-mode tty | Own VT encoder |
| Windows | Console events mapped through `crossterm`, in `lunatui_term::sys::windows` | Own VT encoder, which needs a console with virtual-terminal processing (Windows Terminal, or the console host on current Windows 10 and 11) |

The Windows-only code is the `lunatui-term` Windows session, the event mapping in `sys::windows`, and the input path in `lunatui-runtime`. The Windows-only tests are in `lunatui-term/tests/windows_terminal.rs`.

## Checks

The `CI` workflow (`.github/workflows/ci.yml`) runs on every pull request and on a manual dispatch. A pull request runs `test` and `clippy-windows`; the Linux-only jobs that the `Rookrunner` workflow already runs are skipped there and run on dispatch.

- `test` builds and runs the workspace tests on Linux, macOS and Windows, then runs the same tests with `--all-features`.
- `clippy` and `clippy-windows` run the lint on Linux and Windows.

A change that only compiles on Unix, such as an import used by a `cfg(unix)` function, fails on Windows. Gate such items with `cfg(unix)` rather than leaving them unused.

## Known gaps

- No automated check drives a real Windows Terminal. Someone has to run an example there by hand before a release.

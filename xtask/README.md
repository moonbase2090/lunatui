# xtask

Development tasks for lunatui. Run them from anywhere in the repository with `cargo xtask <task>`.

| Task | What it does |
|---|---|
| `cargo xtask bench` | Prints allocation and write metrics, checks the frame-time ceiling, then runs the criterion benchmarks, and the gungraun benchmarks when valgrind is installed. It fails if any step fails, including a ceiling breach. |
| `cargo xtask check` | Checks the conventions that clippy can't (PRD M14, M16): the fixed license header, the comment ban and its approved exceptions, `///` only on public items, crate docs through `include_str!` of a README, every `#[expect]` listed in `docs/lints.md`, and every member manifest using only the workspace lints. |
| `cargo xtask gen-width` | Downloads the Unicode 18.0.0 character database files it needs into `target/ucd/` (with `curl`), classifies every code point as zero-width, wide or narrow, and writes `lunatui-core/src/width/tables.rs`. The classification rules are in `docs/width-policy.md`. |
| `cargo xtask signoff <base> <head> <labeled>` | Fails when a pull request changes a CODEOWNERS path without the `mb2090-signoff` label. Docs-only PRs (markdown only, including under `docs/` and `.cursor/skills/`, except `docs/lints.md` and `docs/conventions.md`) skip sign-off. CI runs it on every pull request. |

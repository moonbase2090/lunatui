# Lints and lint expectations

This file has three lists, kept current in every PR:

- **Lint rules (PRD M15):** a mistake found more than once during development (review findings, CI failures, bugs) becomes a lint rule in the same PR or the next one.
- **Lint expectations (PRD M16):** `#[allow]` is not permitted. The only suppression is `#[expect(lint, reason = "...")]`, and every one in the codebase is listed here. `cargo xtask check` fails on an unlisted `#[expect]`, one without a reason, or a listing whose `#[expect]` no longer exists.
- **Checks:** what enforces M14 and M16 mechanically.

Changes to this file need MB2090 sign-off (`.github/CODEOWNERS`).

## Lint rules (M15)

| Rule | Enforced by | Mistake it prevents | Added in |
|---|---|---|---|

No rules yet.

## Lint expectations (M16)

| Location | Lint | Reason | Added in |
|---|---|---|---|
| `benches/src/calibration.rs`, `grow_buffer` | `clippy::same_item_push` | Growing one push at a time is the workload being measured | PR 02 (#4) |
| `benches/benches/gungraun_calibration.rs`, crate root | `missing_docs` | gungraun's benchmark macros generate undocumented public items | PR 02 (#4); `#[allow]` replaced in #6 |

## Checks

| Check | Enforced by | What it rejects |
|---|---|---|
| Workspace lints | `[workspace.lints]` in `Cargo.toml`, clippy with `-D warnings` | `missing_docs`, `unsafe_op_in_unsafe_fn`, clippy `pedantic`, and the restriction lints `allow_attributes`, `allow_attributes_without_reason`, `undocumented_unsafe_blocks`, `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` |
| Comments (M14) | `cargo xtask check` | Any comment except the fixed license header, `// SAFETY:` runs directly before `unsafe`, and `///` docs. Also `//!` and every block comment |
| Doc placement (M14) | `cargo xtask check` | `///` on anything but `pub` items, and crate docs other than `include_str!` of a README |
| Expectations (M16) | `cargo xtask check` | Unlisted, unreasoned or stale `#[expect]` |
| Member manifests (M16) | `cargo xtask check` | A member without `[lints] workspace = true`, or with its own `[lints.*]` table |
| Dependency allowlist (M16) | `cargo deny` (`[bans] allow` in `deny.toml`) | Any crate not on the allowlist, including transitive and platform-specific crates |
| Sign-off (M16) | `cargo xtask signoff` in the Sign-off workflow, plus CODEOWNERS review | A change to a path in `.github/CODEOWNERS` without the `mb2090-signoff` label |
| Frame budget (PRD §1.3) | `cargo xtask bench` (ceiling bench) | Any worst-case frame sample over 16 ms on the CI runner |

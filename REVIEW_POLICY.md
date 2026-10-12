# Review policy

This policy adds to lunatui's binding PRD §M and §T rules, including Muse blind review, Opus checkpoints, the §8.0 checklist and T5 authorship; it replaces none of them.

This policy applies to every pull request, whether a person or an agent wrote it.

## 1. Classify every PR

**Trunk**: the change touches shared code that other parts depend on:
- core libraries and shared modules
- auth, secrets, and permissions
- data models, schemas, and migrations
- CI, release, and signing workflows; build configuration
- public APIs, CLI flags, config formats
- anything with 5 or more dependents

**Leaf**: everything else, such as a single UI screen, docs, a self-contained script, or test-only changes.

A PR that is partly trunk is trunk. When unsure, call it trunk. Label trunk PRs `trunk` and leaf PRs `leaf`.

**In lunatui, trunk includes:**
- `lunatui-core/`: the shared core layer, including the packed cell data model in `src/cell.rs`, `src/color.rs`, `src/attr.rs` and `src/style.rs`; grapheme handling in `src/grapheme.rs`; width policy and generated tables in `src/width.rs` and `src/width/tables.rs`; and any future shared core module. The current metadata reports `lunatui` as its direct package dependent, and examples and core tests use the core API.
- Public API: any crate's `src/lib.rs` re-exports or any `pub` item, plus the `lunatui/` facade. A signature, semantic output, or compatibility change is trunk even when the implementation is small.
- Design docs: `docs/adr/`, `docs/architecture.md` and `docs/PRD.md`.
- MB2090 sign-off paths from `.github/CODEOWNERS`: `Cargo.toml`, `Cargo.lock`, `deny.toml`, `_typos.toml`, `.cargo/`, `.github/`, `xtask/`, `benches/src/ceiling.rs`, `docs/lints.md` and `docs/conventions.md`. These need the `mb2090-signoff` label unless the pull request is docs-only (see §7).

Leaf examples include an example under `examples/` that does not change shared or public behavior, a widget file once `lunatui-widgets` has widgets when it changes no shared or public behavior, bench reporting outside `benches/src/ceiling.rs`, documentation outside the sign-off paths above, and test-only changes.

## 2. Proof (every PR)

The PR body has a **Proof** section with real evidence: test output, a CI run link, a screenshot or recording for UI changes, or a before/after for behavior changes. "Tested locally" without output is not proof.

## 3. Leaf PRs

- CI green and proof present.
- One review by anyone other than the author (person or agent), from a **different model family** than the author.
- Merge once green.

## 4. Trunk PRs

- CI green, and the proof shows the change running, not just compiling.
- Agentic validation: an agent other than the author builds and exercises the change.
- Independent review, preferably by a different model than the author.
- Every finding is fixed or explicitly waived in the PR thread.
- Then merge.

## 5. Feature gating

New behavior in trunk code ships behind a Cargo feature or setting that is off by default, unless it is a pure fix. Turning a flag or setting on by default is its own trunk PR, and that PR is trunk.

## 6. Tests

For every test, ask: would it fail if the behavior it names broke? If not, reject it. Reject tests that:

- assert a value against itself, or a constant against the same constant
- compute the expected value with the code under test or a copy of its logic
- mock or stub the unit under test, then assert what the mock returns
- only check that something ran, was called, or didn't panic, with no assertion on the result
- compare against a snapshot or golden file regenerated from current output without review
- still pass when the implementation is deleted or replaced with a stub or default

Use `docs/conventions.md` §6 and PRD M17 for mutation-testing guidance and missed-test analysis.

## 7. Merging

- Merge commits only. No admin overrides.
- Code-scanning threads are resolved only when they are report-only or addressed, never just to unblock a merge.
- Releases and tags need owner approval.
- A PR changing a path protected by `.github/CODEOWNERS` also needs the `mb2090-signoff` label, which Moonbase applies, **except docs-only PRs** (every changed file is a markdown file under the repo root, under `docs/`, or under `.cursor/skills/`, excluding `docs/lints.md` and `docs/conventions.md`). Those never need `mb2090-signoff`; `cargo xtask signoff` and the Sign-off workflow treat them as clear.

## Review exceptions

These rules narrow when a fresh review is needed. They replace nothing above unless they say so.

- **Docs-only and changelog-only PRs** may merge on green checks plus the release-text check, with no review. A PR qualifies only if every changed file is documentation or the changelog; any code, config, CI, or workflow change makes it a normal PR.
- **A SHIP carries over** a clean rebase or a label-only change, as long as the PR's own diff is unchanged. Compare the old and new tips with `git range-diff` or the patch-id (`git diff <base>...<tip> | git patch-id --stable`). If the diff changed at all, the PR needs a new review at the new tip.
- **Leaf Windows fixes** merge on a code SHIP and green CI. Windows runtime proof for them is batched and recorded before each release instead of blocking each PR.
- **Still need a full cross-family review at the exact tip**: security and red-team fixes, trunk PRs, CI and workflow changes, release PRs, and public-API changes. The reviewer is a different model family than the author, and the review names the tip SHA.

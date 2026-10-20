# Review policy

This policy applies to every pull request to lunatui, whoever wrote it.

## 1. Classify every PR

**Trunk**: the change touches shared code that other parts depend on:
- `lunatui-core/` (cell data model, grapheme handling, width policy and tables)
- public API: any crate's `src/lib.rs` re-exports, any `pub` item, and the `lunatui/` facade
- design docs: `docs/adr/`, `docs/architecture.md`, `docs/PRD.md`
- CI, release and build configuration, and every path listed in `.github/CODEOWNERS`

**Leaf**: everything else, such as an example, a self-contained widget, bench reporting, documentation outside the paths above, or test-only changes.

A PR that is partly trunk is trunk. When unsure, call it trunk. Label PRs `trunk` or `leaf`.

## 2. Proof

The PR body has a **Proof** section with real evidence: test output, a CI run link, a capture for visual changes, or a before/after for behavior changes. "Tested locally" without output is not proof.

## 3. Independent peer review

- Every PR gets a review from someone other than the author.
- The reviewer names the exact tip commit they reviewed and answers **SHIP** or **CHANGES**. A new commit needs a new review at the new tip.
- A SHIP carries over a clean rebase or a label-only change when the PR's own diff is unchanged (`git range-diff` or `git patch-id --stable` shows no difference).
- Trunk PRs are also built and exercised by the reviewer, not just read. Every finding is fixed or explicitly waived in the PR thread.
- Docs-only and changelog-only PRs may merge on green checks with no review.

## 4. Required checks

All required status checks on `main` must pass before merging. No check is skipped, and code-scanning threads are resolved only when addressed or report-only.

## 5. Sign-off label

A PR that changes a path in `.github/CODEOWNERS` also needs the `mb2090-signoff` label, applied by a maintainer. Docs-only PRs (every changed file is markdown, excluding `docs/lints.md` and `docs/conventions.md`) are exempt. `cargo xtask signoff` and the Sign-off workflow enforce this.

## 6. Feature gating

New behavior in trunk code ships behind a Cargo feature or setting that is off by default, unless it is a pure fix. Turning it on by default is its own trunk PR.

## 7. Tests

Every test must fail if the behavior it names broke. Reject tests that assert a value against itself, compute the expected value with the code under test, assert what a mock of the unit under test returns, only check that nothing panicked, compare against unreviewed regenerated snapshots, or still pass when the implementation is replaced with a stub. See `docs/conventions.md` §6 for mutation-testing guidance.

## 8. Merging

- No admin overrides and no force-pushes to `main`.
- Commits carry no private details: no personal names, paths, hostnames or addresses.
- Releases and tags need owner approval.

# lunatui-core

Core types for lunatui: cells, grid, styles, geometry, diff, grapheme slot and width tables.

It provides display widths (PRD §8 PR 04): `char_width` and `width` from Unicode 18.0.0 tables (`docs/width-policy.md`), grapheme slots and a per-frame `ClusterTable` (PR 05), the 16-byte packed `Cell` with `Color`, `Attrs` and `Style` (PR 06, ADR 0001), a `Grid` with write APIs and row dirty tracking (PR 07), and `diff_runs` to collect changed-cell runs between frames (PR 08). The `simd` module (roadmap PR 47+) holds runtime dispatch, scalar reference kernels, and vectorized `first_mismatch` used by row diff (PR 48+). Encoder types land in later roadmap PRs.

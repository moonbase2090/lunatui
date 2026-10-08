# lunatui-core

Core types for lunatui: cells, grid, styles, geometry, diff, grapheme slot and width tables.

It provides display widths (PRD §8 PR 04): `char_width` and `width` from Unicode 18.0.0 tables (`docs/width-policy.md`), grapheme slots and a per-frame `ClusterTable` (PR 05), and the 16-byte packed `Cell` with `Color`, `Attrs` and `Style` (PR 06, ADR 0001). Grid and encoder types land in later roadmap PRs.

# lunatui-core

Core types for lunatui: cells, grid, styles, geometry, diff, grapheme slot and width tables.

So far it provides display widths (PRD §8 PR 04): `char_width` and `width` give the number of terminal columns a character or string occupies, from tables generated from Unicode 18.0.0. The policy and its differences from the `unicode-width` crate are in `docs/width-policy.md`. The other types land in later roadmap PRs.

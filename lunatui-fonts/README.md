# lunatui-fonts

Font files that Lunatui draws text with in Graphite mode, compiled into the binary so text looks
the same on every host.

| Family | Version | Files | Use |
|---|---|---|---|
| Inter | 4.000 | Regular, Bold | Default UI text |
| IBM Plex Sans | 3.006 | Regular, Bold | Alternative UI text |
| Noto Sans | 2.015 | Regular, Bold | Alternative UI text, and fallback for characters the UI font lacks |
| JetBrains Mono | 2.304 | Regular, Bold | Monospaced text |
| Noto Emoji | 3.002 | One variable file, drawn at its default instance | Last fallback, monochrome emoji |

The nine files add about 5 MB to a binary that uses this crate.

Inter and JetBrains Mono also have compact files, in `fonts/compact/`, through
[`Family::compact`]. Each keeps only the characters in [`COMPACT_RANGES`] (Latin with Latin-1 and
Latin Extended-A, punctuation, currency, arrows, mathematical and technical symbols, box drawing,
block elements and geometric shapes) and has no hinting or layout tables. They are a quarter to a
tenth the size of the full files and parse several times faster, for programs that must draw
their first text quickly on a tight CPU budget, such as plug-ins in a sandbox. The four compact
files add about 200 KB. `scripts/subset-fonts.py` writes them from the full files.

The full files are unmodified upstream releases, and the compact files are subsets of them. All
are licensed under the SIL Open Font License 1.1, and each family's license text is in `fonts/licenses/` and available through
[`Family::license`]. The Rust code in this crate is MIT licensed.

```rust
use lunatui_fonts::{Family, Weight};

let inter = Family::Inter.data(Weight::Regular);
assert!(inter.len() > 100_000);
assert!(Family::Inter.license().contains("SIL Open Font License"));
```

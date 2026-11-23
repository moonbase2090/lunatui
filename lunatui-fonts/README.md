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

The files are unmodified upstream releases. They are licensed under the SIL Open Font License 1.1,
and each family's license text is in `fonts/licenses/` and available through
[`Family::license`]. The Rust code in this crate is MIT licensed.

```rust
use lunatui_fonts::{Family, Weight};

let inter = Family::Inter.data(Weight::Regular);
assert!(inter.len() > 100_000);
assert!(Family::Inter.license().contains("SIL Open Font License"));
```

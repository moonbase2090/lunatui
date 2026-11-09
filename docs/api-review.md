# API review for 1.0

This is the review pass of roadmap PR 57 (PRD §8, phase 9): the public API of every library crate checked against [`conventions.md`](conventions.md), and the breaking-change list that has to be closed before 1.0. This first change is the inventory and the list. Each later change takes one category and closes its entries.

Every entry has one disposition:

- **do-now**: a breaking change we make before 1.0, in the change named in the entry.
- **accept**: the current API stays, with the reason.
- **defer**: needs a decision or an input this review does not have yet, with who decides.

The list is closed when no entry is open.

## Method and scope

The inventory comes from scanning `src/` of every library crate for public items: `pub enum`, `pub struct` with public fields, error types, `get_` functions, `pub mod` in `lib.rs`, and `mod.rs` files. It is a source scan, not rustdoc output, so an item is counted when it is declared `pub`, whether or not it is reachable from the crate root. Items hidden behind a private module are marked **module-only** and are decided separately. The web crate and the test helper crate are not public API and are left out.

Not yet reviewed, and needed before the list can close:

- Trait surfaces (`App`, `Widget`, the sink traits; `RenderCx` is a struct and falls under the struct rules): whether each is sealed, and which methods may grow defaults.
- Facade parity: whether `lunatui` re-exports what each crate exports, and nothing it should not.
- Feature gating of public items, and items that are public only for tests or benches.
- Deprecated items, and items documented as experimental.

## Inventory

Counts are from the scan. "Struct fields" is public structs with at least one public field and no `#[non_exhaustive]`. Platform-specific copies of one type (the session types in `lunatui-term`) are counted once per file.

| Crate | Public enums | With `non_exhaustive` | Public structs | Struct fields |
|---|---|---|---|---|
| lunatui-badge | 1 | 0 | 2 | 0 |
| lunatui-button | 2 | 1 | 2 | 0 |
| lunatui-card | 1 | 0 | 3 | 0 |
| lunatui-choice | 1 | 1 | 3 | 0 |
| lunatui-core | 7 | 1 | 22 | 10 |
| lunatui-data-table | 0 | 0 | 2 | 0 |
| lunatui-dialog | 0 | 0 | 2 | 0 |
| lunatui-field | 0 | 0 | 1 | 0 |
| lunatui-image | 2 | 1 | 8 | 3 |
| lunatui-layout | 4 | 1 | 8 | 4 |
| lunatui-prismattyc | 4 | 1 | 6 | 3 |
| lunatui-ratatui | 0 | 0 | 6 | 3 |
| lunatui-runtime | 8 | 1 | 22 | 1 |
| lunatui-term | 28 | 1 | 23 | 11 |
| lunatui-text | 4 | 1 | 11 | 7 |
| lunatui-theme | 3 | 1 | 4 | 1 |
| lunatui-ui-tokens | 2 | 2 | 1 | 0 |
| lunatui-widgets | 44 | 22 | 151 | 29 |

Totals: 77 public enum declarations without `#[non_exhaustive]` and 72 public struct declarations with public fields and no `#[non_exhaustive]`. Counted as distinct names per crate, that is 71 enums and 69 structs: the unix, stub and windows copies of `SyncOutput`, `MouseTracking`, `SessionError` and `SessionOptions` count once, and core has two structs named `Set` in different modules. Of the 71 enums, 2 are error enums (A1) and 69 are classified in B1 to B6.

Convention checks that pass: no public function is named `get_*`.

## Findings and the breaking-change list

### A. Error types

`conventions.md` §3 asks for one `Error` enum and one `Result` alias per library crate, both `#[non_exhaustive]`. The crates have nine domain-named error types (eleven declarations, counting the three platform copies of `SessionError`) instead and no `Result` alias:

| Type | Crate | `non_exhaustive` |
|---|---|---|
| `RampError` (unit struct) | core | no |
| `ImageError` | image | yes |
| `GridError` | layout | yes |
| `EncodeError` | prismattyc | yes |
| `SessionError` (unix, stub and windows copies) | term | no |
| `ThemeLoadError` | theme | no |
| `MermaidError` | widgets | no |
| `NodeGraphError` | widgets | no |
| `SplitSpecError` | widgets | yes |

| # | Entry | Disposition |
|---|---|---|
| A1 | `SessionError` (three copies), `NodeGraphError`: add `#[non_exhaustive]`. A new failure kind must not be a breaking change. | done in change 1 |
| A2 | `ThemeLoadError`, `MermaidError`, `RampError`: struct errors. Every `pub` type named `*Error` carries `#[non_exhaustive]`, including struct errors whose fields are private, so one mechanical rule covers them all. | done in change 1; `cargo xtask check` now enforces it |
| A3 | One `Error` and `Result` per crate (§3). Nine domain names exist, each specific and already used by callers. Renaming them gains nothing a reader needs. | accept; amend §3 to say each fallible area names its error `<Area>Error`, is `#[non_exhaustive]`, and a crate adds a `Result` alias only when it has one error type. The amendment needs the maintainer's sign-off. |

### B. Enums without `#[non_exhaustive]`

Rule for the sweep: an enum that names a set the outside world defines or extends (protocol and capability values, input kinds, themes, styles, constraint kinds, AST nodes, fonts and patterns) is **open** and gets `#[non_exhaustive]`. An enum that is a closed two-to-four-way choice with a fixed meaning (an axis, a direction, a boolean-like mode) is **closed** and stays exhaustive, so callers can match it without a wildcard.

Each enum made `#[non_exhaustive]` in this pass carries a `compile_fail,E0004` doctest that matches every value without a wildcard, so removing the attribute fails the test. Downstream code that matches an open enum then needs a wildcard arm. That is the intended cost and the reason to do it before 1.0.

| # | Entry | Disposition |
|---|---|---|
| B1 | Open enums, term: `Event`, `KeyCode`, `KeyKind`, `MouseKind`, `MouseButton`, `Scroll`, `Paste`, `ColorDepth`, `PrivateMode`, `KeyboardProtocol`, `GraphicsProtocol`, `Hyperlinks`, `WidthPolicy`, `Multiplexer`, `RecordedTerminal`. | done in change 2 |
| B2 | Open enums, text: the markdown nodes `Block`, `Inline` and `Alignment`, reachable through the public `markdown` module. | done in change 3 |
| B2a | `Color`, `UnderlineStyle` and `DecodedGlyph` in core: the packed cell format fixes them (`Color` uses all four values of its two kind bits, the glyph slot has three fixed kinds, the underline field has six styles in three bits), so a new value is a cell-format change under the cell ADR, not an additive one. Encoders in other crates match them exhaustively on purpose, so a format change is a compile error there. | accept |
| B2b | `Constraint` and `Flex` in layout: a closed algebra the solver implements and the ratatui parity tests mirror variant for variant. A new variant needs solver work and a parity case, so it is an API decision, not a drive-by addition. | accept |
| B3 | Open enums, widgets and the widget crates: `BorderType`, `SpinnerPattern`, `BigTextFont`, `PixelSize`, `LogLevel`, `DiffKind`, `TimelineStatus`, `EditResult`, `MenuEvent`, `ColorPickerFocus`, `Look`, `ButtonSize`, `Fit` (image). | done in change 4 |
| B4 | Open enums, runtime, theme and prismattyc: `Ease`, `Profile`, `NativeDiscovery`. | done in change 3 |
| B4a | `Variant` in theme: light or dark, a two-way choice with no third meaning. | accept |
| B5 | Closed enums stay exhaustive: `Axis`, `Orientation`, `Direction` (effects), `FocusDirection`, `Alignment` and `TitlePosition` (widgets), `MouseUnit`, `Focus`, `ColorMode`, `SyncOutput`, `MouseTracking`, `ControlFlow`, `State`, `SchedulePoll`, `CellWidth`, `DiffLayout`, `WeekStart`, `SplitAxis`, `RenderDirection`, `Baseline`, `KeymapMode`, `VimMode`, `FileKind`, `AttributeMode`, `NativePolicy`, `PressRoute`. | accept |
| B6 | `RampOutput`, `GlyphSet` and `Face`: the first scan called them module-only, but each is in a public module and in a public signature or used across crates, so they are public API. Each names an extensible set (an output policy, a glyph tier, Graphite's face contract), so they get `#[non_exhaustive]`. | done in change 3 |

### C. Structs with public fields

A struct with public fields and no `#[non_exhaustive]` cannot gain a field without breaking every struct literal. Value types that are complete by nature stay open to construction. Option, result and state types grow.

| # | Entry | Disposition |
|---|---|---|
| C1 | Complete value types: `Size`, `Rect`, `Style`, `Meta`, `Padding`, `PixelPoint`, `PixelRect`, `TextPosition`, `TimelineItemId`, `Divider`, and the geometry shapes `Points`, `Line`, `Rectangle`, `Circle`. | accept |
| C2 | Option and configuration types that will gain fields: `RunOptions`, `SessionOptions`, `ProbeEnv`, `ToastPlacement`. Add `#[non_exhaustive]` and keep `Default` plus builder methods as the way to construct them. | done in change 5 |
| C3 | Result and report types a library produces and callers only read: `Caps`, `ProbeReply`, `TermIdentity`, `DecModes`, `Protocols`, `DiscoveryResult`, `FrameDiff`, `PresentStats`, `SearchMatch`, `PieLayout`, `SplitLayout`, `Solved`, `SolvedGrid`, `Wrapped`, `WrappedLine`, `ImageGeometry`, `TreeRow`, `HeatmapCell`, `TimelineTick`, `LogEntry`, `DiffLine`, `DiffHunk`. Add `#[non_exhaustive]`. `HeatmapCell` is also an input to `Heatmap::selected` and is a complete two-coordinate value, so it stays open to construction (C1). `SolvedGrid` and `TreeRow` have private fields, so they can already neither be built nor listed field by field outside their crate, and the attribute would change nothing. `SearchMatch` is also passed in to `ComboBox::with_matches`, so it gains `SearchMatch::new`. | done in change 6; `HeatmapCell` accepted as a complete value; `SolvedGrid` and `TreeRow` need no change |
| C4 | The rest of the 72 struct declarations in the table (those not named in C1 to C3) (event payloads, state holders, markdown tree nodes, rendering areas). Each is decided with its crate's change using the same rule: complete value stays, grows gets `#[non_exhaustive]`. | defer to changes 2 to 5; the closing change lists the result |

### D. Naming

| # | Entry | Disposition |
|---|---|---|
| D1 | `get_*` functions: none found. | accept |
| D2 | `as_`, `to_` and `into_` conversions, and boolean predicate names (§2) are a review rule with no mechanical check. A scan for the common slips (`to_` on a borrowed cheap view, `is_` on a non-boolean) is part of the closing change. | defer to the closing change |

### E. Module exposure

`pub mod` appears in `lib.rs` of core (5), runtime (2), term (1), text (3), widgets (2) and the web helper (3). Each public module is a path callers can depend on.

| # | Entry | Disposition |
|---|---|---|
| E1 | Review each public module: keep it public only if the crate root does not already re-export what a caller needs. A module made private is a breaking change. | defer to the closing change (needs the facade parity check from the scope list) |

### F. Conventions that are about layout, not API

Eight `mod.rs` files exist (`core`: `simd`, `symbols`; `runtime`: `anim`; `term`: `encode`, `sys`, `session`; `text`: `markdown`, `wrap`) against the rule that a module is `<concept>.rs` plus `<concept>/`. Ten crates also keep more than re-exports in `lib.rs`. Neither changes a public path, so neither is a breaking change.

| # | Entry | Disposition |
|---|---|---|
| F1 | `mod.rs` files and `lib.rs` content. Both rules are marked "CI (planned)". The fix is an `xtask` check plus the file moves, with no API effect. | accept for 1.0 (not a breaking change); tracked as separate conventions work |

## Changes this list expects

1. Error types: A1, A2, and the amendment in A3.
2. Term enums: B1, plus term structs from C4.
3. Core, layout, text, runtime, theme and prismattyc enums: B2, B4, B6.
4. Widget enums: B3, plus widget structs from C4.
5. Option structs: C2.
6. Result structs: C3.
7. Closing change: D2, E1, the C4 leftovers, the scope items above, and a stability policy for 1.0 (what counts as breaking, how `non_exhaustive` and features are used).

Each change adds a `CHANGELOG.md` entry under Unreleased that names the types and shows the one-line fix for downstream code (a wildcard arm, or `..Default::default()` for a struct literal). The sweeps default to the smallest change that closes their entries, and a `non_exhaustive` addition lands in the same change as the fix to any in-repo caller it breaks.

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

- Trait surfaces (`App`, `Widget`, `RenderCx`, the sink traits): whether each is sealed, and which methods may grow defaults.
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

Convention checks that pass: no public function is named `get_*`.

## Findings and the breaking-change list

### A. Error types

`conventions.md` §3 asks for one `Error` enum and one `Result` alias per library crate, both `#[non_exhaustive]`. The crates have eleven domain-named error types instead and no `Result` alias:

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
| A1 | `SessionError` (three copies), `NodeGraphError`: add `#[non_exhaustive]`. A new failure kind must not be a breaking change. | do-now, change 1 |
| A2 | `ThemeLoadError`, `MermaidError`, `RampError`: struct errors. Add `#[non_exhaustive]` where a field is public, and keep fields private otherwise. | do-now, change 1 |
| A3 | One `Error` and `Result` per crate (§3). Eleven domain names exist, each specific and already used by callers. Renaming them gains nothing a reader needs. | accept; amend §3 to say each fallible area names its error `<Area>Error`, is `#[non_exhaustive]`, and a crate adds a `Result` alias only when it has one error type. The amendment needs the maintainer's sign-off. |

### B. Enums without `#[non_exhaustive]`

Rule for the sweep: an enum that names a set the outside world defines or extends (protocol and capability values, input kinds, themes, styles, constraint kinds, AST nodes, fonts and patterns) is **open** and gets `#[non_exhaustive]`. An enum that is a closed two-to-four-way choice with a fixed meaning (an axis, a direction, a boolean-like mode) is **closed** and stays exhaustive, so callers can match it without a wildcard.

Downstream code that matches an open enum then needs a wildcard arm. That is the intended cost and the reason to do it before 1.0.

| # | Entry | Disposition |
|---|---|---|
| B1 | Open enums, term: `Event`, `KeyCode`, `KeyKind`, `MouseKind`, `MouseButton`, `Scroll`, `Paste`, `ColorDepth`, `PrivateMode`, `KeyboardProtocol`, `GraphicsProtocol`, `Hyperlinks`, `WidthPolicy`, `Multiplexer`, `RecordedTerminal`. | do-now, change 2 |
| B2 | Open enums, core, layout and text: `Color`, `UnderlineStyle`, `DecodedGlyph`, `Constraint`, `Flex`, and the markdown nodes `Block`, `Inline`, `Alignment` (module-only, but reachable through the public `markdown` module). | do-now, change 3 |
| B3 | Open enums, widgets and the widget crates: `BorderType`, `SpinnerPattern`, `BigTextFont`, `PixelSize`, `LogLevel`, `DiffKind`, `TimelineStatus`, `EditResult`, `MenuEvent`, `ColorPickerFocus`, `Look`, `ButtonSize`, `Fit` (image). | do-now, change 4 |
| B4 | Open enums, runtime, theme and prismattyc: `Ease`, `Profile`, `Variant`, `NativeDiscovery`. | do-now, change 3 |
| B5 | Closed enums stay exhaustive: `Axis`, `Orientation`, `Direction` (effects), `FocusDirection`, `Alignment` and `TitlePosition` (widgets), `MouseUnit`, `Focus`, `ColorMode`, `SyncOutput`, `MouseTracking`, `ControlFlow`, `State`, `SchedulePoll`, `CellWidth`, `DiffLayout`, `WeekStart`, `SplitAxis`, `RenderDirection`, `Baseline`, `KeymapMode`, `VimMode`, `FileKind`, `AttributeMode`, `NativePolicy`, `PressRoute`. | accept |
| B6 | Module-only enums not reachable from the crate root: `RampOutput`, `GlyphSet`, `Face`. Make them `pub(crate)` if no public signature uses them, otherwise apply the rule above. | do-now, change 3 (check reachability first) |

### C. Structs with public fields

A struct with public fields and no `#[non_exhaustive]` cannot gain a field without breaking every struct literal. Value types that are complete by nature stay open to construction. Option, result and state types grow.

| # | Entry | Disposition |
|---|---|---|
| C1 | Complete value types: `Size`, `Rect`, `Style`, `Meta`, `Padding`, `PixelPoint`, `PixelRect`, `TextPosition`, `TimelineItemId`, `Divider`, and the geometry shapes `Points`, `Line`, `Rectangle`, `Circle`. | accept |
| C2 | Option and configuration types that will gain fields: `RunOptions`, `SessionOptions`, `ProbeEnv`, `ToastPlacement`. Add `#[non_exhaustive]` and keep `Default` plus builder methods as the way to construct them. | do-now, change 5 |
| C3 | Result and report types a library produces and callers only read: `Caps`, `ProbeReply`, `TermIdentity`, `DecModes`, `Protocols`, `DiscoveryResult`, `FrameDiff`, `PresentStats`, `SearchMatch`, `PieLayout`, `SplitLayout`, `Solved`, `SolvedGrid`, `Wrapped`, `WrappedLine`, `ImageGeometry`, `TreeRow`, `HeatmapCell`, `TimelineTick`, `LogEntry`, `DiffLine`, `DiffHunk`. Add `#[non_exhaustive]`. | do-now, change 5 |
| C4 | The rest of the 76 counted structs (event payloads, state holders, markdown tree nodes, rendering areas). Each is decided with its crate's change using the same rule: complete value stays, grows gets `#[non_exhaustive]`. | defer to changes 2 to 5; the closing change lists the result |

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

Eight `mod.rs` files exist (`core`: `simd`, `symbols`; `runtime`: `anim`; `term`: `encode`, `sys`, `session`; `text`: `markdown`, `wrap`) against the rule that a module is `<concept>.rs` plus `<concept>/`. Eleven crates also keep more than re-exports in `lib.rs`. Neither changes a public path, so neither is a breaking change.

| # | Entry | Disposition |
|---|---|---|
| F1 | `mod.rs` files and `lib.rs` content. Both rules are marked "CI (planned)". The fix is an `xtask` check plus the file moves, with no API effect. | accept for 1.0 (not a breaking change); tracked as separate conventions work |

## Changes this list expects

1. Error types: A1, A2, and the amendment in A3.
2. Term enums: B1, plus term structs from C4.
3. Core, layout, text, runtime, theme and prismattyc enums: B2, B4, B6.
4. Widget enums: B3, plus widget structs from C4.
5. Option and result structs: C2, C3.
6. Closing change: D2, E1, the C4 leftovers, the scope items above, and a stability policy for 1.0 (what counts as breaking, how `non_exhaustive` and features are used).

Each change adds a `CHANGELOG.md` entry under Unreleased that names the types and shows the one-line fix for downstream code (a wildcard arm, or `..Default::default()` for a struct literal). The sweeps default to the smallest change that closes their entries, and a `non_exhaustive` addition lands in the same change as the fix to any in-repo caller it breaks.

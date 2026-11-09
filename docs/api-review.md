# API review for 1.0

This is the review pass of roadmap PR 57 (PRD §8, phase 9): the public API of every library crate checked against [`conventions.md`](conventions.md), and the breaking-change list that has to be closed before 1.0. This first change is the inventory and the list. Each later change takes one category and closes its entries.

Every entry has one disposition:

- **do-now**: a breaking change we make before 1.0, in the change named in the entry.
- **accept**: the current API stays, with the reason.
- **defer**: needs a decision or an input this review does not have yet, with who decides.

The list is closed when no entry is open.

## Method and scope

The inventory comes from scanning `src/` of every library crate for public items: `pub enum`, `pub struct` with public fields, error types, `get_` functions, `pub mod` in `lib.rs`, and `mod.rs` files. It is a source scan, not rustdoc output, so an item is counted when it is declared `pub`, whether or not it is reachable from the crate root. Items hidden behind a private module are marked **module-only** and are decided separately. The web crate and the test helper crate are not public API and are left out.

The review also covers trait surfaces, facade parity, feature gating and deprecated items (section G). The rules that come out of it are written down in [`stability.md`](stability.md).

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
| A3 | One `Error` and `Result` per crate (§3). Nine domain names exist, each specific and already used by callers. Renaming them gains nothing a reader needs. | done in change 8: §3 now says each fallible area names its error `<Area>Error`, is `#[non_exhaustive]`, and a crate adds a `Result` alias only when it has one error type |

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
| C4 | The rest of the struct declarations (those not named in C1 to C3): event payloads, state holders, markdown tree nodes, layout areas. Decided in the table below with the same rule: complete value stays, grows gets `#[non_exhaustive]`. | done in change 7 |

C4 decisions. Each struct is listed once; "stays" means a complete value that callers build and match by field.

| Structs | Decision |
|---|---|
| `SectionArea`, `FieldArea`, `PaneArea`, `ItemArea` (widgets) | `#[non_exhaustive]`: layout results that callers only read. |
| `Document`, `List`, `Item`, `Table` (text markdown) | `#[non_exhaustive]`: tree nodes, like the `Block` and `Inline` enums that hold them. |
| `BundledTheme` (theme), `TerminalCell` (widgets) | `#[non_exhaustive]`: metadata and borrowed views the library produces. |
| `KeyEvent`, `MouseEvent` (term) | `#[non_exhaustive]`: an input event gains a field when a protocol does. `KeyEvent` already had builders; `MouseEvent` gains `new`, `modifiers` and `unit`. |
| `GlyphCapabilities` (core) | `#[non_exhaustive]`: a new glyph tier adds a flag. Gains `octants`, `braille` and `blocks` setters. |
| `TabMove` (widgets) | stays: a complete `from` and `to` pair that callers build to reorder their own titles with `TabMove::apply`. |
| `CellFlags`, `ChangedRun`, `RowDirty` (core) | stay: the packed cell format and the diff contract fix them (cell ADR); a new field is a format change. |
| `Set` in `symbols::border` and `symbols::line`, `BarSet` (widgets) | stay: glyph tables that mirror the ratatui symbol sets one for one. |
| `Rect` in `graphite`, `FontRun`, `HeatmapCell`, canvas shapes (see C1) | stay: complete geometry and position values. |
| `StyledSpan`, `StyledLine`, `StyledText` (ratatui) | stay: complete conversion values with no producer-side state. |
| `RowBuilder`, `RowIndex`, `Wrap` (text), `WidgetListState` (widgets) | stay: each has one public field that is its whole meaning, or is a state value with a `Default` and public fields by design. |
| `SolvedGrid`, `TreeRow` | already covered in C3: private fields. |

### D. Naming

| # | Entry | Disposition |
|---|---|---|
| D1 | `get_*` functions: none found. | accept |
| D2 | `as_`, `to_` and `into_` conversions, and boolean predicate names (§2) are a review rule with no mechanical check. The scan for the common slips found none: no `as_` function returns an owned `String`, `Vec` or `Box`, no `to_` function on `&self` returns a reference, no `into_` function takes `&self`, and no `is_` or `has_` function returns anything but `bool`. The scan was by pattern over one-line signatures, so it is a spot check, not a proof. | accept; no mechanical check added |

### E. Module exposure

`pub mod` appears in `lib.rs` of core (5), runtime (2), term (1), text (3), widgets (2) and the web helper (3). Each public module is a path callers can depend on.

| # | Entry | Disposition |
|---|---|---|
| E1 | Review each public module: keep it public only if the crate root does not already re-export what a caller needs. A module made private is a breaking change. Every one of the thirteen modules is a namespace callers import from (`symbols` for the glyph tables, `subcell` for glyph selection, `markdown`, `wrap` and `source` for the text tree and row sources, `canvas` and `graphite` for their widget families, `effect` and `keymap` for the runtime features, `paint`, `oklab` and `simd` for the color and kernel layers, `sys` for the Windows console layer), or a group the root deliberately does not flatten. | accept: none is made private |

### F. Conventions that are about layout, not API

Eight `mod.rs` files exist (`core`: `simd`, `symbols`; `runtime`: `anim`; `term`: `encode`, `sys`, `session`; `text`: `markdown`, `wrap`) against the rule that a module is `<concept>.rs` plus `<concept>/`. Ten crates also keep more than re-exports in `lib.rs`. Neither changes a public path, so neither is a breaking change.

| # | Entry | Disposition |
|---|---|---|
| F1 | `mod.rs` files and `lib.rs` content. Both rules are marked "CI (planned)". The fix is an `xtask` check plus the file moves, with no API effect. | accept for 1.0 (not a breaking change); tracked as separate conventions work |

### G. Trait surfaces, facade, features and deprecated items

| # | Entry | Disposition |
|---|---|---|
| G1 | Trait surfaces. Twelve public traits: `App`, `Widget`, `ImageSink`, `HostRenderer`, `RowSource`, `TextMetrics`, `HostFontMetrics`, `CodeHighlight`, `Shape`, `CellStyle`, `SparklineValue`, `DateStyle`. All are meant to be implemented by downstream code or used as `dyn` objects (`Widget` is held as `&dyn Widget` by `Card` and `Field`), so none is sealed and none can be. The rule instead: a method added to one of them after 1.0 must have a default body, because a new required method breaks every implementer. The required methods today are the stable contract. `App` and `ImageSink` already follow this: their later additions (`render_frame`, `needs_animation_frames`, `token_palette`, `pixel_target`, `submit_raster`) all have defaults. | accept; the rule is in `stability.md` |
| G2 | Facade parity. `lunatui` re-exports only the runtime entry points (`App`, `ControlFlow`, `RunOptions`, `is_quit`, `render_once`, `run`, and the `Result` alias), behind the `app` feature. It does not mirror the other crates, on purpose: a program depends on `lunatui-core`, `lunatui-widgets` and the rest directly. Nothing is re-exported that should not be, and every re-exported item is in the stable surface. | accept; the facade stays this small until a user need says otherwise |
| G3 | Feature gating. Features are additive, named for the capability, and use `dep:` for optional dependencies (conventions §9); the widget features all appear in `all-widgets`. A public item behind a feature is public API under that feature. No public item exists only for tests or benches: nothing is `#[doc(hidden)]`, and the bench and test helper crates are not published API. The long default feature list of `lunatui-widgets` is a product choice (features on by default), not an API problem. | accept |
| G4 | Deprecated and experimental items. No item carries `#[deprecated]`. One surface is documented as experimental: the version-1 accessibility export in `lunatui-runtime`, behind the `accessibility` feature. | accept; the accessibility export is excluded from the 1.0 stability promise until its documentation drops the word "experimental" |

## Changes this list expects

1. Error types: A1, A2, and the amendment in A3.
2. Term enums: B1, plus term structs from C4.
3. Core, layout, text, runtime, theme and prismattyc enums: B2, B4, B6.
4. Widget enums: B3, plus widget structs from C4.
5. Option structs: C2. Done.
6. Result structs: C3. Done.
7. Closing change: C4, D2, E1, section G, and the stability policy in [`stability.md`](stability.md). Done.
8. The conventions §3 amendment (A3), a separate change because `docs/conventions.md` needs the maintainer's sign-off. Done. The breaking-change list is closed.

Each change adds a `CHANGELOG.md` entry under Unreleased that names the types and shows the one-line fix for downstream code (a wildcard arm, or `..Default::default()` for a struct literal). The sweeps default to the smallest change that closes their entries, and a `non_exhaustive` addition lands in the same change as the fix to any in-repo caller it breaks.

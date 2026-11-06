# lunatui-layout

Constraint-based splits and two-axis named grids for terminal applications.

`Layout` divides one axis. `GridLayout` solves shared row and column tracks, then
returns named areas, individual cells and spans. Both use the closed-form engine
from [ADR 0002](../docs/adr/0002-layout-engine.md), including the existing integer
rounding, `Flex`, spacing and shrinking rules. The crate uses `no_std` with `alloc`.

## Named screen areas

```rust
use lunatui_core::Rect;
use lunatui_layout::{Constraint::{Fill, Length}, GridError, GridLayout};

# fn main() -> Result<(), GridError> {
let layout = GridLayout::new(
    [Length(3), Fill(1), Length(1)], // rows
    [Length(18), Fill(1)],          // columns
)
.area("header", 0..1, 0..2)?
.area("sidebar", 1..2, 0..1)?
.area("body", 1..2, 1..2)?
.area("status", 2..3, 0..2)?;

let regions = layout.split(Rect::new(4, 2, 60, 24));
assert_eq!(regions.get("header"), Some(Rect::new(4, 2, 60, 3)));
assert_eq!(regions.get("body"), Some(Rect::new(22, 5, 42, 20)));
# Ok(())
# }
```

Rows come first, columns second. Ranges are zero-based and end-exclusive. An area
can span either axis or both. Names are owned, exact, case-sensitive, nonempty
strings; whitespace and Unicode are preserved. Empty or duplicate names and
empty, reversed or out-of-bounds ranges return `GridError` when declared.
Differently named areas may overlap. The caller controls widget paint order.

Construct the layout once and split it again after a resize. Each `SolvedGrid`
owns its geometry and names, so it remains valid after the builder is dropped.
Splitting allocates through the axis solver and clones the names; it is not a
memoized or allocation-free operation.

## Cells, spans and spacing

```rust
use lunatui_core::Rect;
use lunatui_layout::{Constraint::Fill, GridLayout};

let regions = GridLayout::new([Fill(1); 2], [Fill(1); 3])
    .column_gap(1)
    .split(Rect::new(0, 0, 20, 8));
assert_eq!(regions.cell(0, 0), Some(Rect::new(0, 0, 6, 4)));
assert_eq!(regions.span(0..2, 0..3), Some(Rect::new(0, 0, 20, 8)));
assert_eq!(regions.cell(2, 0), None);
```

Anonymous grids need no area declarations. `cell`, `span` and named `get` return
`Option<Rect>`. Missing names and invalid queries return `None` without clamping.
`iter` returns named rectangles in declaration order.

`row_gap` and `column_gap` accept signed cell spacing, including negative overlap,
as `Layout::spacing` does. Spans enclose all selected tracks and their internal
gaps. `row_flex` and `column_flex` independently pack leftover space; `margin`
insets the whole grid once. `needs_solver` reports a minimum or positive gap that
could not fit on either axis. Coordinates saturate like `Layout`; local widths
and heights are preserved near maximum coordinate values.

The [layout template](../examples/templates/layout.rs) renders real widgets into
named areas. Run `cargo run -p lunatui --example template_layout -- --snapshot`
for a headless frame, or omit `--snapshot` for the interactive example.

## Verification

Run `cargo test -p lunatui-layout --all-features` for splits, limits, wrapping,
named grid areas, spans and boundary cases. `GridLayout` is available by default.
The optional `size-limits` and `flex-wrap` features extend one-axis `Layout`.
Solver fallback, memoization and `Stack` remain future work.

After committing, run `python3 scripts/check-grid-layout-mutants.py` from the
repository root. It checks out that exact commit in isolation, applies seven
mutations to span, name, range, origin and diagnostic behavior, then verifies the
restored implementation. Build output and failure logs default to
`target/grid-layout-mutants`; `--out` selects another directory.

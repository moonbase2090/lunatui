# Heatmap data and geometry

A heatmap borrows labeled rows of `Option<u64>` readings. `Some(0)` is a measured zero; `None` and the missing tail of a short row are missing measurements. The longest row sets the column count. The application owns the data, selection, and keyboard handling.

```rust
use lunatui_widgets::{Heatmap, HeatmapCell, HeatmapRow, WidgetState};

let api = [Some(12), Some(46), None, Some(81)];
let jobs = [Some(0), Some(15), Some(37), Some(64)];
let rows = [HeatmapRow::new("api", &api), HeatmapRow::new("jobs", &jobs)];
let columns = ["00", "01", "02", "03"];
let map = Heatmap::new(&rows)
    .column_labels(&columns)
    .range(0..=100)
    .selected(Some(HeatmapCell { row: 0, column: 1 }))
    .state(WidgetState::Focused);
```

Cells have fixed terminal dimensions, two columns by one row by default. They clip at the supplied rectangle instead of shrinking. `cell_at` excludes labels and returns logical row and column indices, including visible parts of clipped cells. `cell_area` uses the same private geometry to locate a selected cell for a detail panel. `shape` supplies the bounds for keyboard navigation.

The scale uses five semantic theme tokens. This preserves reduced-color palettes without a second color conversion policy. A fixed range keeps colors stable as live readings change. Automatic scaling ignores missing readings and maps constant readings to the lowest band. Missing cells show a dot; selection shows a diamond. Disabled cells use dimmed foreground swatches because ANSI dim does not dim background colors.

## Design choice

Borrowed labeled rows keep each label with its readings and avoid an explicit stride. A flat buffer was also considered. It suits numerical producers but makes the caller coordinate a column count and a separate label array. The row representation has fewer independent inputs and needs no validation error or data-source trait.

The design retains two geometry queries from the flat-buffer alternative, `cell_area` and `shape`, so applications do not repeat clipping or ragged-row calculations. Sample lookup remains with the application that owns the readings.

The first version uses discrete cell blocks. A continuous color ramp, scrolling, and a pixel presenter can be added when a consumer needs them. No renderer, process, or input controller is stored in the widget.

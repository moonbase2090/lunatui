# Timeline data, view and geometry

A timeline borrows labeled lanes of spans and milestones. Times are plain `i64` values in the application's own unit, such as minutes or days, so the widget has no calendar dependency and no formatting policy. The application owns the data, the visible window, the first visible lane, the selection, and keyboard handling.

```rust
use lunatui_widgets::{
    Timeline, TimelineItem, TimelineItemId, TimelineLane, TimelineStatus, TimelineTick,
    TimelineView, WidgetState,
};

const DAY: i64 = 24 * 60;
let api = [
    TimelineItem::span(0, 3 * DAY).label("schema").status(TimelineStatus::Done),
    TimelineItem::span(3 * DAY, 7 * DAY).label("handlers").progress(40),
    TimelineItem::milestone(8 * DAY).label("beta"),
];
let web = [TimelineItem::span(2 * DAY, 6 * DAY).label("forms").status(TimelineStatus::AtRisk)];
let lanes = [TimelineLane::new("api", &api), TimelineLane::new("web", &web)];
let ticks = [TimelineTick::new(0, "Mon"), TimelineTick::new(DAY, "Tue")];
let timeline = Timeline::new(&lanes)
    .ticks(&ticks)
    .view(TimelineView::new(0, 10 * DAY))
    .now(Some(4 * DAY))
    .selected(Some(TimelineItemId { lane: 0, item: 1 }))
    .state(WidgetState::Focused);
```

## Drawing

Each lane takes one row after a label gutter. With ticks, a two-row header puts each label at its time, clipped one cell before the next tick, over an axis rule with `┬` marks. Ticks are expected in ascending order.

A span covers the columns from the floor of its start to the ceiling of its end, and always at least one cell, so short work stays visible. Its label sits inside and clips to the span. Progress keeps the status color for the finished share and the track color for the rest; the label's color follows the background under each cell. A planned span without progress has not started and draws as track only. Done, at-risk, and blocked spans without progress are solid. Statuses map to semantic tokens: planned progress uses accent, done uses ok, at risk uses warning, and blocked uses failure, so reduced-color palettes keep working.

A milestone draws `◆` in its status color with its label to the right. Items draw in slice order, and a later item covers an earlier one where they overlap. Spans clip at the view's edges.

`now` draws a neutral `▼` on the rule and `┊` through empty lane cells. It deliberately avoids the warning and failure colors, which already carry status, and the focus color, which marks focus. A selected item gains bold and underline; when the timeline is focused, its lane label also uses the focus color. Disabled timelines use dimmed `░` swatches, because ANSI dim does not dim background colors, and draw no selection.

## Scrolling and zooming

`TimelineView` is a small `Copy` value the application stores. `pan` moves the window, `zoom_in` and `zoom_out` halve or double it around its center, and `reveal` pans the least distance that brings a range into view, which is how the showcase keeps "now" on screen. The window is never empty and saturates at the `i64` range. `Timeline::bounds` returns the data extent for clamping a pan. `lane_offset` scrolls lanes vertically.

The widget takes ticks from the application rather than generating them, so an overview can switch to weekly labels while a close view shows days. The showcase does exactly that when it zooms out.

## Geometry

`x_at` maps a time to its terminal column, `time_at` maps a column back to the time at its left edge, and `item_at` returns the topmost item under a cell, using the same private layout as rendering. A milestone answers only on its `◆` cell. When the view is zoomed in past one unit per column, neighboring columns share a time.

## Design choice

Borrowed lanes keep each label with its items and need no validation error or data-source trait. Axis labels come in as borrowed ticks rather than a formatter closure, which would break `Clone` and `Debug` and allocate on every frame. Scroll state stays with the application, as with `Heatmap` and `Table`.

The first version draws one row per lane at cell precision. Dependency arrows, multi-row lanes, sub-cell edges and a pixel presenter can be added when a consumer needs them.

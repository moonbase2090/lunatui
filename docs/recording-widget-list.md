# WidgetList comparison

The scene is an activity feed of four item kinds with different heights: a one-row
section heading, a three-row message, a two-row progress bar and a four-row bordered
alert. It runs the native `WidgetList` and [tui-widget-list](https://github.com/preiter93/tui-widget-list)
0.15.3 on identical items, geometry, theme colors and a scripted selection that steps
down the feed one item every ten frames. Items near the edges are cut at the area
edge, so the recording shows rows scrolling in and out. Every native element is a
real widget.

## Reproduce

Install the pinned `agg` named in `scripts/record-widget.sh`, asciinema 3 and FFmpeg.

```sh
CASE=widget-list bash scripts/record-widget-matrix.sh
python3 scripts/check-widget-list-recordings.py target/widget-list-recordings
python3 scripts/export-widget-recordings.py --case widget-list target/widget-list-recordings
cargo test --locked -p lunatui-widgets --test widget_list
cargo test --locked -p lunatui-bench --test widget_suite_parity
python3 scripts/bench-widget.py --case widget-list
```

Recordings cover 80x24 and 200x60 in both themes, with truecolor, ANSI256 and ANSI16.
Both sides run 240 frames at 30 fps. Lunatui is on the left of each side-by-side
video. Screenshots use the same 6.5-second sample on both sides.

The checker decodes whole GIF frames at 0.5, 2.5, 4.5 and 6.5 seconds. It verifies
that the heading stays fixed, that the four samples all differ (the selection and
scroll position move), and that both implementations decode to identical pixels in
every sample. It rejects a recording outside the 7.5-to-12-second timeline.

## Parity and performance

The exact cell test compares glyphs, colors and attributes at every scripted
selection (twenty-four steps, wrapping once), in both themes, at 0x0, 7x3, 13x11,
80x24 and 200x60. It also asserts that the first rows, the scrolled rows and the
last rows hold the expected text, so two blank grids cannot pass.

The benchmark case is gated: the frame ratio must be at most 1.05 and the native
side must not allocate more per frame. Six paired rounds with alternating side order
on the machine that produced the recordings, as medians (min to max):

| Size | Native frame | Baseline frame | Ratio | Allocations native / baseline |
|---|---|---|---|---|
| 80x24 | 18.2 us (17.9-18.7) | 43.1 us (42.1-43.3) | 0.42 | 40.2 / 68.1 |
| 200x60 | 30.8 us (30.3-31.5) | 131.4 us (130.3-133.7) | 0.23 | 53.4 / 91.7 |

Frame timings include the shared scene, diff, encoding and a counting output sink.
They describe this scene on the measured machine.

## Differences from the baseline

`WidgetList` takes a height closure and a draw closure, where `tui-widget-list` takes a
builder that returns a widget and its height. Both draw items that fit straight into
the area and render an item cut by an edge whole into a scratch buffer, then copy
the visible rows. The two scroll the same way for the scripted selection, which the
exact cell test confirms. Scroll padding counts rows in `WidgetList` and items in the
baseline; the scripted scene does not use it, and the unit tests cover it.

## Captures

Screenshots are decoded from each recording at 6.5 seconds. The
[artifact manifest](media/widget-list-proof.json) records source and artifact hashes.
Source hashes describe the recorded commit.

| Theme / profile | 80x24 | 200x60 |
|---|---|---|
| dark / truecolor | [video](media/widget-list-80x24-truecolor-dark-side-by-side.mp4), [native](media/widget-list-80x24-truecolor-dark-lunatui.png), [baseline](media/widget-list-80x24-truecolor-dark-ratatui.png) | [video](media/widget-list-200x60-truecolor-dark-side-by-side.mp4), [native](media/widget-list-200x60-truecolor-dark-lunatui.png), [baseline](media/widget-list-200x60-truecolor-dark-ratatui.png) |
| dark / ansi256 | [video](media/widget-list-80x24-ansi256-dark-side-by-side.mp4), [native](media/widget-list-80x24-ansi256-dark-lunatui.png), [baseline](media/widget-list-80x24-ansi256-dark-ratatui.png) | [video](media/widget-list-200x60-ansi256-dark-side-by-side.mp4), [native](media/widget-list-200x60-ansi256-dark-lunatui.png), [baseline](media/widget-list-200x60-ansi256-dark-ratatui.png) |
| dark / ansi16 | [video](media/widget-list-80x24-ansi16-dark-side-by-side.mp4), [native](media/widget-list-80x24-ansi16-dark-lunatui.png), [baseline](media/widget-list-80x24-ansi16-dark-ratatui.png) | [video](media/widget-list-200x60-ansi16-dark-side-by-side.mp4), [native](media/widget-list-200x60-ansi16-dark-lunatui.png), [baseline](media/widget-list-200x60-ansi16-dark-ratatui.png) |
| light / truecolor | [video](media/widget-list-80x24-truecolor-light-side-by-side.mp4), [native](media/widget-list-80x24-truecolor-light-lunatui.png), [baseline](media/widget-list-80x24-truecolor-light-ratatui.png) | [video](media/widget-list-200x60-truecolor-light-side-by-side.mp4), [native](media/widget-list-200x60-truecolor-light-lunatui.png), [baseline](media/widget-list-200x60-truecolor-light-ratatui.png) |
| light / ansi256 | [video](media/widget-list-80x24-ansi256-light-side-by-side.mp4), [native](media/widget-list-80x24-ansi256-light-lunatui.png), [baseline](media/widget-list-80x24-ansi256-light-ratatui.png) | [video](media/widget-list-200x60-ansi256-light-side-by-side.mp4), [native](media/widget-list-200x60-ansi256-light-lunatui.png), [baseline](media/widget-list-200x60-ansi256-light-ratatui.png) |
| light / ansi16 | [video](media/widget-list-80x24-ansi16-light-side-by-side.mp4), [native](media/widget-list-80x24-ansi16-light-lunatui.png), [baseline](media/widget-list-80x24-ansi16-light-ratatui.png) | [video](media/widget-list-200x60-ansi16-light-side-by-side.mp4), [native](media/widget-list-200x60-ansi16-light-lunatui.png), [baseline](media/widget-list-200x60-ansi16-light-ratatui.png) |

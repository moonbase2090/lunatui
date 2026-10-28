# lunatui

lunatui is a hyperspeed Rust TUI library being built from scratch, keeping a simple "render widgets into a rectangle" model while fixing its structural costs with a 16-byte packed cell, damage-tracked diffing and a single-write output encoder, plus first-class input, focus, animation, capability and theme layers. It is open source under the [MIT](LICENSE) license.

**Status: v0.1.12 preview** — first usable release on the roadmap (Phase 4 widgets, encoder, runtime, tests). APIs and backends still evolve before 1.0. Start with [Getting started](docs/getting-started.md), the [architecture overview](docs/architecture.md), and the [PRD](docs/PRD.md).

## Performance

Lunatui's own dashboard numbers on one machine (Apple M5 Max, release build), median of five runs of 240 frames at 80×24:

| Size | Median frame | p95 | Allocations per frame | Bytes written per frame |
|---|---|---|---|---|
| 80×24 | 60 µs | 65 µs | ≈39 | 603 |
| 200×60 | 249 µs | 285 µs | ≈69 | 2868 |

Method and limits: [docs/performance.md](docs/performance.md).

## Quick start

```sh
git clone https://github.com/moonbase2090/lunatui
cd lunatui
cargo run --example live_dashboard
```

The dashboard draws 240 frames (about four seconds) and exits. Add `-- --bench` for a headless timing run. More examples are listed in [examples/README.md](examples/README.md).

See the [review policy](REVIEW_POLICY.md) for PR classification, proof and merge requirements.

Migration guide: [docs/porting-from-ratatui.md](docs/porting-from-ratatui.md). CI and manual sign-off: [docs/ci.md](docs/ci.md). Examples: [examples/README.md](examples/README.md).

## Widget gallery

Still frames and short recordings from `scripts/record-showcase.sh`: lunatui only, dark theme, 120×36.

### Chart

An operations dashboard with four charts: braille lines with a legend, a dot scatter, hourly bars, and a half-block line chart in the focused state. The series scroll and the points drift while the axes, labels, and titles stay put.

![Chart widget: an operations dashboard in the dark theme](docs/media/chart.gif)

Screenshot: [docs/media/chart.png](docs/media/chart.png). Recorded with `scripts/record-showcase.sh` (`CASE=chart`, 120×36, `FRAMES=200`, dark, about 11 seconds).

### BarChart

A deploy dashboard made of five bar charts: grouped vertical bars with value text and group labels, horizontal bars with region labels, a series of build times with the latest run highlighted, and the same data in the normal, focused, and disabled states.

![BarChart widget: a deploy dashboard in the dark theme](docs/media/barchart.gif)

Screenshot: [docs/media/barchart.png](docs/media/barchart.png). Recorded with `scripts/record-showcase.sh` (`CASE=barchart`, 120×36, `FRAMES=200`, dark, about 11 seconds).

### MarkdownView

A docs reader: a page list beside a scrolling Markdown view, with headings and rules, hanging-indent lists, task items, quotes, code blocks, boxed tables, inline emphasis, code, and underlined links, and a scrollbar sized from the document. The reader moves through three pages.

![MarkdownView widget: a docs reader in the dark theme](docs/media/markdown.gif)

Screenshot: [docs/media/markdown.png](docs/media/markdown.png). Recorded with `scripts/record-showcase.sh` (`CASE=markdown`, 120×36, `FRAMES=200`, dark, about 11 seconds).

### Split panes

A four-pane workspace (files, editor, terminal, inspector) whose dividers are dragged with the pointer: the grabbed divider is highlighted and the panes on both sides follow it one cell at a time, then the editor is zoomed to fill the screen and restored.

<img src="docs/media/split.webp" alt="Split panes: dividers dragged and a pane zoomed in the dark theme" width="1028">

Screenshot: [docs/media/split.png](docs/media/split.png). Recorded with `scripts/record-showcase.sh` (`CASE=split`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Sparkline

Live service board. Ten series scroll in eighth-block bars, the highlight moves from row to row, and the error series turns red while it is over its threshold.

![Sparkline service board](docs/media/sparkline.gif)

![Sparkline service board, still](docs/media/sparkline.png)

Recorded with `scripts/record-showcase.sh` (`CASE=sparkline`, 120×36, 8 s at 30 fps, dark theme).

### Calendar

A team schedule: the month beside that day's agenda, colored release, review, on-call, and incident days, today held in a raised cell, and the selection walking from late October into November.

![Calendar widget: a team schedule in the dark theme](docs/media/calendar.gif)

Screenshot: [docs/media/calendar.png](docs/media/calendar.png). Recorded with `scripts/record-widget.sh` (`CASE=calendar`, 120×36, dark, about 11 seconds).

### BigText

A cutover board: a live clock, a countdown that reaches zero, and ticking metrics. The selection moves, and the error count turns red while it is hot.

![BigText widget: a cutover board in the dark theme](docs/media/bigtext.gif)

Screenshot: [docs/media/bigtext.png](docs/media/bigtext.png). Recorded with `scripts/record-showcase.sh` (`CASE=bigtext`, 120×36, dark, about 11 seconds).

### Image

A media library for an observatory archive. Each thumbnail is an `Image` with `ImageFit::Cover`. The selected one is drawn focused, and the two imports are drawn disabled until they finish. The preview switches between `ImageFit::Contain` and `ImageFit::Cover`, and the details panel shows each frame's tone bands. The pictures are the website's sky, hills and moon art, resized, tinted and layered by the example.

![Image widget: a media library in the dark theme](docs/media/image-120x36-truecolor-dark.gif)

Screenshot: [docs/media/image-120x36-truecolor-dark.png](docs/media/image-120x36-truecolor-dark.png). Recorded with `scripts/record-widget.sh` (`CASE=image`, 120×36, dark, 10 seconds at 30 fps). The recording shows the half-block cell fallback, two image pixels per cell. The recorder captures terminal text and cannot capture Kitty graphics, so the Kitty path is not shown.

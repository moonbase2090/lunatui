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

### LineGauge

A live service health board rendered by Lunatui and Ratatui side by side. Each row uses a real LineGauge widget, with the selected row, disabled state, value label, threshold colors, and line styles visible in the recording.

| Terminal | Truecolor | ANSI-16 |
|---|---|---|
| 80×24 | [Screenshot](docs/media/line-gauge-80x24-truecolor.png) · [Recording](docs/media/line-gauge-80x24-truecolor.mp4) | [Screenshot](docs/media/line-gauge-80x24-ansi16.png) · [Recording](docs/media/line-gauge-80x24-ansi16.mp4) |
| 200×60 | [Screenshot](docs/media/line-gauge-200x60-truecolor.png) · [Recording](docs/media/line-gauge-200x60-truecolor.mp4) | [Screenshot](docs/media/line-gauge-200x60-ansi16.png) · [Recording](docs/media/line-gauge-200x60-ansi16.mp4) |

Recorded from the real widget preview with `scripts/record-widget.sh` (`CASE=line-gauge`, dark theme, 180 frames at 30 fps). The screenshot is extracted from the side-by-side recording. Light theme spot-check at 80×24: [screenshot](docs/media/line-gauge-80x24-light-truecolor.png) · [recording](docs/media/line-gauge-80x24-light-truecolor.mp4).

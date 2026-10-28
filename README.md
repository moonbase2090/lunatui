# lunatui

lunatui is a hyperspeed Rust TUI library being built from scratch, keeping a simple "render widgets into a rectangle" model while fixing its structural costs with a 16-byte packed cell, damage-tracked diffing and a single-write output encoder, plus first-class input, focus, animation, capability and theme layers. It is open source under the [MIT](LICENSE) license.

**Status: v0.1.14 preview** — first usable release on the roadmap (Phase 4 widgets, encoder, runtime, tests). APIs and backends still evolve before 1.0. Start with [Getting started](docs/getting-started.md), the [architecture overview](docs/architecture.md), and the [PRD](docs/PRD.md).

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

<img src="docs/media/markdown.webp" alt="MarkdownView widget: a docs reader in the dark theme" width="1028">

Screenshot: [docs/media/markdown.png](docs/media/markdown.png). Recorded with `scripts/record-showcase.sh` (`CASE=markdown`, 120×36 at 2x scale, `FRAMES=218`, dark, lossless animated WebP, about 11 seconds).

### Breadcrumbs

A file browser: descending through six directories changes the file list, sizes, dates, preview, and visited trail at each step, the path bar narrows until its middle collapses to `…` and widens again, the segment under the pointer is underlined, and a click on `Projects` jumps back up the path.

<img src="docs/media/breadcrumbs.webp" alt="Breadcrumbs widget: a path bar over a file browser with per-directory contents, preview, and visited trail in the dark theme" width="1028">

Screenshot: [docs/media/breadcrumbs.png](docs/media/breadcrumbs.png). Recorded with `scripts/record-showcase.sh` (`CASE=breadcrumbs`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Split panes

A four-pane workspace (files, editor, terminal, inspector) whose dividers are dragged with the pointer: the grabbed divider is highlighted and the panes on both sides follow it one cell at a time, then the editor is zoomed to fill the screen and restored.

<img src="docs/media/split.webp" alt="Split panes: dividers dragged and a pane zoomed in the dark theme" width="1028">

Screenshot: [docs/media/split.png](docs/media/split.png). Recorded with `scripts/record-showcase.sh` (`CASE=split`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Canvas

Six scenes drawn with Canvas, each in thin one-dot Braille strokes with no filled areas: an XY oscilloscope whose Lissajous figure glides between frequency ratios with a fading trail over a faint graticule, a slowly turning globe with great-circle flight paths and planes moving along them, CPU, memory, and network history as scrolling lines, an analog clock running at double time with a sweeping second hand beside a tree that grows branch by branch, a flock of 200 boids with short trails, and a world map with fading ping rings. Each frame is drawn for its own time on a fixed 60 fps step.

<img src="docs/media/canvas.webp" alt="Canvas widget: an oscilloscope, a globe with flight paths, system graphs, a clock and tree, a flock, and a ping map, in the dark theme" width="1028">

Screenshot: [docs/media/canvas.png](docs/media/canvas.png). Recorded with `scripts/record-showcase.sh` (`CASE=canvas FPS=60 FRAMES=321 HEIGHT=50 LINE_HEIGHT=1.0`, 120×50 at 60 fps, dark, 2× scale, lossless animated WebP, about 5.3 seconds plus a 2-second hold on the last frame; every frame is kept and shown for 16 or 17 ms). Coastlines are from Natural Earth (public domain).

### Toast

A timed toast stack over an operations console: boxed toasts stack up in the corner of the pane they belong to, one-row status toasts stack on the bottom rows, and each goes away when its time is up, so a newer message sits above an older one and a status message sits above them all.

<img src="docs/media/toast.webp" alt="Toast stack: pane toasts and status toasts appearing and expiring over an operations console in the dark theme" width="1028">

Screenshot: [docs/media/toast.png](docs/media/toast.png). Recorded with `scripts/record-showcase.sh` (`CASE=toast`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Tab reordering

A release pipelines screen with a boxed tab strip reordered by the pointer. Pressing a tab selects it and lifts it off the strip. While it follows the pointer, the other tabs slide aside and an empty slot marks where it will land. On release it settles into that slot, and the selection stays on it. Three drags run: Builds forward, Deploys to the front, and Logs to the end. Each tab shows live content: a builds table with a running build, deploys with a rollout in progress, and a streaming log.

<img src="docs/media/tabs-drag.webp" alt="TabsDrag: a tab lifted off a boxed strip and dropped into a new position over builds, deploys and logs panes, in the dark theme" width="1028">

Screenshot: [docs/media/tabs-drag.png](docs/media/tabs-drag.png). Recorded with `scripts/record-showcase.sh` (`CASE=tabs-drag`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Key hint bar

A release console whose bottom line is a `KeyHintBar` built from the keymap of the current mode: browsing shows move, open, search, publish, and quit; typing a search filter swaps it for apply, clear, and cancel; the publish confirmation shows yes, go back, and cancel. The bar redraws from whichever keymap is active, so it never lists a key that does nothing.

<img src="docs/media/key-hints.webp" alt="KeyHintBar: the bottom hint line changing with the keymap while a release console moves between browsing, searching, and confirming, in the dark theme" width="1028">

Screenshot: [docs/media/key-hints.png](docs/media/key-hints.png). Recorded with `scripts/record-showcase.sh` (`CASE=key-hints`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### List icons and marks

A pull request list: a status icon before each title, a right-aligned number and area mark on every row, and a highlight symbol on the focused row whose band covers the whole row. The selection walks down the list, the list scrolls to keep it in view, a pull request changes state, and a new one arrives at the top.

<img src="docs/media/list-icons.webp" alt="List: rows with status icons, right-aligned marks, and a highlight symbol over a pull request list in the dark theme" width="1028">

Screenshot: [docs/media/list-icons.png](docs/media/list-icons.png). Recorded with `scripts/record-showcase.sh` (`CASE=list-icons`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Toggle

A settings screen: switches in two groups, each with a one-line description, the focused switch drawn with a bold label and a focus-ring knob, and a live view of the settings file that changes as each switch flips. Focus moves through the switches and each one is toggled in turn, over two passes.

<img src="docs/media/toggle.webp" alt="Toggle: a settings screen with on/off switches flipping while a settings file view updates, in the dark theme" width="1028">

Screenshot: [docs/media/toggle.png](docs/media/toggle.png). Recorded with `scripts/record-showcase.sh` (`CASE=toggle`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Radio group

A preferences screen with three radio groups: a vertical theme list, a horizontal text size row, and an update channel with an option that is not available. The arrow keys step through each group in turn, skipping the disabled option and wrapping around; the focused group's title and option light up, and a preview pane follows every change.

<img src="docs/media/radio-group.webp" alt="RadioGroup: three radio groups stepping through their options with a preview pane following, in the dark theme" width="1028">

Screenshot: [docs/media/radio-group.png](docs/media/radio-group.png). Recorded with `scripts/record-showcase.sh` (`CASE=radio-group`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Form

An account form composed from widgets: a `Form` lays out labelled rows with required markers, hints, and validation messages, and the inputs are the caller's own widgets (text fields, a radio group, a toggle). Fields fill in as focus moves through them, a submit with missing or short values shows messages and jumps focus to the first one, and the messages clear as the fields are corrected.

<img src="docs/media/form.webp" alt="Form: an account form with required markers, hints, and validation messages around text, radio, and toggle inputs, in the dark theme" width="1028">

Screenshot: [docs/media/form.png](docs/media/form.png). Recorded with `scripts/record-showcase.sh` (`CASE=form`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Tooltip

An editor screen with a toolbar, a file tree, source with a warning in the gutter, and a status bar. The pointer rests on Save, Run and Settings, on the gutter warning, and on two status items, and each hint appears after a short delay: under the toolbar buttons, slid left at the right edge, beside the gutter with its message wrapped, and above the status bar where there is no room below. No hint covers what it points at.

<img src="docs/media/tooltip.webp" alt="Tooltip: hints appearing beside toolbar buttons, a gutter warning and status items in an editor screen, in the dark theme" width="1028">

Screenshot: [docs/media/tooltip.png](docs/media/tooltip.png). Recorded with `scripts/record-showcase.sh` (`CASE=tooltip`, 120×36 at 2x scale, `FRAMES=270`, dark, lossless animated WebP, about 9 seconds plus a 2-second hold).

### Number input

A render queue's export settings: each field is a spin box with minus and plus buttons, a step, a range, and decimals, and the screen nudges them one after another while the estimate beside them follows. One field is typed by hand: the empty text shows in the fail color with a caret, digits arrive one at a time, and the committed value snaps to the field's decimals. The widget draws the field and gives back the rules (`nudge`, `parse`, `accepts`, `format`) and the button cells for the app's own key and mouse handling.

<img src="docs/media/number-input.webp" alt="NumberInput: spin boxes stepping and one field being typed on an export settings screen in the dark theme" width="1028">

Screenshot: [docs/media/number-input.png](docs/media/number-input.png). Recorded with `scripts/record-showcase.sh` (`CASE=number-input`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 10 seconds).

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

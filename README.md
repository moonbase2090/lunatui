# lunatui

lunatui is a hyperspeed Rust TUI library being built from scratch, keeping a simple "render widgets into a rectangle" model while fixing its structural costs with a 16-byte packed cell, damage-tracked diffing and a single-write output encoder, plus first-class input, focus, animation, capability and theme layers. It is open source under the [MIT](LICENSE) license.

**Status: v0.1.17 preview** — first usable release on the roadmap (Phase 4 widgets, encoder, runtime, tests). APIs and backends still evolve before 1.0. Start with [Getting started](docs/getting-started.md), the [architecture overview](docs/architecture.md), and the [PRD](docs/PRD.md).

## Performance

Lunatui's own dashboard numbers on one machine (Apple M5 Max, release build), median of five runs of 240 frames at 80×24:

| Size | Median frame | p95 | Allocations per frame | Bytes written per frame |
|---|---|---|---|---|
| 80×24 | 60 µs | 65 µs | ≈39 | 603 |
| 200×60 | 249 µs | 285 µs | ≈69 | 2868 |

Method and limits: [docs/performance.md](docs/performance.md).

## Quick start

Draw a first screen and quit on `q`. The steps and a program under 30 lines are in [Getting started](docs/getting-started.md). The crates are not on crates.io yet, so the commands run from this repository.

From this repository:

```sh
cargo run --example template_minimal
```

Six copy-paste templates (a list, a form, a dashboard, a popover, and a layout skeleton, plus the minimal app) are indexed in [examples/README.md](examples/README.md). `cargo run --example live_dashboard` still runs the longer dashboard demo.

See the [review policy](REVIEW_POLICY.md) for PR classification, proof and merge requirements.

Migration guide: [docs/porting-from-ratatui.md](docs/porting-from-ratatui.md). CI and manual sign-off: [docs/ci.md](docs/ci.md). Examples: [examples/README.md](examples/README.md).

## Widget gallery

Still frames and short recordings of real widget output, in the dark theme. Native pixel demos use the real terminal recorder; cell demos use `scripts/record-showcase.sh`.

### Chart

Requests, Saturation CPU/memory, and latency scatter keep matching Braille and antialiased pixel views. Hourly bars use native pixels. Both views share the same scene data and layout; the axes, labels, and legends stay as text, with native pixel legend swatches. Pixels are enabled by default when the image presenter supports them; `.pixels(false)` retains the original cell rendering.

<img src="docs/media/chart.webp" alt="Matching Braille and native pixel Chart plots, including Saturation CPU and memory, followed by native hourly bars in a real Kitty terminal" width="1028">

Screenshots: [Requests](docs/media/chart.png), [Saturation CPU/memory](docs/media/chart-saturation.png), [Latency scatter](docs/media/chart-scatter.png). Recorded from a clean source tree at `757abcef18301fcbe72dfbd17c1bf57ce1d1ad98` in a fresh Ubuntu arm64 VM, using `EXAMPLE=pixel_chart FEATURES=image-presenter FONT_SIZE=16 SECONDS_TO_RECORD=24 KEYS=3:u,6:Right,9:u,12:Right,15:u,18:Right,21:u scripts/record-pixels.sh`: 100×32 cells, 1301×801 pixels, 30 fps, 24 seconds, lossless WebP 81,152 bytes. Each sample stays on screen for about 3 seconds. All 720 source frames exactly match one of eight inspected states, with no previous plot under new labels. The stills are unmodified video frames at 0, 7.5, and 13 seconds. No compositing or dropped frames.

### BarChart

Grouped requests, regional latency, build runs, and queue states keep equal-size Cells and antialiased Pixels views of the same data and sample. Labels and values remain terminal text. Pixels are enabled by default with a measured presenter; `.pixels(false)` keeps the original eighth-block renderer. Try `cargo run --release -p lunatui --example pixel_bar_chart`: arrows change scenes, `u` advances the sample, and `b` switches the right pane to cells and back.

<img src="docs/media/barchart.webp" alt="Matching cell and antialiased pixel BarChart scenes in a real Kitty terminal, including grouped requests, regional latency, build runs, queue states, and restoring pixels after a cell toggle" width="1028">

Screenshots: [Requests](docs/media/barchart.png), [Regional latency](docs/media/barchart-latency.png). Recorded from a clean source tree at `1843c3a090394c864cb36252bb2c478278f1bbab` in a fresh Ubuntu arm64 VM, using `EXAMPLE=pixel_bar_chart FEATURES=image-presenter WIDTH=168 HEIGHT=36 FONT_SIZE=16 SECONDS_TO_RECORD=24 KEYS=3:u,6:Right,9:u,12:Right,15:Right,18:b,21:b scripts/record-pixels.sh`: 168×36 cells, 2185×901 pixels, 30 fps, 24 seconds, lossless WebP 43,442 bytes. Eight steps hold for about 3 seconds each; restoring Pixels repeats the earlier Queue state exactly. All 720 source frames and all eight WebP frames match the seven inspected states. The stills are unmodified video frames at 1 and 7 seconds. No compositing or dropped frames.

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

Scope, Globe, System CPU/memory/network, Clock/tree, Flock, and Pings keep equal-size Braille and antialiased pixel views of the same scene and sample. Labels stay as terminal text. Pixels are enabled by default with a measured presenter; `.pixels(false)` retains the original cell renderer. Try `cargo run --release -p lunatui --example pixel_canvas`: arrows change scenes, `u` advances the sample, and `b` switches the right pane to cells and back.

<img src="docs/media/canvas.webp" alt="Matching Braille and antialiased pixel Canvas scenes in a real Kitty terminal, followed by switching the right pane to cells and restoring pixels" width="1028">

Screenshots: [Scope](docs/media/canvas.png), [Clock and tree](docs/media/canvas-clock.png). Recorded from a clean source tree at `2c45e6a1d57b4c0ca5cce5ed89b95ac90e8cdfc7` in a fresh Ubuntu arm64 VM, using `WIDTH=120 HEIGHT=36 FONT_SIZE=16 SECONDS_TO_RECORD=24 EXAMPLE=pixel_canvas FEATURES=image-presenter KEYS=3:Right,6:Right,9:Right,12:Right,15:Right,18:b,21:b scripts/record-pixels.sh`: 120×36 cells, 1561×901 pixels, 30 fps, 24 seconds, lossless WebP 292,106 bytes. Each scene stays on screen for about 3 seconds. All 720 source frames retain graph pixels, including cells-to-pixels restoration. The stills are unmodified video frames at 0 and 10.5 seconds. Coastlines are from Natural Earth, public domain.

### Toast

A timed toast stack over an operations console: boxed toasts stack up in the corner of the pane they belong to, one-row status toasts stack on the bottom rows, and each goes away when its time is up, so a newer message sits above an older one and a status message sits above them all.

<img src="docs/media/toast.webp" alt="Toast stack: pane toasts and status toasts appearing and expiring over an operations console in the dark theme" width="1028">

Screenshot: [docs/media/toast.png](docs/media/toast.png). Recorded with `scripts/record-showcase.sh` (`CASE=toast`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Tab reordering

A release pipelines screen with a boxed tab strip reordered by the pointer. Pressing a tab selects it and lifts it off the strip. While it follows the pointer, the other tabs slide aside and an empty slot marks where it will land. On release it settles into that slot, and the selection stays on it. Three drags run: Builds forward, Deploys to the front, and Logs to the end. Each tab shows live content: a builds table with a running build, deploys with a rollout in progress, and a streaming log.

<img src="docs/media/tabs-drag.webp" alt="TabsDrag: a tab lifted off a boxed strip and dropped into a new position over builds, deploys and logs panes, in the dark theme" width="1028">

Screenshot: [docs/media/tabs-drag.png](docs/media/tabs-drag.png). Recorded with `scripts/record-showcase.sh` (`CASE=tabs-drag`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Table sorting, selection and resizing

A live service inventory whose latencies and error rates update while you work. The cursor walks the services, and slow or failing values are colored by a per-cell style closure. A click on the p95 header sorts slowest first and marks the column `▼`, keeping the cursor on its service. Shift+Down selects a range of four services, listed in the details pane. Dragging the Service divider widens that column while the others keep their widths.

<img src="docs/media/table-upgrades.gif" alt="Table: sorting by p95, a range selection, per-cell colors and a resized column in a live service inventory in the dark theme" width="1028">

Screenshot: [docs/media/table-upgrades.png](docs/media/table-upgrades.png). Recorded with `scripts/record-widget.sh` (`CASE=table-sorted`, 120×36, `FRAMES=250` at 30 fps, truecolor, dark, about 8 seconds plus a 3-second hold, 234 KB).

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

### PieChart

A cluster overview of live traffic: a high-resolution braille pie of requests by region sweeps in with an eased reveal and redraws its shares every frame from live rates. The highlight then steps through each region, pulling its slice out and following it in the legend, the regions table and the detail line, while a braille line chart beside it tracks total requests over the last 40 seconds. Slice colors stay apart from each other in the dark, light, and 16-color palettes.

For antialiased pixels, wrap the chart in `PixelPieChart` and attach the frame's image sink. Pixel support is enabled by default. Use `.pixels(false)` on the widget or `RunOptions` to select Braille explicitly. Kitty, iTerm2 inline images, and Sixel use the shared presenter; unsupported terminals and multiplexers use Braille. Try `cargo run --release -p lunatui --features image-presenter --example pixel_pie_chart`. Add `-- --compare` to mirror the Braille and pixel versions side by side with the same data and layout. See the [widget reference](lunatui-widgets/README.md) for text-layering and resource limits.


<img src="docs/media/pixel-pie-chart.webp" alt="Braille and antialiased pixel pies, mirrored with the same data and geometry in a real Kitty terminal" width="1028">

Pixel comparison screenshot: [docs/media/pixel-pie-chart.png](docs/media/pixel-pie-chart.png). Captured from a real Kitty 0.32.2 window in a clean Linux VM (100×32, font size 20, 30 fps, 8 seconds, with the selected region changing from the keyboard). The Braille version remains alongside the pixel version; no compositing.

<img src="docs/media/pie-chart.webp" alt="PieChart: a braille pie of live request shares by region, with a regions table and a request trend, on a cluster overview screen in the dark theme" width="1028">

Screenshot: [docs/media/pie-chart.png](docs/media/pie-chart.png), plus the [light theme](docs/media/pie-chart-light.png) and the [16-color profile](docs/media/pie-chart-ansi16.png). Recorded with `scripts/record-showcase.sh` (`CASE=pie-chart`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 10 seconds).

### HexViewer

A firmware inspector: the cursor walks the image header byte by byte, a shift-selection takes the four-byte version and then a section name, pages jump into the boot log strings and the vector table, and a mouse-wheel scroll moves the view while the cursor stays put. The viewer draws only the visible rows of a 4 KB image, colors bytes by kind, and the inspector pane reads the values under the cursor.

<img src="docs/media/hex-viewer.webp" alt="HexViewer: a cursor and selection moving through a firmware image in the dark theme" width="1028">

Screenshot: [docs/media/hex-viewer.png](docs/media/hex-viewer.png). Recorded with `scripts/record-showcase.sh` (`CASE=hex-viewer`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 10 seconds).

### Progress bar

A release pipeline: a determinate bar for checkout, a stacked bar for compile (cached, compiled, and failed crates in their own colors), a stacked bar for the test suites (passed, skipped, failed), an indeterminate block bouncing along the track while the upload runs, and an overall bar with a percentage. The fill moves in eighths of a cell, and a cell where two segments meet shows both colors.

<img src="docs/media/progress-bar.webp" alt="ProgressBar: determinate, stacked, and indeterminate bars across a release pipeline in the dark theme" width="1028">

Screenshot: [docs/media/progress-bar.png](docs/media/progress-bar.png). Recorded with `scripts/record-showcase.sh` (`CASE=progress-bar`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).
### Accordion

A pipeline settings screen: sections with a caret, a title, and a badge open and close one after another, each body growing and shrinking through a reveal fraction the app raises and lowers every tick. The accordion draws the headers and gives back the body rectangles, so the app fills them with its own content, and the summary pane follows the focused section.

<img src="docs/media/accordion.webp" alt="Accordion: sections opening and closing on a pipeline settings screen in the dark theme" width="1028">

Screenshot: [docs/media/accordion.png](docs/media/accordion.png). Recorded with `scripts/record-showcase.sh` (`CASE=accordion`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).
### Number input

A render queue's export settings: each field is a spin box with minus and plus buttons, a step, a range, and decimals, and the screen nudges them one after another while the estimate beside them follows. One field is typed by hand: the empty text shows in the fail color with a caret, digits arrive one at a time, and the committed value snaps to the field's decimals. The widget draws the field and gives back the rules (`nudge`, `parse`, `accepts`, `format`) and the button cells for the app's own key and mouse handling.

<img src="docs/media/number-input.webp" alt="NumberInput: spin boxes stepping and one field being typed on an export settings screen in the dark theme" width="1028">

Screenshot: [docs/media/number-input.png](docs/media/number-input.png). Recorded with `scripts/record-showcase.sh` (`CASE=number-input`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 10 seconds).

### Slider

A studio mixer: eight channel faders with a focus knob and the value beside each, four range sliders for equalizer bands whose two ends drift independently, and an output level that follows the master and music faders. The track fills with half-cell precision, so a fader moves in smaller steps than the cell grid, and the focused channel sweeps while the others hold their positions.

<img src="docs/media/slider.webp" alt="Slider: channel faders and equalizer range sliders with half-cell positions over a mixer screen in the dark theme" width="1028">

Screenshot: [docs/media/slider.png](docs/media/slider.png). Recorded with `scripts/record-showcase.sh` (`CASE=slider`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 12 seconds).

### Notification center

An inbox beside a deploy dashboard: toasts pop up as events arrive and each one lands in the history with an unread mark and a count in the header, the arrow keys move through the list and read what they pass, a dismissed entry drops out, and marking all read turns the header to "All read".

<img src="docs/media/notification-center.webp" alt="NotificationCenter: a notification history growing as toasts arrive, unread marks turning to read as the selection moves, a dismissed entry, and an all-read header in the dark theme" width="1028">

Screenshot: [docs/media/notification-center.png](docs/media/notification-center.png). Recorded with `scripts/record-showcase.sh` (`CASE=notification-center`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 9 seconds).

### Date picker

A trip form with two months side by side: the arrow keys move the cursor a day at a time, a range stretches from the first chosen day with its ends filled and a band between, days outside the allowed dates are dimmed, the cursor crosses into the next month, and the summary line confirms the range.

<img src="docs/media/date-picker.webp" alt="DatePicker: two months side by side with a date range growing across them, filled range ends, dimmed days outside the bounds, and a confirmed summary in the dark theme" width="1028">

Screenshot: [docs/media/date-picker.png](docs/media/date-picker.png). Recorded with `scripts/record-showcase.sh` (`CASE=date-picker`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 7 seconds).

### Combo box

A deployment form with a region field: the drop-down opens, arrows move the highlighted row and scroll the six-row list, typing narrows the thirteen regions with the matched text marked, and the chosen value closes the list back into the field.

<img src="docs/media/combo-box.webp" alt="ComboBox: a deployment form whose region field opens a filtered drop-down list, narrows as text is typed, and settles on the chosen region in the dark theme" width="1028">

Screenshot: [docs/media/combo-box.png](docs/media/combo-box.png). Recorded with `scripts/record-showcase.sh` (`CASE=combo-box`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 9 seconds).

### Diff view

A code review: a unified diff of a grid change with hunk headers, two line-number columns, removed and added lines in their own colors, and the changed part of each paired line inverted. It scrolls through both hunks, then switches to the side-by-side layout, where each removed line sits across from the line that replaced it.

<img src="docs/media/diff.webp" alt="DiffView: a code review scrolling a unified diff and then a side-by-side diff with highlighted changes in the dark theme" width="1028">

Screenshot: [docs/media/diff.png](docs/media/diff.png). Recorded with `scripts/record-showcase.sh` (`CASE=diff`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 8 seconds).

### Log view

A service console: log lines stream into a ring buffer and scroll in from the bottom with a level label, a time, and a target column. The level filter then rises to warnings and errors, and a text filter keeps only the lines that mention the database, highlights the match, and scrolls back through the history before returning to follow the newest line.

<img src="docs/media/logs.webp" alt="LogView: a service console following new log lines, then filtering by level and text with a highlighted match in the dark theme" width="1028">

Screenshot: [docs/media/logs.png](docs/media/logs.png). Recorded with `scripts/record-showcase.sh` (`CASE=logs`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 10 seconds).

### Search box

A widget catalog with live filtering: the query is typed one character at a time (and backspaced), the list narrows to the items that contain every term with the matched text highlighted, the count on the right follows, and `n` steps the active match with the details pane following it. The widget draws the field and hands back the rules (`filter` with smart case and match ranges, `step` through the matches, `put_highlighted` for drawing a row with its matches marked) and the clear button's cell for the app's own key and mouse handling.

<img src="docs/media/search-box.webp" alt="SearchBox: a catalog filtered live with highlighted matches and a match count in the dark theme" width="1028">

Screenshot: [docs/media/search-box.png](docs/media/search-box.png). Recorded with `scripts/record-showcase.sh` (`CASE=search-box`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 10 seconds).

### Popover

A deploy console with a toolbar, a table of deploys and an activity log. Filter opens a status list under its button, and unticking queued drops those rows. A click on the table closes it. Sort, at the right edge, opens slid left to stay on screen, and choosing By service re-sorts the table. Columns hides the Version column, and Escape closes it. No popover covers its button.

<img src="docs/media/popover.webp" alt="Popover: filter, sort and column panels opening under toolbar buttons in a deploy console, in the dark theme" width="1028">

Screenshot: [docs/media/popover.png](docs/media/popover.png). Recorded with `scripts/record-showcase.sh` (`CASE=popover`, 120×36 at 2x scale, `FRAMES=240`, dark, lossless animated WebP, about 8 seconds plus a 2-second hold).

### Code view

A code review screen with a file list, the source with line numbers and gutter marks for changes, an error, a warning and a breakpoint, and a problems pane whose check run keeps moving. The cursor walks down the file and the view scrolls with it. `]e` jumps to the error, `l` scrolls right to the end of its long line, `0` scrolls back, and `]c` jumps to the next change. A small highlighter written against the highlight hook colors keywords, strings, numbers and comments.

<img src="docs/media/code-view.gif" alt="Code view: line numbers, gutter marks and highlighted Rust source in a review screen in the dark theme" width="1028">

Screenshot: [docs/media/code-view.png](docs/media/code-view.png). Recorded with `scripts/record-widget.sh` (`CASE=code-view`, 120×36, `FRAMES=250` at 30 fps, truecolor, dark, about 8 seconds plus a 3-second hold, 243 KB).

### Button

A release console with live checks and a deploy history. Enter presses Run checks and the four checks start. Tab moves focus to Roll back, then Cancel, then back, and skips the disabled Deploy to prod button until the checks pass. A pointer then presses Deploy to prod and the rollout starts. The buttons show the normal, primary and destructive variants and the focused, pressed and disabled states.

<img src="docs/media/button.gif" alt="Button: focused, pressed, disabled, primary and destructive buttons in a release console in the dark theme" width="1028">

Screenshot: [docs/media/button.png](docs/media/button.png). Recorded with `scripts/record-widget.sh` (`CASE=button`, 120×36, `FRAMES=250` at 30 fps, truecolor, dark, about 8 seconds plus a 3-second hold, 118 KB).

### Dialog

A branch manager with a live activity feed and CI checks that keep moving behind each dialog. Deleting `feature/login` asks for confirmation with focus on Cancel; Tab moves to the red Delete branch button and Enter deletes it, then focus goes back to the branch list. Renaming `feature/search` opens a prompt and the new name is typed into its field. Deleting the protected `main` branch shows an alert. Tab only cycles inside the open dialog.

<img src="docs/media/dialog.gif" alt="Dialog: confirm, prompt and alert dialogs over a live branch manager in the dark theme" width="1028">

Screenshot: [docs/media/dialog.png](docs/media/dialog.png). Recorded with `scripts/record-widget.sh` (`CASE=dialog`, 120×36, `FRAMES=250` at 30 fps, truecolor, dark, about 8 seconds plus a 3-second hold, 227 KB).

### Sparkline

The service board keeps equal-size Cells and antialiased Pixels views of all ten lanes and the selected inspector. CPU, Network, Requests, Errors, Cache and Connections scenes share their data, styles and layout. Labels and statistics remain terminal text. Pixels are enabled by default with a measured presenter; `.pixels(false)` keeps the original eighth-block renderer. Try `cargo run --release -p lunatui --example pixel_sparkline`: arrows select scenes, `u` advances the sample, and `b` switches the right pane to cells and back.

<img src="docs/media/sparkline.webp" alt="Matching Cells and antialiased Pixels Sparkline service boards in a real Kitty terminal, including all ten lanes, the selected inspector, hot error styling, and restoring pixels after a cell toggle" width="1028">

Screenshots: [CPU](docs/media/sparkline.png), [Errors](docs/media/sparkline-errors.png). Recorded from a clean source tree at `489ed9f350a8a7e24f11d4399d0b61f6a9e6e236` after merging current main in a fresh Ubuntu arm64 VM, using `EXAMPLE=pixel_sparkline FEATURES=image-presenter WIDTH=200 HEIGHT=44 FONT_SIZE=16 SECONDS_TO_RECORD=24 KEYS=3:Right,6:Right,9:Right,12:Right,15:Right,18:b,21:b scripts/record-pixels.sh`: 200×44 cells, 2601×1101 pixels, 30 fps, 24 seconds, lossless WebP 108,664 bytes. All eight steps hold for 2.7–3.3 seconds. All 720 source frames and all eight WebP frames match the seven inspected states; restoring Pixels repeats the earlier Connections frame exactly. The stills are unmodified video frames at 1 and 10 seconds. No compositing or dropped frames.

### Calendar

A team schedule: the month beside that day's agenda, colored release, review, on-call, and incident days, today held in a raised cell, and the selection walking from late October into November.

<img src="docs/media/calendar.webp" alt="Calendar widget: a team schedule in the dark theme" width="1028">

Screenshot: [docs/media/calendar.png](docs/media/calendar.png). Recorded with `scripts/record-showcase.sh` (`CASE=calendar`, 120×36 at 2x scale, `FRAMES=300`, dark, lossless animated WebP, about 12 seconds).

### BigText

A cutover board: a live clock, a countdown that reaches zero, and ticking metrics. The selection moves, and the error count turns red while it is hot.

<img src="docs/media/bigtext.webp" alt="BigText widget: a cutover board in the dark theme" width="1028">

Screenshot: [docs/media/bigtext.png](docs/media/bigtext.png). Recorded with `scripts/record-showcase.sh` (`CASE=bigtext`, 120×36 at 2x scale, `FRAMES=330`, dark, lossless animated WebP, about 13 seconds).

### Image

NASA's 1920×1920 Blue Marble Earth, displayed by the `Image` widget through Kitty graphics. The recording switches between Contain and Cover with real key input. Credit: NASA, PIA18033.

<img src="docs/media/image-native.webp" alt="Image widget: full-resolution NASA Blue Marble in a real Kitty terminal" width="1028">

Screenshot: [docs/media/image-native.png](docs/media/image-native.png). Recorded from an actual Kitty window with `scripts/record-pixels.sh` (100×32, font size 16, 1301×801 pixels, 8 seconds at 30 fps, lossless animated WebP). This captures image pixels in the terminal framebuffer. See [Record a pixel widget in a real terminal](docs/recording-pixels.md).

### Nested menu

An editor opens File, steps past a disabled Save item, and opens Recent. Selecting `metrics.rs` changes the active buffer; the edit is typed into the document and saved, updating the activity log and source-control status. Every panel, menu, code line, cursor and status row comes from real widgets.

<img src="docs/media/nested-menu.webp" alt="Nested Menu in a project editor: navigate File, skip disabled Save, open a Recent file, edit and save" width="1028">

Screenshot: [Recent submenu over the editor](docs/media/nested-menu.png), taken at 3.8 seconds. Recorded at 120×36 and 2× scale in the dark theme with `CASE=menu-editor FRAMES=300 FPS=30 LAST_FRAME_DURATION=1 MAX_BYTES=2000000 scripts/record-showcase.sh`, as a lossless animated WebP, about 11 seconds.

Widget-parity Lunatui and Ratatui recordings: truecolor dark at [80×24](docs/media/nested-menu-80x24-truecolor-dark-side-by-side.mp4) and [200×60](docs/media/nested-menu-200x60-truecolor-dark-side-by-side.mp4), ANSI16 light at [80×24](docs/media/nested-menu-80x24-ansi16-light-side-by-side.mp4) and [200×60](docs/media/nested-menu-200x60-ansi16-light-side-by-side.mp4). Screenshots: [truecolor dark](docs/media/nested-menu-80x24-truecolor-dark.png) and [ANSI16 light](docs/media/nested-menu-80x24-ansi16-light.png). Reproduce with `CASE=nested-menu WIDTH=80 HEIGHT=24 PROFILE=truecolor THEME=dark FRAMES=180 FPS=30 scripts/record-widget.sh`, changing the size, profile, and theme for each variant.

### Popup and ScrollView

A release console. The build log is a `ScrollView` wider and taller than its pane, scrolled and panned with both scrollbars. A deploy `Popup` opens over a faded backdrop, the choice moves to Deploy, and the dialog is dragged aside. After the confirm, new log lines fill the last page and the services roll to the new build. An alerts `Popup` then opens above its status-bar chip with a scrolling `ScrollView` inside.

<img src="docs/media/popup-scrollview.webp" alt="Popup and ScrollView: a release console in the dark theme" width="1028">

Screenshot: [docs/media/popup-scrollview.png](docs/media/popup-scrollview.png). Recorded with `scripts/record-showcase.sh` (`CASE=popup-scrollview`, 120×36 at 2× scale, 240 frames at 30 fps, dark, lossless animated WebP, about 10 seconds).

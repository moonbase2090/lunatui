# ADR 0003: Public application API (PR 23)

## Status

Accepted (2026-10-12). Implements the first slice of PRD §3.2 before widgets and `Ui` layout helpers land in PR 28+.

## Context

lunatui needs a single entry point for interactive examples and dogfood apps. PR 22 added the frame scheduler; PR 21 added coalesced input. PR 23 wires them into `lunatui_runtime::run` and the `lunatui` facade.

## Decision

### `App` trait

Applications implement:

- `update(&mut self, &Event) -> ControlFlow` — handle input; return `Break` to quit.
- `render(&mut self, &mut Grid, Rect, &mut ClusterTable)` — draw the current frame into the grid (mutable so render can register hits).

State lives on the app struct. Widgets, IDs, memo, and `Ui` layout sugar are deferred to DD-3 follow-ups in PR 24–28. **`Widget` / `RenderCx` shipped in PR 28** — see [ADR 0004](0004-widget-trait-render-cx.md).

### Feature flag

`App`, `RunOptions`, and `run` live behind the `app` Cargo feature on `lunatui-runtime` and `lunatui` (off by default per `REVIEW_POLICY.md`). The `demo` example enables it with `--features app`.

### `run`

`lunatui_runtime::run(app, RunOptions)` (re-exported as `lunatui::run` when `app` is enabled):

- Unix TTY only; alternate screen via `Session` with synchronized output.
- Input thread via `Input::from_reader(stdin)`.
- `Scheduler::idle()` — event-driven redraws (no animation timer until the app opts in later).
- Double-buffered `Grid` + `diff_runs` + `Encoder::encode_diff` per frame.

Non-TTY callers use `render_once` or example-specific bench paths (see `examples/demo.rs`).

### Examples (M13)

- `cargo run --example demo` — hello screen on `run()`.
- `cargo run --example demo -- --counter` — counter with `+` / `-` / `q`.
- Piped stdout: `--snapshot` or release bench summary (same pattern as `live_dashboard`).

## Consequences

- Facade crate is no longer a stub; prelude and `Widget`/`Ui` expand in later PRs without breaking `App` + `run`.
- Cross-platform `run()` on Windows waits for PR 27; CI uses snapshot/bench modes.

## Proof sketch

```rust
use lunatui::{run, App, ControlFlow, RunOptions, is_quit};
use lunatui_core::{ClusterTable, Grid, Rect};
use lunatui_term::Event;

struct Hello;
impl App for Hello {
    fn update(&mut self, event: &Event) -> ControlFlow {
        if is_quit(event) { ControlFlow::Break } else { ControlFlow::Continue }
    }
    fn render(&self, grid: &mut Grid, area: Rect, clusters: &mut ClusterTable) {
        grid.put_str(area.x + 1, area.y + 1, lunatui_core::Style::new(), "hi", clusters);
    }
}
```

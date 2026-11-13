# lunatui-test

Testing kit for lunatui: styled snapshots, virtual clock, an interaction harness, and vt100 oracle helpers.

## Styled snapshots

`styled_snapshot` serializes a [`Grid`](https://github.com/moonbase2090/lunatui/blob/main/lunatui-core/src/grid.rs) as a text layer plus run-length fg/bg/attr layers (`@text`, `@fg`, `@bg`, `@attrs`). Widget and theme tests use this format for readable diffs.

## Virtual clock

`Clock` is an injectable monotonic clock for deterministic animation tests (PRD §2.10). The same API exists on `lunatui_runtime::Clock` for app code without pulling this crate into the runtime dependency graph.

## Interaction harness

`Harness` mounts a `Component` (state that takes events and draws itself), injects real key and pointer events, and asserts focus, value and the drawn frame. Each event is encoded to the bytes a terminal would send and decoded by the real input parser before the component sees it, so a test can only inject input a terminal can produce. `Harness::advance` moves the virtual clock and ticks the component; the harness never reads the wall clock or sleeps.

```rust
use std::time::Duration;

use lunatui_core::{ClusterTable, Grid, Rect, Style};
use lunatui_term::{Event, KeyCode};
use lunatui_test::{Component, Harness};

struct Counter(u32);

impl Component for Counter {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Key(key) if key.code == KeyCode::Up => self.0 += 1,
            Event::Key(key) if key.code == KeyCode::Down => self.0 = self.0.saturating_sub(1),
            _ => {}
        }
    }

    fn render(&mut self, grid: &mut Grid, _area: Rect, clusters: &mut ClusterTable) {
        grid.put_str(0, 0, Style::new(), &format!("count {}", self.0), clusters);
    }

    fn value(&self) -> Option<String> {
        Some(self.0.to_string())
    }
}

let mut counter = Harness::mount(Counter(0), 10, 1);
counter.keys("Up Up Down");
counter.assert_value("1");
counter.assert_rows(&["count 1"]);
counter.advance(Duration::from_millis(16));
```

Keys are spelled `Enter`, `Esc`, `Tab`, `Space`, `Up`, `F5`, a single character, or a modifier prefix such as `C-a`, `A-Left` and `S-Tab`. For the pointer there are `click`, `click_with`, `hover`, `drag`, `scroll` and `mouse`. Interactive widget tests are expected to use it; see [conventions](../docs/conventions.md).

## Oracle

`Oracle` feeds encoder bytes to the `vt100` parser. `diff_matches` checks that painting the previous grid and then the encoded diff reproduces the next grid. `fuzz_bytes` and `fuzz_campaign` drive that check from arbitrary grid writes.

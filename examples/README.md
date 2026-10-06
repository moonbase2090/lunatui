# lunatui examples

Each example runs from the repository root with one command (PRD M13). Every feature PR adds or extends an example here.

| Example | Command | Shows | Last changed |
|---|---|---|---|
| `hello_box` | `cargo run --example hello_box -- --metrics` | A bordered box with text, including a line of CJK, Hangul, an emoji and an accented letter whose right border stays aligned, because each line is measured with `lunatui_core::width` (PR 04). The box is built as a plain string for now; later PRs move it onto lunatui's grid, encoder and `Block` widget. Multi-codepoint clusters such as ZWJ emoji arrive with PR 05. With `--metrics` it also shows what drawing the box cost (`write()` calls, bytes, allocations), measured with the bench harness counters. | PR 04 |
| `ratatui_dashboard` | `cargo run --release --example ratatui_dashboard` | The ratatui baseline dashboard scene (PR 03), drawn live on ratatui 0.30.2 for 240 frames at about 60 fps. It then prints the draw-time p50, p99 and maximum against the 8 ms frame target, naming the build profile, because only a release build is a fair number. This is the scene lunatui is measured against. | PR 03 |
| `cell_prototype` | `cargo run --release --example cell_prototype` | The evidence for ADR 0001 (DD-1). It packs PR 03's dashboard frames into 16-byte prototype cells, checks that the packed diff finds the same changed cells as ratatui, and compares diff times: ratatui's 48-byte cells, the packed struct, and rows stored as `u128` words. | DD-1 |

Captures (M13) live in `captures/`. A `.png` shows the final screen, and a `.cast` replays with `asciinema play`.

![Capture of the hello box example](captures/hello_box.png)

![Capture of the ratatui dashboard example](captures/ratatui_dashboard.png)

![Capture of the cell prototype example](captures/cell_prototype.png)

Examples live in this directory and are registered as `[[example]]` targets of the `lunatui` facade crate (`lunatui/Cargo.toml`).

Benchmarks are separate: `cargo xtask bench` prints allocation and write metrics and runs the criterion and gungraun benchmarks in `benches/`.

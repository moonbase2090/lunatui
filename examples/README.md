# lunatui examples

Each example runs from the repository root with one command (PRD M13). Every feature PR adds or extends an example here.

| Example | Command | Shows | Last changed |
|---|---|---|---|
| `hello_box` | `cargo run --example hello_box -- --metrics` | A bordered box with text. It is built as a plain string for now. Later PRs move it onto lunatui's grid, encoder and `Block` widget. It counts widths in `char`s, which is correct for its ASCII text; display widths arrive with PR 04 and PR 05. With `--metrics` it also shows what drawing the box cost (`write()` calls, bytes, allocations), measured with the bench harness counters. | PR 02 |
| `ratatui_dashboard` | `cargo run --release --example ratatui_dashboard` | The ratatui baseline dashboard scene (PR 03), drawn live on ratatui 0.30.2 for 240 frames at about 60 fps. It then prints the draw-time p50, p99 and maximum against the 8 ms frame target, naming the build profile, because only a release build is a fair number. This is the scene lunatui is measured against. | PR 03 |

Captures (M13) live in `captures/`. A `.png` shows the final screen, and a `.cast` replays with `asciinema play`.

![Capture of the hello box example](captures/hello_box.png)

![Capture of the ratatui dashboard example](captures/ratatui_dashboard.png)

Examples live in this directory and are registered as `[[example]]` targets of the `lunatui` facade crate (`lunatui/Cargo.toml`).

Benchmarks are separate: `cargo xtask bench` prints allocation and write metrics and runs the criterion and gungraun benchmarks in `benches/`.

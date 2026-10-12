# lunatui examples

Each example runs from the repository root with one command (PRD M13). Every feature PR adds or extends an example here.

| Example | Command | Shows | Last changed |
|---|---|---|---|
| `layout_prototype` | `cargo run --example layout_prototype` | The ADR 0002 closed-form split. A fixed 10 and a fixed 8 stay put when the row grows from 40 to 80, and the two fill panes take the extra cells at weights 1 and 2. Twenty `Fill(1)` panes at width 200 are 10 each, with no solver fallback. | DD-2 |
| `keys` | `cargo run --example keys` | Decoded key, mouse, paste and focus events from a captured byte stream (PR 20). A terminal shows that capture. Piped stdin is decoded instead when it is non-empty. | PR 20 |
| `live_dashboard` | `cargo run --release --example live_dashboard` | Dogfood dashboard with a scrolling log and spinner. The animation loop uses `lunatui_runtime::Scheduler` (idle, dirty, animating) instead of a blind sleep. Piped stdout runs a short bench and prints p50, p99, and max against the frame target. | PR 22 |
| `gradient` | `cargo run --example gradient --features color-profiles` | A hue ramp in four profiles, each step a colored swatch: truecolor, xterm 256, ANSI 16, and black or white. The 16-color row keeps distinct hues. The feature stays off unless this command turns it on. | PR 30 |
| `caps` | `cargo run --example caps` | The probed `Caps` in a box: color, DEC modes 2026 and 2027, keyboard and graphics, the width fallback, and the terminal name (PR 12). On a terminal it runs the 200 ms probe and enables mode 2027 when the terminal reports it reset. Piped stdout uses the environment only. | PR 12 |
| `hello_box` | `cargo run --example hello_box -- --metrics` | A bordered box with text, including CJK, Hangul, an emoji, an accented letter, and a line of ZWJ emoji, VS16 and a flag cluster whose right border stays aligned because each line is measured with `lunatui_core::width` (PR 04–05). The box is drawn into a `lunatui_core::Grid` (PR 07). With `--encode` it emits the frame through `lunatui_term::Encoder` (PR 09). With `--session` it presents that frame through `lunatui_term::Session` in the alternate screen with synchronized output and restores on exit (PR 10, Unix). With `--oracle` it checks the vt100 oracle on the diff between the previous demo frame and this one (PR 11). With `--diff` it diffs two frames with `lunatui_core::diff_runs` and prints the changed runs (PR 08). With `--cells` it prints packed `lunatui_core::Cell` words for sample glyphs (PR 06). With `--metrics` it also shows what drawing the box cost (`write()` calls, bytes, allocations), measured with the bench harness counters. | PR 11 |
| `cell_dump` | `cargo run --example cell_dump` | Sample `Cell` values (blank, box corner, text and a wide lead) as 32-digit hex `u128` words, matching ADR 0001 layout (PR 06). | PR 06 |
| `ratatui_dashboard` | `cargo run --release --example ratatui_dashboard` | The ratatui baseline dashboard scene (PR 03), drawn live on ratatui 0.30.2 for 240 frames at about 60 fps. It then prints the draw-time p50, p99 and maximum against the 8 ms frame target, naming the build profile, because only a release build is a fair number. This is the scene lunatui is measured against. | PR 03 |
| `cell_prototype` | `cargo run --release --example cell_prototype` | The evidence for ADR 0001 (DD-1). It packs PR 03's dashboard frames into 16-byte prototype cells, checks that the packed diff finds the same changed cells as ratatui, and compares diff times: ratatui's 48-byte cells, the packed struct, and rows stored as `u128` words. | DD-1 |

Captures (M13) live in `captures/`. A `.png` shows the final screen, and a `.cast` replays with `asciinema play`. `captures/hello_box_oracle.cast` is `cargo run -q --example hello_box -- --oracle`. `captures/caps.cast` is `cargo run -q --example caps` with `TERM=xterm-256color`, `COLORTERM=truecolor`, and `TERM_PROGRAM=Apple_Terminal` (not a tty, so the box shows the ambiguous-wide fallback). `captures/gradient.cast` is `cargo run -q --example gradient --features color-profiles`. `captures/layout_prototype.cast` is `cargo run -q --example layout_prototype`. `captures/keys.cast` is `cargo run -q --example keys`. `captures/live_dashboard.cast` is `cargo run -q --example live_dashboard -- --snapshot --width 80 --height 24` (one rendered frame). Bench summary: `cargo run -q --release --example live_dashboard -- --width 80 --height 24 --frames 4`.

![Capture of the hello box example](captures/hello_box.png)

![Capture of the cell dump example](captures/cell_dump.png)

![Capture of the ratatui dashboard example](captures/ratatui_dashboard.png)

![Capture of the cell prototype example](captures/cell_prototype.png)

![Capture of the live dashboard example](captures/live_dashboard.png)

Examples live in this directory and are registered as `[[example]]` targets of the `lunatui` facade crate (`lunatui/Cargo.toml`).

Benchmarks are separate: `cargo xtask bench` prints allocation and write metrics and runs the criterion and gungraun benchmarks in `benches/`.

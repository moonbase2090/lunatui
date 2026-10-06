# lunatui examples

Each example runs from the repository root with one command (PRD M13). Every feature PR adds or extends an example here.

| Example | Command | Shows | Last changed |
|---|---|---|---|
| `hello_box` | `cargo run --example hello_box -- --metrics` | A bordered box with text. It is built as a plain string for now. Later PRs move it onto lunatui's grid, encoder and `Block` widget. With `--metrics` it also shows what drawing the box cost (`write()` calls, bytes, allocations), measured with the bench harness counters. | PR 02 |

Captures (M13) live in `captures/`. A `.png` shows the final screen, and a `.cast` replays with `asciinema play`.

![hello_box](captures/hello_box.png)

Examples live in this directory and are registered as `[[example]]` targets of the `lunatui` facade crate (`lunatui/Cargo.toml`).

Benchmarks are separate: `cargo xtask bench` prints allocation and write metrics and runs the criterion and gungraun benchmarks in `benches/`.

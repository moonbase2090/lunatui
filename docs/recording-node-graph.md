# Node graph comparison

The scene uses native `NodeGraph` and pinned `tui-nodes` 0.10.0. Two inputs join a
package node, then connect to a release node. Input port rows, dimensions, titles,
body text, theme styles and scripted selection are the same on both sides.
Selected borders pulse through the runtime's cubic easing curve. Every native
scene element is a real widget.

## Reproduce

Install the pinned `agg` named in `scripts/record-widget.sh`, asciinema 3 and FFmpeg.

```sh
CASE=node-graph bash scripts/record-widget-matrix.sh
python3 scripts/check-node-graph-recordings.py target/node-graph-recordings
python3 scripts/export-widget-recordings.py --case node-graph target/node-graph-recordings
cargo test --locked -p lunatui-widgets --no-default-features --features pixel-node-graph --test node_graph --test pixel_node_graph
cargo test --locked -p lunatui-bench --test widget_suite_parity
python3 scripts/bench-widget.py --case node-graph
```

Recordings cover 80x24 and 200x60 in both themes, with truecolor, ANSI256 and ANSI16.
Both sides run 240 frames at 30 fps. Lunatui is on the left of each side-by-side
video. Screenshots use the same 6.5-second sample on both sides.

The checker decodes actual GIF pixels at 0.5, 2.5, 4.5 and 6.5 seconds. It verifies
that repository/check labels and a connection remain fixed, the release changes
from queued to published, and both implementations have identical pixels in
those regions. It rejects a recording outside the 7.5-to-12-second timeline.
The exact cell test also checks the changing selected-border styles at thirteen
scripted ticks, both themes, offsets and empty/tiny areas.

## Native pixels

```sh
cargo run -p lunatui --example node_graph
```

The interactive scene animates continuously. Space pauses, arrows step, `b`
switches between pixels and cells, `t` switches themes, and `q` exits. The optional
`pixel-node-graph` feature uses measured pixel targets with text underlay for
antialiased geometry and real cell labels. Unavailable, declined, oversized or
incompatible targets retain the cell rendering. The feature is included in
`all-widgets`; it is off by default on `lunatui-widgets`.

A real Kitty framebuffer recording can be reproduced on Linux with Xvfb, Kitty,
xdotool and FFmpeg:

```sh
EXAMPLE=node_graph FEATURES=image-presenter WIDTH=100 HEIGHT=32 \
  KEYS='2:space,3:Right,4:b,5:b,6:t' bash scripts/record-pixels.sh
```

## Fair timing

The benchmark includes construction, layout and drawing for both graph engines.
The baseline consumes its prepared graph when rendered, so comparing that with
a reused native layout would measure different work. Applications may reuse a
native graph between data changes.

Six rounds alternate side order, reuse frame buffers, warm up 200 iterations and
measure 41 samples of 50 iterations. The benchmark covers dark-theme truecolor.
Render timings include scene construction and graph layout. Frame timings also
include full-scene diff, encoding and a counting sink. Terminal emulator rendering,
transport and native pixel presentation are excluded. The frame/allocation gate
requires a native/baseline frame ratio at most 1.05 and no allocation increase.
Raw rounds and median/range summaries go to `target/node-graph-bench` by default;
`WIDGET_BENCH_OUT` selects another directory.

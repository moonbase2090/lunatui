# Effects comparison

The comparison draws one release-pipeline panel with real widgets, then fades it
with the native `lunatui_runtime::effect::Effect` (feature `effects`) and, on the
other side, with ratatui. The run is 240 frames at 30 fps: fade in for two
seconds, hold for four, fade out for two. The native side builds the timeline
once as `Effect::sequence` of a reversed `Effect::fade` and a delayed `Effect::fade`.

## What the baseline is

ratatui has no effect system, and tachyonfx is not a dependency of this
comparison. The baseline draws the same panel with ratatui widgets and then runs a
small hand-written loop over the buffer: it blends each cell's foreground and
background toward the page color by the same amount. It uses the same
perceptual blend (`lunatui_core::oklab::lerp_rgb`), so the two sides produce
identical cells and the comparison is about the cost of getting there, not about
different looks. The native side adds what the loop does not have: theme-token
resolution, composable timing and easing, reversal, a reduced-motion policy and
the other effects (dissolve, sweep, slide, HSL shift), which are covered by
`lunatui-runtime/tests/effects.rs` and not by this recording.

## Reproduce

Install the pinned `agg` listed in `scripts/record-widget.sh`, plus asciinema 3
and FFmpeg. From the repository root:

```sh
bash scripts/record-effects.sh
python3 scripts/check-effects-recordings.py target/effects-recordings
```

The first command records both implementations at 80x24 and 200x60, in light and
dark themes, for truecolor, ANSI256 and ANSI16. The checker decodes real GIF
frames: the fade-in must leave the faded look (and pass through three different looks in truecolor and ANSI256), the held frame must not change, and the held frame must match between the two sides.

The ANSI16 pairs are checked for the fade and the stable hold only. ratatui writes nothing while a frame holds, and the pinned renderer can keep a stale tint on the baseline side for that stretch (for example a cyan title in the light theme) even though the byte stream matches the native one, as replaying both streams to the same final screen shows. Read the ANSI16 baseline hold with that in mind.

## Parity and performance

```sh
cargo test --locked -p lunatui-bench --test widget_suite_parity
cargo bench --locked -p lunatui-bench --bench widget_suite -- --case effects --gate
```

The parity test compares every cell at both sizes across the whole timeline.
The benchmark times the full frame, including diff and encoding. The native
effect writes runs of equal style with one call and reuses the color math for
cells that share a style, which the baseline loop does not.

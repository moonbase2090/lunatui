# Checkbox comparison

The comparison uses the native `lunatui-choice::Checkbox` from the choice
components and [tui-checkbox](https://github.com/sorinirimies/tui-checkbox) 0.5.3.
Both render the same notification preferences, with unchecked, checked and mixed
markers, moving focus and a disabled weekly summary. All native colors use the
existing semantic theme roles. The baseline uses its public custom-symbol and
styling APIs to match those roles and the `[ ]`, `[x]` and `[-]` glyphs.

## Reproduce

Install the pinned `agg` listed in `scripts/record-widget.sh`, plus asciinema 3
and FFmpeg. Then run from the repository root:

```sh
bash scripts/record-widget-matrix.sh
python3 scripts/check-checkbox-recordings.py target/checkbox-recordings
# Verify again, export gallery media, and refresh the artifact manifest:
python3 scripts/export-widget-recordings.py target/checkbox-recordings
```

The script uses the existing widget speed-test recording method. It captures
both implementations at 80x24 and 200x60, in light and dark themes, for truecolor,
ANSI256 and ANSI16. Each side runs 240 frames at 30 fps. The side-by-side MP4
puts lunatui on the left and tui-checkbox on the right.

The checker decodes actual GIF frames at 0.5, 2.5, 4.5 and 6.5 seconds. It checks
that the first choice changes once, the group shows three different glyph states,
the disabled choice remains fixed, and both implementations produce the same
glyph pixels at every sample. It rejects recordings that exceed 12 seconds,
so missed frame deadlines cannot silently shift the sampled states. It uses the pinned renderer's cell padding and the
fixture's fixed positions. The native scene renders real widgets throughout. The preview caches profile
conversions per player, so repeated colors do not repeat a full palette search
for every cell and frame.

## Parity and performance

```sh
cargo test --locked -p lunatui-bench --test widget_suite_parity
python3 scripts/bench-widget.py
```

The cell comparison checks glyphs, foreground/background colors, bold and dim
attributes across all scripted states and both themes, with empty and tiny areas
as additional cases. The benchmark runs the same fixture in release mode, reuses
buffers, warms up, and measures six paired rounds with alternating side order.
Raw runs and medians/ranges are written to `target/checkbox-bench` by default.
`WIDGET_BENCH_OUT` selects another output directory.

Frame timings include the six controls, shared scene, diff, encoding and a
counting output sink. Render-only timing and allocations are reported separately.
The frame gate requires a native/baseline ratio at most 1.05 and no allocation
increase. These timings describe this full scene on the measured machine.

## Captures

Screenshots are decoded from each recording at 6.5 seconds. The native and baseline
columns use the same frame time. The [artifact manifest](media/checkbox-proof.json)
records source and artifact hashes. Source hashes describe the recorded commit,
so later additions to the shared preview do not imply that old recordings were
recaptured.

| Theme / profile | 80x24 | 200x60 |
|---|---|---|
| dark / truecolor | [video](media/checkbox-80x24-truecolor-dark-side-by-side.mp4), [native](media/checkbox-80x24-truecolor-dark-lunatui.png), [baseline](media/checkbox-80x24-truecolor-dark-ratatui.png) | [video](media/checkbox-200x60-truecolor-dark-side-by-side.mp4), [native](media/checkbox-200x60-truecolor-dark-lunatui.png), [baseline](media/checkbox-200x60-truecolor-dark-ratatui.png) |
| dark / ansi256 | [video](media/checkbox-80x24-ansi256-dark-side-by-side.mp4), [native](media/checkbox-80x24-ansi256-dark-lunatui.png), [baseline](media/checkbox-80x24-ansi256-dark-ratatui.png) | [video](media/checkbox-200x60-ansi256-dark-side-by-side.mp4), [native](media/checkbox-200x60-ansi256-dark-lunatui.png), [baseline](media/checkbox-200x60-ansi256-dark-ratatui.png) |
| dark / ansi16 | [video](media/checkbox-80x24-ansi16-dark-side-by-side.mp4), [native](media/checkbox-80x24-ansi16-dark-lunatui.png), [baseline](media/checkbox-80x24-ansi16-dark-ratatui.png) | [video](media/checkbox-200x60-ansi16-dark-side-by-side.mp4), [native](media/checkbox-200x60-ansi16-dark-lunatui.png), [baseline](media/checkbox-200x60-ansi16-dark-ratatui.png) |
| light / truecolor | [video](media/checkbox-80x24-truecolor-light-side-by-side.mp4), [native](media/checkbox-80x24-truecolor-light-lunatui.png), [baseline](media/checkbox-80x24-truecolor-light-ratatui.png) | [video](media/checkbox-200x60-truecolor-light-side-by-side.mp4), [native](media/checkbox-200x60-truecolor-light-lunatui.png), [baseline](media/checkbox-200x60-truecolor-light-ratatui.png) |
| light / ansi256 | [video](media/checkbox-80x24-ansi256-light-side-by-side.mp4), [native](media/checkbox-80x24-ansi256-light-lunatui.png), [baseline](media/checkbox-80x24-ansi256-light-ratatui.png) | [video](media/checkbox-200x60-ansi256-light-side-by-side.mp4), [native](media/checkbox-200x60-ansi256-light-lunatui.png), [baseline](media/checkbox-200x60-ansi256-light-ratatui.png) |
| light / ansi16 | [video](media/checkbox-80x24-ansi16-light-side-by-side.mp4), [native](media/checkbox-80x24-ansi16-light-lunatui.png), [baseline](media/checkbox-80x24-ansi16-light-ratatui.png) | [video](media/checkbox-200x60-ansi16-light-side-by-side.mp4), [native](media/checkbox-200x60-ansi16-light-lunatui.png), [baseline](media/checkbox-200x60-ansi16-light-ratatui.png) |

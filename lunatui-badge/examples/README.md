# `ui_badge`

Release board built from `lunatui-badge` and `lunatui-card`. `ui_badge` plays an animated run
(`--width`, `--height`, `--frames`, `--fps`); `ui_badge --snapshot` prints one frame as text.

# `graphite_chips`

An activity monitor's header and key hints drawn with `Badge` and `Kbd`, in Cell mode and in
Graphite mode. `graphite_chips OUTPUT_DIR` writes `cells.cast`, the Cell mode frames, and
`graphite-00.png` to `graphite-07.png`, the Graphite mode frames: the images the widgets
painted, composited over the cell backgrounds as a host would. It stops with an error if any
cell text is left outside an image. `scripts/record-graphite.py chips` renders the cast with
the pinned agg, checks every frame, and writes `docs/media/graphite-chips.gif` and the
comparison still `docs/media/graphite-chips.png`.

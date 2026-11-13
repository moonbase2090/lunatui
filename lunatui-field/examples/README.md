# `ui_field`

A profile edit built from real `Field`, text input, switch, button, card and badge widgets.
The virtual clock advances through typing, email validation, notification changes, disabled-field
skipping and a saved confirmation. The example plays a deterministic animation and reads no
keyboard input.

`--snapshot --tick N` prints one frame; add `--ansi` for terminal bytes or use
`--accessibility --tick N` for its value-free control snapshot. `--width`, `--height`,
`--frames` and `--fps` configure the live animation.

Record the GIF and PNG still with `python3 scripts/record-named-controls.py`. Set `AGG` to the
pinned `agg 1.9.0+lunatui.2` used by `scripts/record-showcase.sh`. The recorder verifies the
actual decoded duration, distinct frames and switch transitions, plus the names, roles,
disabled state, focus and value exclusion in all 300 source frames.

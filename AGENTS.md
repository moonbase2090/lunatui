# Agent notes

After cloning, run `./scripts/install-hooks.sh` once (or `scripts/install-hooks.ps1` on Windows) so `pre-commit` runs `cargo fmt --check` on staged `.rs` files.

When a widget adds a feature to `lunatui-widgets`, also add it to `all-widgets` in `lunatui-widgets/Cargo.toml`. The CI and rookrunner widget test steps use `all-widgets`, so widget PRs do not edit the workflow files.

| Rule | Enforcement |
|---|---|
| Animated node graph captions repaint the full row when text shrinks. `Block::style` preserves existing glyphs. | `node_graph_captions_erase_previous_steps_in_reused_buffers` and the blank-tail checks in `scripts/check-node-graph-recordings.py`. |

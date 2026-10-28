# Agent notes

After cloning, run `./scripts/install-hooks.sh` once (or `scripts/install-hooks.ps1` on Windows) so `pre-commit` runs `cargo fmt --check` on staged `.rs` files.

When a widget adds a feature to `lunatui-widgets`, also add it to `all-widgets` in `lunatui-widgets/Cargo.toml`. The CI and rookrunner widget test steps use `all-widgets`, so widget PRs do not edit the workflow files.

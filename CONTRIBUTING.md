# Contributing

Read [`REVIEW_POLICY.md`](REVIEW_POLICY.md) for how a pull request is classified, proven, and reviewed. Run `./scripts/install-hooks.sh` once after cloning so `cargo fmt --check` runs on commit.

## Demos show real widget output only

Every recording, screenshot, and gallery image shows what the widgets render. A demo scene lays out real widgets and hands them data and styles through their public API. It never writes cells after a widget renders and never paints over the result: no `put_str`, `set_string`, cell patches, `fill` calls, hard-coded glyph rows, or strings copied from design mocks.

If a widget cannot draw what a scene needs, fix or extend the widget. Do not draw the missing part in the demo.

`scripts/check-gallery-real-output.sh` enforces this for the site gallery and every example its capture script runs, and CI runs it. A helper that takes a render surface must be named in `scripts/gallery-allowlist.txt` with the reason it is safe. `lunatui/tests/site_gallery_repro.rs` renders each scene again and compares it, and the committed PNG, with `site/proof/gallery-manifest.txt`; run `scripts/gallery-manifest.sh` right after `scripts/capture-site-widgets.sh` to refresh it.

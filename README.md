# lunatui

lunatui is a hyperspeed Rust TUI library being built from scratch to replace ratatui, keeping its simple "render widgets into a rectangle" model while fixing its structural costs with a 16-byte packed cell, damage-tracked diffing and a single-write output encoder, plus first-class input, focus, animation, capability and theme layers. It is open source under the [MPL-2.0](LICENSE) license. Status: **pre-alpha**, with no code yet. See the [PRD](docs/PRD.md) and the [architecture overview](docs/architecture.md).

# lunatui-web

Browser driver for the Lunatui quickstart sandbox. It compiles the native minimal, list, and form templates to WebAssembly and exposes a small `Session` API for input, rendering, ticks, and terminal resizing.

This crate supports the Lunatui website and is not published separately.

## Regenerate the browser assets

Use Linux x86-64, Node 24, Rust 1.99.1 and wasm-bindgen 0.2.129, matching CI. From the repository root:

```sh
rustup toolchain install 1.99.1 --profile minimal --target wasm32-unknown-unknown
./scripts/install-wasm-bindgen.sh
npm ci --prefix site --ignore-scripts
RUSTUP_TOOLCHAIN=1.99.1 node scripts/build-web-sandbox.mjs
./scripts/check-release-text.sh --artifact-dir site/assets/sandbox
```

The generator remaps repository and Cargo dependency paths before compiling. The release-text check scans Wasm modules even when they do not have an executable file mode.

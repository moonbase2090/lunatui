# Graphite mode spike: what a frame costs

This is the first measurement behind [Graphite mode option A](graphite-mode-design.md): lunatui rasterizes pixels itself and hands them to the backend as images. It answers one question before any API is frozen: on a real scene, how much of the 8 ms target do raster and handoff use?

The program is [`scripts/graphite-spike`](../scripts/graphite-spike/src/main.rs), a standalone crate outside the workspace. It draws the activity monitor mockup from the design note (header chips, three seat rows with lanes, a 96-bin lane, fifteen timeline rows and a key row) with `tiny-skia` 0.12 shapes and `fontdue` 0.9 text, then times each phase. Run it with `cargo run --release --locked` from that directory; pass a TTF path to use a different font.

## Results

One machine: Apple M5 Max, 18 cores, single-threaded, release build. Medians over 40 to 400 runs; the maximum is in the table's last column. The font is a system TTF used only for this measurement, not a font lunatui can ship.

| Phase | 1x (1480 x 818) | 2x (2960 x 1636) | 2x maximum |
|---|---:|---:|---:|
| Full frame, glyph cache warm | 1.09 ms | 3.06 ms | 3.16 ms |
| Full frame, glyph cache off | 1.24 ms | 3.34 ms | 4.98 ms |
| Typical poll update (rows and one timeline line) | 0.22 ms | 0.45 ms | 0.50 ms |
| Shapes only (the four lanes) | 0.57 ms | 1.03 ms | 1.10 ms |
| Handoff: copy the frame | 0.07 ms | 0.29 ms | 1.49 ms |
| Handoff: base64 the full frame (raw RGBA transfer) | 1.24 ms | 4.95 ms | 6.25 ms |
| Handoff: base64 a changed region (770 x 230 logical) | 0.19 ms | 0.73 ms | 0.77 ms |
| Handoff: SHA-256 of the full frame, `sha2` crate | 9.28 ms | 36.99 ms | 37.59 ms |

For comparison, hashing the same 19.4 MB with the platform's hardware path (OpenSSL through Python, only as a reference) took 6.5 ms.

## What it says

- **Raster is not the bottleneck on this machine.** A full 2x repaint of a dense scene is 3.1 ms, a typical poll update 0.45 ms. Both fit the 8 ms target here, the full repaint with a margin of about 2.6 times.
- **That margin is for one fast machine.** A CPU several times slower, a virtual machine, or a busier scene would put a full 2x repaint past 8 ms. So the design rule stands: repaint only what changed. The 0.45 ms poll update is what a normal frame costs.
- **Hashing is the handoff cost that matters.** The image protocol requires a SHA-256 of each uploaded source. With the pure-software `sha2` crate a full 2x frame costs 37 ms, five times the whole frame budget; even a hardware path costs 6.5 ms, most of the budget. So uploads must be small changed-region tiles, prepared and hashed off the paint thread (the protocol amendment already requires preparation off the event loop), and the hash implementation must use the platform's hardware instructions. Raw transfer to a pixel-protocol terminal is cheaper: base64 of a changed region is 0.7 ms at 2x.
- **A glyph cache helps little at this size.** Cold and warm frames differ by 0.3 ms with 78 distinct glyphs. It matters more for large character sets and larger text.

## What it does not measure

- Presentation: no real terminal transfer and no native-host upload, so nothing here includes the cost of the backend showing the image, or compositor stalls.
- Text shaping. `fontdue` lays out glyph by glyph with no kerning or complex scripts, so real text will cost more and look different.
- Other machines. Run it on a typical laptop, a low-end CPU and a virtual machine before treating 8 ms as met.
- A multi-threaded or GPU path, sub-pixel glyph positioning, and live animation.

## Decisions this feeds

- Shapes and strokes can use `tiny-skia`, which is already in the dependency graph and allowed.
- Outline text needs a new crate such as `fontdue` (MIT) and a bundled open-licensed font, both of which need the dependency sign-off the product requirements ask for.
- The hash step is a design constraint for the image path, not an afterthought: tiles, a worker thread and hardware SHA-256.

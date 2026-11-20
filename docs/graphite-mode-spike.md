# Graphite mode spike: what a frame costs

This is the first measurement behind [Graphite mode option A](graphite-mode-design.md): lunatui rasterizes pixels itself and hands them to the backend as images. It answers one question before any API is frozen: on a real scene, how much of the 8 ms target do raster and handoff use?

The program is [`scripts/graphite-spike`](../scripts/graphite-spike/README.md), kept as text files and run from a crate outside the workspace. It draws the activity monitor mockup from the design note (header chips, three seat rows with lanes, a 96-bin lane, fifteen timeline rows and a key row) with `tiny-skia` 0.12 shapes and `fontdue` 0.9 text, then times each phase. The README there shows how to run it.

## Results

One machine: Apple M5 Max, 18 cores, single-threaded, release build, with other people's work using about half of the cores at the time (a load average near 10). Three consecutive runs agreed within 3 percent; the table is the last. Medians over 40 to 400 runs. The font is a system TTF used only for this measurement, not a font lunatui can ship.

| Phase | 1x (1480 x 818) | 2x (2960 x 1636) | 2x maximum |
|---|---:|---:|---:|
| Full frame, glyph cache warm | 0.99 ms | 2.82 ms | 3.10 ms |
| Full frame, glyph cache off | 1.12 ms | 3.07 ms | 4.56 ms |
| Typical poll update (rows and one timeline line) | 0.21 ms | 0.43 ms | 0.48 ms |
| Shapes only (the four lanes) | 0.52 ms | 0.92 ms | 1.09 ms |
| Handoff: copy the full frame | 0.06 ms | 0.26 ms | 2.66 ms |
| Handoff: base64 the full frame (raw RGBA transfer) | 1.12 ms | 4.42 ms | 5.75 ms |
| Handoff: SHA-256 of the full frame, `sha2` crate | 8.40 ms | 33.61 ms | 38.55 ms |
| Handoff: copy out the changed rectangle (770 x 230 logical) | 0.01 ms | 0.06 ms | 0.13 ms |
| Handoff: copy out and base64 the changed rectangle | 0.18 ms | 0.71 ms | 0.86 ms |
| Handoff: SHA-256 of the changed rectangle, `sha2` crate | 1.23 ms | 4.89 ms | 5.26 ms |

The changed rectangle is the real 770 by 230 logical area of the seat rows, copied out row by row: 0.71 MB at 1x and 2.83 MB at 2x, against 4.84 MB and 19.37 MB for the full frame.

For comparison, hashing the full 2x frame (19.4 MB) with the platform's hardware path (OpenSSL through Python, only as a reference) took 6.5 ms, about 3 GB/s.

## What it says

- **Raster is not the bottleneck on this machine.** A full 2x repaint of a dense scene is 2.8 ms, a typical poll update 0.43 ms. Both fit the 8 ms target here, the full repaint with a margin of nearly 3 times.
- **That margin is for one fast machine.** A CPU several times slower, a virtual machine, or a busier scene would put a full 2x repaint past 8 ms. So the design rule stands: repaint only what changed. The 0.43 ms poll update is what a normal frame costs.
- **Hashing is the handoff cost that matters.** The image protocol requires a SHA-256 of each uploaded source. With the pure-software `sha2` crate a full 2x frame costs 34 ms, four times the whole frame budget, and even the 2.83 MB changed rectangle costs 4.9 ms, more than half of it. A hardware path runs at about 3 GB/s, which would bring that rectangle to about 1 ms and the full frame to 6.5 ms. So the hash must use the platform's hardware instructions, run off the paint thread (the protocol amendment already requires preparation off the event loop), and cover only changed-region tiles. Raw transfer to a pixel-protocol terminal is cheaper: copying out and base64 of the changed rectangle is 0.7 ms at 2x.
- **A glyph cache helps little at this size.** Cold and warm frames differ by 0.25 ms with 78 distinct glyphs. It matters more for large character sets and larger text.

## What it does not measure

- Presentation: no real terminal transfer and no native-host upload, so nothing here includes the cost of the backend showing the image, or compositor stalls.
- Text shaping. `fontdue` lays out glyph by glyph with no kerning or complex scripts, so real text will cost more and look different.
- Other machines. Run it on a typical laptop, a low-end CPU and a virtual machine before treating 8 ms as met.
- A multi-threaded or GPU path, sub-pixel glyph positioning, and live animation.

## Decisions this feeds

- Shapes and strokes can use `tiny-skia`, which is already in the dependency graph and allowed.
- Outline text needs a new crate such as `fontdue` (MIT) and a bundled open-licensed font, both of which need the dependency sign-off the product requirements ask for.
- The hash step is a design constraint for the image path, not an afterthought: tiles, a worker thread and hardware SHA-256.

# Record a pixel widget in a real terminal

`scripts/record-pixels.sh` captures a real Kitty window with ffmpeg. It records the terminal framebuffer, including graphics, and derives the PNG and animated WebP directly from that video. It does not render a text recording or composite pictures into terminal cells.

Use Linux with `Xvfb`, `kitty`, `xdotool`, `ffmpeg`, Python 3, and a Rust toolchain. On Ubuntu, install the terminal tools with:

```sh
sudo apt-get install xvfb kitty xdotool ffmpeg fonts-dejavu-core libgl1-mesa-dri
```

Record the bundled full-resolution NASA image, switching Contain and Cover with real key input:

```sh
EXAMPLE=kitty_image FEATURES=image-presenter,image-decode \
WIDTH=100 HEIGHT=32 FONT_SIZE=16 FPS=30 SECONDS_TO_RECORD=8 \
KEYS=2:f,5:f scripts/record-pixels.sh
```

The script starts an isolated Xvfb display and a Kitty window, builds the example, waits for a real Kitty control response before letting the app probe it, records the window, and closes both processes when finished. `OUT` selects the output directory. `KEYS` is an optional comma-separated list of elapsed seconds and alphanumeric X11 key names. `CARGO_TARGET_DIR` can select an absolute Cargo build directory. Arguments after the script name are passed unchanged to the example and recorded in the receipt.

Inspect every frame before publishing. `capture.json` records the source commit, whether tracked files were clean, terminal and encoder versions, configured cell dimensions and font size, actual pixel dimensions, duration, distinct frame count, input sequence, and artifact hashes. Use that receipt when writing captions. The script rejects a capture whose dimensions differ from the terminal window, an unchanged scripted recording, or a WebP of 2 MB or larger. `terminal.mkv` keeps the original lossless framebuffer video.

With Pillow installed (`apt-get install python3-pil`), check a rectangle inside the widget for colored pixels in every displayed frame:

```sh
python3 scripts/check-pixel-recording.py target/pixel-capture/terminal.webp \
  --region 100,100,1000,600 --min-colored-pixels 10000 \
  --contact-sheet target/pixel-capture/contact-sheet.png
```

Choose a rectangle that includes the graph or image and excludes labels, borders, and legends. Inspect the diagnostic contact sheet as well as the pixel counts; a color threshold detects empty frames but does not prove that the drawing is correct. The checker reads the recording without changing or dropping frames. Its contact sheet is for review, not gallery media.

For a held-state demo, also compare every displayed frame with inspected, unmodified stills from settled states. This detects mixed transitions such as new scatter labels over the previous line plot, which still contains colored pixels. Inspect every reference against the matching cell view before using it.

For the 24-second Chart demo, extract its eight states directly from the lossless source:

```sh
reference_args=()
for second in 1 4 7 10 13 16 19 22; do
  ffmpeg -nostdin -loglevel error -y -ss "$second" \
    -i target/pixel-capture/terminal.mkv -frames:v 1 \
    "target/pixel-capture/state-$second.png"
  reference_args+=(--reference-frame "target/pixel-capture/state-$second.png")
done
python3 scripts/check-pixel-recording.py target/pixel-capture/terminal.webp \
  --region 780,400,300,200 --min-colored-pixels 100 \
  "${reference_args[@]}"
```

The references must have the same dimensions as the recording. A frame must match one reference exactly, including labels and pixels. Do not add an unexpected transition as a reference to make a check pass. The previous Chart capture fails this check at frames 4 and 7, where the title changed before the image.

A text recorder such as asciinema with agg captures the cell fallback. Use this screen-capture path when the gallery must show actual image pixels. Recordings and captions must identify the path actually exercised.

The committed Image recording was captured from code commit `17012b45cb1a24e0372f78b34945c4dcf64d411b` in a fresh Ubuntu arm64 Tart VM with Kitty 0.32.2 and a clean tracked tree. The actual window is 1301×801 pixels at 30 fps. Its three displayed WebP frames show Contain, Cover, then Contain; all 240 lossless video frames retain the image. The lossless WebP is 1,580,600 bytes. Both the README and site use that uncomposited recording.

The corrected Chart recording comes from clean code commit `757abcef18301fcbe72dfbd17c1bf57ce1d1ad98`, with Kitty 0.32.2, 100×32 cells, a 16-point font, and a 1301×801 pixel window. It is 24 seconds at 30 fps, with an 81,152-byte lossless WebP. All 720 raw frames and all eight displayed WebP frames exactly match one of the inspected settled states. The previous capture fails the reference-state check at frames 4 and 7. Requests and Saturation stills remain pixel-identical; the Latency scatter still is an unmodified video frame at 13 seconds.

Record the six Canvas twins and exercise cells-to-pixels restoration, holding each state for about three seconds:

```sh
EXAMPLE=pixel_canvas FEATURES=image-presenter \
WIDTH=120 HEIGHT=36 FONT_SIZE=16 FPS=30 SECONDS_TO_RECORD=24 \
KEYS=3:Right,6:Right,9:Right,12:Right,15:Right,18:b,21:b scripts/record-pixels.sh
python3 scripts/check-pixel-recording.py target/pixel-capture/terminal.webp \
  --region 1000,300,480,350 --min-colored-pixels 100 \
  --contact-sheet target/pixel-capture/contact-sheet.png
```

The committed Canvas recording comes from clean code commit `2c45e6a1d57b4c0ca5cce5ed89b95ac90e8cdfc7`, with Kitty 0.32.2, a 120×36 cell configuration, 16-point font, and an actual 1561×901 pixel window. All 720 source frames and eight displayed WebP frames retain graph pixels. The lossless WebP is 292,106 bytes. A previous capture exposed a 166 ms blank when returning from cells to pixels; the same rectangle check rejects that frame. The runtime now uploads Kitty data before changing cells, then presents cells and placements in a new synchronized frame.

## Graphite pixel frames

`examples/graphite_pixels.rs` presents a frame that Graphite's CPU pixel buffer drew, a command palette over a status bar, through the same `Image` and `PixelLayer` path as any other picture. The frame is committed as 320×96 straight-alpha RGBA8 bytes in `examples/data/graphite-status-palette.rgba`, so the example needs no Graphite dependency.

`cargo test -p lunatui-bench --features graphite-pixels --test graphite_pixels` links `graphite-core` (a git dependency pinned to a commit, only in the unpublished bench crate and off by default) and checks three things:

- the committed bytes equal what Graphite draws now (rerun with `GRAPHITE_PIXELS_BLESS=1` to rewrite the file after a deliberate change);
- the Kitty stream the `PixelLayer` emits for that frame decodes back to exactly the same 122,880 bytes, with one transmit command (`f=32`, `s=320,v=96`);
- the half-block cell fallback of the same frame has every pixel pair in the right cell colors, 15,360 cells checked.

To capture the example in a real Kitty window on Linux:

```sh
EXAMPLE=graphite_pixels FEATURES=image-presenter \
WIDTH=100 HEIGHT=32 FONT_SIZE=16 FPS=30 SECONDS_TO_RECORD=4 scripts/record-pixels.sh
```

The terminal scales the picture to its cells, so compare colors inside the large flat areas (the status bar and the palette background), not edge pixels.

# Record a pixel widget in a real terminal

`scripts/record-pixels.sh` captures a real Kitty window with ffmpeg. It records the terminal framebuffer, including graphics, and derives the PNG and animated WebP directly from that video. It does not render a text recording or composite pictures into terminal cells.

Use Linux with `Xvfb`, `kitty`, `xdotool`, `ffmpeg`, Python 3, and a Rust toolchain. On Ubuntu, install the terminal tools with:

```sh
sudo apt-get install xvfb kitty xdotool ffmpeg fonts-dejavu-core libgl1-mesa-dri
```

Record the bundled full-resolution NASA image, switching Contain and Cover with real key input:

```sh
EXAMPLE=kitty_image FEATURES=image-presenter,image-decode \
WIDTH=100 HEIGHT=32 FONT_SIZE=20 FPS=30 SECONDS_TO_RECORD=8 \
KEYS=2:f,5:f scripts/record-pixels.sh
```

The script starts an isolated Xvfb display and a Kitty window, builds the example, waits for a real Kitty control response before letting the app probe it, records the window, and closes both processes when finished. `OUT` selects the output directory. `KEYS` is an optional comma-separated list of elapsed seconds and alphanumeric X11 key names. `CARGO_TARGET_DIR` can select an absolute Cargo build directory. Arguments after the script name are passed unchanged to the example and recorded in the receipt.

Inspect `terminal.png` and `terminal.webp` before publishing. `capture.json` records the source commit, whether tracked files were clean, terminal and encoder versions, dimensions, duration, distinct frame count, input sequence, and artifact hashes. The script rejects a capture whose dimensions differ from the terminal window, an unchanged scripted recording, or a WebP above the 3 MB gallery limit. `terminal.mkv` keeps the original lossless framebuffer video.

A text recorder such as asciinema with agg captures the cell fallback. Use this screen-capture path when the gallery must show actual image pixels. Recordings and captions must identify the path actually exercised.

The committed Image recording was captured from code commit `ae377d67c9a437a150f632c05e360a05c49dfccc` in a fresh Ubuntu arm64 VM with Kitty 0.32.2. The tracked tree was clean. The actual window is 1601×1025 pixels; three distinct frames show the Contain/Cover changes. The lossless WebP is 2,313,174 bytes. Both the README and site use that uncomposited recording.

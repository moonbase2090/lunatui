# Kitty Image

Run from the repository root in a Kitty terminal:

```sh
cargo run --release -p lunatui --features image-presenter,image-decode --example kitty_image
```

The 100×32 app probes graphics support on startup. Kitty receives the real image through its graphics protocol; other terminals show the same `Image` widget through its cell renderer. Press `q` to exit. For a screen recording, use 100 columns × 32 rows and capture about 8 seconds.

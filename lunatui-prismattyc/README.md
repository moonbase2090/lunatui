# lunatui-prismattyc

Prismattyc backend for lunatui (PRD §2.12).

## In-process grid sink (PR 52)

[`crate::PrismattycBackend`] holds the previous frame and calls a [`crate::HostRenderer`] with a [`crate::FrameDiff`]: the packed [`lunatui_core::Grid`], grapheme [`lunatui_core::ClusterTable`], and [`lunatui_core::ChangedRun`] list from [`lunatui_core::diff_runs`]. No ANSI is emitted (`crate::PresentStats::ansi_bytes` is always zero).

Prismattyc chrome can clone or patch from `frame.next` and `frame.runs` directly.

## Native discovery (PR 53)

[`crate::discover_from_input`] / [`crate::DiscoveryDecoder`] parse Prismattyc capability 0.4 replies for `native.grid.v1` and `native.images.v1`. [`crate::DiscoveryResult::ring_minor`] selects ABI 1.2 for native images, including an images-only host; otherwise it selects grid ABI 1.1 with alpha or ABI 1.0. Discovery is off by default ([`crate::NativePolicy::Disabled`]) and requires [`crate::NativePolicy::DirectLocal`]. The [`crate::native_image`] module implements direct local image sessions: it validates peer identity and ancillary descriptors, maps the shared ring, uploads immutable image sources, and places only host-ready resources. Runtime callers use its sink and session to publish complete grid plus placement snapshots.

```rust,no_run
use lunatui_core::{ClusterTable, Grid};
use lunatui_prismattyc::{HostRenderer, PrismattycBackend};

struct Chrome;

impl HostRenderer for Chrome {
    fn present_frame(&mut self, frame: &lunatui_prismattyc::FrameDiff<'_>) -> std::io::Result<()> {
        let _ = (frame.next, frame.runs);
        Ok(())
    }
}

let width = 80_u16;
let height = 24_u16;
let grid = Grid::new(width, height);
let clusters = ClusterTable::new();
let mut backend = PrismattycBackend::new(width, height, Chrome);
let _stats = match backend.present(&grid, &clusters) {
    Ok(stats) => stats,
    Err(err) => panic!("present: {err}"),
};
```

## Native frame encoder (ring ABI 1.1 attributes)

[`crate::FrameEncoder`] writes complete native grid frames for the shared-memory ring. It starts with alpha and sub-cell attributes stripped, so every frame is a plain ABI 1.0 frame. [`crate::FrameEncoder::offer_alpha`] records whether discovery saw `native.alpha.v1` ([`crate::DiscoveryResult::alpha_offered`]) and [`crate::FrameEncoder::hello_ring_minor`] gives the minor to send in HELLO (0 unless offered). [`crate::FrameEncoder::affirm_alpha`] turns attributes on only after an offer and for an ACTIVE reply whose flags are exactly [`crate::ACTIVE_FLAG_ALPHA`]. A failure goes through [`crate::FrameEncoder::fail`], which ends native output and returns the ANSI repaint prefix.

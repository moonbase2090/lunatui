# lunatui-prismattyc

Prismattyc backend for lunatui (PRD §2.12).

## In-process grid sink (PR 52)

[`PrismattycBackend`] holds the previous frame and calls a [`HostRenderer`] with a [`FrameDiff`]: the packed [`lunatui_core::Grid`], grapheme [`lunatui_core::ClusterTable`], and [`lunatui_core::ChangedRun`] list from [`lunatui_core::diff_runs`]. No ANSI is emitted (`PresentStats::ansi_bytes` is always zero).

Prismattyc chrome can clone or patch from `frame.next` and `frame.runs` directly.

## Native discovery (PR 53)

[`discover_from_input`] / [`DiscoveryDecoder`] parse Prismattyc capability 0.4 replies for `native.grid.v1`. Discovery is off by default ([`NativePolicy::Disabled`]) and requires [`NativePolicy::DirectLocal`]. An affirmative advertisement never activates native ownership: [`DiscoveryResult::uses_ansi`] stays true and [`DiscoveryResult::native_active`] stays false. Shared-memory setup is not implemented in this cut.

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

[`FrameEncoder`] writes complete native grid frames for the shared-memory ring. It starts with alpha and sub-cell attributes stripped, so every frame is a plain ABI 1.0 frame. [`FrameEncoder::offer_alpha`] records whether discovery saw `native.alpha.v1` ([`DiscoveryResult::alpha_offered`]) and [`FrameEncoder::hello_ring_minor`] gives the minor to send in HELLO (0 unless offered). [`FrameEncoder::affirm_alpha`] turns attributes on only after an offer and for an ACTIVE reply whose flags are exactly [`ACTIVE_FLAG_ALPHA`]. A failure goes through [`FrameEncoder::fail`], which ends native output and returns the ANSI repaint prefix. There is no transport yet: nothing in this crate maps memory or talks to a host, and rendering stays ANSI.

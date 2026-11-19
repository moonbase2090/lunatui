# ADR 0010: Native image resources and rearmable sessions

## Status

Proposed. The [ABI 1.2 reference](../native-image-protocol.md) defines the wire
contract. This decision does not enable a runtime feature or change the accepted
ABI 1.0 and 1.1 encodings.

## Problem

The native grid contract carries cells but has no image resources or placements.
Lunatui's `ImageSink` accepts immutable RGBA sources up to 64 MiB, source
identities, fit modes, depth, and opacity. A host needs those semantics before a
presenter can suppress fallback cells.

Two lifecycle constraints also need an explicit amendment. A resized PTY cannot
receive a newly inherited descriptor without relaunching its child. Darwin POSIX
shared memory has a fixed, kernel-rounded extent and a distinct descriptor type,
which cannot satisfy the original regular-file and exact-length checks. Silently
relaxing those checks would change an existing ABI.

## Alternatives

| Design | Consequence |
|---|---|
| Authority owned by the desktop beside the local PTY | Activation and overlay installation share one owner; daemon-backed views retain ANSI |
| Daemon-owned authority with a display-route lease | Supports ordinary attached sessions, but needs installation receipts, removal fencing, route generations, and recovery across two processes |
| Image bytes in small control records | Requires thousands of request round trips for a large source and can delay lease traffic |
| Shared image resources transferred by descriptor | Bounded control work; a worker copies and validates content before rendering |
| Fixed smaller source limit | Rejects sources accepted by `ImageSource` and creates new fallback differences |
| Independent native renderer | Duplicates text rendering, selection, accessibility, and presentation ownership |

## Decision

The first host implementation owns authority beside a direct local PTY. A separate
display interface installs immutable validated scenes and acknowledges installation
before ACTIVE. This keeps a future daemon route possible without adding it to the
first transport. Daemon-backed, log-backed, remote, and nested-mux panes use ANSI.
The existing CPU raster and presentation backends render native scenes.

ABI 1.2 adds explicitly negotiated image resources and placements. It preserves
all earlier wire bytes and validators. Images do not require cell alpha or
sub-cell attributes. A host advertises `native.alpha.v1` only when it actually
renders those attributes.

The inherited socketpair lasts for one PTY launch. Each explicit fresh offer
creates a new session, generation, metrics epoch, and ring. Resize and ownership
loss revoke old authority before further PTY parsing. A new application starts
with HELLO sequence 1 and needs no counter from its predecessor. Bounded draining
and absolute deadlines make stale holders fail to ANSI instead of hanging.

Image uploads transfer one descriptor with declared geometry and SHA-256. The
host admits work without copying on the UI thread. A bounded worker copies
shared bytes into owned storage, hashes that copy, and reports completion through
heartbeat responses. Late completion cannot revive a revoked session. A matching
READY resource is required before `ImageSink::submit` can suppress fallback.

The per-resource limit matches `ImageSource` at 64 MiB. Session and process
budgets still permit an observable admission decline, with sticky client backoff.
Owned storage, received backing, cancelled workers, and scene references remain
charged until their actual cleanup.

Placements carry explicit source crops and physical destination rectangles.
Lunatui computes fit geometry once. Hosts use nearest-neighbor sampling and the
specified integer straight-alpha composition. Pixel goldens cover opaque and
translucent destinations, clipping, ordering, motion, and omission. Ring
consumption remains distinct from scene installation and physical presentation.
Retirement can return busy until an omitting frame is consumed and installed.

Platform backing kinds are explicit. Linux verifies extent seals and requires
write sealing for image content. Darwin validates allocated POSIX shared memory
through a typed kernel query and checks its reported extent. Its upload contents
remain mutable, so only the host-owned copy reaches rasterization. Darwin needs
minor 2 even for applications without images. Translated execution stays
unavailable until its syscall proof passes.

A shared process-launch boundary closes Darwin's non-atomic descriptor/CLOEXEC
window. Every actual spawn or exec in the host's dependency closure uses that
boundary. A type-resolved bypass check, dependency audit, and deterministic race
proof establish this invariant. The lock covers acquisition and spawn, never
waiting for a running child.

## Compatibility and rollout

The single host flag gates both discovery and rendering and remains off by
default under the existing owner requirement. The client still requires
`NativePolicy::DirectLocal`. Rollout uses a release candidate and an owner trial;
a separate reviewed change enables the feature by default.

Missing support, failed setup, budget declines, unsupported metadata, stale
responses, or a broken endpoint preserve fallback. A failure after fallback cells
were suppressed requires a new application render with the ANSI sink before a
reset, clear/home, and full repaint. Retained suppressed cells are insufficient.

## Acceptance

The implementation supplies cross-language wire vectors and negative controls,
real fragmented descriptor transfer and cleanup, deterministic lease and worker
cancellation tests, and actual host activation, resize, release, suspension, and
ANSI recovery. Image proof compares exact pre-encoding pixels with the immediate `PixelLayer`
backend at the same physical metrics. Kitty and Sixel comparisons cover geometry,
crop, order, and omission because their filters and palettes can differ. The protocol reference defines limits and throughput targets.
Current-main performance and visual baselines precede implementation; acceptance
reports measurement noise as well as the result.

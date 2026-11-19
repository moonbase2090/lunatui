# Native image protocol, ring ABI 1.2

Status: proposed. This amendment extends the [native grid protocol](native-protocol.md). It defines the host and client contract before either transport implementation ships. [ADR 0010](adr/0010-native-image-sessions.md) records the decision and alternatives. [Literal vectors](native-image-vectors.md) fix representative control, frame, and pixel bytes.

## Scope and negotiation

The first implementation supports direct local PTYs owned by the desktop only. Daemon-backed, log-backed, remote and nested-mux panes retain ANSI. Authority manages offers, sessions, resources and leases. Display installation accepts an immutable validated scene and returns a local installation receipt. Both run under one desktop event-loop owner; this amendment defines no daemon transport, route or persisted log event.

Frozen capability profiles 0.1 through 0.3, ABI 1.0, and accepted ABI 1.1 retain their bytes and validators. Profile 0.4 may advertise `native.grid.v1` and `native.images.v1` when this path is ready. `native.alpha.v1` is independent and requires implemented cell-alpha/subcell rendering; current host Style/raster support does not establish that readiness. Explicit `NativePolicy::DirectLocal` and the host's single default-off flag remain required. Host activation follows a release candidate and an owner trial before stable use. Enabling discovery and rendering by default requires a separate review.

HELLO=1 remains 56 bytes. Minor 2 requires grid and images tokens and selects SETUP_EXT=9. Only minor-2 HELLO permits header flag bit0, requesting attributes when alpha was offered; all other bits are zero. ACTIVE=4 must affirm images bit1, and sets attributes bit0 exactly when selected and supported. Thus ACTIVE flags are 2 or 3; unknown bits or missing requested affirmation fail this session. Without alpha selection, attribute count/offset and cell attribute references are zero. Minor 0/1 negotiation, flags and SETUP=2 remain unchanged. The first ACTIVATE frame has plain cells and image fallback cells, no attributes or placements. Extensions begin after matching ACTIVE, following actual scene installation.

## Control records

The existing 32-byte little-endian header remains: total bytes as u32 at 0, opcode as u16 at 4, flags as u16 at 6, session ID as u64 at 8, request sequence as u64 at 16, and PTY generation as u64 at 24. The PTY generation is the authority epoch, not an image source generation or metrics epoch. The 4096-byte maximum, one outstanding request, credential checks, and echoed responses remain. Sequences increase without wrap. A fresh one-use offer authorizes HELLO with session zero and sequence 1, independent of any previous application's counter. SETUP_EXT echoes sequence 1 and allocates a nonzero session ID. Session IDs increase across epochs on the same launch endpoint, without reuse or wrap. ACTIVATE is sequence 2; every later request increments that session's sequence by one. Each response echoes its request's session, sequence, and generation, except SETUP_EXT which replaces session zero with the allocated ID. Subsequent records bind that session and PTY generation. All unnamed bytes/flags are zero except negotiated HELLO/ACTIVE flags.

Every receive uses `recvmsg` beginning at the first record byte, reads within that record, accumulates ancillary descriptors through fragmentation, sets CLOEXEC immediately, and rejects truncation or extra descriptors. Every rejection closes every received FD. No ordinary buffered reader may consume this stream.

| Request | Total bytes | Reply | Total bytes | FDs |
|---|---:|---|---:|---|
| HELLO=1 | 56 | SETUP_EXT=9 | 128 | Reply exactly one ring FD |
| ACTIVATE=3 | 40 | ACTIVE=4 | 40 | None |
| RELEASE=5 | 32 | RELEASED=6 | 32 | None |
| PING=7 | 32 | PONG_EXT=15 | 48 | None |
| PUT_IMAGE=10 | 112 | IMAGE_RESULT=11 | 48 | Request exactly one image FD |
| RETIRE_IMAGE=12 | 48 | IMAGE_RETIRED=13 | 48 | None |
| Well-framed older-session request | Opcode length | SESSION_ENDED=14 | 48 | Close received upload FD |

Only SETUP_EXT and PUT_IMAGE carry descriptors on minor 2. The legacy SETUP-only restriction remains unchanged on minors 0 and 1.

HELLO is legal only without a current session and with a fresh one-use offer. After SETUP_EXT, only ACTIVATE and RELEASE are legal. PUT_IMAGE, RETIRE_IMAGE, and PING are legal only after matching ACTIVE. A request in the wrong state is fatal. RELEASE ends the session; it does not authorize further requests without fresh discovery and HELLO.

SETUP_EXT offsets are absolute:

| Offset | Type | Value |
|---|---|---|
| 32,40 | u64 each | Logical ring bytes=4198400, physical backing bytes |
| 48,52 | u32 each | Slots=4, stride=1048576 |
| 56,58 | u16 each | Backing kind, image format=1 |
| 60 | u32 | Zero |
| 64 | u64 | Nonzero metrics epoch |
| 72,76 | u32 each | Physical cell width and height, each 1..4096 |
| 80,84 | u32 each | Guest pixel width and height, exactly columns*cell_width and rows*cell_height |
| 88,96 | u64 each | Per-resource limit=67108864, total live-source budget=134217728 |
| 104,108 | u32 each | Maximum live resources=256, placements=256 |
| 112 | 16 bytes | Zero |

PUT_IMAGE offsets:

| Offset | Type | Value |
|---|---|---|
| 32 | u32 | Nonzero application image ID |
| 36,40 | u32 each | Source width and height |
| 44,46 | u16 each | Format=1, backing kind |
| 48 | u64 | Source generation, zero valid |
| 56,64 | u64 each | Logical pixel bytes, physical backing bytes |
| 72 | 32 bytes | SHA-256 of exactly the logical pixel bytes |
| 104 | 8 bytes | Zero |

Format 1 is packed, top-down, straight-alpha RGBA8 with byte-encoded sRGB channels. Checked `width*height*4` equals logical bytes, at most 64 MiB. Each nonzero axis is at most 16777216; the byte-product bound is authoritative.

IMAGE_RESULT is `{u64 handle@32, u32 status@40, u32 zero@44}`. Status 0 means admitted PENDING, 1 existing READY, 2 byte-budget decline, 3 handle-budget decline, 4 busy, 5 allocation decline. Declines have handle zero and preserve the session. Admission returns a nonzero never-reused handle after bounded metadata/FD checks and credit reservation, without copying pixels on the UI thread.

PONG_EXT is `{u64 handle@32, u32 state@40, u32 error@44}`. States are 0 idle with handle zero, 1 pending, 2 READY, 3 failed. Errors are 0 none, 1 job deadline, 2 allocation, 3 read failure. Pending/READY require error zero. It reports the single upload job; terminal status persists until the next newly admitted job or retirement of that handle. READY can then be referenced. No extra polling RPC or unsolicited message exists. Legacy minors receive the exact 32-byte PONG=8.

RETIRE_IMAGE body is `{u64 handle@32, u64 omitting_frame@40}`. IMAGE_RETIRED contains `{u64 handle@32, u32 status@40, u32 zero@44}`. Status 0 means retired; status 1 means busy because the omission is not yet consumed and installed, or an owned reference still exists. A busy reply preserves the handle and session. The client retries after its next PING and never publishes another reference once it first requests retirement. The host replies within the ordinary 200 ms operation bound, rather than waiting indefinitely for installation. Unknown handles are protocol failures. SESSION_ENDED echoes the rejected request header identity and sequence; its body is `{u64 current_epoch@32, u32 reason@40, u32 zero@44}`. Reasons are 1 resize, 2 PTY output, 3 lease expiry, 4 release. It grants no new authority. RETIRE accepts READY handles only. Legacy bodies are unchanged.

## Backing validation

Kinds are 1=`LinuxSealed`, 2=`DarwinFixedPosix`. The receiving platform accepts only its supported kind. Linux requires a regular shared object with exact physical=logical length and verified grow/shrink seals; image uploads also require a write seal. Ring backing remains writable. Invalid FD classes fail before mapping.

Darwin requires exclusively owned descriptor validation through an exact-size successful `proc_pidfdinfo(getpid(), fd, PROC_PIDFDPSHMINFO, ...)` result. Its typed PSHM UID must equal effective UID, and typed size, `fstat` size and declared physical extent must agree. Mode zero alone proves nothing. An initialized, unlinked PSHM object must have the documented fixed-length, no-retruncate guarantee. Missing, denied or short typed queries fail closed.

The Darwin extent rule is `logical <= physical < logical+65536`, with checked arithmetic. Physical is the actual kernel-reported allocation extent, not a caller-page-size calculation. Mappings request only the logical length. The protocol never reads, hashes, or exposes the rounded suffix. This bounded-suffix rule deliberately avoids assuming Rosetta `getpagesize()` equals kernel allocation granularity. Darwin eligibility requires real SCM_RIGHTS transfer into an exec receiver, including rejection of sockets, pipes, regular files, unallocated PSHM, and forged extents. Translated execution stays unavailable until the same syscall proof passes under Rosetta. Native arm64 evidence cannot establish translated execution. Darwin clients must negotiate minor 2 with grid and images support even when an application uses no images; minors 0 and 1 remain unavailable there. ABI 1.0/1.1 retain exact-length and regular-object checks; this PSHM object cannot satisfy them.

## Ring and frame extension

The ring retains its header, slots, logical size, counters and acquire/release rules. Header minor at 10 is 2; mapping bytes at 16 remain logical bytes. Consumption means all shared-slot reads ended. It does not mean installation or physical presentation.

The 128-byte frame header retains existing fields and amends these reserved bytes:

| Offset | Type | Value |
|---|---|---|
| 76,80 | u32 each | ABI 1.1 attribute count and offset |
| 84,88 | u32 each | Placement count 0..256 and offset |
| 92 | u32 | Zero |
| 96 | u64 | Exact SETUP_EXT metrics epoch |
| 104 | 24 bytes | Zero |

Canonical order is cells at 128, clusters, UTF-8, damage, attributes, placements. Each section begins at `align8(previous_end)`. Empty attribute and placement sections have offset zero; later offsets derive from the actual preceding section end. Minor-2 attributes follow all damage records; minor 1 is unchanged. Used bytes equal the final section end rounded to 8. Padding and unused slot bytes are zero. Existing cell, cluster, damage, geometry and 32768-cell limits remain; the complete frame must fit one slot.

Each placement is exactly 64 bytes:

| Offset | Type | Value |
|---|---|---|
| 0,4 | u32, i32 | Nonzero placement ID, depth |
| 8 | u64 | READY resource handle |
| 16,20,24,28 | u32 each | Source crop x,y,width,height |
| 32,36 | i32 each | Destination x,y in guest physical pixels |
| 40,44 | u32 each | Destination width,height |
| 48 | u8 | Opacity |
| 49 | 3 bytes | Zero |
| 52 | u32 | Submission ordinal, exactly array index |
| 56 | 8 bytes | Zero |

Placement IDs are frame-unique. Crops are nonempty and within their source. Destination axes and endpoints fit [-1073741824,1073741824], dimensions are nonzero and at most 1073741824. Validate arithmetic in wider integers. Every frame contains the complete placement set; omission removes a placement upon scene installation. Frames referencing unready, retired or foreign handles fail.

Lunatui computes `ImageGeometry::fit` for Contain, Cover, Stretch and Native using negotiated physical metrics, then sends exact rectangles. The host never repeats fit rounding. Sampling uses `src_origin+floor(destination_relative*src_size/dst_size)` from the original, unclipped destination. Clip to guest, pane and framebuffer without moving sampling coordinates. Paint cell backgrounds, negative-depth images, glyph/cursor ink, nonnegative-depth images, then host chrome. Images sort by depth then ordinal. Sampling is nearest-neighbor. This is a frozen ABI choice, including downscaling. It matches the sampling formula in `lunatui-term/src/pixel/immediate.rs`; exact pixel parity targets the immediate image backend before terminal encoding. Kitty terminals choose their own placement scaling filter, and Sixel can quantize colors. Those backends are compared for fit geometry, crop, order, and omission rather than exact displayed pixels.

Sources and the CPU framebuffer use straight alpha. Images compose in byte-encoded sRGB, without linear-light conversion. For each channel, let `a=floor(source_alpha*opacity/255)` and `n=a*255+destination_alpha*(255-a)`. Then `out_alpha=floor((n+127)/255)`. If `n=0`, output RGB is zero. Otherwise `out_rgb=floor((source_rgb*a*255+destination_rgb*destination_alpha*(255-a)+floor(n/2))/n)`. All intermediates use checked wide integers. Over an opaque destination this reduces to `floor((source_rgb*a+destination_rgb*(255-a)+127)/255)`, matching PixelLayer's opaque immediate-image conversion. Presentation uses the existing host conversion to premultiplied pixels exactly once. Window transparency does not imply negotiated cell-alpha support. Golden comparisons distinguish pre-encoding RGBA from Sixel palette quantization and from desktop composition.

Any scene, omission or metrics change increments native damage and repaints the full pane initially. Every content reader selects the same immutable native scene or retained classic emulator. No native cells enter the classic grid. Existing CPU raster and `PresentBackend` remain; presenters retain finished framebuffer storage, never source pixels or shared slots.

## Admission, identity and retirement

`ImageSource`'s 64 MiB constructor/decode limit is source acceptance; negotiated budgets govern display admission. The 64 MiB limit per resource and 128 MiB host-owned source bytes per session permit an old and replacement maximum-sized source. A simpler 8 MiB limit would reject valid larger sources or require an explicit pre-fit representation with separate identity and exact sampling parity. This revision preserves individual source capacity, including extreme aspect ratios.

One pending job per session reserves a resource slot and destination allocation. Another job requires the previous worker to exit. A process-wide, nonqueued pool permits at most two copy/hash workers, with 256 MiB image credits across all panes, sessions and cancelled jobs. Credits charge each host allocation once plus every retained upload FD's physical extent. Admission reserves both before retaining the FD; unavailable credits/worker permits decline immediately. Registry, staging and scene references remain charged; no silent eviction occurs. Slot copies remain bounded to four slots plus one installed grid.

A worker maps only logical bytes read-only and copies at most 64 KiB per step, checking cancellation and its deadline between steps. Darwin backing remains writable by its sender. Reads use pointer copies or volatile accesses, never a Rust shared slice over concurrently writable memory. After copying, the worker hashes the host-owned destination bytes, not the shared mapping, with cancellation and deadline checks between hash chunks. Its absolute job deadline is 2 seconds from admission. It owns the FD, mapping, allocation and credits until cleanup. A successful digest comparison yields owned immutable pixels, closes the FD, and posts completion tagged with launch/epoch/session/handle. Only the event-loop authority may change PENDING to READY after checking all tags, current PENDING state, and the admission deadline. No renderer reads upload backing. Digest mismatch is a protocol failure; allocation/read/deadline failures preserve fallback and report failed.

Revocation cancels without joining on the UI thread; late completion cannot install or revive resources. Overdue blocked work remains charged and occupies its worker permit until it exits. No replacement worker bypasses either bound. Unavailable permits can keep later admissions declined. The event loop reports deadline failure without awaiting worker exit.

Client backing preparation and SHA-256 also run outside the event loop. The client permits one preparation job per session, coalesces newer wanted generations, and keeps fallback while preparation runs. Cancellation releases its backing and buffers. Preparation cannot delay a heartbeat, and PUT begins only after the prepared descriptor and digest are complete.

While an upload is pending, the client schedules PING immediately after admission and at intervals of at most 20 ms after each completed response. This wake source runs without application input, preserves one outstanding request, and obeys the 100 ms PONG deadline. READY schedules an application render. A benchmark must sustain at least 30 sequential 64x64 RGBA source generations per second over five seconds on each supported reference host, including FD admission, copy, SHA-256, heartbeat reporting, and scene installation. It records actual results and hardware; this target is not a claim about 64 MiB uploads or every-frame changing sources. Deterministic tests use a controlled clock and completion signals. The benchmark measures real operation latency separately.

Admission declines are sticky for the same source key. The client does not recreate or send its backing again until an acknowledged retirement changes available credits, a new session starts, or an exponential backoff expires. Backoff starts at 250 ms and caps at 2 seconds; PENDING is never retried as PUT. The client stores at most 256 decline entries, coalesces wanted generations per application image ID, and never evicts an entry to bypass its backoff. Overflow remains fallback without an upload until a session reset or a retry credit becomes eligible. Per-session and process-wide credits are host admission limits, not guaranteed reservations for an application. A maximum Darwin source charges slightly more than 128 MiB while its backing and destination coexist, so the 256 MiB process pool may admit only one such upload at a time.

The live source key is `{image_id, source_generation}`. While live, dimensions and digest cannot change. Identical uploads return that same PENDING/READY handle without another allocation. Changed content requires another generation. Generation zero is valid. Failed jobs retain their terminal report until the next admitted PUT or session reset, and release the key binding after worker cleanup. Once their worker exits, failed jobs have no scene references and do not consume a live-resource slot. The next admitted PUT discards the old failed handle; handles are still never reused. Their source keys obey the same client backoff as admission declines. Retirement requires consumption through the omitting frame, a current installed scene omitting the handle, and no remaining pending or raster references. The client must publish no later reference. IMAGE_RETIRED means those references and the registry/key binding are gone. Reappearance after that acknowledgment uploads anew and gets a fresh handle; its content may differ. No retired-key tombstones accumulate. A RETIRE sent immediately after publishing its omission may receive busy because consumption and installation differ. Its next-PING retry can succeed only after both conditions and reference cleanup hold. Handle exhaustion ends the session before u64 wrap, rather than imposing a short animation lifetime.

`ImageSink::submit` returns true only for a READY resource whose placement is admitted. Pending uploads and declines return false before fallback cells disappear. A READY heartbeat schedules a new application render. The existing path is `PixelLayer` and borrowed `dyn ImageSink`. Failure after suppression reruns application rendering with the ANSI sink before reset, clear/home and a full repaint. It cannot reuse suppressed cells.

## Epochs, deadlines and proof

For minor 2, the inherited socketpair persists for one PTY launch. Resize, including metrics changes, release, lease expiry and any PTY byte revoke the overlay before geometry changes or byte parsing. Revocation cancels jobs, invalidates resources/ring/session/offers, and advances the nonwrapping PTY epoch. Fresh explicit discovery may obtain a new one-use offer, session, metrics epoch and ring. New descendant applications need no former sequence number. Legacy behavior remains unchanged; there is no automatic reconnect.

A structurally valid request carrying an older epoch and nonzero session receives SESSION_ENDED echoing its old header. This cannot renew a lease, advance current-session sequences or reactivate anything. At most four such records are serviced per rearm attempt within its 200 ms deadline. No historical session table is required. Invalid opcode/size/flags/ancillary counts, partial-record timeout, future epoch, or a wrong session in the current epoch are fatal. Stale HELLO offers never authorize setup.

Finish an already-started response before another record. Before/during fresh HELLO, the client may drain at most four complete older-epoch responses, at most 16384 bytes, closing all attached FDs. The total setup deadline remains 200 ms. It accepts only its matching fresh SETUP_EXT. A partially consumed stream cannot be resynchronized by scanning; malformed or undrainable remnants close the endpoint and require respawn. Delayed SETUP/ACTIVE responses cannot restore a revoked session. Concurrent clients remain unsupported. A stopped native application loses its lease even when its shell retains the endpoint. On `fg`, its old request receives SESSION_ENDED if framing remains valid; it stops publication and performs the full ANSI recovery. It cannot resume ownership from a late ACTIVE or READY. If a background holder steals another client's response, mismatched identity or the absolute operation deadline ends that attempt. Neither client scans the stream for resynchronization or waits indefinitely. Malformed, partial, stolen, or undrainable replies can make the endpoint unusable until PTY respawn. Every affected client falls back to ANSI; the host revokes the overlay before parsing that fallback output. Same-user processes intentionally consuming another client's stream are outside the identity trust model, but bounded failure remains required.

PTY bytes take priority over native work. Preserve 200 ms control/setup/backpressure bounds, PING within 150 ms, PONG_EXT within 100 ms and the validated-traffic 500 ms lease. PUT admission is short control work; copying/hashing never blocks PONG. The worker's separate deadline does not create a second outstanding control request.

Required proof includes immediate retirement before installation, hashing a mutated source into the owned copy, sticky decline/backoff behavior, Ctrl-Z followed by fg, a background holder stealing a reply, and cross-language byte goldens for all minors; real fragmented SCM_RIGHTS and FD cleanup; wrong types, epochs, digests and budgets; worker timeout/cancellation, stale completion, heartbeat responsiveness, descendant sequence restart, bounded drain; retirement/reappearance and long animation; actual direct-child installation before ACTIVE; lease expiry with retained shell FD; control-byte revocation; resize and fresh setup; and presented crop, opacity, depth, motion and omission pixels. Admission parity must drive real `PixelLayer` and native sinks with the same 64 MiB, over-budget and extreme-aspect sources, assert submit results/fallback cells, and inspect real presented pixels. A budget decline must be observable and preserve fallback; unexpected submit or fallback differences block acceptance. Exact image-byte parity targets the immediate backend; Kitty and Sixel comparisons assert geometry, crop, order, and omission without requiring identical filter or palette output. Preserve both flag states and calibrated no-regression evidence.

The implementation must include byte vectors for every new record and frame extension, deterministic lifecycle tests, and real host proof. A Darwin syscall probe establishes fixed backing extent, not immutable source content. The process-launch boundary must prevent accidental inheritance of native authority into unrelated children during non-atomic CLOEXEC windows, with a failing bypass check and deterministic race proof. Unsupported platforms and unproven execution modes retain ANSI.

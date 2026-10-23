# Native grid protocol proposal

Status: proposed, pending host protocol-owner acceptance. This document specifies proposed native discovery and a future local shared-memory transport. The transport, host capability registration and native renderer are not implemented by this change. The existing in-process HostRenderer/PrismattycBackend remains a separate API.

The existing host [capability envelope](https://github.com/moonbase2090/Prismattyc/blob/main/docs/capability-protocol.md) owns capability versions and identifiers. New behavior requires explicit opt-in and an affirmative host reply. No reply or any failed setup leaves ANSI available.

## Private discovery

Use the host-owned existing envelope verbatim:

```
ESC _ Prismattyc;cap;q;id=17;max=0.4 ESC \\
ESC _ Prismattyc;cap;r;id=17;v=0.4;features=native.grid.v1;native.offer=0123456789abcdef0123456789abcdef;native.generation=7 ESC \\
```

The proposed new registry profile is capability 0.4. These 0.4 examples are proposals, not current host replies. Existing capability 0.1, 0.2 and 0.3 replies must remain byte-for-byte unchanged. A current host may select its existing 0.3 profile and omit the unknown native feature.

`id` is a fresh nonzero u32 per probe. `native.generation` identifies the PTY authority generation. `native.offer` is a nonsecret, single-use 128-bit offer identifier, not a credential. Feature presence without an offer is still only advertisement; later setup requires an offer. Capability envelope 0.4 and ring ABI 1.0 are separate version spaces. The proposed 0.4 client parser accepts printable ASCII bodies of at most 4096 bytes, unknown keys and feature names, required fields in any order, and fragmented introducer/terminator bytes. It rejects duplicate required keys, zero/mismatched IDs, invalid versions, versions above 0.4 or outside major 0, empty feature elements, non-ASCII, BEL termination, and unsupported replies. Duplicate or invalid `native.offer` or `native.generation` is invalid when present. Reply `features` must exist, including an explicitly empty list only if the host contract later permits one. For this cut use the existing host rule rejecting empty feature elements, including an empty list. Any relevant malformed reply ends discovery unavailable. Unrelated APC namespaces are replayed; a valid wrong-ID reply is ignored for discovery and consumed as stale control traffic. No DA sentinel is required, so existing DA response ordering cannot truncate a later native reply. No new APC verbs, paths, socket names, FD numbers, or secrets are introduced. These client validation rules do not alter the frozen host parser or its accepted version range.

## Local authority and control

This section is a specification for later work. The host may advertise `native.grid.v1` only after it implements this contract and its rich-feature flag permits it. Authorization begins only after the host successfully writes the correlated capability reply to the PTY. There is no native feature in the current host.

The host creates a Unix `SOCK_STREAM` socketpair per PTY generation before spawning the child. It retains one endpoint and deliberately inherits exactly one endpoint into that child. `PRISMATTYC_NATIVE_FD` may identify the descriptor number, but its string value grants nothing. The launch contract must establish inheritance. The client checks the descriptor is a connected Unix socket, verifies peer credentials, and sends HELLO only on that endpoint. Socket metadata alone cannot prove the inheritance history. The host binds its retained endpoint to one PTY, generation, and capability offer. There is no pathname socket discovery or connection. An environment variable, terminal name, ordinary same-UID pathname socket, or advertised feature cannot substitute for that inherited authority.

Both peers verify kernel-reported effective UID against their own UID before accepting any message or ancillary FD. macOS uses `getpeereid`; Linux uses `SO_PEERCRED`. UID verifies the user only, not host identity, pane, or generation. The inherited endpoint plus the host's private PTY/generation record supplies the binding. Same-UID malicious processes remain outside this trust model, including processes able to inspect or inherit the authority. The child sets close-on-exec immediately and must explicitly arrange inheritance for a chosen descendant; no automatic forwarding across SSH or mux is allowed. The host rejects expired, reused, or wrong-generation offers even on an otherwise valid inherited endpoint.

Use length-prefixed records and `sendmsg`/`recvmsg` FD transfer on that socketpair. Control headers carry the host-selected nonzero PTY generation from the capability reply's `native.generation` key, parsed as a nonzero u64. A valid capability advertisement lacking this authority metadata remains only advertised support and cannot proceed to setup.

Each record has a 32-byte little-endian header: offset 0 u32 total bytes including header, 4 u16 opcode, 6 u16 flags=0, 8 u64 session ID, 16 u64 request sequence, 24 u64 generation. Size is 32..4096. Request sequence starts at 1 and increases without wrap. Exactly one outstanding request is allowed. Invalid lengths, flags, ordering, credentials, or ancillary truncation close the connection. Every received descriptor gets close-on-exec before further processing; unexpected descriptor counts close all received FDs.

Control messages have these directions and exact total lengths. One client request is outstanding at a time. A response echoes its request sequence. Sequence numbers start at 1 and never wrap.

| Request | Bytes | Body | Response | Bytes | Body |
|---|---:|---|---|---:|---|
| HELLO=1 | 56 | u32 capability ID, u16 ring ABI major=1, u16 ring ABI minor=0, offer[16] | SETUP=2 | 48 | u64 mapping bytes, u32 slots=4, u32 stride=1048576; exactly one FD |
| ACTIVATE=3 | 40 | u64 first committed frame sequence | ACTIVE=4 | 40 | same frame sequence |
| RELEASE=5 | 32 | none | RELEASED=6 | 32 | none |
| PING=7 | 32 | none | PONG=8 | 32 | none |

HELLO has session ID zero and the advertised nonzero PTY generation. The SETUP response header carries the newly assigned nonzero session ID, not zero. Every later message uses that ID and generation. The inherited endpoint's host-owned record must match the HELLO offer, capability ID and generation. Reject a repeated, expired or foreign offer. No message is a one-way notification, and the host sends no unsolicited request. The client duplicates its inherited FD with close-on-exec; the shell's original inherited endpoint does not authorize another active client.

Every control receive uses recvmsg from the first header byte. Read only within the current record, accumulate ancillary data across fragmented reads, and reject MSG_CTRUNC, extra descriptors, descriptors on any opcode except SETUP, and missing SETUP descriptors. Close every received FD on every rejection. Set close-on-exec immediately. Do not wrap the FD-bearing stream in an ordinary buffered reader that can consume ancillary-bearing bytes. The transport implements no pathname or TCP fallback. [Unix ancillary-data semantics](https://www.man7.org/linux/man-pages/man7/unix.7.html) define descriptor transfer.

The host creates and sizes an anonymous shared object, initializes its complete header, and retains its mapping before FD transfer. SETUP sends one regular shared-object FD, not a directory, device, socket, or executable. The client checks fstat type and exact file length, all immutable header values and checked offset arithmetic before accessing payloads. Neither side truncates or resizes a live object. File length checking alone does not prevent a peer from later truncating a mapping. On Linux the host seals against grow/shrink before transfer and the client verifies the seals. On macOS the supported implementation must establish and document an equivalent no-truncation lifetime guarantee under the same-user cooperative trust model; otherwise setup is unavailable. The receiver must never claim that fstat makes a mutable untrusted backing safe. Mapping creation and this platform proof belong to the later host implementation.

Setup and each operation have one absolute 200ms deadline. The client flushes any already queued ANSI frame before setup and publishes one full initial native snapshot. ACTIVATE names its committed frame sequence. The host validates and copies that complete frame, installs it as an overlay, and sends ACTIVE only after installation. It never replaces the retained classic PTY grid with native cells. The client suspends ANSI frame production before ACTIVATE and reports active only after matching ACTIVE. Discovery, peer UID, SETUP and frame publication alone cannot activate output. The ordinary ANSI previous grid remains unchanged during setup.

The active client sends PING at least every 150ms and receives PONG within 100ms. The shorter PING deadline ensures a missing PONG fails the session before another 150ms PING period is due; there is never a second outstanding request. The host renews a 500ms lease only for validated current-session control traffic. This is necessary because a parent shell may retain another reference to the inherited socket, so client death does not necessarily produce EOF. Expiry revokes native ownership and invalidates its session and offer. A stale client cannot resume by publishing more frames.

On normal release the client stops publication and sends RELEASE. The host discards native work and removes the overlay before RELEASED. On lost ACTIVE/RELEASED, setup rejection, malformed frame/control, backpressure timeout, or broken connection, the client closes its duplicate FD and emits an ANSI reset, clear/home and complete repaint. The host must atomically revoke native ownership before processing any subsequent PTY output byte for that pane, including control-only output. It then parses those bytes into its retained classic grid. This rule handles a delayed EOF, inherited duplicate FD or lost acknowledgment without dual display owners. Host-side errors likewise remove only the native overlay. No native content is copied into the classic grid.

## Shared-memory ring ABI 1.0

Only little-endian 64-bit targets with aligned lock-free interprocess u64 atomics are eligible for this ABI. All ordinary integers use explicit little-endian encoding. Never copy Rust structs or `Cell` memory into the mapping. Total mapping is 4,198,400 bytes, consisting of a 4096-byte header and four 1,048,576-byte slots. No variable slot allocation or wraparound record packing is needed.

Header layout:

| Offset | Bytes | Meaning |
|---|---|---|
| 0 | 8 | ASCII `LUNARING` |
| 8 | 2 | ABI major=1 |
| 10 | 2 | ABI minor=0 |
| 12 | 4 | Header bytes=4096 |
| 16 | 8 | Mapping bytes=4198400 |
| 24 | 4 | Slot count=4 |
| 28 | 4 | Slot stride=1048576 |
| 32 | 8 | Control session ID |
| 40 | 24 | Reserved zero |
| 64 | 8 | Client-owned atomic producer sequence |
| 72 | 56 | Reserved zero |
| 128 | 8 | Host-owned atomic consumer sequence |
| 136 | 3960 | Reserved zero |

Both counters initialize to zero before transfer. First reject consumer > producer. The producer writes frame sequence `producer+1` into slot `producer % 4` only when `producer-consumer < 4`. The producer finishes all ordinary writes, then release-stores producer+1. The consumer acquire-loads producer, reads slot `consumer % 4`, requires frame sequence `consumer+1` and the current generation, then copies and validates that committed slot into host-owned memory. The consumer release-stores consumer+1 only when it no longer reads slot bytes. Client acquire-loads consumer before reuse. Neither writes the other's counter. Counter distance >4, inconsistent slot sequence, or wrap exhaustion ends the session. Backpressure never overwrites unread slots. For later implementation the caller retains its latest logical grid, waits at most 200ms for capacity, then detaches to ANSI on timeout. The per-axis bounds are independent ceilings, not a guarantee that their product is supported. The product must be at most 32768 cells and the complete encoded frame must fit 1048576 bytes. For example, 4096 by 4096 is rejected. A frame exceeding any combined limit falls back rather than truncates.

Each slot has this 128-byte frame header. All unused slot bytes are zero for v1.0.

| Offset | Type | Meaning |
|---|---|---|
| 0 | u32 | Used slot bytes, 128..1048576 |
| 4 | u16 | Frame header bytes=128 |
| 6 | u16 | Flags, bit0 full snapshot; other bits zero |
| 8 | u64 | Frame sequence, equals committed sequence |
| 16 | u64 | PTY authority generation, equals current control generation |
| 24 | u32 | Columns, 1..4096 |
| 28 | u32 | Rows, 1..4096 |
| 32 | u32 | Cell count, exactly columns*rows and at most 32768 |
| 36 | u32 | Cells offset |
| 40 | u32 | Cluster record count, at most cell count |
| 44 | u32 | Cluster records offset |
| 48 | u32 | UTF-8 blob bytes, at most 262144 |
| 52 | u32 | UTF-8 blob offset |
| 56 | u32 | Damage record count, at most rows*2 and 8192 |
| 60 | u32 | Damage records offset |
| 64 | u32 | Cursor column |
| 68 | u32 | Cursor row |
| 72 | u32 | Cursor flags, bit0 visible, remaining zero |
| 76 | 52 bytes | Reserved zero |

For hidden cursors coordinates must be zero; visible coordinates must fit geometry. Arrays appear in this order: cells, cluster records, UTF-8 blob, damage records, each offset aligned to 8; padding bytes zero. Start cells at 128, and derive later offsets by round-up of the previous section end. Use checked arithmetic and reject overlap, noncanonical offsets, or out-of-slot ranges. All frames contain complete row-major cell snapshots, even when damage is small. Damage is a rendering hint, not a dependency on retained cluster IDs. This costs bandwidth and makes ring recovery, resize, and validation independent of previous slot contents.

Each 16-byte cell has {u32 glyph at 0, u32 fg at 4, u32 bg at 8, u32 meta at 12}. This is an explicit wire schema, not a commitment to Rust `Cell` layout. Glyph is a Unicode scalar, 0x00110000 continuation, or 0x80000000 | frame-local cluster index. Other bits in cluster references must be zero; indices must exist. Every width-two lead must have the next continuation in the same row; every continuation must follow a width-two lead. Scalar control characters and UTF-8 clusters containing terminal controls are rejected. Width comes from the frame, rather than host Unicode-width recomputation.

Color kind is bits30..31: 0 default with all other bits zero, 1 indexed with low8 palette index only, 2 RGB with low24 0xRRGGBB, 3 forbidden in ABI1.0. Bits24..29 must be zero. The client resolves theme tokens to supported colors before transport. Meta bits0 bold,1 dim,2 italic; bits3..5 underline 0 none,1 single,2 double,3 curly,4 dotted,5 dashed; bit6 blink,7 reverse,8 hidden,9 strike,10 overline; bit11 zero; bits12..13 width 0 continuation,1 single,2 double; bits14..31 zero. Frames with core skip/protected flags or extension-table entries need fallback until a host-approved encoding exists. Do not silently omit them. Underline 6/7 and width 3 are invalid.

Cluster records are 16 bytes: u32 offset into the frame UTF-8 blob at0, u32 byte length at4, u8 terminal width at8, seven reserved-zero bytes at9. Array index is the cluster ID. Entries contain one nonempty valid extended grapheme, at most 1024 UTF-8 bytes, width1 or2. A referring cell's width must agree. Copy only referenced clusters and remap IDs per frame. Blob slices must be packed in record order without gaps or overlaps. A full snapshot carries all referenced cluster strings, so recycling a Rust cluster ID cannot change an earlier frame.

Damage records are 16 bytes: u32 row at0, u32 starting column at4, u32 cell count at8, u32 reserved=0 at12. Records are sorted row/column, nonempty, nonoverlapping, within geometry, and expand to include complete wide glyphs. Full-snapshot flag implies whole-grid damage and zero damage records; non-full zero records means no cell damage, but cursor metadata can still change. Initial and resized frames require the full flag. The host validates and copies before rendering; it does not retain shared-memory pointers after acknowledging consumption.

## Resize and terminal ownership

Every frame carries a complete snapshot and a frame-local cluster dictionary. Damage is an optional rendering hint. No frame requires a prior slot's cluster IDs or incremental cell payload. The host validates the copied cells and geometry before rendering and acknowledges consumption only after it no longer references slot memory. Consumption is not a GPU presentation acknowledgment. Dropped intermediate frames cannot lose a delta chain because each later frame is complete.

Any PTY resize revokes the active native generation, invalidates offers, and returns the host to classic ownership. The client stops publishing, releases its session if possible, and forces a complete ANSI repaint at the new dimensions even when damage was empty. A later native attempt requires a fresh capability query, offer, session, mapping and full initial snapshot. This cut has no in-place mapping resize, counter reset, cross-generation slot reuse or automatic reconnect. Host and client reject frames whose geometry disagrees with the current PTY. A frame exceeding slot capacity or using unsupported cell metadata follows the same full-ANSI recovery path.

## Status and required agreement

This is a concrete proposal, not a registered host capability or implemented shared-memory transport. The current host advertises neither native.grid.v1 nor these authority keys. Existing frozen capability 0.1/0.2/0.3 replies and registries remain unchanged. The concrete recommendation is a new capability 0.4 profile, leaving frozen 0.1/0.2/0.3 registries and fixtures unchanged. The host protocol owner must accept and review that cut before native.grid.v1 can be advertised. The client accepts the proposed 0.4 envelope only as discovery and never treats it as a native rendering grant. An older profile remains unavailable for native setup even if an unregistered native-looking token appears in it. Ring ABI 1.0 does not change the capability major to 1.

Before any host implementation: agree the registry cut, inherited-FD convention and PTY-generation offer binding; verify actual macOS/Linux credential semantics; resolve no-truncation backing guarantees; and prove activation, lease, control-byte revocation and full-repaint recovery against a real host. Same-UID credentials on a socketpair can describe its creator rather than a later inheritor, so UID checks cannot replace the host's launch-time same-user identity check and private inherited-authority record. Same-user malicious delegation is outside this trust model. Unsupported credential platforms, muxes and remote hops remain ANSI.

## Client scope and fallback proof

The proposed client discovery API must be off by default, require explicit direct-local opt-in, use one dedicated 4096-byte APC decoder, correlate IDs, and own a 200ms total deadline. It must enter raw mode before sending and flushing the query, restore modes and FD flags on every exit, and preserve unrelated input for the existing application parser. It must run before any competing input reader and must not reuse the generic capability parser's 256-byte APC/DA-sentinel behavior. A terminal name and environment variables never authorize native setup.

Future discovery must report unavailable or advertised support only, never active ownership. Current rendering remains ANSI. Future client tests must keep ANSI after every discovery result, including a fabricated future affirmative advertisement. Tests must exercise silence, malformed/oversized/fragmented replies, wrong IDs and versions, absent/unknown features, disabled/indirect policy, input replay and the real ANSI encoder's literal output. These tests demonstrate client fallback, not host interoperability or shared-memory safety.

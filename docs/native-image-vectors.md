# Native image protocol vectors

These proposed ABI 1.2 specification literals accompany the [protocol reference](native-image-protocol.md). They define encoding expectations before a runtime implementation exists. The invalid cases describe required rejection behavior. Descriptor values and platform behavior require separate runtime proof.

## Fixed fixture context

Control header is `{u32 bytes@0,u16 opcode@4,u16 flags@6,u64 session@8,u64 request_sequence@16,u64 PTY_epoch@24}`, all LE. Session is 1, epoch 7. HELLO is the required exception, session 0 and sequence 1; capability ID 17, major 1/minor 2, offer bytes `0123456789abcdef0123456789abcdef`. Alpha is not selected. SETUP has metrics epoch 9, cell 8x16, guest 8x16, Darwin kind 2, logical ring length 4198400, physical 4210688. Exactly one ring FD accompanies SETUP_EXT; one image FD accompanies PUT_IMAGE. Descriptor values and SCM_RIGHTS bytes are not wire literals.

The source is one straight RGBA pixel `ff000080`, logical length 4 and Darwin physical extent 16384, image ID 1, source generation 0, eventual handle 1. SHA-256 of the four logical bytes is `c40b4ac0514f62ad2d8677e8f8bc53ed5a80582c5ea3afc395e2d0df6d8828a1`. The physical suffix is excluded. This allocation size is a fixture, not a userspace-page-size calculation or platform proof.

The valid successful trace is HELLO/SETUP sequence1, publish frame1, ACTIVATE/ACTIVE sequence2, idle PING/PONG sequence3, PUT/PENDING sequence4, PING/PENDING sequence5, PING/READY sequence6, publish placement frame2, publish omission frame3, RETIRE/BUSY sequence7, PING/READY sequence8, RETIRE/RETIRED sequence9. After resize to epoch8, old PING sequence10 receives SESSION_ENDED echoing session1/epoch7/sequence10 and current_epoch8. Request PING bodies are empty; their headers change sequence only. `pong_failed` is an alternative timeout branch at sequence6, not a step after READY. Repeated RETIRE at9 changes the request header sequence from7 to9 only.

Every frame is a complete 1x1 grid, full flag1, hidden cursor, metrics9, zero attributes/clusters/blob/damage. Cells start128; empty base-section offsets are144. Initial and omission glyph is `?`, placement glyph is space, foreground RGB white `0x80ffffff`, background RGB black `0x80000000`, meta width1 `0x1000`. Initial/omission used length144; placement frame208 has count1/offset144, placement ID1, handle1, depth0, crop0,0,1,1, destination0,0,8,16, opacity255, ordinal0. Only used slot bytes appear below; bytes through the 1 MiB slot boundary are implicit zero and must be checked by a production parser.

## Discovery selection fixtures

The minor-2 trace below accepts this native Darwin reply without a grid token:

```text
ESC _ Prismattyc;cap;r;id=17;v=0.4;features=native.images.v1;native.offer=0123456789abcdef0123456789abcdef;native.generation=7 ESC \
```

| Advertised features at profile 0.4 | Client implementation | Expected selection |
|---|---|---|
| `native.images.v1` | Minor 2 | Minor 2, SETUP_EXT, ACTIVE flags 2 |
| `native.images.v1` | Minors 0 and 1 only | ANSI before HELLO |
| `native.grid.v1,native.images.v1` | Minors 0 and 2 | Minor 2, SETUP_EXT, ACTIVE flags 2 |
| `native.grid.v1,native.images.v1` | Minors 0 and 1 only | Minor 0, legacy SETUP and ACTIVE |
| `native.grid.v1,native.alpha.v1` | Minors 0 and 1 | Minor 1, legacy SETUP and attribute affirmation |
| `native.alpha.v1` | Minors 0, 1 and 2 | ANSI before HELLO |

The first Linux host must serve minors 0 and 2 and list both transport tokens.
The first native Darwin host must serve only minor 2 and list only the images
token. Alpha is unavailable in both initial host profiles. These discovery
fixtures change no control-record or frame literal below.

## Wire literals

### hello

```hex
38 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
11 00 00 00 01 00 02 00 01 23 45 67 89 ab cd ef
01 23 45 67 89 ab cd ef
```

### setup_ext

```hex
80 00 00 00 09 00 00 00 01 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
00 10 40 00 00 00 00 00 00 40 40 00 00 00 00 00
04 00 00 00 00 00 10 00 02 00 01 00 00 00 00 00
09 00 00 00 00 00 00 00 08 00 00 00 10 00 00 00
08 00 00 00 10 00 00 00 00 00 00 04 00 00 00 00
00 00 00 08 00 00 00 00 00 01 00 00 00 01 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### activate

```hex
28 00 00 00 03 00 00 00 01 00 00 00 00 00 00 00
02 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00
```

### active_images

```hex
28 00 00 00 04 00 02 00 01 00 00 00 00 00 00 00
02 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00
```

### ping_idle

```hex
20 00 00 00 07 00 00 00 01 00 00 00 00 00 00 00
03 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
```

### pong_idle

```hex
30 00 00 00 0f 00 00 00 01 00 00 00 00 00 00 00
03 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### put_image

```hex
70 00 00 00 0a 00 00 00 01 00 00 00 00 00 00 00
04 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 01 00 00 00 01 00 00 00 01 00 02 00
00 00 00 00 00 00 00 00 04 00 00 00 00 00 00 00
00 40 00 00 00 00 00 00 c4 0b 4a c0 51 4f 62 ad
2d 86 77 e8 f8 bc 53 ed 5a 80 58 2c 5e a3 af c3
95 e2 d0 df 6d 88 28 a1 00 00 00 00 00 00 00 00
```

### image_pending

```hex
30 00 00 00 0b 00 00 00 01 00 00 00 00 00 00 00
04 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### pong_pending

```hex
30 00 00 00 0f 00 00 00 01 00 00 00 00 00 00 00
05 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00
```

### pong_ready

```hex
30 00 00 00 0f 00 00 00 01 00 00 00 00 00 00 00
06 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00
```

### pong_failed

```hex
30 00 00 00 0f 00 00 00 01 00 00 00 00 00 00 00
06 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 03 00 00 00 01 00 00 00
```

### retire_image

```hex
30 00 00 00 0c 00 00 00 01 00 00 00 00 00 00 00
07 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 03 00 00 00 00 00 00 00
```

### retire_busy

```hex
30 00 00 00 0d 00 00 00 01 00 00 00 00 00 00 00
07 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00
```

### image_retired

```hex
30 00 00 00 0d 00 00 00 01 00 00 00 00 00 00 00
09 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### session_ended

```hex
30 00 00 00 0e 00 00 00 01 00 00 00 00 00 00 00
0a 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00
08 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00
```

### frame_initial

```hex
90 00 00 00 80 00 01 00 01 00 00 00 00 00 00 00
07 00 00 00 00 00 00 00 01 00 00 00 01 00 00 00
01 00 00 00 80 00 00 00 00 00 00 00 90 00 00 00
00 00 00 00 90 00 00 00 00 00 00 00 90 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
09 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
3f 00 00 00 ff ff ff 80 00 00 00 80 00 10 00 00
```

### frame_placement

```hex
d0 00 00 00 80 00 01 00 02 00 00 00 00 00 00 00
07 00 00 00 00 00 00 00 01 00 00 00 01 00 00 00
01 00 00 00 80 00 00 00 00 00 00 00 90 00 00 00
00 00 00 00 90 00 00 00 00 00 00 00 90 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 01 00 00 00 90 00 00 00 00 00 00 00
09 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
20 00 00 00 ff ff ff 80 00 00 00 80 00 10 00 00
01 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 01 00 00 00 01 00 00 00
00 00 00 00 00 00 00 00 08 00 00 00 10 00 00 00
ff 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

### frame_omission

```hex
90 00 00 00 80 00 01 00 03 00 00 00 00 00 00 00
07 00 00 00 00 00 00 00 01 00 00 00 01 00 00 00
01 00 00 00 80 00 00 00 00 00 00 00 90 00 00 00
00 00 00 00 90 00 00 00 00 00 00 00 90 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
09 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
3f 00 00 00 ff ff ff 80 00 00 00 80 00 10 00 00
```

## Composition and sampling literals

The expected columns are reviewed byte literals. RGB/alpha ordering in this table is RGBA, independent of the CPU buffer's ARGB word representation.

| Source RGBA | Destination RGBA | Opacity | Expected straight RGBA |
|---|---|---:|---|
| ff000080 | 0000ffff | 255 | 80007fff |
| 0164ff80 | 000000ff | 255 | 013280ff |
| ff000080 | 0a141eff | 128 | 470f16ff |
| ff000080 | 0000ff80 | 255 | aa0055c0 |
| 07080900 | c8966400 | 255 | 00000000 |
| 0a141eff | c8966401 | 0 | c8966401 |

The second row distinguishes nearest RGB rounding from floor, which would give `003280ff`. The third distinguishes `470f16ff` from floor's `470e16ff`. For immediate transparent encoding, source `ff000080` at opacity128 becomes `ff000040`, with no RGB flattening. The nonopaque rows are native goldens; the immediate presenter does not itself composite against a translucent destination.

Nearest sampling of the 2x2 source `[red, green; blue, white]` into 3x3 gives `[red, red, green; red, red, green; blue, blue, white]`, using source index `floor(relative*2/3)` on each axis. If destination origin is x=-1 and clipping begins at x=0, visible columns retain relative indices1,2 and therefore sample source columns0,1. Clipping never restarts sampling at relative0.

## Negative invariants

Flipping each listed byte must cause rejection for this fixture. These cases are not a complete parser or platform proof.

| Vector | Offset | XOR | Violated invariant |
|---|---:|---:|---|
| hello | 38 | 01 | Offered minor must be2 |
| setup_ext | 0 | 01 | Declared size must equal128 |
| setup_ext | 112 | 01 | Reserved suffix is zero |
| active_images | 6 | 01 | Unselected alpha cannot be affirmed |
| put_image | 72 | 01 | Digest must match copied ff000080 |
| image_pending | 44 | 01 | Reserved word is zero |
| pong_idle | 32 | 01 | Idle handle must be zero |
| pong_ready | 44 | 01 | READY error must be zero |
| retire_busy | 40 | 02 | Retirement status must be0 or1 |
| session_ended | 44 | 01 | Reserved word is zero |
| frame_initial | 104 | 01 | Reserved header tail is zero |
| frame_placement | 168 | 01 | Source crop width cannot be zero |
| frame_placement | 96 | 01 | Metrics epoch must match9 |
| frame_placement | 88 | 08 | Placement offset must be canonical144 |

Additional invalid cases omit SETUP_EXT's required FD or attach two FDs to PUT_IMAGE. Actual FD class, credentials, copy ownership, asynchronous completion fencing, full-slot zero validation, and presented pixels require runtime acceptance proof.

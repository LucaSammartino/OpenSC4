# network-occupants: open questions
<!-- Questions about [network-occupants.md](../network-occupants.md). Append new entries at the END; when one is answered, move it to answered/network-occupants.md. Rules: README.md. -->

## From the network occupants spec (re-netocc lane, 2026-10-20)

Spec: [network-occupants.md](../network-occupants.md) §§1–10. RE-side follow-ups; nothing here blocks an
implementer of the class tables and records.

<!-- from: the network occupants spec (2026-10-20) -->
- **NO4 - the prebuilt record's first five fields:** the semantics of the prebuilt `Write` fields
  (three f32 at primary `+0x178`/`+0x17C`/`+0x180`, u32 `+0x1B0`, u8 `+0x1B4`; corrected by NO46,
  network-occupants.md §2.8.1): model refs? shear inputs?
  [not specified]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO6 - the pipe's +0x270 field:** the meaning of pipe `Read`'s extra u32 → object `+0x270` (code `+0x268`,
  network-occupants.md §3.3): funding state? watered state? (The byte `+0x260` the read sets is "refresh
  pending", §3.1, re-ugbase 2026-10-21.) [unverified]
<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- **NO10 - the pylon's QueryInterface IIDs:** §1.2's pylon row answers only `0xC9B6D7D6` (and `0x452294AA`), while
  the prose says every class accepts `0xA9764C8C`, and other rows also accept the own class ID. Does the pylon QI
  answer `0xA9764C8C` and/or `0x49B6D69B`? The core follows the row (neither). [ambiguous]

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- **NO11 - the tunnel constructor's flag write:** §2.8 says `+0x13C &= ~0x400 | 0x10C00000`, "clearing the prebuilt
  bit", but the prebuilt bit is `0x400000`. Is the cleared mask `0x400` or `0x400000`, and is the expression
  `(f & ~mask) | 0x10C00000`? The core applies the literal `0x400` (a tunnel keeps `0x400000`). [ambiguous]

<!-- from: core-netocc-records4 (engine lane, 2026-10-21) -->
- **NO18 - pipe/subway `Read` after a failed underground record:** §3.3 says pipe `Read` "reads the same and then
  sets the byte `+0x260 = 1`"; when the inner underground `Read` returns 0 (wrong version, failed base), does the
  pipe still read its u32 `+0x270` and set `+0x260`, or return at once? The core returns at once. [ambiguous]

<!-- from: core-netocc-records4 (engine lane, 2026-10-21) -->
- **NO19 - the v < 7 cell rounding:** §2.3's read tail computes `floor(((v0.x + v2.x) · 0.5 + dx) · 0.0625)` "in
  x87 then rounded through f32". Is the sum `v0.x + v2.x` itself rounded to f32 before `· 0.5 + dx`, or only the
  final product? The core keeps every step in f64 and rounds the product to f32 before the floor. [ambiguous]

<!-- from: core-powerline4 (engine lane, 2026-10-21) -->
- **NO24 - §5.2, `ComputePolePosition` "guarded against `-FLT_MAX`":** what happens when the terrain answers
  `-FLT_MAX` (`0x00AA9F0C`): is y left at `-FLT_MAX`, replaced by a value (0? the previous y?), or does the call fail
  and the pole is skipped? The core returns no position. [ambiguous]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO27 - §10.3 Bind constants and format:** what numeric colour is stored at `0x00B0E37C`, what numeric
  format does `PackStandardVertexFormat(2)` return, and what is the processor's destination format?
  The core requires these three constants explicitly; there is no assumed default. [missing]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO28 - §10.3 processor LightModel:** the owner dispatch, flip-adjusted lighting transform and
  ReleaseModel order are published, but the processor's normal generation and colour-lighting arithmetic
  are not. What are those contracts? The core records an explicit pending LightingRequest and releases
  its processor source references, without accepting an original owner output or inventing colours. [missing]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO29 - §10.3 Clip mode 0:** the six-plane clipping path is expressly unspecified. What is its exact
  edge walk/interpolation/output order? The core returns UnspecifiedClipMode for a non-null box in mode 0;
  the network caller always uses the fully implemented mode 1. Null lo still returns immediately. [missing]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO31 - §10 instance constructor/destructor inventory:** beyond the published object size, base-class
  identity, processor member, resource reference and prebuilt cache lifetimes, what are the initial state
  and teardown side effects of instance ctor `0x00614A30` and destructor `0x00614830`? The pure core models
  resource/mesh/processor reference lifetime with RAII; it does not claim unreported native body effects. [missing]

  NO29 implementation refinement: the published mode-0 x/z bounds early-out is implemented too. Only a
  mode-0 call that actually needs six-plane cutting returns `UnspecifiedClipMode`; null lo or contained
  x/z bounds return unchanged. No six-plane output is fabricated.

<!-- from: re-ugbase (2026-10-20) -->
- **NO38 - §2.1, §2.2:** what the property holder's fixed `true` for `HasProperty(0x88696402)` (`0x00619500`)
  serves, and what the small interface at `+0x20` (vtable `0x00AA7F60`, slots `0x005E06A0`, `0x005E06B0`,
  `0x00458E60`, `0x0060CE80`) is. [as read]

<!-- from: re-ugbase (2026-10-20) -->
- **NO39 - §2.3 field 12:** the record stores only the cached path info's resource key, and `Read` fetches that
  key from the resource manager. The cached path info is the occupant's own transformed copy (traffic-network.md
  §7.2), so where is that copy kept in a save, and under which key? `0x005D9940` writes the key, `0x005D9820` reads
  it. [as read]

<!-- from: re-powerpoles (2026-10-21) -->
- **NO42 - §4.1, §4.4:** what the pole's saved bool `+0x110` means (record row 11); nothing read in this lane sets
  it. [as read]

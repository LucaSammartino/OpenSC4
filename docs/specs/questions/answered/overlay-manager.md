# overlay-manager: answered questions

<!-- from: glue-overlays-shadow5 (2026-10-24); retrieved from that branch because this base predates its question -->
- ~~**OV12 - §§1–3 record-only native shadow enumeration and replay inputs:** specify the exact begin/end/capacity field offsets and pointer semantics of all three arenas, how to distinguish every live/free record including a live/free tail whose link is -1/0, the complete record layout (secondary transform/ownership fields included), and a read-only enumeration contract for static cell lists and texture/source identity, dimensions and data. Which documented native inputs allow reconstruction of existing overlays (including model shadows) without feeding computed native matrices/rectangles back into the core? The public interface has no enumerator; the record sizes and table offsets alone do not establish a safe walk or mutation history/order. [default-off overlays_shadow measures only the documented -1 scalar/color/placement getters and direct-ID placement no-write contract on all four original managers; live record IDs/order/parameters, cell indices and texture data remain blocked, with no full-overlay verification claim]~~

  **ANSWERED 2026-10-24 (re-overlays-ov12):** DIFFERS — §§2.4–2.7 give exact arena begin/constructed-end/allocation-end/free-queue offsets, live word −1 versus free-tail word zero, complete record fields (including secondary matrix, stamp, byte padding and references), ordered static-cell lists, binding/source identity and dimension observations. Driver handles are not CPU pixel data; arbitrary source objects have no universal field/readback layout. §3.8 answers the inverse question: a current snapshot cannot independently reconstruct all overlays. Model geometry/pairs/descriptor/lighting inputs and mutation/upload history are discarded, not latent in the matrices. Capture original ordered creation/setter/removal calls and resource/lighting inputs from before allocation for full replay; comparing injected native matrices/rectangles cannot verify their production. Twelve §9.1 paper vectors cover enumeration, reuse, lists, metadata and information loss. No full live-record shadow or image-data verification is claimed. OV1/OV7/OV8 residuals remain separate.

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV9 - §3.1 slot7:** specify radius/centre fields, the eight-part partition, half-chord row sample coordinates/rounding, city-edge clipping, texture mapping and terrain redisplay order, and empty/failing part cleanup. The prose states eight rectangles but not their equations/partition. [no host-computed eight rectangles or guessed disc approximation accepted; round add reports a gap]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §3.1 proves an eight-part outline partition omitting the centre, outer/90% inner radii, four integer split-row chord samples, ordered rounding and binary32 partition constants, shared full-footprint texture matrix, per-part clipping and redisplay order. Empty parts remain allocated; the native add has no partial-allocation rollback path. The H=20 split-row discriminator detects substituting exact decimal constants.

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV4 - §§1,2,3 getters:** please specify constructor overlay/handle defaults, Init failure/repeat-Init behavior, factory slot used for key 0xAA592B77, slot 22's field identity/type, and scalar getter behavior for groups and handle -1 (besides alpha=0). [core owns acquired Arc references, rejects invalid/repeated Init atomically, defers shadow-binding lookup until its slot is known; unidentified/group scalar getters report a gap, null output getters leave output untouched]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §§1.1/2.3 specify defaults and unspecified inactive scratch, mutating repeat Init with no rollback/failure result, binding factory slot5, slot22 boolean shown, slot26 self-lit, group part0 getters, and −1/null-output behavior. No atomic native stale/null/Init rejection is implied.

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV5 - §§2,3 slots12-15:** specify handle/group allocation and free-list reuse order, stale/out-of-range behavior, slot13/14 behavior on direct overlay IDs, and slot12's x/z offsets in the nine floats. Is a handle record always allocated for every add? Do any setters early-return on equal input or mark unchanged rectangles? [private LIFO arenas and atomic stale-record rejection; no public construction/re-placement until OV1; null calls do nothing]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §§2.3/3.2: FIFO reuse, no generations/range checks, direct IDs ignored by placement accessors, X/Z at public offsets0/4, direct city overlays need no placement handle. Position uses a native 1e−4 binary32 tolerance; full placement always re-places, unchanged static rectangles still mark once, equal shown/colour/state writes still mark; handle is freed before group parts.

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- ~~**OV6 - §3 slot17:** what is the numerical value/precision of 0x00AB39A0, strict versus inclusive step comparison, and does a sub-step alpha write update the stored alpha, or compare against the last redrawn alpha? Specify clamping/NaN behavior, dynamic redraw behavior, and equal endpoint writes. [dynamic and endpoint changes supported; static non-endpoint changes report a gap atomically rather than accept an external threshold]~~

  **ANSWERED 2026-10-22 (re-overlays-contract4):** DIFFERS — §3.3: binary64 0.05, strict greater-than; static sub-step/equal writes leave stored alpha unchanged, no clamp, static NaN rejected, dynamic always stores without redraw; endpoint equality is rejected on static managers. Native bytes and paper vectors cited there.

<!-- from: re-overlays (2026-10-20) -->
- ~~**OV2 - §3:** what slot 20 (`0x007364F0`, no function in the export) does, and how the add forms of slots 8, 9 and 11 differ from slot 4 (slot 11 is the largest and is inferred to build a shadow from a model's footprint). [unread]~~

  Prior partial resolution (re-overlays-contract4): slot 20 and slots 9/10 were specified; city/model add bodies remained.

  **ANSWERED 2026-10-22 (re-overlays-final4):** DIFFERS — §§3.5–3.7 and slot-20 review specify each owner form. City add resolves factory slot 5 and makes a direct flags-0x13 rectangle record with an unusual nx-only matrix. Model add uses six arguments, lighting projection and a separate live-global-quality rectangle branch, makes a direct flags-9 record, then derives its secondary transform from projected-pair extrema. Neither creates an ordinary placement handle. Outside-owner projection production remains explicitly OV1, not a guessed decal rule.

<!-- from: re-overlays (2026-10-20) -->
- ~~**OV3 - §5:** what message `0xC9F775BB` is and what the managers do on it. [inferred: the overlays are placed again after a terrain or zoom change]~~

  **ANSWERED 2026-10-22 (re-overlays-final4):** DIFFERS — §5: render-settings refresh; complete handler 0x00736100 copies cached quality, second-stage/debug setting and selection byte, always returns true, and never re-places overlays, rebuilds the index, invalidates terrain or resets counters. Native SetRenderProperties dispatch provides independent sender evidence.

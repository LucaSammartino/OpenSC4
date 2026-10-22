# snapshots: open questions
<!-- Questions about [snapshots.md](../snapshots.md). Append new entries at the END; when one is answered, move it to answered/snapshots.md. Rules: README.md. -->

<!-- from: re-mysimui (2026-10-21) -->
- **SN1 - §1.1, §1.3:** the number in the album folder name (`<name>--<n>`) and the ninth info field; and what
  the digit keys' percentage (`+0xE8`) controls. [as read]

<!-- from: re-mysimui (2026-10-21) -->
- **SN2 - §1.3:** the exact caption layout the album builds from the info fields (strings `0xEA946EFE`–
  `0xEA946EFA` of table `0x6A231EAA`). [as read]

<!-- from: core-snapshots4 (2026-10-22) -->
- **SN3 - §1.1-1.2:** Publish SC4SanitizeFileName character/replacement/reserved-name rules, text encoding and 48-byte boundary rules, MakeTempFileName collision suffix/search order, and region-only naming. [Portable implementation accepts already-valid UTF-8 components and trims on UTF-8 boundaries; names needing sanitization, collisions and region-only naming return explicit contract gaps. No native naming equivalence claimed.]

<!-- from: core-snapshots4 (2026-10-22) -->
- **SN4 - §1.1, §3.2-3.3:** What are album collation, picture order, extension case handling, initial/default picture, invalid-index handling, next/previous endpoints and post-delete selection? Does album switching save the previous description? [Literal portable policy: bytewise sorted lists, ASCII-insensitive extensions, first picture initially, checked indices, clamped browse endpoints, successor-or-last after deletion, save before switching. These choices are provisional, not native-exact.]

<!-- from: core-snapshots4 (2026-10-22) -->
- **SN5 - §1.3:** Publish info/description text encoding and reader behavior for missing/extra fields, embedded line feeds, trailing line feed, invalid numbers, large files and missing companions. [Writer uses typed UTF-8 LF-free text and documented decimal fields; reader preserves raw field bytes, rejects incomplete marked records with a gap and exposes structured caption inputs pending SN2. Missing companions are empty.]

<!-- from: core-snapshots4 (2026-10-22) -->
- **SN6 - §2:** Publish the size list/default/index signedness, oversized-size behavior, exact positioning and drag anchor/release/capture rules, repeated Enter/pending-capture behavior and full-view five-pixel test coordinates. [Caller supplies sizes; indices clamp; fitting frames clamp into view; oversized sizes return a gap; drag preserves click offset; repeated capture input is ignored while pending; full-view threshold uses absolute relative x/y <=5.]

<!-- from: core-snapshots4 (2026-10-22) -->
- **SN7 - §2-3 / owner inventory:** Publish complete taker Shutdown/GZPaint/destructor and album destructor/lifecycle contracts, window/script/control/button/message identities, Init help composition, slot 70/71 semantics and precise returns. ABI lists taker 151 slots (604 vtable bytes, 0x10C object) and album 6 slots (24 vtable bytes, unknown object size); most are toolkit targets. [Only typed portable workflows/resources are exposed; no toolkit inference, ABI binding, destructor or full-owner credit.]

<!-- from: core-snapshots4 (2026-10-22) -->
- **SN8 - §2:** Publish capture pixel formats/alpha masks, crop coordinate conventions and capture/encode failure/modal behavior. [A typed image primitive forces opaque alpha for explicit 16/32-bit depth; masks and encoding stay primitive image/codec boundaries. Core owns delay, target rectangle, album/name/metadata and modal completion.]

<!-- from: core-snapshots4 (2026-10-22) -->
- **SN9 - §3.2-3.3:** Publish filesystem/copy/metadata/delete failure order and rollback, overwrite policy, nested-folder deletion and same-folder file comparison. [Portable operations stop on errors and retain dirty description/selection when possible; copy precedes source removal; partial failed capture output is removed as specified; no invented transaction/recursive deletion. Paths use exact validated virtual components.]

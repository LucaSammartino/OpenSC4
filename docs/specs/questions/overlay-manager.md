# overlay-manager: open questions
<!-- Questions about [overlay-manager.md](../overlay-manager.md). Append new entries at the END; when one is answered, move it to answered/overlay-manager.md. Rules: README.md. -->

<!-- from: re-overlays (2026-10-20) -->
- **OV1 - §3:** the layout of the nine-float placement record (slots 12-14) and the last arguments of the add
  calls (slots 3-6, 9-11); a centre, extents and an orientation are inferred from how `PlaceOverlay`
  (`0x0073A1B0`) turns it into a cell rectangle and a texture matrix. [the record as copied]

  Partial resolution (2026-10-22, re-overlays-contract4): §3.2/§3.2.1 now give all nine fields, retained aspect,
  ordinary/round add arguments, owner placement and ordered rotated coefficients. §3.5 now proves slots 9/10's
  resource-key arguments, byte flipV, primary V flip and ordered secondary transform. Still open: slot 11 model
  add argument roles and its owner-derived placement/post-transforms; do not treat it as a plain decal.

<!-- from: re-overlays (2026-10-20) -->
- **OV2 - §3:** what slot 20 (`0x007364F0`, no function in the export) does, and how the add forms of slots 8, 9
  and 11 differ from slot 4 (slot 11 is the largest and is inferred to build a shadow from a model's
  footprint). [unread]

<!-- from: re-overlays (2026-10-20) -->
- **OV3 - §5:** what message `0xC9F775BB` is and what the managers do on it. [inferred: the overlays are
  placed again after a terrain or zoom change]

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- **OV7 - §§3,4 slots21/Draw:** specify the placement-to-world/texture equations, matrix axis/storage/composition conventions, slot21 matrix entries and behavior for size=0, and camera transform inputs/precision. The public text gives semantic scales 1/size and -1/size with second-axis offset 1+offset/size but not their world matrix positions. [core stores those semantic coefficients only; no externally computed owner matrices accepted; full world transform deferred with OV1]

  Partial resolution (2026-10-22, re-overlays-contract4): §3.2/§3.2.1 specify row-vector placement coefficients;
  §3.4 specifies all slot21 entries, FP behavior, no group recursion and redisplay. §4.7 proves post-reset context
  matrix input, binary32 storage, C×M order and differing dot-product orders. Still open: complete inverse
  cofactor/spill/singular behavior and all sixteen output accumulation orders, plus shadow composition branches.

<!-- from: core-terrain-overlays4 (2026-10-22) -->
- **OV8 - §§2.2,4:** specify rectangle endpoint conventions, clipping/margin and cell-index insertion/update/removal ordering; decal/shadow classification (bit4 is described as both decal/shadow-part and decal eligibility); shadow selection/strength minimum, alpha-test equation, second transform/height coordinate, DrawRect land/water/dynamic array input formats, GPU restoration and counter reset/increment points. Does empty or skipped Draw preserve last-frame stats? [core uses explicitly internal half-open rectangles and sorted/deduplicated selection only; no host draw result, fabricated successful GPU pass or guessed stats; normal Draw reports a gap]

  Partial resolution (2026-10-22, re-overlays-contract4): §§2.1–2.3 and §3.2 distinguish shadow bit3/decal bit4,
  half-open bounds, floor/ceil conversion without a margin, strict separation versus touching-edge clipping,
  native index traversal/prepend/removal and same/changed-rectangle redisplay. §4.4 proves selection-empty,
  pass-disabled and snapshot counter preservation, nonempty pass resets, masks and increment-after-submission.
  §§4.5/4.6 now prove visible-row endpoint arithmetic and native encounter order, stamp wrap without reset,
  cell-to-vertex rectangle dispatch, exact zero minimum/alpha reference and bounded exit-state actions.
  Still open: terrain consumers' vertex/index-array bodies, full shadow secondary/height composition and
  water adjustment/short-rectangle routing. No complete renderer or bit-exact shadow draw is claimed.

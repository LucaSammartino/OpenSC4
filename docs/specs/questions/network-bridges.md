# network-bridges: open questions
<!-- Questions about [network-bridges.md](../network-bridges.md). Append new entries at the END; when one is answered, move it to answered/network-bridges.md. Rules: README.md. -->

## From the network bridges spec (re-specs17a lane, 2026-10-16)

Open points from network-bridges.md §16. These NB numbers are the bridge spec's own; they are not the neighbours NB1–NB19
above. The spec's tentative answer is in brackets.

<!-- from: From the network bridges spec (2026-10-16) -->
- **NB2 (bridges):** the suspension profile (`0x0062E690`), the cable plotting (`0x00630F70`) and
  `AddSuspendedBridgeStructures` (`0x0062DF30`) in detail. [drawing only *[inferred]*] **PARTLY ANSWERED 2026-10-16
  (re-answers26):** MATCHES, with one side effect (network-bridges.md §11.4). The profile is a leaf that writes only
  the piece records' weights; the cable plotting fills each record's cable vector; only `InsertBridgePiece` reads
  them, turning them into textured triangles on the occupant (network occupant slot 31, `0x0061C800`: append to the
  triangle list `+0x118`, recompute the bounding box, which includes the triangles). The occupant is inserted into the
  occupant manager after that (`0x0062F5A8`), so the triangles can widen the cells it is filed under. Still open:
  whether any shipped suspension or cable triangle reaches past its piece's own cell (an in-game occupant query beside
  a suspension bridge would settle it), and the formulas (visual only).

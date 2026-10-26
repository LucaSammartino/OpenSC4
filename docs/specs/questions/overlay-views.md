# overlay-views: open questions
<!-- Questions about [overlay-views.md](../overlay-views.md). Append new entries at the END; when one is answered, move it to answered/overlay-views.md. Rules: README.md. -->

<!-- from: re-overlayviews (2026-10-20) -->
- **OVV1 - §1.1, §2.1:** which lot flags `0x304` (marks raised 15.5 m) and `0x100000` (marks hidden unless the lot
  says otherwise) stand for, and where a path's texture repeat factor r (path record `+0x38`) is set. [as read]
  **Partial answer 2026-10-23 (re-overlay-views-contracts4):** §1.2 establishes the second binary32
  SetPathTexture argument as the repeat source; NewPath does not initialize a default. Lot-flag
  identities remain open, so this bundled entry is retained here.

# lighting: open questions
<!-- Questions about [lighting.md](../lighting.md). Append new entries at the END; when one is answered, move it to answered/lighting.md. Rules: README.md. -->

<!-- from: re-lighting (2026-10-19) -->
- **LT1 - §2.1, §2.2:** which setting is the byte `+0x39C` of the `+0x0C` member of the settings object at
  `0x00B43CC4`? When it is set the fade colour is black, the fade strength 0 and no dawn or dusk message is posted.

<!-- from: re-lighting (2026-10-19) -->
- **LT2 - §4.3:** which network occupants answer true to slot 8 with `0x40000`? Their models are lit with the up
  normal and the default weight, ahead of the type tests.

<!-- from: re-lighting (2026-10-19) -->
- **LT3 - §3, §4.5:** what the fade term does on screen. It grows across the city from one edge (which edge follows the
  view rotation), scales with `2^-zoom` and with the time-of-day fade strength (`0x29189BF5`, 0.15 at midday), and is
  only recomputed on a view change.

<!-- from: re-lighting (2026-10-19) -->
- **LT4 - §5.6:** the meaning of the decal shadow values `0x085489C9` (a float pair the shipped exemplar lacks) and
  `0x085489CA` (12.0), and of the shadow parameters `0x085489CC` (0.4) and `0x085489CD` (0.55). Their users draw
  decal shadows outside this lane.

<!-- from: core-lighting-complete4 (2026-10-22) -->
- **LT12 - §2.2, §5.7, §5.8.2 inherited numeric environment:** which precision-control and
  rounding-control modes reach these lighting entries in the normal game call paths? §5.8 says inherited
  control word, and vectors 19–22 hold at 53/64 significant bits, but does not establish the caller's
  mode or whether it changes between entries. [Portable core requires an explicit 24/53/64-bit register
  precision at construction; implemented arithmetic assumes nearest-even and masked exceptions. No
  implicit game-mode default or whole-owner numeric exactness is claimed for unproved CW modes.]

<!-- from: core-lighting-complete4 (2026-10-22) -->
- **LT13 - §5.8 NaN output encoding:** for NaN inputs and invalid operations which proceed through the
  unordered determinant path, are NaN payloads/signs or signaling-NaN exception flags part of the exact
  64-byte result contract? The arithmetic schedule establishes propagation/classification but does not
  specify those encodings. [Core returns true and propagates quiet NaN values without a finite fallback;
  its integer register primitive canonicalizes payloads. NaN bit-pattern equivalence is not claimed.]

<!-- from: glue-lighting-shadow9 (2026-10-24) -->
- **LT14 - §§1.3, 2, 3, 6 live-owner input replay:** what public/serializable interfaces provide the lighting
  owner's complete ordered external inputs at city load and thereafter: the time-of-day driver (independent
  of GetTimeOfDay), tint/reset events, render settings including the unidentified LT1 byte, view changes and
  terrain notifications? There is no owner Read/Write slot in §7. Please specify a safe input snapshot/replay
  contract, including initial state and notification ordering, rather than copying native lighting outputs
  into the expected model. [Default-off shadow measures only §5.6 GetShadowParams and
  GetDecalShadowParams(null), using §1.4's explicitly shipped parameter inputs in the Rust core. The published
  §0 global locates the borrowed owner; no hardware observers, setters or native lifecycle calls. Slots
  0–13, 15, 17–29 and nonnull-holder slot 16 remain uncompared; whole-owner verification is not claimed.]

<!-- from: glue-lighting-shadow9 (2026-10-24) -->
- **LT15 - §1.4 shipped decal texture:** the table gives `0x69D3BC3B` for property `0x085489C8`, but the
  house-switch run lighting9-house-2032-r1-20261024 returns `0x69D3A23B` in slot 16(null) on all 46 captures
  (2026-01-27 through 2029-10-01). Is the shipped texture value in the table a transcription error?
  [The first literal shipped-input measurement is preserved as 0/46 exact, one mismatch per line. The
  next revision captures the six §5.6 input properties independently from resource
  `6534284A/A9189CF0/00000001` through the public resource/property interfaces, then feeds the Rust
  property-reading implementation; no expected value is copied from a native lighting getter.]

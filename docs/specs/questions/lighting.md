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

<!-- from: core-lighting4 (2026-10-22) -->
- **LT10 - §5.6 CalcShadowProjection:** specify the plane basis, model-frame direction transform, complete input/output matrix layout, flag-dependent composition and arithmetic precision. Does the determinant test use a signed or absolute determinant? [Only the stated 2D affine least-squares subproblem with signed determinant cutoff 0.01 is implemented; complete projection returns ContractGap and preserves output, without an original-method provider.]
  **Partial native evidence 2026-10-22 (re-lighting-contracts4):** §5.8 proves count-first arguments, separate transform/output layouts, full basis-helper behaviour and signed binary64 0.01 comparison (NaN proceeds). Count-zero failure preserves all output. Remaining: model-frame direction convention, full flag-dependent composition/coefficient layout and exact spill/round schedule. The question stays open; no complete-owner projection credit.

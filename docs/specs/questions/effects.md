# effects: open questions
<!-- Questions about [effects.md](../effects.md). Append new entries at the END; when one is answered, move it to answered/effects.md. Rules: README.md. -->

<!-- re-answers34 (2026-10-20) answered EF1, EF4, EF5, EF6, EF7, EF8, EF9 and EF10; they moved to answered/effects.md.
     EF2 (closed re-integrate1) and EF3 (answered re-integrate1) were archived earlier. No open questions remain. -->

<!-- re-effects-contracts4 (2026-10-21) answered EF11, EF12 and EF13; they moved to answered/effects.md. No open questions remain. -->

<!-- re-effects14-4 (2026-10-21) answered EF14; it moved to answered/effects.md. No open questions remain. -->
No open questions. EF15 and EF16 are archived in answered/effects.md.

<!-- from: core-effects-close5 (2026-10-24) -->
- **EF18 - §6.1 EffectsParser queue wrapper, EffectsBootstrap, collection lifecycle:** what are the queue/file processing and bootstrap lifecycle contracts for the separate `EffectsParser` and `EffectsBootstrap` owners, and any `cSC4EffectsCollection` behavior beyond the component directory in effects-parser §11.1? §6.1 only names `ParseQueuedFiles` in the manager's reload sequence; effects-parser §1 expressly excludes that wrapper, and effects.md supplies no bootstrap algorithm. [Queue processing/bootstrap return explicit Unsupported; the settled component directory remains implemented. EF17 is reserved by the coordinating brief and is not duplicated here.]

<!-- from: core-effects-close5 (2026-10-24) -->
- **EF19 - §§3, 8.3, 9.2 component transform decoding and numeric order:** what is the mapping from the 56-byte `S3DTransformLegacyLoad` placement to rotation/scale/translation, and the intermediate operation/store order for component world positions? [Implement the literal semantic parent translation + parent rotation/scale of local offset using binary32 products and left-to-right sums, with direct parent translation for a zero local offset. Do not decode the opaque placement bytes or claim native numeric equivalence. Manager legacy placement decoding remains outside this component API.]

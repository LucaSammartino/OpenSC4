# watercraft: open questions
<!-- Questions about [watercraft.md](../watercraft.md). Append new entries at the END; when one is answered, move it to answered/watercraft.md. Rules: README.md. -->

<!-- from: re-watercraft-wc9, narrowed residual of WC3 (2026-10-24) -->
- **WC10 - transitive model initialization only:** complete the executable contracts for animated-resource acquisition `0x007FEDE0` (sprite `0x007FD7D0`, mesh `0x007FED50`), LoadAuxiliaryModel `0x0050EB20`, and PositionModelInstance `0x005094A0` / heading-basis helper `0x0080B830`. Include acquisition/output/reference failures, auxiliary-key mapping, transform arithmetic and scale/version/flags. [LoadModels ordering, boolean meaning, bounds, partial failure and retry are settled in §3.6; shared tween and effect-controller internals are settled by §§5.1.6–5.1.7. No opaque provider may replace these remaining owner computations; WC3's scoped answer is archived.]

<!-- from: re-watercraft-wc9, narrowed residual of WC4 (2026-10-24) -->
- **WC11 - inherited model publication only:** specify slot 100 `0x00508610` and the complete PositionModelInstance `0x005094A0` / heading-basis helper `0x0080B830` computations, call order, flags/scale/version and failure contract. [AI targets/publication, common tween sampling and shared effect-controller update arithmetic are settled in §§5.2.2, 5.1.6 and 5.1.7. Initialization resource/auxiliary work belongs to WC10; no full-owner closure or final-transform provider substitution is accepted. WC4's scoped answer is archived.]


# effects: open questions
<!-- Questions about [effects.md](../effects.md). Append new entries at the END; when one is answered, move it to answered/effects.md. Rules: README.md. -->

<!-- re-answers34 (2026-10-20) answered EF1, EF4, EF5, EF6, EF7, EF8, EF9 and EF10; they moved to answered/effects.md.
     EF2 (closed re-integrate1) and EF3 (answered re-integrate1) were archived earlier. No open questions remain. -->

<!-- re-effects-contracts4 (2026-10-21) answered EF11, EF12 and EF13; they moved to answered/effects.md. No open questions remain. -->

<!-- re-effects14-4 (2026-10-21) answered EF14; it moved to answered/effects.md. No open questions remain. -->
No open questions. EF15 and EF16 are archived in answered/effects.md.

<!-- re-effects-ef18 (2026-10-24) answered EF18 and EF19; moved to answered/effects.md.
     EF17 remains reserved by the coordinating brief. No open questions remain. -->

<!-- re-effects-ef20 (2026-10-24) copied EF20 from glue-effects-shadow7 and answered
     it in effects.md section 18.7; original question archived in answered/effects.md. -->
<!-- re-effects-ef21 (2026-10-24) answered EF21 from glue-effects-capture8 in
     effects.md section 18.8; original question moved to answered/effects.md. -->

<!-- re-effects-ef21 (2026-10-24) answered EF21 from glue-effects-capture8 in
     effects.md section 18.8; original question moved to answered/effects.md. -->

<!-- from: glue-effects-whole9 (2026-10-24) -->
- **EF22 - section 18.8 safe native interceptor installation:** EF21 specifies observation sites and calling conventions, but not the complete instruction spans, relocation rules, expected bytes/version guards or adjusted thunk entries needed to install those interceptors without reading/disassembling executable code. Please specify a safe attachment/removal contract before natural Init: the version-validated patch spans and continuation/exception behavior for Init/bootstrap, resource requests/Read/publication/releases, Begin/End, parse and outer reload/tick, including exact this adjustments at virtual entries and how every possible writer is excluded while copying. Section 18.1 identifies the +8 message-target vtable but does not give the callable adjustor thunk entry value for a validated slot replacement. Is there an existing documented interception mechanism that avoids code patching and still observes these natural operations? [No guessed hooks or private executable inspection. Current plugin measures only SDK name membership at load/month; retained visual/built-in payload and historical owner comparison stay unavailable until thread/depth/writer conditions are established. The core implements bounded EF21 payload copying and observation bookkeeping for a future authorized host attachment.]

<!-- re-effects-ef24 (2026-10-24) answered EF23 in effects.md section 18.8.4;
     original question moved to answered/effects.md. EF22 remains open. -->

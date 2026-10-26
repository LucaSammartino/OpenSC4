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

<!-- re-effects-ef24 (2026-10-24) answered EF23 in effects.md section 18.8.4;
     answered EF22 in sections 18.8.6–18.8.7; originals moved to answered/effects.md. -->

No open questions remain.

<!-- re-ef29 (2026-10-26) answered EF25 from replace-effectsres28 in effects.md
     section 18.9; original question moved to answered/effects.md. -->

<!-- re-ef30 (2026-10-26) answered EF26 from replace-effectsres28 in effects.md
     section 18.10; original question moved to answered/effects.md. -->

## EF27 — Typed visual-name dictionary insertion boundary (replace-effectsres30)

EF26 is implemented for the renderer codecs. Nonempty resource Read now reaches
the visual name/index map, whose retained layout and traversal are specified in
effects.md 18.7.2 and whose insertion/terminator behavior is in 18.9.6. The current
adapter refuses the first nonterminator pair instead of guessing a native call;
this prevents replacement acceptance for the shipped directory.
Reproduced with the native manager in `build-runs/game-results/effectsres30-codecs-on1.log`
in the main checkout, line 166: all description sections were traversed before
the explicit EF27 refusal; the short run's clean verdict is not payload credit.

Please specify the independent typed native dictionary-insertion dependency
used here (not native resource Read): address, receiver, ordered arguments,
hidden result storage if any, stack cleanup, return channel, key/value layout,
copy/allocator ownership, duplicate-key behavior, and any spelling normalization.
The ABI inventory has a string-keyed insert_unique at 0051FDC0 with two stack
words and void return, but does not identify those words or establish that this
is the resource's operation. The published string-key lookup cannot establish
insertion's typed arguments. Alternatively specify the complete string hash,
bucket growth and chain insertion contract for a compatible independent insert.

Worked case requested: retain an old entry, read two entries with the same mixed
case spelling and different indices, then the exact (end,FFFFFFFF) terminator;
state the resulting nodes/indices/spelling and writer traversal. No helper call
or hash algorithm has been guessed. effectsres remains default-off and not ready.

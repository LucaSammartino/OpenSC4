# effects: open questions
<!-- Questions about [effects.md](../effects.md). Append new entries at the END; when one is answered, move it to answered/effects.md. Rules: README.md. -->

<!-- re-answers34 (2026-10-20) answered EF1, EF4, EF5, EF6, EF7, EF8, EF9 and EF10; they moved to answered/effects.md.
     EF2 (closed re-integrate1) and EF3 (answered re-integrate1) were archived earlier. No open questions remain. -->

<!-- re-effects-contracts4 (2026-10-21) answered EF11, EF12 and EF13; they moved to answered/effects.md. No open questions remain. -->

<!-- re-effects14-4 (2026-10-21) answered EF14; it moved to answered/effects.md. No open questions remain. -->

<!-- from: core-effects-commands5 (2026-10-23) -->
- **EF15 — Effects-script command owner contracts missing from effects.md:**
  The requested owner family is `(anonymous_namespace)::c*Command`, including
  effect-description parser commands. Which exact classes belong to that family,
  and which spec sections define each one's token inputs, state transitions,
  defaults, validation, block enter/exit, foreign calls and conformance cases?
  `effects.md` §§0–18 contains no command-class inventory or command algorithms;
  its §17 exercises runtime effects, not script commands. The separate
  `effects-parser.md` §6.1 supplies 21 root literal/descriptor/hook associations,
  expressly excludes their execution algorithms, and §6.2 supplies only the
  `sequenceEffect` normal handler, expressly excluding other nested entries.
  Please publish the class-to-descriptor inventory, all remaining root/nested
  handler contracts and command-specific conformance cases (including any
  commands absent from that bounded dictionary). Distinguish these owned
  algorithms from foreign renderer, sound, resource and generic-reader services.
  [Retain existing registration and sequenceEffect implementation; do not infer
  handler behavior or class names from command literals or addresses. Whole
  command-family closure remains blocked on this publication; EFP1–EFP7 also
  continue to bound the existing parser's supported domain.]

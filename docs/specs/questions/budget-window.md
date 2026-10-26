# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- BW11 is narrowed to the explicit runtime observation in budget-window.md §4.0; archived with no game verdict.
     BW17's duplicate open parent was removed by re-bw30 (2026-10-26). Its partial answer is archived;
     budget-window-bodies.md §§7/12 preserve the still-open BW22/BW24/BW25 audits; BW23 is now settled. -->
<!-- from: core-budget-window5 (2026-10-23) -->
- **BW19 - budget-window-bodies.md §3.2 and §6 invalid raw inputs:** what safe replacement behavior is required when combo-edit signed division has a zero divisor or INT_MIN/-1, or UpdateValues receives an empty budget-group vector? The literal native operations would fault. Core returns InvalidDivision for the affected row (continues matching rows and records the single pending neighbor refresh), or None for the empty group vector while clearing its busy guard. These are explicit safe divergences, not successful empty rows. [open]

<!-- from: core-budget-window5 (2026-10-23) -->
- **BW20 - §4 group initialization / budget-window-bodies.md §6 raw department list references:** §6 explicitly releases each department returned by budget slot 31 during refresh; does initial group population own the same temporary references and release them after both build passes (including the empty branch)? Core uses that literal shared public-provider contract in both paths, separately from saved-pair and slider references. Please confirm the initial path's list ownership and release order. [open]

<!-- from: re-budget-window-bw22 (2026-10-24), narrowed remainder of archived BW17 -->
- **BW22 - budget-window-bodies.md §3 neighbour rendering/options:** specify set_comboBox (`0x0077DAB0`) option generation and selection, updateNeighborBudget (`0x00781C90`) row reuse/amount formatting, the +8/+12 words of retained row metadata, and the six temporary enumeration lists' reference balance. §3 already specifies enumeration/counts and combo arithmetic; supply remaining raw interface calls rather than a same-owner refresh callback. [open]
  <!-- Partial re-bw30 2026-10-26: §§3.3–3.4 settle counted list clearing/releases, unused stack-residue
       words +8/+12 and set_comboBox inputs/options/selection/deal writes/failure paths. Remaining:
       updateNeighborBudget control reuse, section/row amount formatting and cleanup. -->

- **BW24 - budget-window-bodies.md §6 group refresh remainder:** finish UpdateValues (`0x007898A0`): A item text/amount rendering, B traversal and slider rebuilding with counted department references, and final stale-row cleanup. Preserve the proven A header/item quota distinction; do not generalize it to the unproved B pass. BW20 separately covers initial temporary department-list ownership. [open]

- **BW25 - budget-window-bodies.md §1 setup/widget ownership:** specify property replacement (`0x007795A0`), empty/popup creation (`0x0077BEC0` and button path), SetupWindow/Big/Indiv (`0x0077A480`, `0x0077A6F0`, `0x0077A960`), widget factory/QI failure references and exceptional stale controls; classify the still-unlabelled helpers in §1. GetBudgetSegment (`0x0077A390`) is a resource-backed bitmap-segment helper, not a budget-row provider. Which references survive each failed factory/query? [open]
  <!-- Partial BW17 evidence, re-bw30 2026-10-26: companion §§1.2–1.3 now settle the non-owning 0x0F
       property payload/caller boundary, GetBudgetSegment, final setup-generator temporary and bounded
       SetEmptyWindow success/failure references. Other widgets/popup paths, generic window replacement /
       procedure-reference behavior and unclassified helpers remain open; this is not a BW25 closure. -->

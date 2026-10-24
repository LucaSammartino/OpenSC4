# win-filebrowser: answered questions

<!-- from: re-filebrowser4 (2026-10-22) -->
- **FBQ2 - sections 8.2 / 8.5 / A:** Full control creation/configuration sequence, geometry/layout, size constraints, popup visibility/focus and message dispatch in `009A1F77`, `009A0ACC` and `009A0A17`?
  **ANSWERED 2026-10-24 (re-widget-controls-te13b):** DIFFERS — 8.2 specifies all twelve eagerly created children, order, factory/fallback policy, retained interfaces, numeric child configuration and unsafe failures; it sets no child rectangles or minimum browser dimensions. Init's 396 × 236 is actual size; popup placement/sizing/focus are in 8.1.5. Section 8.2.2 specifies every recognized message, ID/pointer test, payload, operation order and final base delegation, including temporary borrowed selection notification. Section 8.5 specifies paint rectangle and dirty-refresh order; slot 92 is a folded true-return hook, not invalidation. FB-C32–33 and FB-C39–43 distinguish these rules. Foreign child/style/property/pixel implementations retain their existing boundaries; FBQ1/FBQ4 remain separate.

<!-- from: re-filebrowser4 (2026-10-22) -->
- **FBQ5 - sections 8.1 / A.1:** Directory/drive population order and configuration in `009A4B2E` and `009A473F`, and remaining file-row configuration in `009A4EBA`?
  **ANSWERED 2026-10-24 (re-widget-controls-te13b):** DIFFERS — 8.1.3–8.1.6 specify unsigned column sizing, first sorted restoration/fallback, depth-record removal and component insertion order, collapsed display, popup dimensions/indentation/scroll/focus, icons, styles and fitted display copies. Native popup column count is span+8, correcting pseudocode's +2. No matching drive preserves old collapsed cells; disabled filename columns still receive text; empty grids still receive selection/visibility calls. FB-C28–31 and FB-C34–38 distinguish the behavior. No new question or whole-browser attribution claim is needed; FBQ6 remains the existing ownership question.

FinalRelease qualification for FBQ3: primary slot 89 (`009CFA1D`) calls virtual
browser Shutdown, then secondary deleting-destructor slot 148 with flag 1.
Normal final Release supplies the required Shutdown call; direct destruction
lacks it. This shared method's export name belongs to another owner.

<!-- from: re-filebrowser4 (2026-10-22) -->
- **FBQ3 - sections 1 / 8.5 / A:** Where is the full browser constructor, what defaults and minimum dimensions does it establish, and which cleanup steps are guaranteed on destructor versus explicit Shutdown? Prove exact retained-field release counts, entry/filter ownership and initialization failure preconditions from constructor/destructor and `0099EC98`/`009A3CC1`; primary table begins `00ADD190`, secondary `00A8D000`.
  **ANSWERED 2026-10-22 (re-filebrowser-contracts4):** DIFFERS — Sections 1.1, 8.5 and 8.6 identify constructor `009A57C4` (710 B), outside the preserved union, installing secondary table `00ADCF30`; `00A8D000` is a second table copy with a different destructor thunk. Init sets actual size 396 × 236, not minimum dimensions, and returns true without rollback on control failure; it requires usable resource-manager/buffer paths. Shutdown clears owned file/drive/custom records and conditionally releases 29 retained fields; `+0x280` is omitted. Actual destructor `009A31A2` destroys embedded strings/container storage and calls the base destructor, never browser Shutdown. Explicit Shutdown is therefore a separate ownership obligation. Defaults, clear-method behavior, wrapper flags and Release channels are specified with native citations.

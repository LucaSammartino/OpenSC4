# re-mysims-pool-answers4 — answer MS11 (dispatch location) / MS12 (pool) for mysims §4.7.1 / §4.12

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysims-pool-answers4 (answer MS11/MS12 dispatch/pool contracts) | `re-mysims-pool-answers4` / `.claude/worktrees/re-mysims-pool-answers4` | ready (spec only) | core lane: apply §4.7.1 location contract and §4.12 import rules to the dispatch/pool cores + conformance rows |

## Report

REA had no app open (target_unavailable); all findings are from re_query.py asm (address-cited) and decomp/.

- **MS11 answered (85e83546), DIFFERS**: new §4.7.1 "Dispatch location (`cLocation`) contract": layout and
  defaults (ctor 0x00431320: +0x14/+0x18 = -1, world -16.0), `IsValid` 0x00434F90 (flag set: cells in city
  bounds, occupant not consulted; flag clear: region city exists and a non-ferry transport connection exists, type
  order table 0x00B07DD0), `SetByOccupant` 0x00431430 (full reset incl. byCommute 0, cell = floor(pos/16)),
  `+0x40` is the dispatch trip state (setState 0x00431C10 -> 0x4BB2CFC0 -> §4.6), callers table: `ChangeJob`
  passes notify true; `FailCommute` and `SetResidence` run a new job search (FailCommute text corrected).
  Conformance rows added.
- **MS12 answered (6af0f01a), DIFFERS on a/b/c/e**: absent exemplar props write nothing, template ctor leaves
  bitmap/is-male/sign uninitialised (reimplement 0/false/0); ini reads = framework-rz-files §3.5 (case-insensitive,
  `%ld`), nummembers result untested + unsigned loop; picture folder is `<ud>\Web Pages\<value>\FamilyGFX\` under
  the UserData dir, not the install path; F.ini next to the .pub (matches); record length byte = len mod 256 with
  that many bytes (no 255 cut); UserData separator always inserted (matches); cSLString miss -> empty (matches).
  Conformance rows added.
- Kept separate (no claims): trip automaton, CheckCommute pathfinder details.
- Residual (minor, not filed): value of `sims_family_nummembers` when the key is missing (prior local value not traced).
- No engine/tools/ABI edits; no game runs.

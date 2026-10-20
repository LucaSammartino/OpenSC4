# Lane: re-netocc

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc (network occupants spec) | `re-netocc` / `.claude/worktrees/re-netocc` (docs lane) | in progress: reading done, no spec text written | write docs/specs/network-occupants.md from the findings in build-runs/research-notes.md; then questions NO1+ in docs/specs/questions/network-occupants.md; register in docs/specs/QUESTIONS.md via tools/questions.py index; final lane report; merge-queue json (kind docs) |

## Handoff
- Goal: NEW spec `docs/specs/network-occupants.md` (cSC4NetworkOccupant family, 754 fns / 114,523 bytes) +
  questions NO1+ in `docs/specs/questions/network-occupants.md`, QUESTIONS.md registration, final lane row +
  report, `build-runs/merge-queue/re-netocc.json` (kind `docs`). Do NOT touch engine/ or tools/.
- Done: all RE reading done. Every measured fact is in `build-runs/research-notes.md` (class IDs, IIDs,
  layouts, vtables, records, fields, addresses). no-iface-*.txt have the interface rows; vtable-extents.txt the
  primary vtable slot counts. Worktree created; lane row above written.
- Next (fresh agent): Read `build-runs/research-notes.md` FIRST (it has everything), then docs/specs/README.md
  style (skim prop-occupant.md §0-2 + questions/README.md), then write the spec per the original task brief:
  identity/inheritance; per class state + lifecycle + vtable slot table (mark *[compat]*); cSC4NetworkTypeInfo
  (loaders, placement params - network-geometry.md §2.3/§2.4 already cover the tables; cite, do not repeat);
  cSC4NetworkRoutines (one function); occupant↔network-lot relationship (network-manager.md §8.6 CreateLotObjects
  creates props/flora, NOT occupants; the pieces ARE the network lots' geometry); quirks; conformance-tests
  section (names only). Keep under ~100 KB. House style: docs/specs/prop-occupant.md.
- Facts: worktree C:/Users/tlori/FreeCity4/.claude/worktrees/re-netocc; main C:/Users/tlori/FreeCity4; base
  a2e6fdf5; today 2026-10-20; budget 150k; no game runs; no ABI rows proposed so far. NM11 of glue24 was
  written only to a QUESTIONS.md append that the train10 merge dropped (never re-indexed); its content
  (tool classes) is out of this spec's scope; do not re-file it here. `cSC4NetworkLotOccupant` does not exist
  in the binary (only cSC4NetworkLot, network-manager.md §2.3).
- Restart command: `python C:/Users/tlori/FreeCity4/.claude/lane-bin/ctx.py re-netocc --budget 150000` after
  committing.

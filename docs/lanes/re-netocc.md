# Lane: re-netocc

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc (network occupants spec) | `re-netocc` / `.claude/worktrees/re-netocc` (docs lane) | ready: spec + questions written and committed | merge train: docs batch (network-occupants.md, questions/network-occupants.md, QUESTIONS.md index) |

## Report (what changed, measured results, questions, next steps)

- **`docs/specs/network-occupants.md`** (new, ~73 KB): the cSC4NetworkOccupant family. §1 identity/class IDs/
  inheritance + per-class QueryInterface and GetGZCLSID tables; §2 base layout, save record (v8, read accepts 4+,
  with v<5 legacy transform, v<6 ConvertV5Flags, v<7 position recompute), path connections, flags/lighting; §2.8
  prebuilt (+bridge +tunnel); §3 underground/pipe/subway + the connection occupants (derive DIRECTLY from
  cSC4Occupant); §4 power pole (19 slots, record v5); §5 power line (8 slots); §6 pylon (occupant type
  0xA9B6D7DE — measured, corrects the earlier guess); §7 grid/ownership + the occupant↔network-lot rule (no
  cSC4NetworkLotOccupant exists); §8 cSC4NetworkTypeInfo loaders (all property IDs + defaults);
  §9 cSC4NetworkRoutines; §10 model instance; §11 13 quirks; §12 NO1–NO8 filed; §13 25 conformance tests (names
  only).
- **`docs/specs/questions/network-occupants.md`** (new): NO1–NO8 (flag 0x40; pole delegate 0x26A68448; underground
  type; prebuilt Write fields +0x170..+0x180; 0x4835C6A7-AC property names; pipe +0x268; GetSortValue field;
  connection-registration strings). QUESTIONS.md regenerated (prefix NO, next free NO9); `questions.py check` ok.
- Two earlier questions settled during writing and never filed: pole Write version = 5 (PUSH 0x5 at 0x0064B01D);
  pylon occupant type = 0xA9B6D7DE (MOV at 0x005F19F9). Both are in the spec (§4.4, §6.1, §12 NO9/NO10 struck).
- Measured this round (all re_query.py asm / PE reads, no REA needed): cISC4NetworkOccupant = 81 slots (0..80)
  shared by base/prebuilt/bridge/tunnel with per-class overrides (prebuilt 16, bridge 4+tunnel 5 including the
  extra slot 81); underground 106, pipe 110, subway 112 (slots 81.. = the underground interface); connection
  primaries 45 slots; pole 19; line 8; pylon 21. RTTI column boundaries found by scanning for 0xFFFFFFFC.
  Base record Write 0x0061A190 / Read 0x0061CC00 disassembled to field order (the quad/triangle/bbox/ref/int64
  layout is new to the docs; ABI tables only had slot names).
- No ABI rows proposed (the docs/abi tables were sufficient once the compiled-vs-SDK slot notes were cited).
- Next: nothing for this lane; the merge train picks the docs batch.

## Handoff
- Goal: NEW spec `docs/specs/network-occupants.md` (cSC4NetworkOccupant family, 754 fns / 114,523 bytes) +
  questions NO1+ in `docs/specs/questions/network-occupants.md`, QUESTIONS.md registration, final lane row +
  report, `build-runs/merge-queue/re-netocc.json` (kind `docs`). Do NOT touch engine/ or tools/.
- Done: all RE reading done (commit fb505627: research notes; 78414930: handoff commit). Spec written from the
  notes plus this round's vtable-extent and record-field verification (vtdump.py in build-runs, reads the exe
  PE directly, read-only). Spec, questions file and QUESTIONS.md index committed on this branch.
- Next (fresh agent): write `build-runs/merge-queue/re-netocc.json` (kind docs, state ready) and return the
  structured result. The merge-queue JSON is the only remaining deliverable.
- Facts: worktree C:/Users/tlori/FreeCity4/.claude/worktrees/re-netocc; main C:/Users/tlori/FreeCity4; base
  a2e6fdf5; today 2026-10-20; budget 150k; no game runs; no ABI rows. cSC4NetworkLotOccupant does not exist in
  the binary (spec §7.1 says so). NO1–NO8 filed; NO9/NO10 dropped (settled in-spec).
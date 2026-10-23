| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh5 (VH1–VH9 vehicle contracts) | Codex / `re-vehicles-vh5` (reserved detached worktree) | WIP: VH3 and VH8 answered; doccheck PASS; seven questions remain open | Continue VH1/VH2/VH4/VH5/VH6/VH7/VH9; core-vehicles5 remains blocked |

## Report

Partial closure only: this round does **not** satisfy the requested complete VH1–VH9 closure.
Milestone `8f7ed3e9` extends vehicles.md §12 with two evidence-backed contracts and
13 conformance vectors (seven record vectors, six state vectors). The initial
lane record is commit `3553af23`.

- **VH3:** corrects Train's inheritance to TrainCar; specifies both exact-version-2
  wrappers, nested Vehicle record, ordered train tail, requested car/network IIDs,
  failure consumption, null-car skipping, append semantics and absence of wrapper-level
  geometric reconstruction. Folded functions named Bus implement the TrainCar wrapper.
- **VH8:** separates initial descent speed, subsequent acceleration and splash ascent;
  specifies the entry wobble range draw and the single splash-duration uniform draw,
  binary32 narrowing and seconds/milliseconds reuse. A loaded nonzero PullOver elapsed
  counter bypasses vector initialization; no reconstruction is invented.
- Both original question entries moved to `docs/specs/questions/answered/vehicles.md`
  with dated DIFFERS answers; `python tools/questions.py index` regenerated
  `docs/specs/QUESTIONS.md`.
- **Still open:** VH1 player physics; VH2 neighbour/look-ahead contracts; VH4 constructor
  and Init contracts; VH5 train-owned algorithms; VH6 collision/damage/blur; VH7 route
  geometry; VH9 inherited states and dispatch hooks. No new questions were filed.

Validation: `python tools/doccheck.py check` **PASS**. This is a documentation-only
RE change; cargo, clippy, native CTest and game/shadow runs are n/a. Conformance
vectors are specified, not implemented or executed. No ABI exports changed and
no proposed ABI rows were needed. No engine/, plugin, capi or coverage-map reads/edits.

Sources: read-only main-checkout export queried through `tools/re_query.py`, bounded
reads of the relevant class decompilations and `_unnamed/region_00550000.c` /
`region_00560000.c`, plus instruction bytes for version checks, car-list reconstruction,
plummet initialization and splash duration. No REA call was attempted: the task
explicitly said its session has no binary open. No REA failure/fallback claim is made.
An ignored local PE-data reader is `build-runs/evidence.py`; it is evidence tooling,
not implementation output and is not committed.

The detailed core closure table is inside prohibited engine/; the lane instead read
the explicit question-to-open-section inventory in `docs/lanes/core-vehicles5.md`.
The research checkpoint was 95,920 / 150,000 measured context units; after writing
the two answers it was 104,847. Research stopped for the requested two-thirds cap.
The remaining contracts cannot responsibly be marked answered from partial reads.

The first milestone staging attempt incorrectly named a nonexistent question index
path, so it committed nothing. Corrected staging used the generated QUESTIONS.md and
produced `8f7ed3e9`. Git also printed a shared packed-refs lock permission diagnostic;
both detached commits completed successfully without changing shared refs.

## Handoff

- Goal remaining: close VH1/VH2/VH4/VH5/VH6/VH7/VH9; do not reopen VH3/VH8 without contrary evidence.
- Done: `8f7ed3e9`, vehicles.md §12.1/§12.2, moved questions, index generated; doccheck PASS.
- Next exact step: query `0x00554EA0` asm and read its bounded decomp to finish VH2 neighbour ranking; then slots 159/167/168/175.
- Slot targets in traffic/vehicle primary vtable: 159=`0x0054DA00`, 167=`0x00550F40`, 168=`0x005527E0`, 175=`0x00504FE0`.
- Unnamed functions live in main decomp/_unnamed/region_00550000.c; train generation/following helpers include `0x005577F0`, `0x00559C30`, `0x0055A050`, `0x0055A8B0`.
- Vehicle constructor is unnamed `0x005628F0` in region_00560000.c; TrainCar ctor `0x0055B150`, Train ctor `0x0055B2F0`.
- Do not trust decompiler field typing/offset arithmetic for numeric algorithms; verify against paged instruction bytes.
- No game requests outstanding; queue state WIP, not an owner-completion claim.

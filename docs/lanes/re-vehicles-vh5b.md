| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh5b (vehicle owner behaviour) | Codex / `re-vehicles-vh5b` (reserved detached worktree) | WIP: VH4/VH6 answered; questions check and doccheck pass; five questions remain | Continue VH1/VH2/VH5/VH7/VH9 before owner closure |

## Report

This is partial delivery, not owner closure. Commit `d309110be` adds vehicles §12.3 (construction/Init),
§12.4 (collision/damage/blur), and partial §12.5 (neighbour ranking/look-ahead). The finishing commit adds
the Train collision override and corrects the old 5000 ms train summary to 20000 ms. No engine, plugin,
capi, tools or ABI files changed. No ABI rows are proposed.

VH4 and VH6 are moved to `docs/specs/questions/answered/vehicles.md`, marked DIFFERS. The question index
is regenerated. VH1, VH2, VH5, VH7 and VH9 remain in `docs/specs/questions/vehicles.md`; no new IDs filed.
The specified Init contract retains failure side effects rather than inventing rollback. Damage selects
named effects rather than replacing a primary mesh. Blur uses four model samples and staggered opacity.
Train collision scales the 3D random vector before discarding Y, has a 20000 ms timeout, and can return
success early on an already-colliding car. Conformance tables cover these boundaries and quirks.

Validation: `python tools/questions.py check` PASS and `python tools/doccheck.py check` PASS at the spec
milestone; both rerun after the final spec/report edit. Cargo, clippy, CTest and game/shadow are n/a for
this documentation-only RE work. No game runs requested or pending.

Evidence was read from the main checkout export/decomp using absolute paths. No REA query was attempted:
the brief says no binary is open. Numeric findings cite instruction addresses; constants were checked
against instruction bytes and the executable's mapped initialized data. Runtime tuning globals are stated
as inputs rather than inferred from file bytes. The local PE helper is ignored under `build-runs/`.

The safe owner report `docs/lanes/core-vehicles5.md` identifies the gaps. Its closure table is under
`engine/`, which this RE lane was forbidden to read; no engine file was opened. The q.py wrapper failed
to start Git Bash with CreateFileMapping error 5; bounded PowerShell reads worked. Git emitted a
packed-refs.lock permission diagnostic during commits, but detached HEAD commits completed successfully;
no shared ref/config repair was attempted.

Broad answer investigation stopped at measured context 99905/150000 (the requested two-thirds limit).
The remainder was used for spec text, conformance vectors, verification of the Train collision correction,
archive/index maintenance and this report. The task's seven-question ready criterion is **not met**;
the merge-queue entry is WIP to avoid presenting unresolved owner algorithms as complete.

## Handoff
- Goal: close the seven vehicle owner questions with behaviour and conformance vectors.
- Done: lane plan c4cc37cec; two answers, partial VH2, vectors and archive/index d309110be; finishing commit adds Train override/report.
- Next: complete VH2 by paging `re_query.py asm 00550f40` and `005527e0` and specifying packed path bytes, direction ranking and fallback paths; §12.5 is partial only.
- Then VH5: `decomp/_unnamed/region_00550000.c` functions `00558510`, Train vtable slots 195–199, and `cSC4Train.c` StartTrip `005588d0`.
- Then VH1: `cSC4Vehicle.c` driving `005603f0`, `0055ea20`, `cSC4Train.c` `005591a0`, Tank `0054b030`; verify all missing pseudocode blocks in asm.
- VH7: path advance/placement helpers, serialized record meanings and route reconstruction remain unresolved.
- VH9: base RandomWalk/PlopJump/Zot/AttachAnim and dispatch getter/setter contracts still need source investigation.
- Owner closure table lives under forbidden engine/; read safe owner report instead; no engine files opened.
- Key corrections: Init has no Traffic-local unwind; cars created on later ticks; injury/crash are effects; Train timeout 20000 ms versus Vehicle 5000 ms.
- Further RE evidence: vehicle speed asm has missing pseudocode blocks; do not use its pseudocode alone for VH1.
- Evidence: main checkout `C:/Users/tlori/FreeCity4`; use re_query.py and decomp only; no REA binary open.
- q.py cannot start Git Bash (CreateFileMapping error 5); use bounded PowerShell commands.
- No engine, plugin, capi, ABI or coverage-map edits permitted.

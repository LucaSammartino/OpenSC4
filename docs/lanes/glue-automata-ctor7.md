| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-ctor7 (owners capture references and construction codec) | Codex / glue-automata-ctor7 | in progress | Audit references and construction record; gate and three house shadow runs to 2032. |

## Handoff
- Goal: fix owners capture reference warning and construction occupant 254/250 byte mismatch.
- Worktree reused at reserved detached HEAD; no branches or main changes permitted.
- Prior crash6 report read; its narrow automata-only runs passed but house shadows exposed regression.
- Implemented: capture pin ownership per entry; separately counted replay acquisitions,
  guarded releases, and abandoned replay cleanup. Added malformed/truncated/failed writer tests.
- Filed LR13: native construction writer identity/QI/temporary lifetime contract missing.
  Kept literal IID checks; no guessed non-null coercion. Added unavailable-IID diagnostics.
- Done: 582d19fda implements the capture accounting and LR13 diagnostics.
- Gate: qbuild all PASS, Rust 6933/0, clippy diagnostics=0, native 245/245,
  CTest 76/76; queued request qbuild-20261024-054455-build-ctest collected to terminal PASS.
- Gate: mandated verify PASS, native 46/46 and CTest 76/76;
  qbuild-20261024-054910-build-ctest collected to terminal PASS.
- Done: run 423 ctor7-house-2032-20261024-a1 clean to 2032, owners 73/73 and
  automata 73/73 exact; ref_warnings=0, foreign_exceptions=0, no construction failures.
- Done: run 424 ctor7-house-2032-20261024-b1 clean to 2032, owners and automata
  73/73 exact; construction 189/189 exact; ref_warnings=0, foreign_exceptions=0.
- Next: submit/collect c1 with the same house switches, then final report/READY queue.
- No game outstanding yet; next unique label ctor7-house-2032-20261024-c1.
- LR13 filed in docs/specs/questions/lot-records.md;
  original construction failure not reproduced in a1; do not claim a codec fix.
- a1 used full-gate DLL prefix b1817bc5. Separate verify rebuilt the same engine source
  to SHA256 a31aa558b0cd48603685d474478c4fd65c715499de038ae9e3c21e0a80e326f1.
- Exact train180 house INI available at main/build-runs/regress-train180-shadows.ini;
  copied settings in build-runs/ctor7-house-template.txt; requests in game-queue/.
- Event-based blocking verdict waiter: build-runs/wait-verdict.py (no qwait/polling).

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
- Next: collect house run ctor7-house-2032-20261024-a1 (event waiter exec session 68550).
- Required gates: stable final qbuild all and queued verify; three unique turbo house shadow runs to 2032.
- Game outstanding: ctor7-house-2032-20261024-a1, exact train180 house switches,
  turbo to 2032; sole request. LR13 filed in docs/specs/questions/lot-records.md.
- Exact train180 house INI available at main/build-runs/regress-train180-shadows.ini;
  copied settings in build-runs/ctor7-house-template.txt (not yet submitted).
- Event-based blocking verdict waiter: build-runs/wait-verdict.py (no qwait/polling).

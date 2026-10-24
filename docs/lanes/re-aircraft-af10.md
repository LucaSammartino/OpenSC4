| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-af10 (aircraft rotor preset AF9) | Codex / `re-aircraft-af10` | ready: AF9 answered; doccheck passed; partial closure of brief | Continue AF3/AF4/AF5; implement AF9 from aircraft §4.1.1 and vectors 14–16 |

## Report

Closed AF9 in aircraft §4.1.1 and moved the original entry to
`docs/specs/questions/answered/aircraft.md`, marked ANSWERED 2026-10-24. Regenerated
`docs/specs/QUESTIONS.md`. The other requested questions AF3/AF4/AF5 remain open;
this is the partial closure explicitly permitted by the lane brief's wind-down rule.

The specification now gives all thirteen initial binary32 rotor words, identifies
InitInstance(0) as the preset installation point, and separates Read from later
runtime reconstruction. InitInstance applies player parameters before AI parameters;
PhysicsInit selects phase 0 and disables the engine. Repeated InitInstance on an
existing runtime model preserves its parameters. A present zero-speed player record
uses yaw-rate maximum 400 as the speed fallback, versus preset MaxSpeed 227 when
no player record exists. Conformance vectors 14–16 cover exact words, that fallback,
overlay precedence, reconstruction and repeated initialization.

Evidence was read from main-checkout re_query.py and decomp/cSC4Helicopter.c;
numeric behavior was checked against instruction bytes and file-backed PE constants.
No REA was attempted (the brief reports no binary open). Preset address 00B09B70,
selection 00538607–00538620, overlay order 00538622–0053863D, phase reset
005361D5–005361EE, Read 00534400–00534456. The first local PE diagnostic paired
the correct bits with the next word's float; the helper was corrected and rerun
before writing the specification. Only corrected-source values appear in the spec.

Validation: `python tools/doccheck.py check` passed, captured in
`build-runs/doccheck-af10.log` by foreground `build-runs/run-doccheck.py`.
The earlier q.py attempt failed to start Bash (CreateFileMapping Win32 error 5),
so it was not a gate result. No cargo/clippy/ctest or game runs were needed for
these documentation-only edits. No ABI exports, proposed ABI rows or engine files
changed. No new questions were filed. AF9 milestone commit: `5fa412f30`.

## Handoff
- Goal remainder: AF3 complete aircraft AI, AF4 lifecycle/AEMS beyond AF9, AF5 player weapons/spotlight.
- Done: AF9 specification, vectors, answered move and index, committed at 5fa412f30; doccheck passed.
- Next: trace AF3 reset/hover using re_query.py asm 00534F70 and 005353C0; then MoveXZ/building-lift and goal switches, or assign a fresh round per broad question.
- Helicopter pseudocode: main decomp/cSC4Helicopter.c reset lines 435–503, hover 505–547. Numeric pseudocode has suspicious signs/casts; verify instructions before specifying it.
- AF9 normal preset is 0, not the second static table at stride 7C. Player zero-speed fallback is yaw max 400; no player record preserves speed 227.
- AF3/AF4/AF5 have not been moved or claimed answered. No game labels outstanding, ABI rows, or external dependencies.
- Detached HEAD is intentional; supervisor publishes commits to reserved branch re-aircraft-af10.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-af5 (aircraft parameter and station-mask contracts) | Codex / detached `re-aircraft-af5` | Ready spec increment; AF1/AF7/AF8 answered; AF2–AF6 remain open; doccheck passed; ABI unchanged | Implement parameter/mask corrections; continue RE for five remaining contracts before owner closure |

## Report

The full AF1–AF8 owner-closure task is **not complete**. This increment resolves
three questions and leaves the five broader flight-owner contracts open. It is
ready to land as a documentation correction, not as evidence that either owner
is closed. Research stopped at the brief's two-thirds measured-context boundary
(the first milestone guard reported 97,172 / 150,000).

### Changes

- AF1: identified Lua automata-group parameter records, their named airplane
  and helicopter fields, complete-record inheritance and conversion constants.
- AF7: specified named missing-field import, empty records, unchecked non-finite
  values, absent cycle detection and exact binary32 airplane default encodings.
  The earlier rounded decimal defaults must not be used as exact constants.
- AF8: specified the station mask's wrapping shift and signed count gate,
  including the previously incorrect count-32 case.
- Added aircraft.md conformance vectors 4–7, archived all three original
  questions with dated DIFFERS answers, and regenerated the question index.
- AF2–AF6 remain in the open question file. No partial flight-model or weapon
  reconstruction has been promoted to a complete compatibility contract.

### Evidence and limits

Queried the main checkout's function/instruction export first, then read selected
local decompilation sections. Numeric parameter and mask rules were checked
against instruction bytes; the spec cites their addresses. Exact constants were
read from the original executable. No decompiled output or binary artifacts are
committed. Only the specifically requested owner-closure README was read under
engine; no engine source was read or changed.

REA batch request for nine missing procedures failed with `target_unavailable`
(no app open), **0/9 succeeded**. As directed, continued with the local export
and decompilation. This is an unavailable-source result, not a successful REA
check. No additional tools or sessions were configured.

### Validation

- `python tools/doccheck.py check`: **PASS** (`doccheck check: ok`).
- `python tools/questions.py check`: **PASS** (`questions: ok`).
- `git diff --check`: **PASS**.
- Cargo, clippy, CTest and game runs: **n/a**, documentation-only lane.
- No C ABI exports or ABI tables changed; no proposed ABI rows.
- Work milestone: `8406c609`; detached HEAD retained. Git printed a denied
  shared packed-refs lock attempt, but the commit itself succeeded; no refs or
  configuration were repaired or modified by this lane.

### Next steps

Implementers can correct parameter loading and mask initialization from the new
contracts. Keep Unsupported(AF2–AF6) until follow-up research supplies verified
physics, AI controls/transitions, lifecycle/effects, U-Drive-It/weapons/spotlight,
and messages/dispatch. No game-request labels are outstanding.

## Handoff

- Goal still outstanding: AF2–AF6, not the three archived answers.
- Done: `8406c609` AF1/AF7/AF8; final report and validation follow in this tip.
- Next exact step: verify airplane CalculateForces basis/angle behavior and UpdatePhysics against export instructions before writing AF2.
- Main sources: C:/Users/tlori/FreeCity4/decomp/cSC4Airplane.c and cSC4Helicopter.c.
- Airplane decomp offsets: CalculateForces line 724; AI_PursueGoal 2775; AI_Land 2357; player commands 1512.
- Airplane Init, UpdatePhysics and runway chooser are stubs locally: use export asm at 005017C0, 00503CB0, 00502520.
- Helicopter offsets: PhysicsInit 626; InitInstance 1532; NextFrame 2113; commands 2197; AI_PursueGoal 3097.
- REA has no open target; batch failed 0/9. Do not retry expecting usable pseudocode without coordinator setup.
- Numeric decompiler types/casts are untrusted until verified in instruction bytes.
- No game runs, ABI changes or ABI-row proposals. Detached HEAD must remain detached.
- build-runs/read-va.py is an ignored read-only PE constant helper, not engine input.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-af10 (aircraft owner closure) | Codex / `re-aircraft-af10` | ready: AF9 answered; partial AF3/AF4/AF5 contracts and vectors 14–27; final doccheck passed | Land committed RE documentation; remaining owner contracts are narrowed in questions/aircraft.md |

## Report

Final round completed 2026-10-24. Ready to land the partial closure permitted
by the final-round brief. AF9 is answered; AF3/AF4/AF5 remain open with precise
residuals in `docs/specs/questions/aircraft.md`. No new owners or question IDs.
Only AF9 was moved to the answered file, in round 1. Moving the other original
questions would incorrectly claim complete owner contracts.

Final milestone `7169ec715` adds aircraft §§4.2.4–4.2.5 and 4.3.1–4.3.2,
with conformance vectors 24–27:

- AF3: two-point building-clearance sampling at P and P+0.3V, auxiliary/base
  height maximum, home-height floor, +20 demand and descending subtraction
  capped at 50. The final requested hover height can fall below the clearance
  plane; this literal behavior is documented as a quirk.
- AF4: first runtime allocation ownership (state, parameters, ten transforms),
  absence of recoverable allocation failure/rollback, post-base Shutdown effect
  stop/release order, runtime/control-buffer frees and false-result propagation.
- AF5: takeover direct-record preference/fallback, converted speed and yaw-rate
  fallback, four zero-preserving gain refreshes, and spotlight pair activation
  gates/status/start/stop ordering. Inactive demand still continues geometry work.

Instruction evidence: main-checkout re_query.py, class decomp, and file-backed
PE constants. Numeric checks cite instruction addresses in each section.
No REA attempted: the brief states no binary is open. No binary/game captures.
The auxiliary height service is specified by its observed call contract;
its physical interpretation is deliberately not guessed.

Remaining AF3: airplane steering/runway/approach/clearance/fire-drop behavior,
helicopter MoveXZ and complete non-chase/claim/RESCUE ordering. Remaining AF4:
constructor defaults, probes, effect construction, pre-base Shutdown scale
handling, complete Init/PhysicsInit order and physics/collision phase triggers.
Remaining AF5: airplane refresh, takeover/release scheduling, full command
translation, base cadence, projectile construction and spotlight aiming/terrain
projection. The new sections do not claim a complete flight tick or spotlight.

Validation: question index regenerated; final foreground
`python tools/doccheck.py check` **passed once**, captured by
`build-runs/run-doccheck.py` in `build-runs/doccheck-af10.log`.
`git diff --check` passed. Cargo/clippy/ctest: n/a; game runs/shadow: n/a.
No engine/plugin/capi/tools coverage-map edits, ABI changes or proposed ABI rows.
Git printed a packed-refs.lock permission diagnostic while committing, but
created the detached-HEAD milestone successfully; no ref/config changes attempted.

## Handoff

Final round finished; no additional RE work queued by this lane.
Land the ready merge-queue entry at its full committed HEAD.
Prior milestones: AF9 `5fa412f30`; controls/chase/phase/weapon `da05bf073`;
clearance/cleanup/refresh/spotlight `7169ec715`.
Owner closure requirements read from main helicopter README lines 35–47.
AF9 already appears only in answered/aircraft.md; never add another open copy.
AF3/AF4/AF5 remain the original questions with narrowed residuals, not new IDs.
Final gate passed; no game runs or ABI proposals outstanding.
Detached HEAD is intentional; supervisor publishes to the reserved lane branch.

## Prior round reports

Round 2 adds seven conformance vectors (17–23) and instruction-verified rules:
aircraft §§3.3.1/4.2.1/4.2.2 give reset/hover controls, chase lead and distance/
opacity gates, and LOITER countdown/ammunition behavior. The literal chase
multiply by 1000 receives an already converted **seconds** delta. LOITER expires
at remaining countdown **≤0**, correcting the strict elapsed >2000 premise.
Read does not restore its drop flag, countdown or ammo; LOITER itself decrements
zero ammo to −1 without a positive-ammo gate.

AF4: §§3.5/4.2.3 give both phase setters' ordered sound requests, airplane ammo
refill and helicopter engine/motion/model notifications. Only the airplane has
a same-phase early return; helicopter side effects repeat. Phase 4 starts rotor
group 9. automata §6.6 now distinguishes these per-owner rules. AF5: §4.3 gives
weapon request ordering and success/failure accounting: laser is a fallback
after missile failure, not a UFO type branch. No projectile setup was invented.

AF3/AF4/AF5 remain open with their precise residual scope appended to their
original questions. No fully answered broad question was moved this round;
AF9 remains in answered/aircraft.md from round 1 with no duplicate entry. This
is partial closure under the brief's wind-down rule, not complete flight ticks.
Remaining work is airplane steering/runway/fire/approach, helicopter MoveXZ/
building lift and non-chase goals, remaining lifecycle/probes/attachments and
physics phase triggers, full player controls/cadence/projectiles/spotlight.

Evidence: read-only main re_query.py instruction bytes, class decomp and
file-backed PE constants; numeric rules cite addresses in the spec. Finite
inputs only for the control vectors. No REA attempted (no binary open).
Airplane Init/Shutdown and helicopter Init/Shutdown have failed pseudocode in
the class exports and still need instruction review. No engine/plugin/capi,
ABI exports, proposed ABI rows or tools/coverage-map.tsv changed.

Round-2 validation: questions index regenerated; foreground doccheck **passed**
through build-runs/run-doccheck.py, captured in build-runs/doccheck-af10.log.
Cargo/clippy/ctest and game runs: n/a for RE documentation. No new question IDs.
q.py failed to start Bash; capped PowerShell and re_query were used. An audio
qgrep search failed on console encoding, not a gate or numeric evidence.

### Prior round 1 report

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
- Goal remainder: complete AF3/AF4/AF5; AF9 already closed at 5fa412f30 (report 41582d462).
- Done round 2: reset/hover, chase gates, LOITER, phase actions and weapon requests; vectors 17–23; doccheck passed, all in this commit.
- Next AF3: instruction review of AI_ControlMoveXZ 005368B0 and LiftOverBuildings 00537130, then non-chase goals and airplane steering/runway/fire/approach.
- Next AF4: Init/Shutdown 005017C0/004FD2C0 (airplane), 0053A410/00539660 (helicopter), then PhysicsInit/probes/attachments and phase triggers beyond the setters.
- Next AF5: player selection/reload/takeover/release, remaining translator controls, base cadence, projectile setup, UpdateSpotlight 00537B80 and aim helper 005372F0.
- Chase h is seconds; distances 192/32, teleport behind lead 128, hover +20. Fade duration is runtime global, no literal measured here.
- Helicopter repeated phase requests repeat side effects; only airplane has an equality early return.
- AF3/4/5 retain original question entries and narrowed residuals. No new answered copies, game labels or ABI row files.
- Guard reported context 125155/150000 before wind-down; no new RE after that report.
- Detached HEAD intentional; supervisor publishes commits. Queue uses actual full HEAD.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc9: watercraft behavior contracts | Codex / re-watercraft-wc9 (detached HEAD) | in progress | Round 2: audit AI targets and shared tween; narrow any transitive model/effect residual. |

## Report

This round answers WC9 and WC5 and narrows WC3/WC4. The requested four-question
closure is incomplete; no whole-owner or runtime equivalence claim is made.

- `e3904bdd8`: confirmed ordered steering sign gates at 0056EC87–0056ECF2;
  corrected WC8-08 to T=-0.1 and L=R=100; archived WC9.
- `809ce4b06`: §5.7.5 grounding/contact/heel matrix, axes, strict boundaries,
  retained speed reduction, publication and feedback; archived WC5. Added
  §3.5 tuning/lifecycle, §5.1.5 subsidiary animation/effect/light boundaries,
  §5.2.1 AI midpoint/consumption gates, and 19 new conformance vectors.
- Remaining WC3/WC4: complete AI target-production formulas, shared tween
  sampling arithmetic and transitive model-loading/effect-controller contracts.
  No original owner calculation may be accepted as a provider input.

Measured validation on substantive commit 809ce4b06 and final report 3156d5ee0:
doccheck check PASS; questions check PASS; diff whitespace check PASS.
Cargo/clippy/CTest: n/a (specs only). Game runs:
none; shadow: n/a. No pending game requests, ABI exports or proposed ABI rows.

Evidence: main-checkout re_query export/instructions with bytes, bounded decomp
reads, and a read-only PE constant/vtable inspection helper retained only under
ignored build-runs/. No REA call: the supplied session has no binary open.
No engine/, plugin or capi files read or changed. The engine lane report was
read; its Owner table apparently requires an engine/ read forbidden by this
task, so the report and spec §10.2 supplied the residual list instead.
Existing attribution is 75 entries / 38,073 bytes, not historical 79 / 38,556.

Git reported packed-refs.lock permission errors during commits, but each
detached-HEAD commit completed successfully. No refs/config/hooks changed and
no maintenance run. Supervisor publication of the actual final tip is required.

## Handoff

- Round 2 active (2026-10-24): reuse detached HEAD 198b9cd62; WC5/WC9 already answered.
- Done round 2: 562a56ff9, §5.2.2 AI targets/publication and §5.1.6 shared tween, 12 vectors; first-branch absolute turn versus second-branch signed turn and NaN-preserving angle cap.
- Done round 2: §3.6 LoadModels control/results, §5.1.7 shared effects binding and seven more vectors; scoped WC3/WC4 answers archived, exact model-only residuals filed as WC10/WC11.
- Next: run final docs/questions/whitespace checks, replace this old report/handoff with round-2 READY report, commit, write ready re queue using full actual HEAD.
- Required endpoint: READY re queue for certified spec additions; retain exactly scoped unresolved model/effect questions if necessary.
- Goal: answer WC9, WC3, WC4, WC5 with prose and conformance vectors; no engine reads or edits.
- Worktree reused at detached HEAD; main/export are read only.
- Read core-watercraft9's lane report in its worktree: WC9 exists there but has not reached this worktree.
- Owner closure table is apparently in engine's module README, which this task expressly forbids reading. Use the engine lane report's remaining-work list and spec §10.2 instead.
- Current measured owner attribution in spec: 75 entries / 38,073 bytes, not historical 79 / 38,556.
- Done: WC9 sign gates verified at 0056EC87–0056ECF2; corrected WC8-08 to T=-0.1, L=R=100 and recorded answer.
- Done: §5.7.5 grounds/heels with explicit quaternion matrix, axes, contact/publication ordering; WC5 moved to answered. Nineteen new vectors added.
- Done: §3.5 tuning/lifecycle, §5.1.5 subsidiary animation/publication and required rope effect, §5.2.1 AI midpoint/consumption gates.
- WC3/WC4 remain open: full AI target formulas, shared tween sampling and transitive model/effect internals were not completely audited. Do not claim whole-owner closure.
- Final doccheck/questions/whitespace checks passed; changes committed. Queue is WIP because WC3/WC4 remain unresolved. No more RE queries this round.
- Successor: start with Engine_AIBoatMotion 00571880 turning branches/target signs, then common TweenStruct GetCurrent and transitive LoadModels 0050E8B0/effect-controller internals. Read questions/watercraft.md and spec §10.2 first.
- No game requests, ABI changes, or REA calls.

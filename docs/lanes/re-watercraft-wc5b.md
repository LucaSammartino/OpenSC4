| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc5b (watercraft remaining contracts) | Codex / `re-watercraft-wc5b` (RE) | ready: incremental specs; WC3–WC5 open | Land verified additions; continue crash, AI and inherited audit before owner closure. |

## Report

This continuation adds verified behavior and eight conformance vectors to
watercraft.md. It does **not** finish the whole owner. WC1/WC2 were already
answered; WC6–WC8 were not re-audited. WC3/WC4 residuals are narrowed in
the open question file; WC5 remains open without a new transform claim.

- `f252a4e3`: §3.3 adds movement-gate short-circuit/state-query order,
  straight-integrator arguments/output ownership, and docking cleanup distance,
  ignored result and manager-null behavior. §5.1.4 specifies inherited animation
  dispatch, tween completion, generic seek equations, two time arguments and
  unconditional normal success. It also gives six-key wake request order,
  forwarded arguments, ignored results and per-stop effect-update calls.
- `76f80af7`: reconciles stale §10.1 status (normal attitude, footprints and
  WC2 caller were already resolved) and records remaining WC3/WC4 work.
- Validator confirms 124 unique named spec-side vectors, eight added here;
  these are requirements for implementers, not executed engine tests.
- Gate: doccheck, questions index/check, diff check and vector/open-question
  validator passed. Cargo/clippy/ctest and game runs: n/a for this spec lane.
- Evidence: main-checkout read-only export queries, local decomp and PE table/
  constant checks; arithmetic/order citations are in the spec. No pseudocode,
  assembly listings, source dumps or evidence scripts committed. No REA call
  or REA failure. q.py failed with Bash CreateFileMapping access denied;
  bounded native PowerShell reads and export queries succeeded instead.
- No engine files read or changed, no plugin/capi/coverage-map changes, ABI
  exports/proposed ABI rows, game requests or runs. Detached commits succeeded
  despite shared packed-refs.lock diagnostics; shared refs remained untouched.
- Ready means the incremental documentation can land; owner closure remains
  incomplete. No open question was moved to answered in this continuation.

## Handoff

- Goal: finish the remaining WC3–WC5 behavior contracts and conformance vectors, with an honest residual if unresolved.
- Read prior wc5 report and Handoff, rules, question list and landing format. WC1/WC2 already answered; do not redo.
- Worktree is reserved detached HEAD; commit normally without changing refs.
- Done: added §3.3 helper signatures/failures and §5.1.4 inherited animation arithmetic/dispatch, unconditional normal true result and wake request ordering; eight vectors (124 total). Doccheck and diff check pass.
- Next WC3: remaining route/math (notably 00567ED0/00567FD0), land/tow, tuning/debug signatures and inherited initialization/teardown. Next WC5: crash transforms 0056A440/0056AF90. Next WC4: AI 00571880, far approach 00572770, player-drive, subsidiary slot-83/103 animation and effect-controller/slot-104/107 computations.
- Evidence correction: primary table is 00A9DBB8 from 0056D9BB. Initial scratch lookup at 00A9DB88 was wrong and discarded; no spec claims use it. PE b22a80 is runtime-initialized, not a file constant.
- Main evidence: `C:/Users/tlori/FreeCity4/tools/re_query.py`, `decomp/cSC4Watercraft.c`, `decomp/region_00560000.c` (read-only).
- q.py Bash wrapper fails CreateFileMapping access denied; use native PowerShell with bounded output.
- No engine files accessed, ABI changes, game requests or REA calls.
- Milestone f252a4e3: helper and inherited-dispatch contracts committed. Follow-up reconciles stale inventory status and narrows open WC3/WC4 residuals; no question fully closed this round.
- Answer investigation stopped for report reserve. Sink source read did not establish a complete instruction-verified transform; no new WC5 claim was published. Next source work: 0056A440/0056AF90 complete basis/contact audit, 00571880 AI, 00572770 far approach.

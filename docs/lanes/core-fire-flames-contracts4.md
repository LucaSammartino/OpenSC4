| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fire-flames-contracts4 (settled FF3/FF4 contracts) | Codex / `core-fire-flames-contracts4` (detached reserved worktree) | ready: Rust 5524/0, Clippy 0, native PASS, CTest 68/68 | Land bounded FF3/FF4 source; no additional family round or whole-owner/default credit |

## Report (2026-10-23, final child round 2/2)

Ready for the bounded source change: the same full checker returned terminal RC0 at
07:47:12 UTC, with unchanged engine source. No additional family round, checker restart,
native-original equivalence, or default coverage is claimed.
Parent remains frozen at its first round; this is original family round 3/3.

### Settled implementation retained

Production source is unchanged in this round. Commit `33ce3bf0` corrected FF3 physical
zero-origin walking and safe backing. Commit `cceac07ffda6c3a1ad52c8b25bf7889b2fcd258a`
retains the settled FF3/FF4 implementation and 40 owned fixtures, engine tree
`f7eb6242f0efdfe6bb978010ca99d6a5b764eb60`.

FF3 uses index step six, the first three indices, raw count, ignored requested offset,
and safe backing. PC24 significant arithmetic retains wider exponents and the published
spill, barycentric, area, and store associations. Exact dyadic sqrt and exact reciprocal
qualification remain explicit. FF4 retains temporary header entries 2/5, side-zero
transforms, raw visibility/store and reference replacement, two-transform placement,
old-to-new position ordering, raw stage -1, and shutdown/storage boundaries.

These are individual raw mesh/height/effect/ref/clock primitives, not a host whole-owner
oracle. PC53, irrational sqrt, subnormal/nonfinite and unsafe inputs, and nonzero-side
hardware trig remain unsupported or unqualified as documented. FF1/FF2 helpers, shared
slots, unknown numerical/foreign failure scopes and whole-owner/default/shadow equivalence
remain held. No game run, game binding, ABI export change, or new question in this round.

### Corrected R1 history

The old handoff's statement that no full gate had started was incorrect. The R1 all
started at 08:04:21 BST, journal PID 11396 / checker 19440, on exact `cceac07f` source.
Its 26 raw Rust summaries total **5524 passed / 0 failed / 0 ignored**; Clippy finished
in 40.27 seconds with no lint diagnostics. Native output stopped at compiling fc4-core;
there was no DLL/PDB, CTest log, end/after receipt, or terminal RC. Historical exit and
cause remain **UNKNOWN**, including after the provider's selected-model-at-capacity
failure. Present drainage does not establish that historical exit.

Focused history remains distinct: 28/28, then 30/32 reciprocal restriction failures,
then 37/37, then 40/40 terminal RC0 at 07:56:31-07:57:03 BST. That final focused run
preceded final formatting and the cce commit. Initial fmt failure (1008 diff lines),
encoding/relative-path and apply_patch failures, later in-place fmt RC0, Bash
CreateInstance E_ACCESSDENIED, and packed-refs.lock PermissionDenied despite the
successful cce commit are retained, not replaced with clean-history claims.

Before fixed logs were reused, 22 evidence files were copied with SHA-256 hashes to
`build-runs/contracts4-r1-preserved-20261023-r2/`; `archive-manifest.json` includes the
old journal/stdout, raw Rust/Clippy/native logs, focused record/log, q failure, and
supplied source/host/provider/reservation evidence. No old log was discarded.

### Current canonical all: terminal PASS with unchanged engine source

Exactly one new canonical `qbuild.py --wt <WT> all` started at
**2026-10-23 07:27:59 UTC**, on clean exact `cceac07f` / engine `f7eb6242`.
Journal PID **10496**, checker PID **21264** (creation epoch 1792740480.008031),
exec session **35392**. Command, cwd, creation times, before hashes and paths are in
`build-runs/contracts4-r2-stable-all.json`; transport mapping is in
`build-runs/contracts4-r2-transport.json`. This same session was collected repeatedly
with 60-second waits, and subsequently collected to its actual terminal RC0.

The earlier **07:43:39 UTC** observation, preserved in
`contracts4-r2-progress-074339.json`, recorded the following incomplete state:

- Rust: **5524 passed / 0 failed / 0 ignored**, 26 summaries; **40 flames test lines**
  pass in that final-source workspace run, independently scoped from the old focused run.
- Clippy finished in **0.11 seconds**, no Clippy lint diagnostics; the log includes
  `warn: could not canonicalize path C:\Users\tlori`.
- Native log reaches steps 175/175. `FreeCity4.dll` (18423296 bytes) and PDB
  (101781504 bytes) exist, timestamp 08:29:57 BST. Build-helper terminal exit remains
  unrecorded: do not treat artifact existence or link-start lines as an exit verdict.
- Live own descendants: `fc4verify.py build` PID **35432**, then `sccache.exe
  --show-stats --stats-format json` PID **14916**. No CTest log/footer or count exists.
  Overall all/native exit is **UNKNOWN**, CTest **unqualified**.
- All tracked engine hashes still match the gate's before snapshot, as do all owned
  hashes. Engine SHA-256: `406981615dd8c097e70410be0670133a12450fb73487cb5f84d8d1a06f82a08a`.
  HEAD was still exact cce at this observation; subsequent commits change only this
  report. A late journal comparison of HEAD must distinguish that documentation change
  from unchanged engine content.

Raw logs remain in `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-configure.log`, `fc4verify-build.log` and `contracts4-r2-stable-all.log`.
There was no extra baseline, focused, quick, fmt, or all run and no production edit.

The **same exec session 35392** subsequently returned **exit 0** and wrote its terminal
receipt at **2026-10-23 07:47:12.649402 UTC**. Final results supersede the incomplete
observation above, without changing the old R1 UNKNOWN verdict:

- Workspace Rust: **5524 passed / 0 failed / 0 ignored**, 26 summaries; test exit 0.
- Clippy: **exit 0, diagnostics 0** (`-D warnings`); path-canonicalization warning
  recorded above is retained in the raw log.
- Native build: **PASS 175/175 edges**, 81.1 seconds, successful helper result.
- Full CTest: **68/68 passed, 0 failed**, footer `100% tests passed, 0 tests failed out
  of 68`, real test time 131.28 seconds (helper summary 131.4 seconds).
- `fc4verify: PASS (1116.8 s)` and `qbuild: PASS`; overall journal **terminal_rc=0**.

The receipt truthfully has **source_stable=false**, and fc4verify reports
`[the tree changed during the run]`: before HEAD is `cceac07f`, after HEAD is
`1cdcd5f299fa51a0e369f7e8841328c0bdb106d7`. That intervening commit changes **only this
lane report**, recording the then-incomplete observation while the original checker
continued. Both receipt snapshots have identical engine tree `f7eb6242`, identical
full tracked-engine SHA-256 `406981615dd8c097e70410be0670133a12450fb73487cb5f84d8d1a06f82a08a`,
and identical hashes for every owned file. Engine source was stable for the entire
qualification; no receipt or HEAD-change flag was rewritten to conceal the report
commit. Subsequent edits also affect only this report. No redundant all was launched.

### Operational limits and next step

Fresh supplied host proof was GO0/brief0 with both dependencies landed. Its finite
08:18:46 BST census reported old PIDs absent and own outputs drained; this was not
historical exit proof. This round's targeted CIM attempt was access-denied. The local
psutil argv check found no old operation or new own checker before launch; unreadable
cwd fields are a limitation, preserved in the drainage records. No process/cache/client/
compiler/validator kill, restart, RPC, helper intervention, agent, or provider change
was performed. Supplied root evidence records no R1 process intervention either.

No checker remains outstanding: session 35392 was collected to terminal, not replaced
or interrupted. The cache-statistics wait resolved without process intervention; no
cause or cache-service repair is inferred. The documentation commit also produced a
packed-refs.lock PermissionDenied message while succeeding, confirmed by actual HEAD.
Next: coordinator may land the bounded source and this report. No new family round or
native-original/default/whole-owner credit is authorized. Game: n/a. Questions: none.

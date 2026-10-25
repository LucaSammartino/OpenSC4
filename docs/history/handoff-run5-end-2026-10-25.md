# Run 5 end stop note (archived from docs/HANDOFF.md §0, 2026-10-25)

## 0. Current state — RUN 5 END (Codex lane trial), 2026-10-25 10:00 BST

**STOP is set** (`python tools/lanes.py stop`); the watcher and heartbeat are stopped. Live Codex rounds were
killed at the stop; their worktrees and queue entries are kept. Verified reads 27.6 on the train238 report because
landvalue flickered 71/72 there (27.7-27.8 on clean reports). Clear STOP only when the user resumes.

**Coverage** (`tools/coverage.py`): specified 89.2%, implemented 60.2%, verified 27.6%, replaced 27.8%
(run 5 started at 57.8 / 36.1 / 19.7 / 18.1). Trains 126-238 landed (96 batches), plus tloriato/OpenSC4#16
(amarret, 65 RE spec lanes, merged locally, specified +6.7).

**First thing next run:**
- `infra-subsysreg23` (ready, 196f4e2ef) splits `FC4Subsystems.cpp`/CMake/`plugin_load_test.cpp` into per-owner
  fragments. It needs one more merge of main (keys added in trains 231-238), then land it **alone**. It removes
  the hotspot conflicts that forced most recuts this run.
- Shadow summary labels: `nettool_plan_shadow`, `powerline_plan_shadow`, `dbpf_bounds_shadow`, `formats_shadow`
  ran exact in the train237 house shadows run (log lines `nettool plan shadow ... mismatches=0`,
  `dbpf_bounds shadow ... clean`, `formats shadow: total ... EXACT`), but the regress summary emits no
  `<key> N lines` row for them, so coverage.py cannot credit them (cSC4NetworkTool 1.9%, cSC4PowerLineTool 0.26%,
  cGZDBSegment*, exemplar rows). Fix the log prefixes or the summary, then credit. `gzfw_shadow` is credited.
- The merge train's regress runs **main's** `tools/regress.ps1`: shadow-profile keys must be on main before the
  batch whose regress should measure them.

**Ready, unlanded** (queue entries, no credit claimed): harvest-orphans16 (view3d conflicts; r7 merging main),
glue-floraocc-replace22, glue-regionalcity-replace23 (r5: FFI handle owner; Claude review found the replace key
needs an owned RegionalCity ABI, >1k lines, save path), glue-overlays-replace17 (r6: native manager interception),
glue-netlotobj-replace22, glue-netocc-factories24, core-textedit-close14 (newer tip), core-vehicles-close21,
glue-viewlight-shadow25, glue-watercraft-shadow25, glue-cfgrule-shadow25.

**Parked:** glue-lotrec-replace18 (spec §15.1 boundary), fix-shadowcrash23 (Codex cyber filter refused it).

**Known issues:** landvalue 71/72 and propmgr 219/220 flicker one month in some house shadows runs (each costs
~0.2 verified on that report); `plugin_autotest_test` watchdog timings fail under 13-lane load (re-gate passes);
lane worktrees' `engine/build-x86` can hold stale objects whose ninja deps point at `../train-integration`.

The stop note this replaces is archived verbatim in
[handoff-user-stop-2026-10-23-1110.md](history/handoff-user-stop-2026-10-23-1110.md).

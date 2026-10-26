# Running the game

Coordinator/harness-maintainer reference. Lanes use [harness.md](../harness.md) and their
[prompt](../../.claude/workflows/fc4-lane.js). Plugin switches are documented in `engine/README.md`.

- **Batch regression:** after the integration gate, run detached
  `tools/regress.ps1 -Tag <tag> -Dll <dll> -Queue`. The script owns shadow/replacement lists; do not copy them here.
  It submits coordinator-priority shadows and replace requests, turbo to 2032-01-01 on Konradshohe by default
  (`-TurboTimeout` 1800 s). Output: table, `REGRESS <tag>: PASS/FAIL`, delta against the previous same-pacing PASS
  (`-Baseline <tag>`). Read the shadow columns: `traffic_shadow_hypotheses = 7` still has known differences
  (TS63 state-15 stop, TS64 scanning iterator); compare its baseline. `terrain` records exact/total, `unf`
  (unfollowed: a problem), `late` (exact, message after month reread) and `pend` (exact so far at shutdown).
  `disasters_shadow` requires scripted disasters and stays out of the standard profile.
  `-ShadowSplit N` (1..5, default 1) partitions the current single house shadow profile and any `-AddShadows`
  extras into N groups; N > 1 requires `-Queue`. All groups and the replacement request are submitted before
  waiting. Longest-first balancing uses isolated traffic/demand/pollution/resource timings projected to the
  actual 2032 horizon, the measured residential group, and proxy timings for remaining shadows.
  Probe dates can overshoot during shutdown: project the simulation interval to the end-date event and
  retain the measured shutdown tail separately. Helper keys stay with their owner.
  Occupant/property/lot readers, automata/owners, resource-cache/weather readers, and overlay/simgrid readers
  each stay together. Per-group logs remain as evidence; `regress-<tag>-shadows.log` concatenates them with
  named headers. The ordinary summary sums shadow lines and uses the worst group verdict (missing or blocked
  groups fail). The `profile:` line records `shadows combined` or `shadows split N`; coverage and delta readers
  use the same summary format. `merge-train.py regress` and `run` keep the combined profile by default
  pending strict parity proof. `--split` opts in to 3 groups, except every fourth batch still uses 1;
  `--combined` forces 1. These flags are mutually exclusive. The train state and
  `build-runs/regress-<tag>.json` record the chosen profile.
  Read the shadow column, not only the verdict. `-Parallel` (direct slots 1 and 2) only when the queue is stopped.
  `tools/run_summary.ps1 -Log <log>` summarises any single run.
- **One run:** `tools/ingame_run.ps1 -Label runN -AfterLoadSeconds 150 -Seconds 400 -Ini $ini`, with the ini built in
  PowerShell with real `` `n `` newlines, for example
  `autoload_city = C:\Users\tlori\Documents\SimCity 4\Regions\FC4 Test\City - Konradshohe.sc4`, `popup_dialogs = 0`,
  `autotest_unpause = 2`, `autotest_end = quit`, plus the keys under test. It writes `Plugins\FreeCity4.ini`, clears
  the log, starts the game from this shell (not elevated, so it can be killed; never use the Start-menu shortcut),
  waits for the plugin's `AUTOTEST VERDICT`, copies the log to `build-runs/<label>.log`, and restores the city.
  `-Actions "<s>=<sc4ctl command>; ..."` drives the UI (see `tools/sc4ctl.ps1`; the 3D view needs `realdrag` /
  `realclick`, posted clicks work for the UI).
- **ini keys:** `enable = a,b` / `disable = a,b` choose subsystems; `<sub>_shadow = 1` runs a same-process shadow;
  `budget_default` is the value of every game-call switch the ini leaves unset (not "all subsystems on"). Full list:
  `engine/README.md` and `engine/plugin/FC4Config.h`.
- **Test cities:** autoload works only inside the region the game starts in (FC4 Test). Konradshohe (`864E0A88…`)
  has a water system (2 producers, 403 pipes); Spandau has none. Backups with verified hashes are in
  `build-runs/city-backups/`.
- **Plugins folder:** every harness run rewrites the ini and stages its own DLL. Before manual play, stage a main build
  (DLL and PDB as a pair) and reset the ini. Read the staged DLL's hash from the log's attach line.
- **Verdicts:** [harness.md](../harness.md#what-comes-back) owns their meanings.
- **Popups:** each advisor popup calls `HiddenPause()` on our simulator, so `hidden` in the heartbeat counts open
  popups. Esc/Enter do nothing, Close works. Use `popup_dialogs = 0` (set at ConnectionsReady, so power, plumbing and
  neighbours still connect); the harness also clears dialogs it recognises.
- **Hazards:**
  - Launch long runs detached (`Start-Process powershell -WindowStyle Hidden ...`); a background tool call stops
    after 10 minutes. PowerShell `*>` output is UTF-16: `tr -d '\000\r'` before grep.
  - Never `tail -F` the plugin log from a monitor: the orphaned tails lock `FreeCity4.log`. Poll with grep.
  - If a run aborts, check for a live `SimCity 4` process before the next one.
  - Writing the ini from bash passes `` `n `` literally; build it in PowerShell and read the bytes back.
  - A manual quit shows as `exited (code 0) without a verdict` with an orderly `PostAppShutdown` in the log.

## The game queue

Start the queue detached from the main checkout and leave it running (one default instance at a time; a second one
exits; see `-Instance` below for a second queue):

```
Start-Process powershell -WindowStyle Hidden -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File C:\Users\tlori\FreeCity4\tools\game_queue.ps1 -Slots 6"
```

It logs to `build-runs\game-queue.log` of that checkout. `-Once` runs what is queued and exits; `-Roots a,b` scans
only those checkouts. Slot games live in `build-runs\sc4slots\slotN` of the main checkout (the game ignores a
`-UserDir` with a path component that starts with `.`, such as `.claude`, and falls back to Documents). Stopping the
queue does not stop its games. Restart at a quiet moment: stop queue, `ingame_run` children and games together;
otherwise old games hold slots and results are lost while requests rerun. Inspect `-Status` first.

**Your requests go first.** A request in the main checkout's `build-runs\game-queue\` is taken before any waiting lane
request (a running game is never stopped; yours gets the next free slot). With `priority = high` it also goes before
your other requests and does not wait for them: several run at once, one slot each (the main checkout's normal
requests still run one at a time, in order). `priority = high` in a lane's request is ignored and noted in its
verdict. The queue's start line in the log says `priority: coordinator <root>`.

**Regression through the queue.** `tools\regress.ps1 -Tag <tag> -Dll <dll> -Queue` submits its two runs as
`priority = high` requests (`regress-<tag>-shadows`, `regress-<tag>-replace`), waits for their verdict files, copies
the logs to `build-runs\regress-<tag>-<run>.log` and prints the same blocks, table, delta and verdict line as
`-Parallel`, plus each run's queue line (slot, time waited) and the wall time. The DLL must be inside the main checkout
(`build-runs\dlls\FreeCity4-<name>.dll` is fine); one outside it is copied to `build-runs\dlls\regress-<tag>.dll`
first. A request that gets no slot within `-QueueWait` seconds (default 3600) is taken back and reported BLOCKED.
`-Parallel` starts games directly in slots 1 and 2: use only with the queue stopped (a held slot blocks after 60 s).

**Regress profile.** `-EndDate` defaults to 2032-01-01 with `sim_turbo = 1`; `-NormalSpeed [-Seconds 150]`
selects a timed run. `-Turbo` remains accepted.

**Pacing.** Original work can depend on frames/real time; cross-run baselines/digests require matching pacing.
Same-process shadows are unaffected. The `profile:` line records turbo/date or normal/seconds (absent means
normal); the table records days/s. `-AnyPacing` overrides a profile mismatch with a warning. Different end dates
or a greater-than-2x days/s difference also warn.

**Queue status.** `powershell -File tools\game_queue.ps1 -Status [-Last 10]` prints, without disturbing the queue, each
slot and what it runs (lane, label, elapsed, game pid), the waiting requests in the order the queue will take them
(lane, label, class, age; "after its root's earlier request" for a lane's later ones), and the last results with
their verdict words. It reads queue logs, `.running` markers and request folders.
`-Check <file>.req` prints what the queue would run for one request, or the exact
rejection it would write.

**More slots, more instances.** `-SlotList 4,5` runs in those slots instead of 1..`-Slots`. `-Instance <name>` starts
a second queue next to the default one, with its own mutex (`FC4GameQueue-<name>`), log
(`build-runs\game-queue-<name>.log`) and work folder; `-CoordinatorRoot <dir>` sets which root has priority (default
the main checkout). Instances of this version claim each request and each slot with a named mutex, so two of them can
scan the same roots without running a request twice. Give each instance its own
slots. Slot N runs on logical CPU 2N: on this i7-12700K, slots 1 to 7 each get their own P-core (CPUs 2 to 14),
slots 8 and 9 land on E-cores (16, 18) and run slower.

Use `-Slots 6` (CPUs 2-12), leaving P-cores 0/7 and E-cores for builds/system. Seven slots is the maximum
with one P-core per game; do not use 8 or 9. Build load on a pinned core slows its game.

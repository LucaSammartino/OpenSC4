# Running the game

Moved from HANDOFF §3 on 2026-10-20. Who reads it: the coordinator, engine lanes that queue game runs, and
tooling lanes that touch the harness. Switch and ini key details are in `engine/README.md`; the harness
itself is [docs/harness.md](../harness.md).

- **Regression after every merge that touches the plugin** (run detached):
  `tools/regress.ps1 -Tag <tag> -Dll <dll> -Queue` submits a shadows run and a replace run (32 replacement keys,
  `development`, `tract`, `history`, `clock24`, `commercial`, `industrial`, `civic`, `parks`, `lotdev`, `zonedev`, `traffic`,
  `automata`, `netmgr`, `netlot`, `nettool` and `demolition` among them; every exact shadow, including `zonedev_shadow`, `lotdev_shadow` (since run 132, exact after
  apply13's LD29), `history_shadow`, `clock24_shadow`, `commercial_shadow`, `civic_shadow` and `parks_shadow`, plus
  `traffic_shadow` with `traffic_shadow_hypotheses = 7` as a **known difference**: not exact to 2032 (TS63 state-15
  stop, TS64 scanning iterator; 43 of 72 month lines clean in run 121, the share moves with the pacing), so compare
  its column with the baseline; and `terrain_shadow = 1` since run 138, whose vertex totals go into the table's
  `terrain` column: exact/total per query, `unf` (unfollowed vertices: a problem, counted in the problem column),
  `late` (followed late: exact, its queued message came after the month re-read) and `pend` (pending at shutdown:
  exact so far); `disasters_shadow` stays out (needs scripted disasters, not exact); lists of run 138, baseline
  `build-runs/regress-rl5.out`; the traffic shadow makes the shadows run the long one, 2.5-3.5 days/s, 10-15 min,
  `-TurboTimeout` 1800 s) through the run queue with coordinator priority
  (`priority = high`: they jump the lanes' requests and run side by side), turbo to 2032-01-01 on Konradshohe by
  default (`-NormalSpeed` for the old timed runs), and prints one table, `REGRESS <tag>: PASS/FAIL`, and a delta
  against the previous PASS of the same pacing (`-Baseline <tag>`; the original's results depend on days/s, DG31).
  `-ShadowSplit N` (1..5, default 1) partitions the current single house shadow profile and any `-AddShadows`
  extras into N groups; N > 1 requires `-Queue`. All groups and the replacement request are submitted before
  waiting. Longest-first balancing uses isolated traffic/demand/pollution/resource timings projected to the
  actual 2032 horizon, the measured residential group, and train196 proxies for remaining shadows.
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
  Queue status: `tools/game_queue.ps1 -Status`; check a request without running it: `-Check <file>.req`. The queue
  runs with `-Slots 6` (measured safe; slots 8-9 would be E-cores); restart it with care: stop the queue, its `ingame_run` children and their games together (runs in
  flight are re-run by the new queue).
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
- **Verdicts:** `clean`, `corrupt` (a foreign first-chance AV), `stalled`, `ended` (the game ended the city itself).
  `0xC0000409` is a Rust panic (`panic = "abort"`).
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

## The game queue (moved from docs/harness.md)

Start the queue detached from the main checkout and leave it running (one default instance at a time; a second one
exits; see `-Instance` below for a second queue):

```
Start-Process powershell -WindowStyle Hidden -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File C:\Users\tlori\FreeCity4\tools\game_queue.ps1 -Slots 6"
```

It logs to `build-runs\game-queue.log` of that checkout. `-Once` runs what is queued and exits; `-Roots a,b` scans
only those checkouts. Slot games live in `build-runs\sc4slots\slotN` of the main checkout (the game ignores a
`-UserDir` with a path component that starts with `.`, such as `.claude`, and falls back to Documents). Stopping the
queue does not stop the games it started; their runs end at their timeout, their results are lost, and the restarted
queue runs those requests again (in other slots while the old games still hold theirs). Restart at a quiet moment:
`-Status` (below) shows what runs.

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
Against a queue started before the priority change, regress leaves the key out and the two runs go one after the
other. `-Parallel` still starts the two games directly in slots 1 and 2: use it only while the queue is idle (a slot
the queue holds blocks it after 60 s).

**Regress profile.** Since main `c367684` (the power shadow is exact under turbo) regress runs the turbo profile by
default, in both modes: each run goes to `-EndDate` (default 2032-01-01) with `sim_turbo = 1`, so every run gives the
same month and shadow lines. `-NormalSpeed [-Seconds 150]` runs the old timed profile; `-Turbo` is still accepted.
The shadows run includes `development_shadow = 1` and the replace run `development` (our development unit runs about
3x slower per game day than the original, 23-29 game days/s under turbo against 80-90). Since run 121 the shadows run
also has `traffic_shadow = 1`, which makes it the long one (2.5-3.5 game days/s, 10-15 minutes to 2032; the turbo
timeout is 1800 s), and the replace run `traffic`. Since run 138 the shadows run also has `terrain_shadow = 1`; its
vertex totals (exact/total per query, unfollowed, followed late, pending at shutdown) are the table's `terrain` column,
and unfollowed vertices count as problems.

**Pacing.** With the original simulators a city run at about 85 game days per real second ends with other income and
funds than at 8-20 days/s (some original work is bound to frames or real time). Shadow lines are unaffected (both
sides run in one process at one pacing), but a comparison across runs (`-Baseline`, digests) needs runs of the same
pacing. Regress records it in its `profile:` line (turbo to <date>, or normal with its seconds; an older `.out` without
the line counts as normal) and the measured days/s per run in the table. The delta picks a baseline of the same
profile only, refuses a `-Baseline` of the other profile (`-AnyPacing` compares anyway, with a warning), and warns
when the end dates differ or a run's days/s differ by more than 2x (a loaded machine slows turbo runs: in the slot
measurements below the same run did 16 to 43 days/s depending on the lanes' builds).

**Queue status.** `powershell -File tools\game_queue.ps1 -Status [-Last 10]` prints, without disturbing the queue, each
slot and what it runs (lane, label, elapsed, game pid), the waiting requests in the order the queue will take them
(lane, label, class, age; "after its root's earlier request" for a lane's later ones), and the last results with
their verdict words. It reads the queue log(s), the `.running` markers and the request folders, so it also works on a
queue started before this change. `-Check <file>.req` prints what the queue would run for one request, or the exact
rejection it would write.

**More slots, more instances.** `-SlotList 4,5` runs in those slots instead of 1..`-Slots`. `-Instance <name>` starts
a second queue next to the default one, with its own mutex (`FC4GameQueue-<name>`), log
(`build-runs\game-queue-<name>.log`) and work folder; `-CoordinatorRoot <dir>` sets which root has priority (default
the main checkout). Instances of this version claim each request and each slot with a named mutex, so two of them can
scan the same roots without running a request twice; a queue started before this change does not claim, so do not
run a second instance on the roots it scans (the queue started now scans every worktree). Give each instance its own
slots. Slot N runs on logical CPU 2N: on this i7-12700K, slots 1 to 7 each get their own P-core (CPUs 2 to 14),
slots 8 and 9 land on E-cores (16, 18) and run slower.

**How many slots.** Measured 2026-10-14 with the queue on slots 1-3 and the lanes building at the same time: extra
games in slots 4-9 (`ingame_run -Slot N`, main's DLL, 16 replacements, turbo to 2032-01-01). Up to 8 games at once:
no failed start in 26 extra games, every verdict clean, at least 44 GB free (a game uses about 400 MB), total CPU
55-87% on average. A game's speed depends on the build load on its pinned CPU far more than on the number of games:
slot 4 did 32 days/s with 2 other games and 33 with 7 others a minute earlier, but 16 days/s during a heavy build;
slots 8 and 9 (E-cores) did 22-26 days/s next to 31-40 on P-cores. Use `-Slots 6` (CPUs 2-12; P-cores 0 and 7 and
the E-cores stay for the builds and the system); 7 is the most that keeps one P-core per game; do not use 8 or 9.

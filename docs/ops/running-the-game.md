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

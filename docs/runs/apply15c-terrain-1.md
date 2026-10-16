# Run apply15c-terrain-1

Lane `apply15c`, worktree `.claude/worktrees/apply15c`, DLL `engine/build-x86/FreeCity4.dll`
(sha256 `647ead03` in the harness line).

## Request

| Key | Value |
|---|---|
| label | `apply15c-terrain-1` |
| city | `Konradshohe` |
| seconds | 900 (timeout 1050) |
| turbo | 1 |
| end_date | 2032-01-01 |
| ini | `enable = tutorial`, `terrain_shadow = 1` |

Konradshohe starts on 2026-01-27, so this is a turbo run of about six game years. `terrain_shadow = 1`
is what exercises this lane's TR20 change: the shadow compares our `IsNavigable` (and so our
`HostPowerPole`) against the game's own answers on every sample point.

`enable = tutorial`, `lotcfg` and `propmgr` are **not registered subsystems** in this tree (the
tutorial, lot configuration manager and prop manager have no plugin glue yet), so those keys are
no-ops here. PL12, PL13 and TU5 are covered by their unit and conformance vectors only.

## Verdict

```
verdict: clean
line: AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0,
      ref_warnings=0, julian=2463252, date=2032-01-20, real_seconds=47.781,
      days_per_second=46.04, switches_off=-, disabled=-)
```

Queued 2026-10-16 13:43:13, started 13:43:30 in slot 2 after 17 s in the queue, wall 61 s.
Run 13:43:33 to 13:44:20. Last date 2030-12-01 in the heartbeat; the turbo clock reached
2032-02-01 (the next month boundary after `end_date`).

## Problems (all 0)

| Problem | Count |
|---|---|
| foreign first-chance exceptions | 0 |
| watchdog stalls | 0 |
| reference warnings | 0 |
| Rust panics | 0 |
| structured exceptions | 0 |
| shadow INCOMPLETE lines | 0 |
| terrain unfollowed vertices | 0 |
| other WARNING lines | 0 |

Expected: 2 x87 start-up warning lines (foundations 18.2); the plugin re-asserts 24-bit.

## Shadow summary

| Subsystem | Lines | Exact |
|---|---|---|
| terrain | 92 | 92 with every mismatch count 0 |

73 terrain shadow month lines, `unfollowed>0` in 0 and `ahead>0` in 3.

Terrain shadow totals from its city shutdown line:

```
terrain queries   7/7 exact: altitude 21650/21650, normal 21004/21004, water 36742/36742,
  land 36742/36742, depth 36742/36742, type 36742/36742, navigable 36742/36742;
  other checks 16927, mismatched 0
terrain vertices  unfollowed 0, followed late 8, pending at shutdown 0
```

**`navigable 36742/36742`** is the TR20 check: every navigability query our engine answered,
including the power-pole cell test, matched the game's own `IsNavigable`. `ref_warnings=0` also
covers the new `QueryInterface` / `FC4_RELEASE` pair in `HostPowerPole`.

Shutdown lines: budget Shutdown.

## Artifacts

- `build-runs/game-results/apply15c-terrain-1.verdict`
- `build-runs/game-results/apply15c-terrain-1.log`

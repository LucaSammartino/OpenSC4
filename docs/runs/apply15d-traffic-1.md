# Run apply15d-traffic-1

Lane `apply15d`, worktree `.claude/worktrees/apply15d`, DLL `engine/build-x86/FreeCity4.dll`
(sha256 `858cd698` in the harness line).

## Request

| Key | Value |
|---|---|
| label | `apply15d-traffic-1` |
| city | `Konradshohe` |
| seconds | 900 (timeout 1050) |
| turbo | 1 |
| end_date | 2032-01-01 |
| ini | `enable = traffic`, `traffic_shadow = 1`, `traffic_shadow_hypotheses = 7`, `traffic_shadow_lines = 1` |

Konradshohe starts on 2026-01-27, so this is a turbo run of about six game years. The request was
written as the task asked: the traffic replacement **and** its shadow together.

**The shadow does not run in this combination.** The plugin says so itself at start-up:

```
traffic shadow: requested (traffic_shadow from FreeCity4.ini) but INACTIVE: it needs FreeCity4's
simulator and the original traffic simulator (do not enable traffic); here the simulator is ours
and traffic is ours
```

and again at city load: `traffic shadow: inactive: the city's traffic simulator is FreeCity4's own
(the shadow compares against the ORIGINAL traffic simulator: do not enable traffic)`.

That is the point of a shadow: our core runs beside the game's own simulator on the same inputs.
With `enable = traffic` ours *is* the simulator, so there is nothing to compare against. The
shadow half of the requested pair therefore lives in `apply15d-shadow-1`, which drops
`enable = traffic` and keeps the shadow switches. This run is the replacement-in-game half.

## Verdict

```
verdict: clean
line: AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0,
      ref_warnings=0, julian=2463234, date=2032-01-02, real_seconds=38.828,
      days_per_second=56.37, switches_off=-, disabled=-)
```

Queued 2026-10-16 14:35:53, started 14:35:59 in slot 1 after 6 s in the queue, wall 51 s.
Log 14:36:01 to 14:36:35. `ref_warnings=0` — the TG25 borrow did not disturb any reference count.

## Problems (all 0)

| Problem | Count |
|---|---|
| foreign first-chance exceptions | 0 |
| watchdog stalls | 0 |
| reference warnings | 0 |
| Rust panics | 0 |
| structured exceptions | 0 |
| shadow INCOMPLETE lines | 0 |
| other WARNING lines | 0 |

Expected: 2 x87 start-up warning lines (foundations 18.2); the plugin re-asserts 24-bit.

## Shutdown

`traffic Shutdown; budget Shutdown` — the TG26 path: the traffic-source iterator's connection
snapshot is released before the connections go with the city (traffic-simulator.md §1, §4.1).

## What this run covers

- **TG23** — `update_network_costs` wired, so `UpdateRoadsBudget` / `UpdateMassTransitBudget`
  rewrite the department line items at their five call sites (every road, rail and subway
  insert or remove, every CreateMaps pass, PostCityInit). The run is clean with the budget
  replacement active.
- **TG24** — `path_info_exits_from_entry` and `path_info_entries_near` wired, so
  `RemoveBadElevatedPaths` (lane continuity) runs against the game's real path infos on every
  cell-graph rebuild.
- **TG25** — `tunnel_paired_portal` wired as a borrow of the counted `+0x1D0` member. No
  reference warning in six game years of rebuilds is the refaudit evidence.
- **TS64** — state 13 drives the saved scanning iterator.

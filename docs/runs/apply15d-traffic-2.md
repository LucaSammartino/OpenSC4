# Run apply15d-traffic-2

Lane `apply15d`, worktree `.claude/worktrees/apply15d`, DLL `engine/build-x86/FreeCity4.dll`
(sha256 `858cd698` in the harness line).

## Request

| Key | Value |
|---|---|
| label | `apply15d-traffic-2` |
| city | `Konradshohe` |
| seconds | 900 (timeout 1050) |
| turbo | 1 |
| end_date | 2032-01-01 |
| ini | `enable = traffic` |

The traffic replacement on its own, no shadow, so the replacement is judged without the shadow's
own cost (and so the two runs are not the same request twice). Same city and dates as
`apply15d-traffic-1`, which asked for the shadow as well.

## Verdict

```
verdict: clean
line: AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0,
      ref_warnings=0, julian=2463236, date=2032-01-04, real_seconds=38.813,
      days_per_second=55.98, switches_off=-, disabled=-)
```

Queued 2026-10-16 14:36:07, started 14:36:51 in slot 1 after 44 s in the queue, wall 51 s.
Log 14:36:53 to 14:37:27. `ref_warnings=0`.

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

Two runs of the same request shape (`apply15d-traffic-1` with the shadow switches, this one
without) both reach 2032 clean at 56 days/s, and both report `ref_warnings=0`, so the new
callbacks' reference behaviour is the same with and without the shadow present.

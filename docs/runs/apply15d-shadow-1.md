# Run apply15d-shadow-1

Lane `apply15d`, worktree `.claude/worktrees/apply15d`, DLL `engine/build-x86/FreeCity4.dll`
(sha256 `858cd698` in the harness line).

## Request

| Key | Value |
|---|---|
| label | `apply15d-shadow-1` |
| city | `Konradshohe` |
| seconds | 1200 (timeout 1350) |
| turbo | 1 |
| end_date | 2032-01-01 |
| ini | `traffic_shadow = 1`, `traffic_shadow_hypotheses = 7`, `traffic_shadow_lines = 1` |

Konradshohe starts on 2026-01-27: about six game years. `enable = traffic` is **absent** on
purpose — the shadow compares FreeCity4's core against the *original* traffic simulator, and the
plugin refuses the combination (`apply15d-traffic-1` documents the exact message).
`traffic_shadow_hypotheses = 7` is re-answers13's bits 1/2/4 (TS55, TS56, TS57), which that
answer confirmed.

## Verdict

```
verdict: clean
line: AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0,
      ref_warnings=0, julian=2463234, date=2032-01-02, real_seconds=399.344,
      days_per_second=5.42, switches_off=-, disabled=-)
```

Queued 2026-10-16 14:38:01, started 14:38:03 in slot 1 after 2 s in the queue, wall 410 s.
Log 14:38:05 to 14:44:47. The shadow doubles the work, hence 5.42 days/s against the
replacement's 56.

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

## Shadow summary

```
traffic         89 lines, 55 with every mismatch count 0
```

89 lines = 72 month lines + 17 day lines. Month lines exact: **55 of 72**. Day lines exact: **0 of
17**. No `INCOMPLETE` line anywhere.

## What differs

Every one of the 17 day lines is the same pacing difference:

```
traffic shadow 2027-04-05 day (state 18 -> 15, progress 0, units 191): mismatches=290;
  run: ours ran past the original's stop (state 15, progress 0) ...
```

That is §3.4 *[choice]*: FreeCity4 replaces the original's 4 ms deadline with a count of work
units (`trafficUnitsPerSlice`), so at the end of a trip-phase day ours runs a little further into
the next phase than the original's deadline allows. The original's stop is at the 18 → 15
boundary every time. This is the difference HANDOFF's `regresslists3` row already recorded as
"the share moves with the pacing", not a behaviour difference.

One further day line is a trip-list day (`2027-02-13`): `trips of lot (24, 95) (not written by
ours)`, the same known trip-list day difference.

The 17 differing month lines carry record-field mismatches in exactly four categories, always in
equal counts:

| Field | Count on a 2-mismatch line | On a 4-mismatch line |
|---|---|---|
| cycle state | 2 | 4 |
| phase positions | 2 | 4 |
| scanning iterator | 2 | 4 |
| statistics (working) | 2 | 4 |

These are the four fields that record *where each simulator was when the month line was taken*.
Their always-equal counts are the signature of the pacing difference above: when the sample lands
mid-phase, all four differ together. On the 55 clean month lines the record is compared field by
field with `mismatched=0` — for example `record fields compared=2310 mismatched=0`.

## TS64 specifically

No difference line mentions state 13, the scanning region or the subnetwork info, and the
`scanning iterator` field never mismatches on its own or more often than `cycle state`. So the
TS64 model (entering 13 resets the saved iterator `+0x818`, the deadline is tested before each
cell, the pass rests at `(x0, z1 + 1)`) shows no residual difference here: whatever the two
simulators write into that field on a clean line agrees. The `scanning iterator` mismatches on
the paced lines are the sampling position, which the pacing difference already explains.

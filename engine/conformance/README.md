<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Conformance vectors

Language-neutral test scenarios for the simulator (`docs/specs/simulator.md`). Each file in
[`vectors/`](vectors/) is a JSON document that describes a starting configuration, a script of operations, and the
expected observable results: agent deliveries, broadcast posts, reference-count calls, return values, and state
snapshots.

The vectors are the **migration contract**. Any implementation of the simulator, in any language, must pass every
vector unchanged. The Rust runner in this crate (`cargo test -p fc4-conformance`) is one implementation of this
document. When the runner and this README disagree, fix the runner.

Vectors are derived from the spec, never from the original program. Each vector names the spec sections it covers
in `spec`. Behaviours that the spec leaves open and that a vector pins down are marked `[choice]` in the vector's
`description`.

## File layout

```json
{
  "format": 1,
  "name": "slow_day_agent",
  "spec": ["5.2", "10.3"],
  "description": "free text",
  "clock": {"start_ms": 0, "auto_step_ms": 0},
  "agents": {"D": {"finish_after": 3}},
  "steps": [ {"op": "add_agent", "agent": "D", "level": 4}, ... ]
}
```

| Key | Meaning |
|---|---|
| `format` | Must be `1`. Runners reject other values. |
| `name`, `spec`, `description` | Documentation only. |
| `clock` | Fake millisecond clock. `start_ms` (default 0). `auto_step_ms` (default 0) is added after **every** clock read. |
| `agents` | Scripted agent behaviours, by label (see below). An agent used without an entry gets the defaults. |
| `steps` | Operations, executed in order. A failing step fails the vector. |

Unknown keys inside an operation are ignored, so `note` can be used for comments. Unknown `op` names, unknown state
fields and unknown event or filter keys are errors.

### Clock rules

The simulator reads time only through the injected clock. With `auto_step_ms: 0` time moves only through
`advance`/`set_clock` and through agent `cost_ms`. Vectors that check exact timings use a frozen clock plus agent
costs, so their results do not depend on how often an implementation reads the clock. Vectors with
`auto_step_ms > 0` check bounds only.

A frozen clock combined with `on_tick` needs some agent with `cost_ms > 0`. Otherwise the frame loop never reaches
its budget. Runners should abort such runaway loops. The Rust runner stops after 50 million clock reads in one step.

The slice timer is restarted only by `on_tick`. Before the first tick it counts from clock value 0, which is also
the clock value when the simulator is created in every current vector (`start_ms` 0). So, outside `on_tick` and
with the clock 15 ms or more past the last slice start, every scheduler step is followed by "budget exhausted". In
practice, a direct `simulate_agents` call then makes exactly one delivery. With the clock still at the slice start,
the budget is never exhausted and `simulate_agents` walks every level. (§3 lists the timers but not their initial
value; this is a [choice].)

### Agents

An agent is identified by a **label** (a string). Each label maps to one opaque handle. Registering the same label
at several levels registers the same target several times.

| Behaviour key | Default | Meaning |
|---|---|---|
| `finish_after` | 1 | Calls needed per interval. The agent returns "finished" on every N-th consecutive call, and the counter resets after it finishes. `0` = never finishes. |
| `cost_ms` | 0 | Added to the fake clock on every call, after the `deliver` event is recorded. |
| `on_call` | `{}` | `{"<n>": [op, ...]}`: operations executed **inside** the n-th call (1-based, counted over the agent's lifetime), before it returns. Used to test re-entrancy. |

### Events

The runner records, in order:

| `event` | Fields |
|---|---|
| `deliver` | `agent`, `type`, `data1`, `data2`, `data3`, `t` (clock when the call started) |
| `post` | `type`, `data1`, `data2`, `data3`, `t` |
| `add_ref` | `agent` |
| `release` | `agent` |

`type` may be written as a number, as a `"0x…"` string, or as one of these names:

| Name | ID | Name | ID |
|---|---|---|---|
| `SimBegin` | 0x66956810 | `SimSpecificDay` | 0x66956818 |
| `SimEmergency` | 0x66956811 | `SimSpecificTime` | 0x66956819 |
| `SimNewMinute` | 0x66956812 | `SimIdle` | 0x6695681A |
| `SimNewHour` | 0x66956813 | `PauseChanged` | 0xAA7FB7E0 |
| `SimNewDay` | 0x66956814 | `HiddenPauseChanged` | 0x4A7FB7E2 |
| `SimNewWeek` | 0x66956815 | `EmergencyPauseChanged` | 0x4A7FB807 |
| `SimNewMonth` | 0x66956816 | `SpeedChanged` | 0x4A7FB7E9 |
| `SimNewYear` | 0x66956817 | | |

A **filter** is an object with any of `event`, `agent` and `type`. An event matches when every given key is equal.

### Operations

Every operation that calls a simulator method accepts an optional `"expect": <bool>`, which is compared against
the method's return value.

**Lifecycle and frame**

| op | Arguments | Calls |
|---|---|---|
| `init` | `tuning?`: `{"ms_per_day": [..], "animation_dilation": [..]}` | `Init` (core part: §9 steps 1, 2, 5–7) |
| `shutdown` | | `Shutdown` (core part: §9 steps 2, 3, 7) |
| `tick` | `count` (default 1) | `OnTick` |
| `simulate_time_passage` / `simulate_agents` / `simulate_idle_time` | | §5.2 / §5.3 / §5.4 |
| `begin_new_interval` | `level` | §5.3 step 1 for one level |
| `repeat` | `count`, `steps` | runs `steps` `count` times |

**Clock**

| op | Arguments | Effect |
|---|---|---|
| `advance` | `ms` | clock += ms |
| `set_clock` | `now_ms?`, `auto_step_ms?` | sets the clock and/or its auto-step |

**Speed, pause, misc** (§6)

`pause`, `resume`, `hidden_pause`, `hidden_resume`, `emergency_pause`, `emergency_resume`; `set_speed {speed}`;
`set_sim_time {time}`; `set_max_ms_per_tick {ms}`; `set_city_established {value}`.

**Notifications** (§9): `post_city_init` delivers notification `0x26D31EC1` (PostCityInit), which calls
`HiddenResume`. Unsubscribing is the host's job and is not modelled.

**Agents** (§4)

| op | Arguments |
|---|---|
| `add_agent` | `agent`, `level`, `name?` (default: the label), `flags?` (default 3) |
| `remove_agent` | `agent`, `level?`. Without `level` this is `RemoveAgent(target)` across all levels. |
| `remove_all_agents` | `level?` |
| `set_agent_enabled` | `name`, `enabled` |

**Cheats** (§9): `cheat {id, text}`. `id` is a number, a hex string, `"FPS"` or `"SimDate"`. `text` is the whole
typed line, for example `"SimDate 3 15 2001"`.

**Persistence** (§8)

| op | Arguments | Effect |
|---|---|---|
| `save` | `slot` | stores the bytes of `Write` under a name |
| `load` | one of `slot`, `hex`, or `record`; `truncate?` (n bytes); `expect?`: `"ok"` (default), `"unsupported_version"` or `"truncated"` | `Read` |
| `expect_save` | one of `slot`, `hex`, or `record` | the bytes `Write` produces now must be equal |

A **record object** describes a save record field by field. It is encoded by the runner independently of the
implementation under test. Missing fields default to the values shown:
`{"version": 4, "in_tick": false, "city_established": false, "pause_count": 0, "sim_speed": 3, "sim_time": 0,
"frame_budget_ms": 15, "level_counters": [0 × 11], "level_fired": [false × 11], "julian": 2451545}`. Use
`"date": {"year", "month", "day"}` instead of `julian` to give a calendar date. Per-level arrays can also be
written as objects keyed by level, such as `{"4": 125}`. Unlisted levels are then 0.

**Expectations**

| op | Arguments | Checks |
|---|---|---|
| `expect_events` | `events`: list of partial event objects; `filter?` | The events recorded since the last `expect_events`/`clear_events` (after filtering) match the list one-to-one, in order. Each expected object checks only the keys it gives. **Clears all recorded events.** |
| `expect_event_count` | `filter?`, `count?`, `min?`, `max?` | number of matching events (does not clear) |
| `expect_spacing` | `filter?`, `min_ms?`, `max_ms?` | `t` difference between consecutive matching events (does not clear) |
| `expect_sequence` | `filter?`, `field` (`data1`/`data2`/`data3`), `start`, `step` (default 1) | the i-th matching event has `field == start + i·step` (does not clear) |
| `clear_events` | | forget recorded events |
| `expect_state` | `state`: partial snapshot | see below |
| `expect_date` | any of `julian`, `year`, `month`, `day`, `day_of_year`, `weekday` | the current simulation date |
| `expect_julian` | `year`, `month`, `day`, and any of `julian`, `day_of_year`, `weekday`, `leap_year`, `days_in_february` | the pure date functions (§2.1), including the round trip |
| `expect_agent_enabled` | `name`, `value` | `GetAgentEnabled` |
| `expect_agent_names` | `names` | `EnumerateAgentsByName` (in order) |
| `expect_dilation` | `value` | `GetAnimationTimeDilation` (±1e-6) |
| `expect_paused` | any of `paused`, `hidden`, `emergency`, `any` | `IsPaused` / `IsHiddenPaused` / `IsEmergencyPaused` / `IsAnyPaused` |

`expect_state` fields: `initialized`, `in_tick`, `city_established`, `pause_count`, `hidden_pause_count`,
`emergency_pause_count`, `sim_speed`, `sim_time`, `speed_change_enabled`, `frame_budget_ms`, `gate_level`,
`ms_per_day`, `target_fps`, `julian`, `ms_per_day_table` (array), `animation_dilation` (array, ±1e-6),
`level_counters`, `level_fired`, `agent_counts`, `cursors` (per-level, as arrays or `{"level": value}` objects), and
`idle_cursor`. Cursor values are the index of the agent under the cursor in registration order, or `null` (or -1)
for "at end". Booleans may be written as `true`/`false` or `1`/`0`.

## Adding a vector

1. Name it after the spec section it mainly covers, for example `s5_3_….json`.
2. Prefer a frozen clock and exact expectations. Derive every expected value from the spec, by hand, and say how in
   `description` or `note`.
3. Run `cargo test -p fc4-conformance`. The Rust runner generates one test per file.
4. If the core fails the vector, decide whether the spec, the vector or the core is wrong. Never edit a vector to
   match an implementation without a spec reason.

## Coverage (spec §10)

| §10 item | Vectors |
|---|---|
| 1 date conversions | `s10_1_date_conversions` |
| 2 day cadence at speed 2 | `s10_2_day_cadence_ticks`, `s10_2_day_cadence_free_running_clock` |
| 3 slow day agent | `s10_3_slow_day_agent` |
| 4 week/month/year | `s10_4_week_month_year` |
| 5 one-shot agents | `s10_5_one_shot_agents`, `s10_5_emergency_agents_ticks` |
| 6 pause / emergency | `s10_6_pause_and_emergency` |
| 7 enabled bits | `s10_7_enabled_bits`, `s10_7_disabled_agents_skipped` |
| 8 save round trip | `s10_8_save_load` |
| other sections | `s4_reentrancy_and_cursors`, `s5_1_on_tick_guards`, `s5_2_minute_and_hour_levels`, `s5_3_begin_new_interval_flush`, `s5_4_idle_time`, `s6_speed_and_dilation`, `s6_invalid_speed_from_save`, `s9_cheats`, `s9_post_city_init` |

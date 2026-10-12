<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Conformance vectors

Language-neutral test scenarios for the simulator (`docs/specs/simulator.md`), the shared foundations
(`docs/specs/foundations.md`) and the budget (`docs/specs/budget.md`). Each file in
[`vectors/`](vectors/) is a JSON document that describes a starting configuration, a script of operations, and the
expected observable results: agent deliveries, broadcast posts, reference-count calls, return values, and state
snapshots.

The vectors are the **migration contract**. Any implementation of these subsystems, in any language, must pass every
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
| `publish` | `cycles_to_ms`, `ms_to_cycles` (limit-timer coefficients, §5.5; compared with relative tolerance 1e-6) |

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

A **filter** is an object with any of `event`, `agent`, `type`, or any other event key (as in `expect_events`). An event matches when every given key is equal.

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

**Limit-timer calibration** (§5.5): the runner provides a fake wall clock (`qpc`, ticks) and a fake TSC (`tsc`,
cycles). Both start at 0 and move only through these ops. Every publish is recorded as a `publish` event.

| op | Arguments | Effect |
|---|---|---|
| `set_timer_source` | `qpc_frequency`, `initial_ms_per_cycle` | installs the fake source/sink (`expect` = return value) |
| `set_timers` / `advance_timers` | `qpc?`, `tsc?` | set, or add to, the fake counters. Negative deltas wrap. |
| `calibrate_tick` | | one calibration step, without the rest of OnTick |
| `expect_calibration` | `none?`, `cycles_to_ms?`, `ms_to_cycles?`, `samples?` | the current coefficients (relative tolerance 1e-6) and the number of samples folded in |

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
| `expect_debug` | `debug`: any of `ready`, `one_shot_or_emergency_pending`, `hours_complete`, `day_level_done`, `tomorrow_starts_week/month/year`, `blocked_by_week/month/year`, `day_gate_open`, `interval_elapsed_ms`, `ms_per_day` | the day-barrier diagnostics view (§5.2 step 8): why the next day can or cannot open |

`expect_state` fields: `initialized`, `in_tick`, `city_established`, `pause_count`, `hidden_pause_count`,
`emergency_pause_count`, `sim_speed`, `sim_time`, `speed_change_enabled`, `frame_budget_ms`, `gate_level`,
`ms_per_day`, `target_fps`, `julian`, `ms_per_day_table` (array), `animation_dilation` (array, ±1e-6),
`level_counters`, `level_fired`, `agent_counts`, `cursors` (per-level, as arrays or `{"level": value}` objects), and
`idle_cursor`. Cursor values are the index of the agent under the cursor in registration order, or `null` (or -1)
for "at end". Booleans may be written as `true`/`false` or `1`/`0`.

## Foundations ops

Pure functions of `docs/specs/foundations.md`; they need no simulator state. Vector files are named `f7_…`.

| op | Arguments | Checks |
|---|---|---|
| `stream_write` | `values`: list of one-key objects (`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`, `f64`, `bool`, `str`, `bytes` (hex), `res_key` (`[type, group, instance]`), `null_ref`); `hex` | the bytes written (§2.2) equal `hex` |
| `stream_read` | `hex`; `reads`: list of `{get, expect?, position?}` with `get` one of `u8`, `u16`, `u32`, `i32`, `u64`, `i64`, `f32`, `f64`, `str`, `rz_char_str` (+ `max_bytes`, result as hex), `skip` (+ `n`); `error?` | each read's value; `expect: null` means the read must fail without consuming; `position` after the read; the sticky error flag at the end |
| `crc` | `ascii` or `hex`; `expect` | CRC-32/MPEG-2 (§3.4) |
| `store_entry` | `class_id`, `payload_hex`, `entry_hex`, `index_hex?` | the class-object store entry (and index) written for one object with ID 1 whose `Write` emits the payload (§3.3) |
| `curve` | `points` (flat x, y list), `absent?`, `eval`: `[[x, y], ...]`, `tolerance?` (relative, default 1e-6) | response-curve construction and evaluation (§4) |
| `rng` | `generator` (`RzRandom` or `RzRandomSuper`), `seed`, `wall_clock_seed?`, `process_clock?`, `calls`: `{method, args?, weights?, expect?, lcg?, mul?}` | outputs and internal states, bit-exact (§6) |

| `variant_write` | `variant`, `hex`, `ok?` (default true), `round_trip?` (default true), `position?`, `error?` | `WriteVariant` (§9.2) gives `hex`; unless `ok` is false or `round_trip` is off, the bytes read back to the same variant |
| `variant_read` | `hex`, `preset?` (the destination's value before the read), `expect` (a variant, or `null` = the read fails), `unchanged?`, `position?`, `error?` | `ReadVariant` (§9.2) |
| `holder_write` | `props`: `[{id, value}]` in the order they are added; `hex` | the plain holder record (§10.2) |
| `holder_read` | `hex`, `preset?` (props), `ok?`, `expect?` (props in holder order), `rewrite?`, `position?`, `error?` | reading the plain holder; `rewrite` writes it back and compares with `hex` |
| `exemplar_holder` | `exemplars?` (a stub resource manager: `[{key, props}]`), `preset_parent?` (key), `hex`, `ok?`, `own?`, `parent?` (key or `null`), `lookup?` (`[{id, value or null}]`), `rewrite?`, `position?` | reading the exemplar holder (§10.3); keys are `{group, type, instance}` |
| `crt_rand` | `seed?` (default 1), `srand?`, `expect?` (outputs), `shuffle?` (`{input, expect, calls?}`) | the C runtime `rand()` and `random_shuffle` (§12) |

Floats in `stream_write`, `stream_read` and `curve` may be numbers or `"0x…"` bit patterns (for NaN and −0.0).

A **variant** in a vector is `{"type": T, "value": v}` for a scalar (`T` one of `bool`, `u8`, `i8`, `u16`, `i16`,
`u32`, `i32`, `u64`, `i64`, `f32`, `f64`, `char`, `unichar`, `rzchar`, `voidptr`), `{"type": "T[]", "values": [..]}`
for an array (also `object[]`), `{"type": "string", "value": "…"}`, `{"type": "bytes", "hex": "…"}` (the void
array), `{"type": "empty"}`, and `{"type": "object", "id"?, "class"?}` (no `id` = a null reference; written with
that save ID, read back as the host object whose token is the ID).

## Budget ops

The budget vectors (`b15_…`) drive the budget simulator with a fake host: property holders (named by label), a lot
list, ordinance and neighbour stubs, LTEXT strings, the history warehouse, the disaster layer and the save store.
Every op name starts with `budget_`.

### Environment and objects

| op | Arguments | Effect |
|---|---|---|
| `budget_env` | any of `difficulty`, `year`, `month`, `established`, `city_size` (`[x, z]`), `ordinance_income`, `ordinance_expense`, `neighbor_income`, `neighbor_expense`, `rng_seed`, `advance_month` (true: next month), `disasters` (IDs that exist), `ltext` (`[{group, instance, text}]`) | sets the fake world. Defaults: hard, 2000-01, established, seed 1, everything else 0. Missing LTEXT reads as `LT:gggggggg:iiiiiiii`, building-type names as `BT:iiiiiiii`. |
| `budget_create` | | a new budget simulator (label `budget`); it replaces the current one |
| `budget_init` | `tuning?` (`{"0xPROPID": value or [values]}`, the exemplar of §9; without it, the code defaults), `fix_cs3?` (§13.1 fix), `expect?` | `Init` (core part) |
| `budget_shutdown` | `expect?` | `Shutdown` (core part) |
| `budget_message` | `type` (name or ID), `data1?`, `expect?` | `DoMessage` (§4.3) |
| `budget_cheat` | `id` (`RiskyMoney`, `WeaknessPays`, `Moolah` or an ID), `text?`, `expect?` | the cheat notification |
| `budget_holder` | `holder` (label), `props?` (`{"0xPROPID": {"u32"\|"i64"\|"f32": [...]}}`, replaces all properties), `occupant?` | creates or changes a property holder |
| `budget_lots` | `lots`: `[{purpose, building_wealth, lot_wealth, value, land_value, size_x, size_z, has_building, livable, repeat}]` (defaults 1, 1, 1, 0, 0, 1, 1, true, true, 1) | the lot list `ComputeTax` iterates |

### Calls

`budget_call` (`method`, `args?`) calls the budget simulator, `budget_dept` (`dept`, `method`, `args?`) a department
and `budget_line` (`dept`, `line`, `method`, `args?`) a line item. Methods are the snake-case names of the spec
methods (`deposit_funds`, `issue_bond`, `set_funding_percentage`, `set_local_funding_percent`, `full_expenses`, …;
see `src/budget.rs` for the list) plus `settle_month` (§5.1), `simulate_day` (§5.7), `compute_tax` (§5.2) and
`tamper_funds` (writes `funds` only, for the shadow check). Holders are passed by label. Results are checked with
`expect` (booleans, integers, floats with relative tolerance 1e-6, strings, lists, `null` for a null pointer) and,
for integers, `expect_min` / `expect_max`. Departments and line items return their IDs; holders their labels.

### Checks

| op | Arguments | Checks |
|---|---|---|
| `budget_expect` | `state`: any of `initialized`, `loaded_from_save`, `agriculture_rate_fix`, `panel_flag`, `funds`, `shadow_funds`, `total_borrowed`, `last_tax_income`, `ytd_income`, `ytd_expense`, `ytd_tax_income`, `ytd_reserved`, `ytd_ordinance_income`, `ytd_ordinance_expense`, `ordinance_income_by_month`, `ordinance_expense_by_month`, `ytd_bond_payments`, `last_month_income`, `last_month_expense`, `yearly_income`, `yearly_expense`, `bond_count`, `risky_money_counter`, `department_ids`, `tax` (`{"i": {rate, max_rate, weighted_assessed, monthly_tax, reserved, history}}`), `bonds` (list of partial bonds) | the budget state (§3.1). Arrays may be given as `{"index": value}` objects. |
| `budget_expect_items` | `holder`, `purpose?`, `items` (list of partial `{department, line, purpose, cost, index, local_funding}`, or `null` for none) | `GetBudgetItemInfo`, or `GetBudgetItemForPurpose` when `purpose` is given |
| `budget_expect_loans` | `loans` (partial `{months_remaining, years_remaining, total_loan_amount, remaining_payment, monthly_payment}`) | `GetAllLoans` |
| `budget_expect_groups` | `groups` (partial `{id, name, total_expense, total_income, departments}`) | `GetAllGroups` plus each group's department set |
| `budget_expect_tuning` | `tuning`: any of the §9 values by snake-case name; `land_value_curve` as `[[x, y], ...]` evaluations | the tuning in effect |
| `budget_expect_holder` | `holder`, `prop`, `values` (`{"u32"\|"i64"\|"f32": [...]}`) or `absent: true` | a property of a holder (floats relative 1e-6) |
| `budget_expect_refs` | `refs`: `{"label": count}` | the reference count of a holder or object (1 = only the world's own) |

### Persistence

| op | Arguments | Effect |
|---|---|---|
| `budget_save` | `slot` | saves the budget through the class-object store (foundations §3.5) |
| `budget_load` | `slot`, `expect?` (default true: the store reports a clean load), `read_ok?` (whether the budget record was accepted) | loads a new budget from the slot and makes it current. Holders come back as the same objects, with one reference each. Record `Read` results are ignored by the store, so a rejected record leaves its object in the initial state. |
| `budget_expect_save` | `slot` | saving now gives byte-identical records |
| `budget_patch` | `slot`, `class` (`budget`, `department`, `line` or an ID), `entry?` (default 0), `offset` (into the payload), `hex` | overwrites bytes of a saved entry and fixes its CRC |

Budget events: `post` events carry `void1` and `unknown` as object labels (`""` for none); `history` (`series`,
`value`), `history_queue` (`series`, `length`), `show_budget_window` (`panel`), `disaster` (`id`, `x`, `z`),
`add_ref` / `release` (`agent` = label). Message names: `FundsChanged`, `BondIssued`, `BondFinished`,
`EndGameForBudgetFailure`, `BudgetGroupChanged`, `FundingPercentageChanged`, `IdealMonthlyFundingChanged`,
`BudgetForOccupantChanged`, `CityEstablished`, `ToggleBudgetDialog`.

## Simgrid ops

The simgrid vectors (`g10_…`) drive the core's model of the game's grid class (`src/simgrid.rs` in both crates).
Grids are named; `type` is one of `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32`, `f64`.

| op | Arguments | Effect / check |
|---|---|---|
| `grid_city` | `cells` (`[x, z]`), `cell_width?` | the city the grids are sized from |
| `grid_new` | `grid`, `type`, `size` (tract size S) | a new grid, `Init` included |
| `grid_set_tract_size` | `grid`, `size` | `SetTractSize` (§3.2) |
| `grid_expect_geometry` | `grid`, any of `size`, `shift`, `count_x`, `count_z`, … | the derived geometry |
| `grid_set` / `grid_expect` | `grid`, `all?`, `values` (`[[x, z, v]]`) | write / check tract values |
| `grid_call` | `grid`, `method` (`cell_value`, `tract_value`, `average_cell_rect`, `average_tract_rect`, `tract_in_bounds`, `position_to_tract`, `tract_center`, `tract_corner`, `clsid`, `init`, `set_instance_id`), `args`, `expect` | one method |
| `grid_write` / `grid_read` | `grid`, `hex`, `zero_tail?`, `position?`, `expect?` | the save record (§5) |
| `grid_radius_effect` | `grid`, `center`, `radius`, `center_value`, `outer_value` | the radial effect (§4.6) |
| `grid_distance_effect` | `grid`, `mask`, `mask_value`, `radius`, `near`, `far` | the distance effect (§4.7) |
| `grid_two_grids` | `a` (u8), `b` (i16), `expect` (`[[x, z, v]]` in visit order) | the paired walk |
| `iter_scan` | `rect` (`[x0, x1, z0, z1]`), `step`, `reset?`, `expect` | the scan iterator |

## Land value ops

The land value vectors (`lv16_…`) drive the land value simulator with a fake host: terrain (land vertices,
altitudes, allowed heights), zones, developers with their desirability maps, and grids created through the host
(the plugin creates the game's own grid classes). Every op name starts with `lv_`.

| op | Arguments | Effect / check |
|---|---|---|
| `lv_world` | any of `cells`, `cell_width`, `land`, `water`, `land_vertices`, `clear_land`, `altitude`, `altitudes`, `heights`, `zoned`, `clear_zones`, `developers` (`{"0x1030": {size, all, values} \| null}`) | the fake world |
| `lv_create` / `lv_init` / `lv_shutdown` | `tuning?` for init (`"shipped"` or an object of §9 values; `null` = absent) | lifecycle (§4) |
| `lv_step` | `budget`, `count?`, `expect_state?` | SimIdle steps (§6) |
| `lv_run` | `budget`, `max`, `expect_calls?` | steps until a pass completes |
| `lv_message` | `type`, `data3?`, `expect?` | `DoMessage` (§5) |
| `lv_set` | any of `dirty`, `state`, `average`, `mean_alt`, `min_alt`, `max_alt`, `mode_alt` | debug overrides |
| `lv_expect` | state fields, `histogram`, `mean_alt_bits`, tuning fields, `has_grids`, `developer_weights` | the simulator state |
| `lv_set_grid` / `lv_expect_grid` | `grid` (`intrinsic`, `total`, `slope`, `scratch`), `all?`, `values?`, `rows?` (`rows[z][x]`), `tract_size?` | grid contents |
| `lv_query` | `method` (`land_value`, `land_value_type`, `average_land_value`, `mode_altitude`), `args`, `expect` | the interface queries (§11) |
| `lv_altitude` | `setup` (`[mode, mean, min, max]`), `max_allowed`, `curve?`, `expect?`, `effects?` | the altitude effect (§7.4) |
| `lv_desirability` | `weight`, `curve?`, `cases` | the desirability step (§7.5) |
| `lv_save` / `lv_load` | `ids` / `hex`, `position?`, `expect?` | the save record (§10) |
| `lv_expect_events` / `lv_clear_events` | `events` | host calls: `create`, `set_tract_size`, `grid_shutdown`, `release`, `history_length`, `history`, `post`, `load` |

## Adding a vector

1. Name it after the spec section it mainly covers, for example `s5_3_….json` (simulator), `f7_…` (foundations
   §7) or `b15_…` (budget §15).
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
| 9 limit-timer calibration | `s10_9_limit_timer_calibration` |
| other sections | `s4_reentrancy_and_cursors`, `s4_month_registration_order`, `s5_1_on_tick_guards`, `s5_2_minute_and_hour_levels`, `s5_3_begin_new_interval_flush`, `s5_4_idle_time`, `s6_speed_and_dilation`, `s6_invalid_speed_from_save`, `s9_cheats`, `s9_post_city_init` |

## Coverage (foundations §7)

| §7 item | Covered by |
|---|---|
| 1–4 primitives, sticky error, strings, resource key | `f7_01_04_stream_primitives` |
| 5 references | `fc4-core` unit tests (`stream`, `store`): null/object bytes, shared instances, cycles, unknown class |
| 6–7 CRC and class record | `f7_06_07_crc_and_class_record` |
| 8–9 store save order and load | `fc4-core` unit tests (`store`); `b15_16_save` (three real classes, renumbered IDs) |
| 10 city references | not yet (no city object) |
| 11 response curves | `f7_11_response_curves` |
| 12 messages | `engine/tests/message_test.cpp` (the GZCOM message class) |
| 13–15 random generators | `f7_13_15_random_generators` |
| §14 16–17 variants | `f14_16_17_variants` |
| §14 18–19 property holders | `f14_18_19_property_holders` |
| §14 20–22 regional-city record, deal claiming, import/export | not yet (neighbours milestone) |
| §14 23 `rand()` | `f14_23_crt_rand` (the prime-step iterator is defined in the weather spec: not yet) |
| §14 24 month order | `s4_month_registration_order` (registration order within a level; the ordinance-lag scenario needs the ordinance simulator) |

## Coverage (budget §15)

| §15 item | Vector |
|---|---|
| 1 funds | `b15_01_funds` |
| 2 tax | `b15_02_tax` |
| 3 settlement | `b15_03_settlement` |
| 4 year end | `b15_04_year_end` |
| 5 bankruptcy | `b15_05_bankruptcy` |
| 6 bonds | `b15_06_bonds` |
| 7 department percentage | `b15_07_department_pct` |
| 8 redistribution | `b15_08_redistribution` |
| 9 per-building funding | `b15_09_local_funding` |
| 10 re-synchronisation | `b15_10_resync` |
| 11 budget items | `b15_11_budget_items` |
| 12 groups | `b15_12_groups` |
| 13 tuning | `b15_13_tuning` |
| 14 daily | `b15_14_daily` |
| 15 estimates | `b15_15_estimates` |
| 16 save | `b15_16_save` |
| other sections (4.1, 4.3: messages, cheats, new-city history) | `b4_3_messages_and_cheats` |

## Coverage (simgrid §10)

| §10 item | Vector |
|---|---|
| 1 geometry | `g10_01_geometry` |
| 2–3 positions and bounds | `g10_02_03_positions_bounds` |
| 4–8 values, averages, tract size changes | `g10_04_08_values_averages` |
| 9–10 persistence | `g10_09_10_persistence` |
| 11–14 radius effect, rounding, clamping | `g10_11_14_radius_effect` |
| 15–16 distance effect | `g10_15_16_distance_effect` |
| 17 two grids with callback | `g10_17_two_grids` |
| 18 scanning iterator | `g10_18_scan_iterator` |

## Coverage (land value §16)

| §16 item | Vector |
|---|---|
| 1 land value type | `lv16_01_land_value_type` |
| 2 code defaults, lifecycle | `lv16_02_defaults_lifecycle` |
| 3–4 water bonus | `lv16_03_04_water_bonus` |
| 5 altitude statistics | `lv16_05_altitude_statistics` |
| 6 mode tie | `lv16_06_mode_tie` |
| 7 zoning fallback | `lv16_07_zoning_fallback` |
| 8–9 altitude effect, topIndex | `lv16_08_09_altitude_effect` |
| 10 desirability | `lv16_10_desirability` |
| 11 average | `lv16_11_average` |
| 12–13 save and load | `lv16_12_13_save_load` |
| 14 messages | `lv16_14_messages` |
| 15 pipeline call count | `lv16_15_pipeline` |
| 16 missing altitude curve | `lv16_16_missing_altitude_curve` |

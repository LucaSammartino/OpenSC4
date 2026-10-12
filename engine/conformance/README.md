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

Pure functions of `docs/specs/foundations.md`; they need no simulator state. Vector files are named `f7_…` (and `f6_…` for §6 items outside §7).

| op | Arguments | Checks |
|---|---|---|
| `stream_write` | `values`: list of one-key objects (`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`, `f64`, `bool`, `str`, `bytes` (hex), `res_key` (`[type, group, instance]`), `null_ref`); `hex` | the bytes written (§2.2) equal `hex` |
| `stream_read` | `hex`; `reads`: list of `{get, expect?, position?}` with `get` one of `u8`, `u16`, `u32`, `i32`, `u64`, `i64`, `f32`, `f64`, `str`, `rz_char_str` (+ `max_bytes`, result as hex), `skip` (+ `n`); `error?` | each read's value; `expect: null` means the read must fail without consuming; `position` after the read; the sticky error flag at the end |
| `crc` | `ascii` or `hex`; `expect` | CRC-32/MPEG-2 (§3.4) |
| `store_entry` | `class_id`, `payload_hex`, `entry_hex`, `index_hex?` | the class-object store entry (and index) written for one object with ID 1 whose `Write` emits the payload (§3.3) |
| `curve` | `points` (flat x, y list), `absent?`, `eval`: `[[x, y], ...]` (x and y may be `"0x…"` bit patterns; an expected NaN matches any NaN), `tolerance?` (relative, default 1e-6) | response-curve construction and evaluation (§4) |
| `rng` | `generator` (`RzRandom` or `RzRandomSuper`), `seed`, `wall_clock_seed?`, `process_clock?`, `calls`: `{method, args?, weights?, expect?, lcg?, mul?}`. Methods: `RzRandom`: `next_raw`, `uniform_u32`, `range_i32`, `uniform_f64`, `range_f64`, `gaussian`, `range_gaussian`, `range_gaussian_fast`, `range_i32_gaussian_fast`, and the pure mappings `uniform_of` (`[i]`, one signed draw) and `gaussian_to_range` (`[g, a, b]`); `RzRandomSuper`: `next_u32`, `uniform_u32`, `uniform_f64`, `range_f64`, `weighted_choice` | outputs and internal states, bit-exact (§6) |
| `derive_seed` | `cases`: `{session, k or generator, k_expect?, expect, next_raw?}`; `generator` names a file-local generator of the §6.3 table (`building_development`, `growth_developer`, `tract_developer`, `lot_developer`, `zone_developer`, `lot`, `budget`, `crime`, `pollution`, `wealth_requester`, `network_lot_manager`, `network_construction_crew`, `construction_occupant`, `base_texture_occupant`, `retaining_wall_occupant`, `prop_occupant`, `fire_occupant`, `earthquake`, `pipe_burst`, `riot`, `automata`, `region_creation`, `advice`) | the derived seed (§6.3), the generator's `K`, and the first `NextRaw` words of a `cRZRandom` with that seed |

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

## Scalar simulator ops (shared)

The demand, ordinance and weather vectors share a few conventions (`src/scalar.rs`):

- Each family keeps its own fake world for the whole vector, created by its first op.
- Numbers may be written as integers, floats or `"0x…"` strings. Results are compared with a relative tolerance
  of 1e-5 (demand spec §14), exactly when both sides are integers; objects are compared key by key, with keys
  compared as integers when both parse.
- Events go to one log. `*_expect_events` (`events`, `filter?`) checks the events since the last check, one to one
  and in order, each given as a partial object (only the keys written are checked), after keeping only the events
  that match `filter`; it then clears the log. `*_expect_event_count` (`filter?`, `count` / `min` / `max`) counts
  without clearing; `*_clear_events` clears.
- Property maps are written `{"0xPROPID": {"u32": 5}}` (a scalar), `{"f32": [1, 2]}` (an array) or
  `{"str": "text"}`; types `u8`, `u16`, `u32`, `i32`, `i64`, `f32`, `bool`.
- Saves go through the class-object store (foundations §3.5) with a root record that references the simulator.
  `*_patch` (`slot`, `class`, `entry?`, `offset`, `hex`, `insert?`, `delete?`) overwrites, inserts or removes
  payload bytes of one entry and fixes its size and CRC.

## Demand ops

The demand vectors (`d16_…`) drive the demand simulator (`fc4_core::demand`) over a scripted city. Every op name
starts with `demand_`.

### Environment

| op | Arguments | Effect |
|---|---|---|
| `demand_env` | any of `cells` (`[x, z]`, read at `Init`), `today` (Julian day; default 2000-01-01), `date` (`{year, month, day}`), `advance_days`, `clock_step` (ms the fake clock advances per reading; default 0), `air`, `sea` (port trips), `history_max` (`{"0xSERIES": max}`; absent = no data in the window), `ordinance` (`{"0xPROP": factor}`; default 1.0), `nation` (`{"0xDEMANDID": modifier}`), `nation_default`, `dev_types` (`[{purpose, wealth, dev}]`) | sets the fake world. `GetDeveloperType` defaults to R `0x1000`, CS `0x3100`, CO `0x3300`, Tourism `0x3200` (+ `0x10`·wealth), I `0x4100`/`0x4200`/`0x4300`/`0x4400`. |
| `demand_exemplar` | `instance`, `group?` (default `0xC7BB4816`), `exemplar_type?` (default 3), `key?` (0x21), `min?`, `max?`, `supply?`, `demand?` (initial values), `drives?`, `satisfies?`, `contributes?` (`[[target, percent], …]`), `caps?`, `sums?` (key lists), `region_use?`, `props?` (extra raw properties) | adds a demand exemplar; enumeration order is the order of these ops, and a later exemplar with the same key overrides an earlier one when loaded singly |
| `demand_tuning` | `props` (the Demand Simulator exemplar `0x4A1F3CE6` as a property map) or `null` | replaces or removes the tuning exemplar |
| `demand_building` | `type`, `purpose?`, `capacities?` (`[[[devType, capacity], …], …]`), `creates?`, `satisfies?` (pairs), `creates_amounts?`, `satisfies_amounts?` (float lists), `props?` | a building type: its profile (BDS) and exemplar |
| `demand_cells` | `cells`: `[{x, z, w?, h?, building_type, wealth, has_capacity, desirability, capacity, eq, workforce}]` (defaults 1, 1, true, 100, 0, 0, 0), `replace?` (default true) | census cells; `w`/`h` repeat an entry over a rectangle |
| `demand_lot` | `lot` (ID), `building_type?`, `excluded?` (`0x27812844`) | a lot object for StateChanged |
| `demand_connections` | `connections`: `[{type, city, traffic}]` | the neighbour connections and their freight trips |
| `demand_cities` | `cities?` (`[{id, tax, allowable, population, extrapolated, workforce}]`, maps keyed by type; replaces all), `all_connected?`, `connected?`, `self?` | region cities and the two neighbour lists |
| `demand_developers` | `developers`: `[{sub_type, kind, source?: [supply, demand], last_build?}]` | growth developers (`source` absent = no source node) |

### Calls and checks

| op | Arguments | Effect |
|---|---|---|
| `demand_create` | | a new demand simulator |
| `demand_init` / `demand_shutdown` | `expect?` | `Init` / `Shutdown` (core parts) |
| `demand_message` | `type` (`SimBegin`, `SimNewDay`, `SimNewMonth`, `SimIdle`, `PreSave`, `ConnectionCreated`, `ConnectionDeleted`, `IngredChangesMade` or an ID), `data1?`, `data2?`, `data3?`, `expect?` | `DoMessage` |
| `demand_state_changed` | `lot`, `old`, `new`, `building_type`, `wealth` | kMessageStateChanged (§7) |
| `demand_call` | `method`, `args?`, `expect?` | a simulator method: `get_demand`, `simulator_type`, `update_occupant_effects`, `jobs_per_unit`, `jobs_by_census`, `neutral_tax_rate`, `local_population_summary`, `do_idle`, `propagate` (DoDemandIndexLinks), `simulator_begin`, `daily`, `monthly`, `update_modifiers`, `evaluate_connection_utilization`, `export_demand`, `reset_regional_demands`, `perform_census`, `apply_census`, `record_census_history`, `add_agent_effect`, `update_capacity`, `set_economy_bonuses`, `reload_tuning` |
| `demand_new_node` | `node` (label) | a free-standing node (class factory) |
| `demand_node` | `node` (label) or `key`, `method`, `args?`, `expect?` | a node method by snake-case name (`add_to_supply`, `end_of_cycle`, `set_active_max`, `calculate_active_demand` (`args: [smooth]`), …) |
| `demand_expect_node` | `node` or `key`, `state` (partial: `id`, `supply`, `demand`, `new_supply`, `new_demand`, `active_demand`, `active_min`, `active_max`, `economy_modifier`, `tax_modifier`, `demand_cap`, `region_use`, `flag`, `loaded_from_save`, `initialized`) | the node state |
| `demand_expect` | `state` (partial: `initialized`, `loaded_from_save`, `census_state`, `cursor`, `co_economy_bonus`, `i_economy_bonus`, `commercial_relief`, `industrial_relief`, `current_census`, `previous_census`, `node_keys`, `link_count`, `effect_building_types`, `export_cities`) | the simulator state |
| `demand_expect_refs` | `refs` (`{"0xKEY": count}`) | host reference counts of the simulator's nodes |
| `demand_save` / `demand_load` / `demand_expect_save` | `slot`; load: `expect?` (clean store load) | the class-object store; a load makes the loaded simulator current |
| `demand_patch` | see the shared conventions; `class` is `simulator`, `node` or an ID | |
| `demand_expect_payload` | `slot`, `class`, `entry?`, `hex` | the payload bytes of one saved entry |
| `demand_node_record` | `node`/`key`, `hex`, `read?`, `expect?` | with `read`, reads the bytes into the node (`expect` = the result); otherwise the node's own record must equal `hex` |

Demand events: `post` (`key`, `data2`: ActiveDemandChanged), `history` (`series`, `value`), `history_queue`
(`series`, `length`), `extrapolate` (`city`, `type`, `amount`: `cRegionalCity::ExtrapolateGrowth`), `warn` (`text`).

## Ordinance ops

The ordinance vectors (`o13_…`) drive the ordinance simulator and the standard ordinance classes
(`fc4_core::ordinance`) over a scripted city. Every op name starts with `ordinance_`. Ordinance ops also work
inside scheduler agents' `on_call` (see `o13_10_month_order`).

### Environment

| op | Arguments | Effect |
|---|---|---|
| `ordinance_env` | any of `year` (sim year; `null` = no simulator; default 2000), `population`, `budget` (budget simulator present; default true), `departments` (`{"0xDEPT": totalExpenses}`), `line_items` (`{"0xDEPT": {"0xLINE": fullExpenses}}`), `landfill_capacity`, `power_plants` (type-3 count), `police`, `recycling` (recycling-center effect), `lots` (`[{lot, building_type}]`), `view_control` (current input control ID), `ltext` (`[{type?, group?, instance, text}]`, default type `0x2026960B`, group `0x6A231EAA`), `classes` (registered ordinance classes; `null` = all 21 standard classes) | sets the fake world |
| `ordinance_exemplar` | `instance`, `group?` (default `0xA9C2C209`), `exemplar_type?` (default 14), `id?` (default the instance; `null` omits it), `enact?`, `repeal?`, `constant?` (s32), `factor?`, `chance?` (f32), `year?`, `advisor?`, `income?`, `name?`, `description?` (string properties), `name_ltext?` / `description_ltext?` (`true` = the shipped LTEXT key for the instance, or `[type, group, instance]`), `props?`, `replace?`, `remove?` | adds (or replaces / removes) an ordinance exemplar; enumeration order is the order of these ops |

### Calls and checks

| op | Arguments | Effect |
|---|---|---|
| `ordinance_create` | | a new ordinance simulator |
| `ordinance_init` / `ordinance_shutdown` | `expect?` | `Init` / `Shutdown` (core parts) |
| `ordinance_message` | `type` (`SimNewMonth`, `SimBegin` or an ID), `expect?` | `DoMessage` |
| `ordinance_call` | `method`, `args?`, `expect?` | a simulator method: `add_ordinance` / `remove_ordinance` (`args: [label]`), `is_ordinance_available`, `is_ordinance_on`, `is_ordinance_available_but_off`, `set_ordinance_available`, `set_ordinance_on`, `available_count`, `available_array`, `on_count`, `on_array`, `id_array` (`args: [capacity]`, result `{ids, total}`), `is_ordinance_enabled`, `set_ordinance_enabled`, `ordinance_id_by_name`, `ordinance_by_id` / `ordinance_by_name` (result: the ID or `null`), `monthly_expense`, `monthly_income`, `cumulative_factor`, `cumulative_offset` (`args: [prop, index]`), `do_monthly_simulation`, `simulation_begin` |
| `ordinance_new` | `ordinance` (label), `class`, `instance?`, `group?` | a free-standing standard ordinance (class factory), optionally `SetKey` |
| `ordinance_foreign` | `ordinance` (label), `id`, `name?`, `enact?`, `repeal?`, `monthly?`, `conditions?`, `available?`, `on?`, `props?` | a scripted plugin `cISC4Ordinance` |
| `ordinance_object` | `ordinance` (label) or `id`, `method`, `args?`, `expect?` | an ordinance method by snake-case name (`init`, `current_monthly_income`, `check_conditions`, `simulate`, `set_on`, `force_available`, `has_misc_property`, …; `set_key` / `from_data` take `[instance]` and `group?`) |
| `ordinance_expect_object` | `ordinance` or `id`, `state` (partial: `class`, `plain`, `initialized`, `key` (`[type, group, instance]` or `null`), `id`, `name`, `description`, `year_first_available`, `chance_availability`, `enactment_income`, `retracment_income`, `monthly_constant_income`, `monthly_income_factor`, `advisor_id`, `is_income_ordinance`, `available`, `on` (the raw flag), `enabled`, `adjusted`, `misc_parent` (the parent exemplar's instance or `null`)) | a standard ordinance's state |
| `ordinance_expect` | `state` (partial: `initialized`, `loaded_from_save`, `keys`, `monthly_expense`, `monthly_income`, `pending`) | the simulator state |
| `ordinance_expect_refs` | `refs` (`{"label" or "0xID": count}`) | host reference counts |
| `ordinance_save` / `ordinance_load` / `ordinance_expect_save` | `slot`; load: `expect?` (clean store load) | the class-object store; a load makes the loaded simulator current |
| `ordinance_patch` / `ordinance_expect_payload` | see the shared conventions; `class` is `simulator` or a class ID | |
| `ordinance_record` | `ordinance`/`id`, `hex`, `read?`, `expect?` | with `read`, reads the bytes into the ordinance (`expect` = the result); otherwise its record must equal `hex` |

Ordinance events: `post` (`type`: `OrdinanceUpdated`, `AvailabilityChanged`, `MonthlyDone`; `id` = data2, `value` =
data3), `demolish` (`lots`), `hide` (`building_type`), `remove_control` (`id`).

## Weather ops

The weather vectors (`w14_…`) drive the weather simulator (`fc4_core::weather`) over a scripted city and terrain.
Every op name starts with `weather_`. Cell coordinates are terrain vertex / cell indices; world units are 16 per
cell. Grids are x-major (`rows[x][z]`).

### Environment

| op | Arguments | Effect |
|---|---|---|
| `weather_env` | any of `size` (`[cellsX, cellsZ]`; the city is 16 × that in world units; default 16 × 16), `cell_width` (`[x, z]`, city cell width, default 16), `hemisphere` (default 1), `month` (the simulator's calendar month, default 1), `water_prefs` (`[lo, hi]` or `null` = no region), `terrain` (default true), `flat` (the height of every vertex; clears `heights`), `heights` (`[{x, z, w?, d?, h}]`), `water` (`[{x, z, w?, d?}]` cells with `CellHasWater`, or `null`), `normals` (`[{x, z, w?, d?, n: [x, y, z]}]`, default (0, 1, 0)), `sea_level` (default −1) | sets the fake world. `GetAltitude(worldX, worldZ)` reads the vertex `floor(world / 16)` |
| `weather_tuning` | `shipped` (the §9 values), `props` (a property map merged over them), or `none` (no exemplar) | the tuning exemplar the next `Init` reads |
| `weather_create` | `seed?` (the wall-clock seed), `rng?`: `{seed}` (a real generator with that seed) or `{stub: {uniform, gaussian, int}}` (queued values; each queue repeats its last value, defaults 0.5 / 0 / 0) | a new simulator (the default generator is wall-clock seeded; a stub maps its uniform u to a range with the 24-bit `r24(r24(u × r24(b − a)) + a)` of foundations §6.1) |
| `weather_rng` | `seed` or `stub` as in `weather_create` | replaces the simulator's generator |

### Calls and checks

| op | Arguments | Effect |
|---|---|---|
| `weather_init` / `weather_shutdown` | `expect?` | `Init` / `Shutdown` (core parts) |
| `weather_message` | `type` (`SimNewDay`, `SimNewWeek`, `SimNewMonth`, `PostCityInit` or an ID), `date?` (`{year, month, day}`, the day message's data1), `data1?`, `data3?` (the month), `expect?` | `DoMessage` |
| `weather_call` | `method`, `args?`, `expect?` | a simulator method: `do_post_city_init`, `do_complete_simulation`, `do_monthly_simulation`, `do_weekly_simulation`, `do_daily` (`[julian]`), `new_current_ambient_values` (`[seasonMonth]`), `onshore_direction` (result `[x, z]`), `simulate_cell_temperature`, `simulate_moisture` (`[x0, z0, x1, z1]` cells), `update_moisture` (world units), `generate_moisture_variation`, `rebuild_water`, `set_manual_moisture_addition`, `cell_width_x/z`, `cell_count_x/z`, `position_to_cell` (`[result, cx, cz]`), `cell_to_position`, `cell_to_standard_city_cell`, `standard_city_cell_to_cell`, `current_season`, `current_season_month`, `moisture_value`, `moisture_type`, `moisture_map` (the grid), `humidity`, `ambient_wind` / `wind_at_cell` (`{speed, v}`) |
| `weather_set` | `state` (scalar fields as in `weather_expect`; `*_fill` fills a grid; `sea_mask_cells`, `moisture_cells`, `moisture_base_cells`, `temperature_cells`: `[{x, z, w?, d?, v?}]`) | pokes state (test setup) |
| `weather_expect` | `state` (any §3 field in snake case, plus `water_rects`, `iterator` (`state`: `running`/`exhausted`, `mode`, `rect`, `count`, `index`, `current`, `step`) and `rng` (`lcg`, `mul`, `has_cached`, `cached`); `iterator` and `rng` match partially), `exact?` (true: the other numbers must be bit-equal, for precision tests) | the simulator state |
| `weather_expect_grid` | `grid` (`temperature`, `moisture`, `moisture_base`, `sea_mask`) and any of `size`, `rows`, `all`, `cells` (`[{x, z, w?, d?, v}]`), `rect` (`[x0, z0, x1, z1)`) with `inside` / `outside`, `range` (`[lo, hi]`), `min`, `max` | grid contents |
| `weather_iterator` | `rect` (`[x0, z0, x1, z1]`, inclusive), `shuffle?` (default true), `crt_seed?` (default 1), `rand_calls?`, `step?`, `count?`, `current?`, `first?` (`[[x, z], …]`), `full_pass?` | a free-standing prime-step iterator (§6.7) |
| `weather_save` / `weather_load` / `weather_expect_save` | `slot`; load: `rng?` (the loaded object's generator before Read), `expect?` | the class-object store; a load makes the loaded simulator current |
| `weather_patch` / `weather_expect_payload` | see the shared conventions (the class is always the weather simulator) | |

Weather events: `post` (`type`: `SeasonChanged`, `WeatherCycleCompleted`, `MoistureMapUpdated` or `other`;
`data2`), and with a stub generator `rng` (`kind`: `uniform`, `gaussian`, `int` (with `lo`, `hi`); `value`).
## Simgrid ops

The simgrid vectors (`g10_…`) drive the core grid class and map utilities of `fc4_core::grid` (`src/simgrid.rs`
here). Grids are named; `type` is one of `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32`, `f64`.

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
| `grid_distance_effect` | `grid`, `mask?` (default the target), `mask_value`, `radius`, `near`, `far` | the distance effect (§4.7) |
| `grid_two_grids` | `a` (u8), `b` (i16), `expect` (`[[x, z, v]]` in visit order) | the paired walk |
| `iter_scan` | `rect` (`[x0, x1, z0, z1]`), `step` (below 1 counts as 1), `reset?`, `expect`, `rests?` (`[x, z]` after the end) | the scan iterator |

## Land value ops

The land value vectors (`lv16_…`) drive the land value simulator with a fake host: terrain (land vertices,
altitudes, allowed heights), zones, developers with their desirability maps, and grids created through the host
(the plugin creates the game's own grid classes). Every op name starts with `lv_`.

| op | Arguments | Effect / check |
|---|---|---|
| `lv_world` | any of `cells`, `cell_width`, `land`, `water`, `land_vertices`, `clear_land`, `altitude`, `altitudes`, `heights`, `zoned`, `clear_zones`, `developers` (`{"0x1030": {size, cells?, all, values} \| null}`; `cells: [x, z]` makes the map cover a city of that size) | the fake world |
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

## Tract grid ops

The tract grid vectors (`tg10_…`) drive the trait-based grid module `fc4_core::grid` (the storage-agnostic
`TractGrid` used by the M3 grid simulators; `docs/specs/simgrid.md`). Every op name starts with `tg_`. Grids are named; values are JSON numbers (integers for
integer grids). Tract coordinates are `[x, z]`; `rows` are x-major (`rows[x][z]`).

| op | Arguments | Effect |
|---|---|---|
| `tg_city` | `cells?` (`[countX, countZ]`, default 256 × 256), `width?` (`[x, z]` cell width, default 16) | the city later `SetTractSize` calls read |
| `tg_create` | `name`, `type` (`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`, `f64`) or `clsid` (a §2 factory ID; `expect: false` if none), `expect_type?`, `init?`, `tract_size?`, `instance?` | a new core grid |
| `tg_call` | `grid`, `method`, `args?`, `expect?` | a slot: `init`, `shutdown`, `is_init`, `instance_id`, `set_instance_id`, `clsid`, `set_tract_size`, `tract_size`, `shift`, `count_x/z`, `width_x/z`, `inv_width_x/z`, `geometry` (an object of all of these), `tract_in_bounds`, `position_to_tract` (`[tx, tz]`), `tract_corner`, `tract_center`, `get_cell_value`, `get_tract_value`, `set_tract_value`, `set_tract_values`, `average_in_cell_rect`, `average_in_tract_rect` |
| `tg_set` | `grid`, any of `fill` (first), `rows`, `cells` (`[[x, z, v], …]`) | writes values |
| `tg_expect` | `grid`, any of `counts`, `cells` (`[[x, z, v], …]`), `others` (every unlisted tract), `rows` | checks values |
| `tg_record` / `tg_read` | `grid`, `hex`, `zeros?` (trailing zero bytes); read: `expect?` | the §5 record written / read |
| `tg_store_round_trip` | `grid`, `class?` (the record class it must be saved under), `expect_type?` | saves a reference through the class-object store, loads it through the §2 factories, compares records |
| `tg_radius_effect` | `grid` (`u8`, `i8`, `i16`), `center`, `r`, `c`, `o`, `counts?` (default the grid's) | `AddRadiusEffect` |
| `tg_table_distance` | `dx`, `dz`, `expect` | the §7.1 distance table |
| `tg_distance_effect` | `target`, `mask` (`u8` grids, may be equal), `value`, `radius`, `near`, `far` | `AddDistanceEffect` |
| `tg_two_grids` | `a`, `b`, `add?` (the callback adds B to A), `calls?` (`[[x, z, b], …]` in order) | `ProcessTwoGridsWithCallback` |
| `tg_scan` | `rect`, `step?`, `resets` (`[{at: [x, z], expect: [[x, z], …]}]`, every cell until false) | the scanning iterator |
| `tg_random_iterator` | `mode` (`table`, `prime`, `constructed`), `rect`, `crt_seed?`, `first_next?`, `resets` (`[{rect?, shuffle?, rand_calls?, count?, table?, first?, full_pass?}]`) | the random cell iterator |

## Flammability ops

The flammability vectors (`fl16_…`) drive the flammability simulator (`fc4_core::flammability`) with a fake host:
occupants with their flammability property, lots, plumbing, zones, the ordinance factor, the season, the fire
disaster manager, and the two u8 grids it creates through the host (`GridProvider<u8>`; tract size 1). Every op
name starts with `flam_`. Tract coordinates are `[x, z]`; grid cells are `[[x, z, v], …]`.

### Environment

| op | Arguments | Effect |
|---|---|---|
| `flam_env` | any of `cells` (`[countX, countZ]`, default 64 × 64), `occupants` (`[{id, type (building, prop, flora or an ID), flam (an integer (a Uint32 scalar), a variant object (the shared types plus `i8`, `i16`, `u64`, `f64`, `char`), or null = no property), preview?, lot?, cell or rect ([x0, z0, x1, z1]), pos? ([x, y, z])}]`, added to the listing), `unlist` (ids no longer listed in their cells; still known by id), `forget` (ids removed completely), `lots` (`[{id, condition, building?}]`), `random_lot` (id or null), `factor` (the ordinance factor 0x28F42AA0, null = none; default 1), `season` (null = no weather; default 0), `unwatered` / `unwatered_all`, `zoned` / `zoned_all`, `tutorial`, `fire_manager` (default true), `plumbing` / `zone_manager` (false = the service is missing; default true) | the fake world |
| `flam_tuning` | `shipped` (the §8 exemplar), `props` (a property map; merged over the shipped values with `shipped`, else alone), `remove` (property IDs removed afterwards), or `none` (no exemplar) | the tuning exemplar the next `Init` reads |
| `flam_create` | `rng?` (`{r: [...]}` queued `RzRandom` doubles, repeating 50 when empty, or `{process_clock}` the real generator with that seed; default the queue), `crt_seed?` | a new simulator |
| `flam_rng` | as `rng` in `flam_create` | replaces the simulator's generator |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `flam_init` / `flam_shutdown` | `expect?` | `Init` / `Shutdown` (portable parts) |
| `flam_message` | `type` (`SimNewDay`, `SimBegin`, `InsertOccupant`, `RemoveOccupant`, `ChangeOccupant`, `SeasonChange`, `OrdinanceUpdated`, `ConditionChanged`, `ExemplarChanged` or an ID), `occupant?`, `lot?`, `expect?` | `DoMessage` |
| `flam_days` | `count`, `ends?` (the days, 1-based in this op, on which SimCycleComplete was posted) | `count` SimNewDay messages |
| `flam_call` | `method`, `args?`, `expect?` | pure functions: `occupant_raw_value` (`[p, m]`), `daily_value` (`[r, g, watered, noWater]`), `global_multiplier` (`[factor, season, summer]`, null = missing), `compute_hotspots` (`[candidates, radius, minWeight]`), `query_interface` (`[iid]`), `notifications` (the subscribed message types, in order); simulator methods: `occupant_raw`, `tract_raw`, `occupant_update` (`[id, add]`), `compute_global_multiplier`, `is_cell_flammable`, `hotspot` (`[v, x, z]` or false), `average`, `flammability_grid` (its instance ID or null), `daily`, `end_of_cycle`, `compute_hotspot_list` (the new list), `fire_roll`, `curve` (`[x]`, null without a curve), `add_ref`, `release` |
| `flam_set` | any of `raw`, `adjusted` (cells), `raw_fill`, `adjusted_fill`, `state` (`global_multiplier`, `average`, `zoned_sum`, `zoned_count`, `tracts_per_day`, `candidates`, `hotspots` (`[[x, z, weight]]`), tuning fields, `curve` (flat pairs or null)) | pokes state (test setup) |
| `flam_expect` | `state`: any of `initialized`, `ref_count`, `destroyed`, `count_x/z`, `average`, `global_multiplier`, `zoned_sum`, `zoned_count`, `candidates`, `hotspots`, `tracts_per_day`, the tuning fields, `has_curve`, `has_adjusted`, `has_raw`, `iterator` (`state`, `mode`, `dirty`, `rect`, `count`, `index`, `current`, `step`; matches partially) | the simulator state |
| `flam_expect_grid` | `grid` (`raw`, `adjusted`) and any of `counts`, `tract_size`, `instance`, `cells`, `others` / `all` (every unlisted tract) | grid contents |
| `flam_expect_rand` | `calls` | the number of `rand()` calls on the subsystem's CRT generator since the last check |
| `flam_expect_visits` | any of `all`, `cells` (`[[x, z, n]]`), `total` | daily visits (one zone-type query each) since the last check |
| `flam_expect_events` / `flam_expect_event_count` / `flam_clear_events` | see the shared conventions | host calls |
| `flam_save` / `flam_load` / `flam_expect_save` | `slot`; load: `rng?`, `expect?` (the store-level result; the simulator's Read result is ignored, foundations §3.5), `into_current?` (read the record into the current simulator instead of a new one) | the class-object store; loaded grids are registered with the fake host |
| `flam_patch` / `flam_expect_payload` | see the shared conventions; `flam_patch` takes `class?` (default the simulator) to patch a grid record | |

Flammability events: `create_grid` (`instance`, `size`), `grid_shutdown` (`instance`), `random_lot` (`lot`),
`history` (`type`, `value`), `ignite` (`x`, `z`, `use_probability`, `teleport`, `fire_iid`), `cause` (`cause`),
`post` (`type`: `SimCycleComplete`), and with a queued generator `rng` (`value`, `a`, `b`).

## Pollution ops

The pollution vectors (`p11_…`) drive the pollution simulator (`fc4_core::pollution`) over a scripted city. Every
op name starts with `pollution_`. The city defaults to 128 × 128 cells of 16 m (64 × 64 tracts), the shipped tuning,
ordinance factors 1.0, zone 0 everywhere, no traffic, no deals, no landfill, funding 100 %, a budget department and
working visual effects. Grid coordinates are tracts unless an argument says cells; the garbage value grid has
1-cell tracts.

### Environment

| op | Arguments | Effect |
|---|---|---|
| `pollution_env` | any of `cells` (`[x, z]`), `cell_width` (`[x, z]`), `altitude` (terrain height everywhere), `zone_default`, `zones` (`[{x, z, w?, d?, zone}]` cells), `traffic` (`[{x, z, w?, d?, v}]` air-polluting traffic per cell), `factors` (`[{id, index?, value}]` cumulative ordinance factors; no `index` = indices 0..15), `deals` (`[{deal, type: import/export/ID, amount, used?}]`, replaces all; `null` = none), `population`, `landfill` (`[usage, capacity]`), `landfill_tiles`, `funding` (the department's funding fraction), `department` (whether `CreateDepartmentBudget` succeeds), `effects` (whether `CreateVisualEffect` succeeds) | sets the fake world |
| `pollution_tuning` | `shipped?` (default true), `props?` (merged over it), `remove?` (property IDs), or `none` | the tuning exemplar read at Init / TuningChanged |
| `pollution_occupant` | `occupant` (label), `props?` (property map), `tract?` (`[tx, tz]`: position at the tract centre), `position?` (`[x, y, z]`), `cells?` (`[x0, z0, x1, z1]` inclusive footprint → bounding box; default the position as a point), `bbox?` (`[[x, y, z], [x, y, z]]`), `powered?` (default true), `budget?` (`{line, cost?, index?, purpose?}`, the item `GetBudgetItemForPurpose` returns) | defines or redefines an occupant (the test holds one reference) |

### Calls and checks

| op | Arguments | Effect |
|---|---|---|
| `pollution_create` | `fx_seed?` (the effect generator's seed, default 1) | a new simulator (its `rand()` seeded 1) |
| `pollution_init` / `pollution_shutdown` | `expect?` | `Init` / `Shutdown` |
| `pollution_message` | `type` (`SimBegin`, `SimNewDay`, `SimNewMonth`, `OccupantInserted`, `OccupantRemoved`, `PostCityInit`, `TuningChanged`, `PreViewLevelChange` or an ID), `occupant?`, `view_level?` (PreViewLevelChange), `count?` (repeat), `expect?` | `DoMessage` |
| `pollution_call` | `method`, `args?` (occupants by label), `expect?` | a simulator method: `simulate_day`, `simulate_month`, `end_of_cycle`, `update_visual_effects`, `reload_tuning`, `add_radiation_to_city` (`[x, y, z, strength, radius]`), `dispose` (`[g]`, `remaining?: {wtp, inc, wte}`; result `{taken, remaining}`), `pollution_grid` (result the grid name or `null`), `pollution_value`, `air_value`, `water_value`, `garbage_value`, `average_*_value`, `is_air_polluted`, `is_water_polluted`, `is_garbage_accumulated`, `is_radioactive`, `worst_*_tract` (`[x, z, value]`), `garbage_supply`, `landfill_tile_capacity`, `garbage_capacity`, `is_garbage_uncollected`, `total_garbage_uncollected`, `set_is_radioactive`, `set_air_value`, `set_water_value`, `set_garbage_value`, `add_to_garbage_supply`, `set_garbage_capacity`, `monthly_incinerator_capacity`, `monthly_wte_capacity`, `recycling_center_effect`, `garbage_abducted_prev_month`, `garbage_{produced,imported,exported,recycled,incinerated,wte,landfilled}_last_month`, the four counts, `garbage_buildings` (labels), `building_garbage_capacity`, `garbage_scaling_factor`, `is_active_landfill`, `set_is_active_landfill`, `garbage_put_in_landfill`, `max_capacity` / `current_capacity` (`[prop, occupant]`, result `[found, value]`), `recycling_center_max_capacity`, `recycling_center_current_capacity`, `recycling_center_current_capacity_bits` (`[found, float32 bits]`), `current_occupant_condition`, `current_occupant_efficiency`, `incinerator_used_capacity`, `wte_used_capacity`, `incinerator_current_capacity`, `wte_current_capacity`, `water_treatment_{max,current,used}_capacity`, `calc_occupant_maintenance_cost`, `max_level_for_ui`, `requested_share` (`[x0, z0, x1, z1]` cells), `requester_satisfaction_id`, `uses_custom_prop_selector`, `select_prop`, `set_occupant_list_used_capacity` (`[list (0 water treatment, 1 incinerators, 2 waste-to-energy), total, used]`, §4.9.3) |
| `pollution_set` | `state` (scalar fields as in `pollution_expect`) | pokes state (test setup) |
| `pollution_expect` | `state` (partial: `initialized`, `loaded`, `cell_x`, `cell_z`, `tracts_per_day`, `iterator` (`mode`, `rect`, `count`, `index`, `table_len`, `exhausted`; partial), the §3.1 scalars in snake case (and `efficiency_bits`, the float32 bits), this month's counters `produced` … `landfilled` and last month's `last_produced` … `last_landfilled`, `worst_air` / `worst_water` / `worst_garbage` (`[x, z, value]`), `department` (label or `null`), the lists `wtp`, `incinerators`, `wte_plants`, `recycling` (labels), `blocks_x`, `blocks_z`, `grids` (eight booleans)) | the simulator state |
| `pollution_set_grid` | `grid` (`air_src`, `water_src`, `garbage_src`, `rad_src`, `air_val`, `water_val`, `garbage_val`, `rad_val`), `fill?`, `tracts?` / `cells?` (`[{x, z, w?, d?, v}]`; `v` may be a raw `"0x8064"`) | writes grid tracts directly (no dirty flags) |
| `pollution_expect_grid` | `grid`, `tracts?` / `cells?`, `others?` (every tract not listed), `nonzero?` (count), `size?` (tract counts) | grid contents |
| `pollution_expect_occupant` | `occupant`, `props?` (`{"0xID": value or null for absent}`), `writes?` (`{"0xID": count}`: how often the simulator wrote the property) | the occupant's properties |
| `pollution_expect_refs` | `refs` (`{"label": count}`; occupants start at 1, the department at 0) | host reference counts |
| `pollution_expect_effects` | `water?`, `air?`, `radiation?`, `garbage?` (effects the simulator holds), `live?` (effect objects not yet released) | visual effects |
| `pollution_crt` | `seed?`, `mark?`, `rand_calls?` (the simulator's `rand()` calls since the last mark) | the iterators' generator |
| `pollution_expect_fx` | `seed?` (default 1), `draws` | the effect generator equals a fresh one after `draws` draws |
| `pollution_save` / `pollution_load` / `pollution_expect_save` | `slot`; load: `expect?` (clean store load), `fx_seed?` | the class-object store (grids are saved as their own objects; occupants and the department are host objects that come back with a reference); a load makes the loaded simulator current |
| `pollution_read` | `slot`, `expect`, `into?` (`current` (default) or `new`) | `Read` of the slot's payload outside the store (the store ignores `Read`'s result) by the current simulator or a new one (made current); occupants and the department resolve to their saved tokens, each resolution handing over a reference |
| `pollution_patch` / `pollution_expect_payload` | see the shared conventions (the class is always the pollution simulator); `expect_payload` takes `hex?`, `at?` (`[{offset, hex}]`), `length?` | |

Pollution events: `add_agent` (`level`, `name`, `flags`), `remove_agent`, `subscribe` / `unsubscribe` (`types`),
`add_prop_requester`, `remove_prop_requester`, `post` (`type`: `RadiationChanged`, `AirChanged`, `WaterChanged`,
`GarbageChanged`, `DealStatus` or the ID; `data1`), `history` (`series`, `value`), `history_queue` (`series`,
`length`), `deal_used` (`deal`, `used`), `mark_deals_dirty`, `lot_condition` (`x`, `z`, `condition`),
`create_department` (`id`, `group`), `department_name` (`group`, `instance`), `max_funding` (`value`), `line_item`
(`line`, `local`, `group`, `instance`), `full_expenses` (`line`, `value`), `budget_add` / `budget_remove`
(`occupant`, `index`), `budget_cost` (`occupant`, `index`, `cost`), `line_expenses` (`line`, `delta`),
`effect_create` (`effect`, `id`, `kind`), `effect_position` (`effect`, `position`), `effect_param` (`effect`,
`param`, `value`), `effect_start` / `effect_stop` (`effect`, `t`), `effect_release` (`effect`).

## Crime ops

The crime vectors (`c17_…`) drive the crime simulator (`fc4_core::crime`) over a scripted city. Every op name
starts with `crime_`. The city is 64 × 64 cells (16 m) unless `crime_env` changes it; the tuning defaults to the
shipped exemplar and the type list to the 14 shipped types. Grids are addressed by tract `[x, z]`; lots, occupants
and props by integer tokens. The fake lot class (for saves) is `0x12345678`.

### Environment

| op | Arguments | Effect |
|---|---|---|
| `crime_env` | any of `cells` (`[x, z]`), `clear_zones`, `zones` (`[{x, z, w?, d?, v}]` cell zone types), `demand` (`{"0x4100": [value, max, min] \| null}`), `jail` (`GetJailPopulation`), `released` (`ResetUnjailedCriminals`), `ordinance` (the cumulative factor, `null` = no ordinance simulator), `disaster_layer` (default true), `police` (`{size?, fill?, cells?: [[x, z, v]]}` or `null`; default none), `ltext` (`[{key: [t, g, i], text}]`) | the fake world |
| `crime_residential` | `wealth` (0..2), `size?` (default 2), `pop` / `eq` (`[[x, z, v]]`), or `none` | that wealth's population (u16) and EQ (f32) grids |
| `crime_lots` | `lots` (`[{id, rect: [x1, z1, x2, z2], location? ([x, z], default the rect corner; `null` = GetLocation fails), condition?, building? ({token?, purpose, wealth}; default none), center? (default the rect middle)}]`), `random` (the `GetRandomLot` results in order, ids or `null`) | lots and the random-lot queue (an exhausted queue gives no lot) |
| `crime_occupants` | `occupants` (`[{id, preview?, effect? (u8 array for 0xCA5B9306), pos}]`) | occupants for the inserted / removed messages |
| `crime_nearby` | `occupants` (`[{token, type, group?, incident_property?, pos, iid? (default true), state?}]`) | what the occupant manager's box query sees (it applies the x/z box) |
| `crime_props` | `props` (`{"0xPROP": crimeIncidentID}`) | prop profiles for `SelectProp` |
| `crime_types` | `shipped?` (default true), `types` (`[{id, props?}]`, property maps appended in order) | the crime type exemplars the next load reads |
| `crime_tuning` | `shipped`, `props` (merged over the shipped values when `shipped`), or `none` | the tuning exemplar |
| `crime_create` | `rng?`: `{seed}` (a real `cRZRandomSuper`) or `{stub: {uniform, range, uint}}`; `new_process?` | a new simulator; the process state (type list, generator) is shared with earlier simulators of the vector unless `new_process`; `rng` replaces the generator |
| `crime_rng` | `seed` or `stub` | replaces the shared generator |

The stub generator logs `rng` events: `uniform` (`value`), `range` (`lo`, `hi`, `value` = `lo + u(hi − lo)`),
`uint` (`n`, `value`). Each queue repeats its last value (defaults 0.5, 0.5, 0); `uint` values are returned as
they are.

### Calls and checks

| op | Arguments | Effect |
|---|---|---|
| `crime_init` / `crime_shutdown` | `expect?` | `Init` / `Shutdown` |
| `crime_message` | `type` (`SimNewDay`, `SimNewMonth`, `SimIdle`, `OccupantInserted`, `OccupantRemoved`, `OrdinanceUpdated`, `ExemplarChanged` or an ID), `data1?`, `data2?`, `occupant?` (void1), `count?`, `expect?` | `DoMessage` |
| `crime_call` | `method`, `args?`, `expect?` | `do_idle`, `do_census`, `calculate_economic_factors`, `compute_criminal_base`, `compute_crimes_to_commit`, `decay_crime_grid`, `compute_average_crime`, `load_types`, `read_tuning`, `add_crimes` (`[n]`), `add_crime_event` (`[lot]`, the severity), `lot_bits` (`[lot]` → `{zone, wealth, purpose, condition}`), `average_crime`, `monthly_crime_count`, `crime_hotspot` (`[v, x, z]` or false), `nearest_sim` / `nearest_prop` (`[[x, y, z], r]` → token or null), `incident_array` (`[initial length]` → `{result, events: [{cell, type, visuals, icon}]}`; pre-filled elements have type 0xFFFFFFFF), `set_visuals_created`, `visuals_created`, `simulator_type`, `requested_share` (`[x1, z1, x2, z2]`), `requester_satisfaction_id`, `uses_custom_prop_selector`, `select_prop` (`[[props], cx, cz]`), `curve` (`[name, x, wealth?]`, names `eq`, `jobless`, `crimes`, `attenuation`, `prop_request`) |
| `crime_eval` | `fn`, `args`, `expect` | pure functions: `bucket` (`[eq]`, null outside 0..3), `jobless` (`[a, b, c]`), `decay` (`[v, decay]`), `mask` (`[bytes \| null]`), `lot_bits` (`[zone, [purpose, wealth] \| null, condition]`), `choose` (`[weights, r]`), `rhe` (`[x]`, x a number or a hex string of float32 bits) |
| `crime_set` | `state`: any of `hotspot` (`[x, z, v]`), `average_crime`, `ordinance_factor`, `monthly_crime_count`, `state`, `criminal_base`, `crimes_to_commit`, `loaded_from_save`, `census`, `jobless`, `incidents` (`[{lot (token or null), type, name?, icon?, visuals?}]`, no AddRef) | pokes state |
| `crime_expect` | `state`: `initialized`, `loaded_from_save`, `has_message_server`, `has_crime_grid`, `has_effect_grid`, `state`, `criminal_base`, `crimes_to_commit`, `census`, `jobless`, `average_crime`, `average_crime_bits` (hex float32 bits), `ordinance_factor`, `monthly_crime_count`, `hotspot`, and, matched partially, `incidents`, `iterator` (`state`, `mode`, `dirty`, `rect`, `count`, `table`, `index`, `current`, `step`), `tuning` (`tract_size`, `decay`, `reoffend`, `prop_request_empty`), `types` (`id`, `name`, `icon`, `zone`, `wealth`, `purpose`, `condition`, `severity`, `arson`, `weight`; null before the first load) | the simulator state |
| `crime_grid_set` / `crime_expect_grid` | `grid` (`crime`, `effect`); set: `fill?`, `cells?`; expect: any of `counts`, `size`, `instance`, `cells`, `others` (every unlisted tract) | grid contents |
| `crime_save` / `crime_load` / `crime_expect_save` | `slot`; load: `expect?` (clean store load) | the class-object store; lots come back as the tokens they had; a load makes the loaded simulator current |
| `crime_read` | `slot`, `expect`, `position?`, `into?` (`new` (default) or `current`) | `Read` of the slot's payload outside the store (the store ignores `Read`'s result) by a new simulator (made current) or by the current one |
| `crime_patch` / `crime_expect_payload` | see the shared conventions (the class is always the crime simulator) | |

Crime events: `load_types`, `agent` (`name`, `levels`, `flags`), `remove_agent`, `subscribe` / `unsubscribe`
(`types`), `acquire_services`, `release_services`, `add_prop_requester`, `create` (`instance`, `size`),
`grid_shutdown` (`instance`), `jail_population`, `reset_unjailed`, `crime_stats`, `add_ref` / `release` (`lot`),
`ignite` (`building`, `cause`), `post` (`type`: `CrimeCommitted` with `crime`, `lot`, `name`, or
`CrimeMapUpdated`), and with a stub generator `rng`.

## Aura ops

The aura vectors (`a17_…`) drive the aura simulator (`fc4_core::aura`) with a scripted city implementing
`AuraHost`: lots (rectangles, zone, building occupant, wealth, populations), occupants and their Sint32-array
properties, budget funding, tax rates, the ordinance offset, the land value type, the grids of other simulators,
the limit timer and the registry. Every op name starts with `aura_`. The default city is 64 × 64 cells of 16 m
(16 × 16 tracts at S = 4; the work grid has one tract per cell). Grid cells are `[[x, z, v], …]`.

### Environment

| op | Arguments | Effect |
|---|---|---|
| `aura_env` | any of `cells` (`[x, z]`), `cell_width` (`[x, z]` m), `date` (default 1000), `tuning` (`"shipped"` (default), `"empty"`, or null = no exemplar), `tuning_props` (a property map merged into the tuning), `tuning_remove` (property IDs), `funding` (`{"0xID": f, "nan" or null}`; Parks and Recreation is `0x8A4260A6`), `tax` (`{"index": rate}`, default 0), `ordinance_offset`, `land_value_type` (default 1), `timer_expired` (default false), `timer_reads_before_expiry` (n or null: the clock answers "not expired" to the first n reads after each timer start, then "expired"; overrides `timer_expired`), `log_timer_reads` (log every clock read), `registry_ok` (default true), `lots` (`[{rect: [x1, z1, x2, z2], zone? (1), building? (occupant ID, 0 by default, null = none), wealth? (1), pops? ([[type, count]])}]`, or null), `occupants` (`{"id": {type?, pos: [x, y, z], props: {"0xID": [ints]}, dept?} or null}`), `grids` (`{name: {size? (4), fill?, cells?} or null}` for `air`, `water`, `garbage`, `radiation`, `school`, `hospital`, `crime`, `trip`, `congestion`, `pop1`..`pop3`) | the scripted city |
| `aura_create` | `rng?`: `{seed}` (a real `cRZRandom`) or `{stub: {uniform: [u…], int: [k…]}}` (a double draw returns `a + u (b − a)`, an integer draw returns `k`; the last value repeats); default a clock-seeded generator. Draws are logged | a new simulator |
| `aura_rng` | as `rng` above | replaces the generator |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `aura_init` / `aura_shutdown` | `expect?` | `Init` / `Shutdown` |
| `aura_message` | `type` (`SimBegin`, `SimNewDay`, `SimNewMonth`, `SimIdle`, `PostCityInit`, `OccupantInserted`, `OccupantRemoved`, `OrdinanceUpdated`, `ExemplarChanged` or an ID), `data1..3?`, `occupant?`, `expect?` (the `DoMessage` result) | `DoMessage` |
| `aura_idle` | `ms?` (5), `count?` (1), `expect_state?` | `DoIdle` calls |
| `aura_run` | `ms?`, `max?`, `expect_calls?` | `DoIdle` until state 0 |
| `aura_call` | `method`, `args?`, `expect?` | `add_transient_effect` (`[x, z, radius, magnitude]`), `add_transient_effect_type` (`[x, z, type]`), `aura_value` (`[x, z]` cells), `mayor_rating`, `max_mayor_rating` / `min_mayor_rating` (`[rating, x, z]`), `bust_strike` (`[type]`), `registry` (`command`, `args`: floats are Float32 variants, integers Sint64), `occupant_changed` (`[id, add]`), `centre_cell` (`[x, z]` m), `lot_base_aura` (`[lot]` as in `lots`), `tax_change`, `read_tuning`, `query_interface` (`[iid]` → `"primary"`, `"message_target"`, `"serializable"` or false), `class_id`, `slot_pops` (`[slot]`) |
| `aura_set` | `state`: any of `state`, `cycle_start_day`, `department_cursor` (null = none), `mayor_rating`, `unused_rating`, `global_effect`, `effective_base_aura`, `base_aura`, `hotspot`, `long_decay`, `short_decay`, `max_rating`, `min_rating`, `max_location`, `min_location`, `low_tracts`, `hotspots` (`[[x, z, w]]`), `re_add_landmarks`, `previous_tax_rate`, `slice_step` (`[step, misses]`) | pokes state (test setup) |
| `aura_expect` | `state`: the `aura_set` fields plus `initialized`, `riot_roll`, `transient_magnitude`, `transient_radius`, `curves` (partial `{park, air, water, garbage, radiation, school, hospital, crime, trip, congestion, tax_change, value_wealth, riot}` booleans: whether the curve exists), `departments` (IDs, ascending), `lot_iterator` | the simulator state |
| `aura_set_grid` | `grid` (`aura`, `base`, `transient`, `work`, `building_effect`, `park_map`, `raw_park`, `landmark_map`, `landmark_work`, `dept:0xID`), `fill?`, `cells?` | writes a grid; a missing department grid is first created through a landmark occupant with no cone (§7.2) |
| `aura_expect_grid` | `grid` and any of `exists`, `tract_size`, `counts`, `cells`, `others` (every unlisted tract) | grid contents |
| `aura_expect_events` / `aura_expect_event_count` / `aura_clear_events` | see the shared conventions | host calls |
| `aura_save` / `aura_load` / `aura_expect_save` | `slot`; load: `rng?`, `expect?` (the store-level result; the simulator's Read result is ignored, foundations §3.5), `into_current?` (read the record into the current simulator instead of a new one) | the class-object store |
| `aura_patch` / `aura_expect_payload` | see the shared conventions (the simulator's record) | |

Aura events: `create_grid` (`type` i8/i16, `instance`, `size`), `base_aura` (`values`), `lot_iterator`, `history`
(`series`, `value`), `disaster` (`manager`, `x`, `z`), `strike` (`service`: police, fire, health, school), `post`
(`type`), `timer_start` (`ms`), `timer_read` (`lots_taken`: lots taken from the current iterator; only with `log_timer_reads`), `subscribe` / `unsubscribe` (`types`), `add_agent` (`level`, `name`, `flags`),
`remove_agent`, `registry_add`, `registry_remove`, and `rng` (`kind` double with `value`, or int with `n` and
`value`).

## Power ops

The power vectors (`pw15_…`) drive the power simulator (`fc4_core::power`) with a scripted city implementing
`PowerHost`: occupants (buildings, poles, lines, network pieces) with their properties, footprints and budget
items; lots; zones; the budget's departments and line items; neighbour deals and PowerPole connections; the
ordinance factor; the 3D view's input control. Every op name starts with `power_`. The default city is 256 × 256
cells of 16 m with the shipped "Utilities" tuning and factor 1.0. Host objects are numbers (occupant, lot, deal and
connection IDs as given); departments and line items get tokens from `0x10001` on (the simulator itself takes one
token per `power_create`). IDs in events are exact `"0x%08X"` strings (numbers compare with a relative tolerance,
which would let neighbouring IDs match).

### Environment

| op | Arguments | Effect |
|---|---|---|
| `power_env` | any of `cells` (`[x, z]`), `cell_width` (`[x, z]` m), `factor` (the ordinance factor), `sim_time`, `department_ok` (whether `CreateDepartmentBudget` succeeds), `view_control` (ID or null), `zones` (`[{x, z, w?, d?, zone}]`, zone 0 clears), `occupants` (`[{id, type ("building", "pole", "line", "network" or an ID), cell? ([x, z], poles/lines/network pieces), cells? ([x1, z1, x2, z2], default the cell), bbox? ([[x, y, z], [x, y, z]], default the cells × 16), pos? (default the bbox centre), props? (a property map), lot?, building_type? (null = not a building occupant; default 0x1234 for buildings), qi? (true; the type's QueryInterface), ignored? (pole slot 13), preview? (line), flags? (network flags), poles? ([a, b], a line's connected poles), query_rect? ([x1, z1, x2, z2]: the rect the occupant-manager query matches it by, default its cells), in_manager? (true), distressed?, budget? ({department? (0x8910BC8A), line? (0), cost? (0), index? (0), funding? (1.0)})}]`), `pole_set` (`{first, count, cost? (1), origin? ([x, z])}`: `count` poles with consecutive IDs at origin + (i mod 50, i div 50)), `occupant_props` (`{"id": {"0xID": value or null}}`), `occupant_state` (`{"id": {in_manager?, distressed?, ignored?, budget? (or null)}}`), `lots` (`[{id, rect, state? (0), needs_power?, has_power?}]`), `departments` (`[{id, funding? (1.0), budget_funding? (0), lines? ([line or {line, secondary?}…])}]`; an existing department is updated, an existing line item is kept and only its `secondary` is set), `deals` (`[{id, type ("buy", "sell" or an ID), city, amount, used? (0)}]` or null; replaces the list), `connections` (`[{id, city, cell, visited?, active?, other? (the connection on the other side)}]` or null; replaces the list) | the scripted city |
| `power_tuning` | `none: true` (no exemplar), or `shipped?` (true; false starts empty), `props?` (merged property map), `remove?` (IDs) | the tuning exemplar |
| `power_create` | `crt_seed?` (C `rand()` seed, default 1) | a new simulator (the old one is dropped without `Shutdown`) |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `power_init` / `power_shutdown` | `expect?` | `Init` / `Shutdown` |
| `power_message` | `type` (`SimNewMonth`, `SimNewDay`, `SimIdle`, `OccupantInserted`, `OccupantRemoved`, `ZoningChanged`, `TuningReload`, `TerrainChanged`, `OrdinanceUpdated`, `StateChanged`, `DealStatus`, `ConnectionsReady`, `PostCityInit` or an ID), `data1..4?`, `occupant?` / `void1?` (the IGZUnknown), `occupants?` (`[id…]` or `{first, count}`: the message once per occupant), `rect?` (zoning), `count?`, `expect?` (the `DoMessage` result) | `DoMessage` |
| `power_idle` | `count?` (1), `expect_state?` | `DoIdle` calls |
| `power_cycle` | `recompute?` (request one first), `count?` (repeat), `max?` (16), `expect_calls?` | `DoIdle` until the state is back at 4 after leaving it |
| `power_call` | `method`, `args?`, `expect?` | slots: `position_to_cell` (`[x, z]` m), `cell_to_position`, `cell_to_standard_city_cell`, `annual_consumption`, `annual_consumption_by_type`, `monthly_requirement`, `monthly_production`, `monthly_import`, `monthly_export`, `monthly_demand`, `monthly_consumption_at` (`[x, z]`), `monthly_consumption`, `supply_value` / `is_powered`, `is_wired`, `unpowered_consumer_count`, `powered_consumer_count`, `power_plant_count_of_type`, `power_plant_count`, `power_plants` (IDs; each AddRef'ed), `power_pole_count`, `overworked_plant_count`, `aged_plant_count`, `distressed_plant_count`, `find_power_plants` (`{lowest: [id, funding] or null, worn: [id, pct] or null}`), `is_occupant_powered`, `occupant_power_consumed` / `occupant_max_power_produced` (value or false), `occupant_power_produced` (`[produced, used]` or false), `is_on_strike`, `begin_strike`, `end_strike`, `chance_of_strike`, `strike_start_date`, `strike_length`, `current_plant_efficiency`, `current_plant_condition`, `current_power_produced`, `strike_efficiency`, `funding_percent` (null without a department), `monthly_generation_cost`, `plant_funding_percentage` (false without a budget item), `set_plant_funding_percentage` (`[id, fraction]`); internals: `update_power_grid` (`[x, z]`), `flood` (`[x, z]` → `[rowMin, rowMax]`), `age_plant`, `apply_pole_funding` (`[fraction]`), `monthly`, `do_idle`, `occupant_inserted`, `occupant_removed`, `cell_changed` (`[rect]`), `expand` (`[rect]`), `read_tuning`, `curve` (`[name, x]`, name `funding_efficiency`, `funding_decay`, `usage_decay`, `pole_decay`, `strike_response`); identity: `query_interface` (`[iid]` → `"primary"`, `"message_target"`, `"serializable"` or false), `class_id`, `slot_pops` (`[slot]`) |
| `power_set` | `mark` / `wire` / `powered` / `next` (cells `[x, z]` or rects `[x1, z1, x2, z2]` to set), `consumption` (`[[x, z, v]]`), `state`: any of `enabled`, `initialized`, `running`, `recompute`, `age_plants`, `wire_dirty`, `state`, `ring_index`, `production`, `consumption`, `import`, `export`, `demand`, `consumers`, `powered`, `unpowered`, `requirement`, `cost_base`, `deficit`, `broken_fraction`, `broken_count`, `pole_count`, `factor`, `strike_start`, `strike_flag`, `unused_flag`, `radius`, `annual`, `ring_row` (the current row), `per_type` | pokes state (test setup) |
| `power_expect` | `state`: the fields above plus `department` (token or null), the `last_*` values, `plants` (`[[visited, id]]`), `plant_count`, `poles`, `dirty_rect`, `changed`, `overworked`, `aged`, `distressed`, the tuning scalars (`max_funding`, `strike_efficiency`, `max_production_ratio`, `min_acceptable_funding`, `strike_duration`, `min_funding_end_strike`), their exact f32 bits as `"0x%08X"` strings (`max_funding_bits`, `strike_efficiency_bits`, `max_production_ratio_bits`, `min_acceptable_funding_bits`, `min_funding_end_strike_bits`) and `deficit_bits`, `map_size` (`[rows, columns]`), `bitmap_words`, `has_grid` | the simulator state |
| `power_expect_bitmap` | `bitmap` (`mark`, `wire`, `powered`, `next`), `cells?` (set), `rects?` (all set), `unset?`, `count?` (set bits), `exact?` (no other bit set) | a bitmap |
| `power_expect_grid` | any of `exists`, `instance`, `tract_size`, `counts`, `cells` (`[[x, z, v]]`), `rects` (`[[x1, z1, x2, z2, v]]`), `others` (every unlisted cell) | the powered grid |
| `power_expect_occupant` | `occupant`, `props?` (`{"0xID": value or null}`), `distressed?` | an occupant's properties as the simulator left them |
| `power_expect_world` | any of `lots` (`{"id": {has_power}}`), `deals` (`{"id": {used, flags}}`), `connections` (`{"id": {visited, active}}`), `line_items` (`[{department, line, exists?, expenses?, secondary?}]`), `distressed` (count of distressed poles), `departments` (IDs), `department_lookups` (`{"0xDEPT": n}`: `GetDepartmentBudget` calls since the last `power_clear_events`) | host objects |
| `power_expect_rand` | `calls` | C `rand()` calls since the last check (or `power_create` / `power_load`) |
| `power_expect_refs` | `refs` (`{"token": count}`, counts start at 0) | host reference counts |
| `power_expect_events` / `power_expect_event_count` / `power_clear_events` | see the shared conventions | host calls |
| `power_save` / `power_load` / `power_expect_save` | `slot`; load: `expect?` (the store-level result), `crt_seed?` | the class-object store; host objects in the save are an occupant class `0x0CC00001` and a department class `0x0CC00002` |
| `power_patch` / `power_expect_payload` | see the shared conventions (the simulator's record) | |
| `power_read` | `slot` (the saved record) or `hex`, `expect?` | calls `Read` of the current simulator directly (the store ignores its result) |

Power events: `acquire_services`, `release_services`, `factor` (`id`, `index`), `add_agent` (`level`, `name`, `flags`),
`remove_agent`, `subscribe` / `unsubscribe` (`types`), `post` (`type`, `data`, `unknown`), `history` (`series`,
`value`), `history_queue` (`series`, `length`), `create_grid` (`instance`, `size`), `set_property` (`occupant`, `id`,
`value`), `break_connections` (`pole`, `flag`), `distressed` (`pole`, `on`), `remove_connection` (`pole`, `other`: `pole.RemoveConnection(other, 0)`),
`occupants_in_cells` (`rect`), `remove_occupant`, `tool_zone_change` (`rect`, `data3`), `lot_has_power` (`lot`, `on`),
`lot_condition` (`lot`, `condition`), `demolish` (`lots`, `flags`), `radiation` (`position`, `strength`, `radius`),
`remove_view_control`, `deal_used` (`deal`, `used`), `deal_flags` (`deal`, `flags`, `on`), `mark_deals_dirty`,
`conn_active` (`conn`, `on`), `create_department` (`id`, `group`), `department_name`, `max_funding` (`department`, `value`, `bits`),
`create_line_item` (`department`, `line`, `local`), `create_line_item_for_type`, `line_item_name`, `display_flag`,
`secondary_info` and `full_expenses` (`item` = `[department, line]`, `value`), `budget_line`, `budget_cost`,
`add_funded` / `remove_funded`, `local_funding`, `local_full_funding`.

## Plumbing ops

The plumbing vectors (`pl14_…`) drive the plumbing (water) simulator (`fc4_core::plumbing`) over a scripted city
implementing `PlumbingHost`: cells, terrain water, water pollution values, the water department's funding and the
ordinance factor; building and pipe occupants (properties, lots, network flags, `ConnectsToNeighbor` directions,
budget items); neighbour deals and connections; and the budget (departments, line items). Every op name starts
with `plumbing_`. The default city is 64 × 64 cells with the shipped tuning, funding 1.0, ordinance factor 1.0 and
wall-clock seed 1. Objects are named by label; host objects in a save use the test-only classes `0x0CC00011`
(occupant) and `0x0CC00012` (department).

### Environment and objects

| op | Arguments | Effect |
|---|---|---|
| `plumbing_env` | any of `cells` (`[x, z]`), `funding`, `factor` (null = no ordinance simulator), `water` (`[[x, z]]`), `water_rects` (`[{x, z, w, d}]`), `clear_water`, `water_values` (`[{x, z, v}]`), `no_terrain`, `no_disaster`, `no_department`, `no_grids`, `log_water` (log every `CellHasWater` call as a `has_water` event), `wall_seed`, `departments` (`[{label, id}]`) | the scripted city |
| `plumbing_tuning` | `none` (no exemplar), `code` (an empty exemplar), or the shipped exemplar plus `props` (`{"0xID": value}`; `{"remove": true}` removes one); `reload?` sends the tuning message | the tuning exemplar |
| `plumbing_building` | `label`, `x`, `z`, `w?`, `d?`, `anchor?`, `props?`, `lot?` (default true: a lot `<label>_lot` over the rectangle), `lot_rect?`, `needs_power?`, `has_power?`, `building_type?`, `budget?` (`{department, line, cost, index?}`), `local_funding?`, `insert?` (default true) | a building occupant, inserted through the message |
| `plumbing_pipe` / `plumbing_pipe_row` / `plumbing_pipes` | pipe: `label`, `x`, `z` (or a rectangle), `dirs?`, `flags?`, `insert?`; row: `prefix`, `from`, `to` (connected to each other); pipes: `prefix`, `count`, `width?`, `x0?`, `z0?` (unconnected) | pipe occupants |
| `plumbing_occupant` | `occupant`, `props?`, `net_flags?`, `budget?` (null removes it), `local_funding?` | changes an occupant |
| `plumbing_lot` | `lot`, `needs_power?`, `has_power?`; or a new lot with `label` and a rectangle | lots |
| `plumbing_deal` / `plumbing_connection` | deal: `label` or `deal`, `type` (`import`, `export` or a number), `amount`, `used?`, `city`, `flags?`; connection: `label`, `x`, `z`, `city`, `active?`, `visited?`, `other?` | neighbour deals and connections |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `plumbing_create` / `plumbing_init` / `plumbing_shutdown` | `expect?` | a new simulator, `Init`, `Shutdown` |
| `plumbing_rng` / `plumbing_gaussian` | rng: `seed`; gaussian: `seed`, `calls` (`[{lo, hi, expect}]`), `raws?` | the generator; `RandomSint32RangeGaussianFast` on a fresh one |
| `plumbing_slice` | `cells` per call in states 6 and 7 (null = the whole map) | the time slice ([choice]) |
| `plumbing_message` | `type` (`SimNewMonth`, `SimNewDay`, `SimIdle`, `ConnectionsReady`, `InsertOccupant`, `RemoveOccupant`, `TuningReload`, `PipeBurst`, `DealStatus` or an ID), `data1..3?`, `occupant?`, `deal?`, `expect?` | `DoMessage` |
| `plumbing_insert` / `plumbing_remove` | `occupant` | the occupant messages |
| `plumbing_idle` | `count?` (1), `ms?` (5), `expect?` | `DoIdle` calls |
| `plumbing_pass` | `calls?` | starts a pass and idles until state 9 |
| `plumbing_month` | `expect?` | the month message |
| `plumbing_call` | `method`, `args?`, `expect?` | the §10 slots (`query_interface`, `add_ref`, `release`, `supply_value`, `supply_value_map_ptr`, `are_watered_buildings_in_cell`, `is_building_watered`, `water_production`, `is_cell_watered`, `num_water_sources`, `num_polluted_water_sources`, `water_production_capacity`, `water_consumed`, `water_imported`, `water_exported` (EAX), `num_pipes`, `num_distressed_pipes`, `num_burst_pipes`, `num_watered_buildings`, `num_unwatered_buildings`, `water_demand` (EAX), `water_demand_64`, `water_producer_info`, `water_producer_condition_percent`, `occupant_monthly_upkeep_cost`, `find_water_buildings`, `water_producers`), `slot_return` (`[slot]`), and the steps `nearest_water` (`[x, z, type, reach]`), `nearest_water_ring`, `update_water_production`, `age_water_producers`, `building_consumption`, `is_connected_to_current_network`, `pipe_cost`, `trunc`, `eax_of` |
| `plumbing_set` | `state` (any of `state`, `recompute`, `ageing`, `enabled`, `pipe_condition`, `num_pipes`, `num_distressed`, `num_burst`, `demand`, `pass_counter`, `total_water_cost`, `last_produced`, `last_consumed`, `last_imported`, `last_exported`), `flags?` (`[{x, z, v}]`), `production?` (`[{occupant, production}]`), `crt?` | pokes state (test setup) |
| `plumbing_expect` | `state`: a partial object (nested objects match partially) of the state fields (`state`, `recompute`, `initialized`, the counters and totals, `flag_map_size`, `flag_map_nonzero`, `producers`, `current`, `pipes`, `leaked`, `iterator` (`state`, `x`, `z`, `step`, `rect`), the grids, `rng_lcg`, `crt_state`, …) | the simulator state |
| `plumbing_expect_tuning` | the scalar fields (and `max_funding_bits`, the hex float32 bits), `eval?` (`[{curve, x, y}]`), `empty?` | the tuning |
| `plumbing_expect_flags` / `plumbing_expect_grid` | flags: `cells`; grid: `grid` (`published` or `working`), `instance?`, `cells?`, `rects?`, `nonzero?` | the flag map and the u8 grids |
| `plumbing_expect_occupant` / `_deal` / `_connection` / `_line_item` / `_refs` | occupant: `props?` (null = absent), `net_flags?`, `budget_cost?`; deal: `used?`, `flags?`; connection: `active?`, `visited?`; line item: `line`, `expenses?`, `secondary?`, `display?`, `exists?`; refs: `{label: count}` | host objects |
| `plumbing_expect_events` / `plumbing_expect_event_count` / `plumbing_clear_events` | see the shared conventions | host calls |
| `plumbing_save` / `plumbing_load` / `plumbing_expect_save` | `slot`; load: `expect?` (the store-level result; the simulator's Read result is ignored, foundations §3.5) | the class-object store |
| `plumbing_patch` / `plumbing_expect_payload` | see the shared conventions (the simulator's record) | |

Plumbing events: `agent`, `remove_agent`, `subscribe`, `unsubscribe`, `post` (`type`, `data1`), `history` (`id`,
`value`), `create_department`, `department_name`, `max_funding` (`value`, `bits`), `create_line_item`, `line_item_name`,
`remove_line_item` (`line`), `create_building_line_item`, `display_flag`, `add_expenses` (`line`, `delta`),
`set_expenses`, `set_secondary`, `change_budget_line`, `change_budget_cost`, `pipe_watered` (`pipe`, `watered`),
`redisplay` (`min`, `max`), `burst` (`x`, `z`, `manager`), `lot_condition` (`lot`, `condition`), `set_used`,
`deal_flags`, `deals_dirty`, `create_grid` (`instance`, `size`), `grid_shutdown` (`grid`) and, with `log_water`,
`has_water` (`x`, `z`).

## Protection ops

The protection vectors (`pr18_…`) drive the fire and police simulators (`fc4_core::protection`) with a scripted
city implementing `ProtectionHost`: occupants (type, position, prop flag, lot road access, station effect
properties, a property map, budget items), lots, zoned cells, dispatches, departments and their funding, the
police coverage percentage, sea level and altitude, the disaster flag and the date. Every op name starts with
`protection_`. The default city is 64 × 64 cells of 16 m (32 × 32 tracts at S = 2). Grid cells are
`[[x, z, v], …]`; IDs may be written as `"0x…"` strings.

### Environment

| op | Arguments | Effect |
|---|---|---|
| `protection_env` | any of `cells` (`[x, z]`), `day` (default 1000), `police_pct` (the police simulator's percent coverage, or null), `sea_level` (250), `altitude` (300), `disaster`, `departments_fail`, `dept_funding` (`{"0xID": f}`, default 1.0), `tuning` (`"shipped"` (default), `"empty"`, or null = no exemplar), `tuning_props` (a property map merged into the tuning), `zones` (`[{x, z, w?, d?, v?}]` cells), `lots` (`{"id": [x, z] or null}`, the lot's cell), `dispatches` (`[{id, station, call, active, automaton, pos: [x, z]}]`) | the scripted city |
| `protection_occupants` | `{"id": {type?, pos?: [x, y, z], prop?, road? (true/false = a lot with/without road access, null = no lot), station?: [kind, centre, outer, radius, units] (writes the kind's effect, unit count and dispatch properties), props?: {"0xID": {"u32" / "i32" / "f32": value or array} or null}, items?: {"0xPURPOSE": {local, funded? (true), cost?, dept? (true; false = no department; or a department ID)} or null}} or null}` (merged into existing occupants) | occupants |
| `protection_create` | `kind` (`fire` or `police`), `rng?`: `{stub: {uniform: [u…], range: [u…]}}` (a range draw returns `a + u (b − a)`; the last value repeats) or `{seed}` | a new simulator; the shipped tuning when none is set |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `protection_grids` | `size` (the grids' tract size) | replaces both coverage grids of the current simulator with grids of that tract size on the same city (to show that the simulator uses its own S = 2) |
| `protection_init` / `protection_shutdown` | `expect?` | `Init` / `Shutdown` |
| `protection_message` | `type` (`SimBegin`, `SimNewDay`, `SimNewMonth`, `OccupantInserted`, `OccupantRemoved`, `OccupantChanged`, `FundingChanged`, `ReloadTuning`, `DispatchBegin`, `DispatchActive`, `DispatchInactive`, `DispatchEnd`, `Crime` or an ID), `data1?`, `data2?`, `occupant?`, `unknown?` (the IGZUnknown: an occupant or lot ID), `count?` (repeat), `expect?` | `DoMessage` |
| `protection_call` | `method`, `args?`, `expect?`, `bits?` | a simulator method by its snake-case name (for example `station_efficiency [occ]`, `station_effect [occ, record?]` (`record` `{x, z, radius, centre, outer, original_centre}` is the caller's record, default zeroed), `funding_of [occ, purpose]`, `jail_funding [occ]`, `jailed_percentage_bits`, `station_stats [occ]` (padded to 4), `max_possible_dispatch_count`, `dispatch_from_station [occ, id]`, `occupant_coverage [occ, p, cover_before?, station_before?]`, `striking_stations [list_len]`, `jailbreak_buildings [list_len]`, `dispatches_at_station [occ, flags?, base?]`), plus `add_ref`, `release`, `slot [n]` (name, pops, return convention), `slot_count`, `query_interface [iid]`, `class_id`, `curve [funding/strike/jail/jailbreak, x]`, `tuning` and `read_tuning` |
| `protection_eval` | `fn`, `args`, `expect` | the pure helpers: `adjusted_value`, `scale_effect`, `occupant_coverage`, `combine_strike_chance`, `unit_mask`, `dispatch_mask`, `revoke_mask`, `base_call_number`, `rehabilitation`, `reset_unjailed`, `jailed_percentage` (NaN is `"nan"`), `jailed_percentage_bits`, `pct` (x × 0.01f), `jailbreak_ends` (`[percentage, threshold]`), `jailbreak_starts` (`[chance, draw]` → `[draw taken, starts]`), `ratio_share`, `escapees`, `jail_capacity`; float arguments of `jailbreak_ends` / `jailbreak_starts` may be `"0x…"` float32 bit patterns |
| `protection_set` | `state`: any of `tracts_per_day`, `factor`, `counters` (`[zoned, sum, nonZero]`), `avg`, `pct`, `strike_flag`, `strike_date`, `next_call`, `capacity`, `population`, `jailed`, `arrests_last`, `arrests_this`, `unarrested_this`, `crimes_last`, `crimes_this` | pokes state (test setup) |
| `protection_expect` | `state`: the `protection_set` fields plus `initialized`, `ref_count`, `destroyed`, `loaded`, `department`, `jail_department`, `purpose`, `stations`, `air_source`, `jails`, `funded_jails`, `jailbreaks`, `crt` (the rand() state) and `iterator` (partial `{state, mode, rect, count, index, current, step}`) | the simulator state |
| `protection_expect_occupant` | `id`, `props` (`{"0xID": value or array}`) | occupant properties |
| `protection_expect_refs` | `all_zero?`, `refs?` (`{"id": count}`) | outstanding host references |
| `protection_grid_set` / `protection_expect_grid` | `grid` (`adjusted` or `unadjusted`); set: `cells`; expect: any of `exists`, `size`, `counts`, `instance`, `cells`, `others` | the coverage grids |
| `protection_expect_events` / `protection_expect_event_count` / `protection_clear_events` | see the shared conventions | host calls |
| `protection_save` / `protection_load` / `protection_expect_save` | `slot` | the class-object store (load recreates the simulator, its grids and the host objects) |
| `protection_read` | `slot` or `hex`, `expect`, `position?`, `reuse?` | `Read` into a new simulator, or with `reuse` into the current one (a re-read); `position` is the bytes consumed |
| `protection_patch` / `protection_expect_payload` | see the shared conventions (the simulator's record) | |

With `"bits": true` on a `protection_call` or `protection_eval`, every float in the result becomes its float32 bit
pattern (`"0x3DCCCCCC"`), compared exactly; other floats compare with the usual relative tolerance.

Protection events: `acquire_services` / `release_services` (`kind`), `create_grid` (`type` u8/i16, `instance`,
`size`), `set_instance`, `shutdown_grid`, `add_agent` (`level`, `name`, `flags`), `remove_agent`, `subscribe` /
`unsubscribe` (`types`), `register_dispatch` / `unregister_dispatch`, `department` (`id`, `group`, `name`, `max`),
`add_funded` / `remove_funded` (`dept`, `occupant`, `purpose`), `set_funding` (also `p`), `add_controllers` /
`remove_controllers` (`group`: `striking_fire`, `striking_police`, `jailbreak`; `occupant`), `history`
(`series`, `value`), `post` (`type`, `subject?`), `air_create`, `air_init` / `air_shutdown` (`source`), and `rng`
(`kind` uniform or range with `lo`, `hi`, `value`).

## Flora ops

The flora vectors (`fo16_…`) drive the flora simulator (`fc4_core::flora`) with a scripted city implementing
`FloraHost`. Every op name starts with `flora_`. The default city is 1024 m with 64 × 64 cells (cell =
floor(position / 16)), sea level 250 m, flat ground 260 m (normal.y 1), moisture type 8, no lots, the terrain
initialized, an occupant manager that accepts inserts, the shipped "Flora Tuning Parameters" (burn rate 1, removal
step 1, period 24, wild seeds 0, elevation scale 800) and a process-wide registry shared by every simulator of the
vector (`flora_registry reset` starts a new process). Occupants are numbers; the simulator's new occupants get IDs
from 1000 on. A flora occupant's bounding box is its position ± its record's half width (1) in x and z and
[y, y + height (3)], unless `bbox` is given. The occupant manager lists only inserted occupants (flag 1). Host
references are counted per occupant (an inserted occupant holds one). The scripted generator returns queued
integers (`range_i32`, `uniform_u32`; empty → the lower bound / 0), queued uniforms `u` (`uniform`; `range_f64`
returns `r24(r24(u × r24(b − a)) + a)`; empty → 0.5) and queued Gaussians (empty → 0); every draw is an `rng` event.
`rng: {seed, process_clock?}` uses the real generator (`FloraRng`) instead.

### Environment

| op | Arguments | Effect |
|---|---|---|
| `flora_env` | any of `exists`, `size` (`[x, z]` m), `cells` (`[x, z]`), `established`, `date` (the sim date number), `terrain_initialized`, `sea`, `ground`, `grounds` (`[[x0, z0, x1, z1, h]]`, last match wins), `normal_y`, `normals` (`[[x0, z0, x1, z1, y]]`), `lots` (`[[x0, z0, x1, z1]]`), `moisture`, `moistures` (`[[x0, z0, x1, z1, m]]`), `manager` (an occupant manager exists), `insert_ok`, `segment` (the new-city specification's obliterated segment, or null), `occupants` (`[{id, type ("flora", "building", "prop", "landfill", "network" or an ID), pos?, bbox? ([[x, y, z], [x, y, z]]), in_manager? (true), preview?, network? (flags), flora_type?, birth?, last_seed?, state?, orientation?, record? (type or null; default the flora type)}]`), `forget` (IDs to drop) | the scripted city |
| `flora_types` | `types` (`[{type, exemplar_type? (0x0F), next?, water?, floats? (values of 0x083DC35B) or the named floats maturity, seeding_age, interval, seeds, spread, lifespan, max_age, slope, min_score, probability, half_width, unused, clearance, height, cluster_radius?, cluster_height?, table? (one value for all 256 cells), table_cells? ([[e, m, v]]; without `table` the rest is 0.6 where both indices are in 5..12, else 0), never_auto?, flag_switch?, families?, plop_cost?, props? (extra property map)}]`), `append?` | the flora exemplars, in enumeration order |
| `flora_tuning` | `none: true` (no exemplar), or `shipped?` (true), `props?`, `remove?` | the tuning exemplar |
| `flora_registry` | `reset?` (a new process registry), `default_flags?` (the key-0 record's flags, [choice] default 7), `app_shutdown?` | the process-wide registry |
| `flora_create` | `rng?` (`{ints?, doubles?, gauss?}` or `{seed, process_clock?}`), `crt_seed?` | a new simulator (the old one is dropped without `Shutdown`) |
| `flora_rng` | as `rng` above | replaces the simulator's generator |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `flora_init` / `flora_shutdown` | `expect?` | `Init` / `Shutdown` |
| `flora_message` | `type` (`SimNewDay`, `SimNewMonth`, `InsertOccupant`, `RemoveOccupant`, `TerrainHeightsChanged`, `ExemplarChanged`, `CityEstablished`, `CityObliterated`, `PlaceFlora`, `Cheat` or an ID), `data1..3?` (`data3` may be an occupant type name), `occupant?` (void1), `text?` (the cheat line), `params?` (`[[as_u32, as_f32]]`), `expect?` | `DoMessage` |
| `flora_days` | `count?` (1) | SimNewDay messages |
| `flora_call` | `method`, `args?`, `expect?` | pure: `elevation_index` (`[alt, sea, water, scale]`), `table_byte`, `blocks_per_month` (`[cx, cz, period]`), `scale_wild_seeds` (`[w, sx, sz]`), `blocks_flora` (`[type, flags or null]`), `query_interface` (`"primary"`, `"message_target"`, `"serializable"` or false), `advance_cluster_type`; simulator: `elevation_index_at` (`[x, z, water]`), `score` (`[type, x, z]`), `best_type` (→ `[type, score]`), `location_ok` (`[type, x, z, exclude?]`), `add_new` (`[type, [x, y, z], force]` → `{occupant, pos}`), `add_flora_occupant`, `drop_seed`, `simulate_existing` (`[occupant]`), `monthly`, `wild_seeds`, `daily`, `kill`, `occupant_inserted` (`[occupant, type]`), `terrain_change` (`[[x0, z0, x1, z1]]`), `demolish`, `demolish_cells`, `demolish_bbox` (`[min, max]`), `cost`, `enable_flora`, `blast` (`[n, m, s]`), `static_data` (→ type or null), `all_types`, `all_types_into` (`[buffer length or null, count]` → `{ret, count, copied}`), `family_types`, `is_valid`, `is_family_valid`, `types_by_filter` (`[accepted types or null]` → `{ret, set}`), `types_by_family`, `add_static` (`[type spec or null]`), `save_obliterated` / `restore_obliterated` (`[segment]`), `read_tuning`, `exemplar_changed` (`[inst]`), `add_ref`, `release`, `draw_uniform` |
| `flora_set` | `state`: any of `enabled`, `propagation`, `propagation_forced`, `preserve`, `size`, `cells`, `today`, `period`, `blocks_per_month`, `wild_seeds_per_month`, `keep_iterator_flag`, `elevation_scale`, `burn_accumulator`, `burnt_removal_accumulator`, `burn_rate`, `burnt_removal_step` | pokes state (test setup) |
| `flora_expect` | `state`: the fields above plus `initialized`, `ref_count`, `destroyed`, `burnt` (occupant IDs), `simulator_held`, `rng` (`[lcg, mul, flag, cached]`), `iterator` (partial: `state`, `mode`, `dirty`, `rect`, `count`, `table_len`, `index`, `current`, `step`) | the simulator state |
| `flora_expect_occupant` | `occupant`, any of `exists`, `in_manager`, `type`, `flora_type`, `birth`, `last_seed`, `state`, `orientation`, `record`, `pos` | an occupant |
| `flora_expect_record` | `type`, any of `exists`, `next`, `seed_type`, `water`, `flags`, the named floats, `cluster_radius`, `cluster_height`, `unnamed`, `families`, `plop_cost`, `table` (`[[e, m, byte]]`) | a static flora record |
| `flora_expect_registry` | any of `types` (the type array), `keys` (map keys, ascending), `families`, `family` (`{id: [types]}`), `loaded`, `default_flags` | the registry |
| `flora_expect_rand` | `calls` | C `rand()` calls (block shuffles) since the last check (or `flora_create` / `flora_load`) |
| `flora_expect_refs` | `refs` (`{"id": count}`) | host reference counts |
| `flora_expect_segment` | `segment`, `count?`, `records?` (`[{key: [t, g, i], u32? or flora_type?}]`) | an obliteration segment |
| `flora_expect_events` / `flora_expect_event_count` / `flora_clear_events` | see the shared conventions | host calls |
| `flora_save` / `flora_load` / `flora_expect_save` | `slot`; load: `rng?`, `expect?`, `crt_seed?` | the class-object store; burnt-list occupants are host objects of class `0x0CC00001` |
| `flora_patch` / `flora_expect_payload` | see the shared conventions (the simulator's record) | |
| `flora_read` | `slot` or `hex`, `expect?`, `consumed?` | calls `Read` of the current simulator directly |

Flora events: `app_callback`, `post` (`type`, `data2`), `subscribe` / `unsubscribe` (`types`), `register_command` /
`unregister_command` (`id`, `name`), `add_agent` (`level`, `name`, `flags`), `remove_agent`, `acquire_services`,
`release_services`, `register_cheat` / `unregister_cheat` (`id`, `name`), `rng` (`method`, arguments, `value`), and
per occupant (`occupant`): `create`, `set_type` / `set_birth` / `set_last_seed` / `set_state` / `set_orientation`
(`value`), `set_record` (`record`), `set_position` (`pos`), `set_flag` (`on`), `change` (0x99EF1145), `init`,
`insert`, `remove`, `move`, `read_occupant` (`key`); `seg_write` (`segment`, `key`, `value` or `occupant` and
`in_manager`).

## Development helper ops

`development_eval`: `fn`, `args`, `expect` for the shared growth helpers of `fc4_core::development`
(building-development §4.7.3): `developer_type [p, w]`, `wealth [devType]`, `building_map_key [p, w]`,
`building_fits_in_zone [bz, cz]`, `growth_stage_fits_in_zone [stage, zone]`.

## Building development ops

The building development vectors (`bd11_…`) drive `fc4_core::development_simulator` with a scripted city
implementing `DevelopmentHost` and `GrowthDevelopers` (the growth developers, one to one with
`fc4_core::growth_developer`). Every op name starts with `bds_`. The default city: day 1000, difficulty 1,
64 × 64 cells of 16 m (cell = floor(position / 16), centre = (c + 0.5) × 16), terrain altitude 300, area altitude
250, no tuning exemplar, no census, funds 100000, demolition cost 0, an occupant manager that accepts inserts.
Host objects are numbers handed out from 256 (`bds_world next_token` moves the counter): the BDS itself at
`bds_create`, then developers, the building map and the construction node as `Init` creates them, then created
occupants. Growth developers are scripted per developer type (the type is the exemplar's property `0x21`, read at
the developer's `Init`): active demand (default 0, the value the developer's source node answers
`QueryActiveDemandValue` with; the fake node is named by the developer's number; `no_node` = no source node), purpose (default from the type),
sub-type (default the type), lists valid, growth and decline lists (`[[x, z, stage]]`), and how many times
`IsMapUpdateFinished` answers false after each `StartMapUpdate`. The tract developer returns queued values
(`grow`, `decline`; then the defaults, 0). The process statics (catalogue, tried-tract marks, generator) are one
`BdsProcess` per `bds_process` (seed default 0); every generator draw is a `draw` event (`n`, `value`, `stepped`)
logged at the moment it happens, so the events show how draws interleave with the tract developer's calls.
Saves write references as `objectID, classID` (foundations §2.7): the map `0x49B9E602`, the node `0x8990C12D`,
developers `0x0990C138`, occupants `0x0CC00001`; `bds_load` resolves an ID to the object saved under it, else to
the host object with that number (which the world may not know: unresolved).

### City

| op | Arguments | Effect |
|---|---|---|
| `bds_process` | `seed?` (0) | new process statics (empty catalogue and marks, the generator seeded with `seed`) |
| `bds_world` | any of `day`, `next_token`, `difficulty`, `cell_widths` (`[x, z]`), `cell_counts`, `altitude`, `area_altitude`, `census` (`{id: supply}`), `tuning` (property map, `"shipped"` = {8192, 20480} and {1.2, 1.1, 1.0}, or null), `funds`, `demolish_cost`, `occupant_manager`, `insert_ok` | the scripted city |
| `bds_exemplars` | `list`: `{building, purpose?, wealth?, capacity? (for its own developer type), families?, size? ([x, y, z]), name?, localized?, group?, props?}`, `{developer: type}` (0: no property `0x21`), `{foundation}`, `{wall}` or raw `{instance, group?, props}`; `append?` | the exemplars, in enumeration order |
| `bds_lot_configs` | `list`: `{size: [w, h], zones: [...], ids: [...], orientation? (0), pos? ([x, z]; default the lot centre), no_object?}` | lot configurations for the density table |
| `bds_developers` | `devs`: `{"type": {ad?, no_node?, purpose?, sub_type?, valid?, growth?, decline?, finish_after?}}` (merged) | scripted growth developers |
| `bds_tract` | `grow?`, `decline?` (queues), `grow_default?`, `decline_default?` | tract developer returns |
| `bds_lots` | `lots`: `[{id, building_type?, rect? ([x1, z1, x2, z2]), stage? (or null), building? (or null), state?, center?, slope?, dev_slope?, demolishable?}]` (merged), `cells`: `[{x, z, zone?, powered?, lv_type?, lv?, lot? (or null)}]`, `cell_default?` | lots and cells |
| `bds_occupants` | `list`: `[{id, groups?, preview?, pos?, bbox? ([min, max]), cost?, load_fails?}]` (occupant 0 is the template for created occupants), `building_cells?` (`[[x, z]]`) | occupants |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `bds_create` | `grow_budget?` (u32; default u32::MAX) | a new BDS (reference count 1; the old one is dropped without `Shutdown`) with that daily Grow attempt budget ([choice]: at most N tract-developer Grow calls per `Grow`; vectors that run `Grow` set it explicitly) |
| `bds_grow_budget` | `value` | sets the Grow attempt budget of the current BDS |
| `bds_init` / `bds_shutdown` | `expect?` | `Init` / `Shutdown` |
| `bds_add_ref` / `bds_release` | `expect?` (the count) | |
| `bds_message` | `type` (`SimBegin`, `SimNewDay`, `SimNewMonth`, `SimIdle`, `Cheat`, `PlaceBuilding`, `TerrainChanged`, `OccupantInserted`, `OccupantRemoved`, `StateChanged` or an ID), `data1..4?`, `occupant?`, `lot?`, `params?` (`[[as_u32, as_f32]]`), `expect?` | `DoMessage` |
| `bds_day` | `count?` (1) | SimNewDay, then day + 1 |
| `bds_call` | `method`, `args?`, `expect?` | `grow`, `plan_decline`, `execute_decline_plan`, `do_idle`, `monthly`, `daily`, `read_tuning`; `total_resources`, `free_resources`, `claim` / `release` (`[n]`); catalogue: `is_building_type_valid`, `is_building_family_type_valid`, `is_building_in_family`, `types_by_family` (`[f, vector?]` → `{count, vector}`), `types_by_family_set` (`{count, set}`), `types_by_filter` (`[accepted or null, set?]`), `profile` / `profile_ptr` (→ `{purpose, wealth, capacity, size}` or null), `building_key` / `foundation_key` / `wall_key` (→ `[t, g, i]` or null), `exemplar_name`, `building_name`, `all_building_types` / `all_family_types` / `all_foundation_types` / `all_wall_types` (`[vector?]`), `all_building_types_array` (`[n, length or null]` → `{ret, n, array}`), `density` (the table, `[[zone, key, value]]`), `max_capacity` (`[zone, p, w]`), `app_shutdown`; `developer_type`, `wealth`, `developer`, `developer_by` (`[p, w]`), `developers`, `map_key`, `map_key_of_type`, `day_of_last_build`, `stage_multiplier`, `cheats`; `suitable` (`[profile]` → `{result, reason}`, reason starts at -1), `location_ok` (`[type, x, z]`), `add_occupant` (`[type, [x, y, z], orientation]`), `add_occupant_at_cell` (`[type, cx, cz, orientation]`), `plop` (`[profile]`), `insert_occupant`, `demolish_foundation`, `demolition_cost`, `foundation_cost`, `building_changed` (`[lot, wealth, built]`), `terrain_changed` (`[rect, kind]`), `unpack_terrain_rect` (`[data1, data2, cx, cz]`), `query_interface` (→ `"primary"`, `"message_target"`, `"message_target2"`, `"serializable"` or null), `class_id`, `special_buildings`. A profile is `{zone, wealth, purpose, lv_min, lv_max, type, x, z}` |
| `bds_set` | `state`: any of `supply`, `demand`, `map_id`, `node_id`, `decline_plan` (`[[day, x, z, developer, stage?]]`), `state`, `ignore_plumbing`, `ignore_power`, `ignore_caps`, `coffee_bar`, `bias`, `loaded_from_save`, `special_buildings`, `highest_stage` (`{type: stage}`, merged) | pokes state (test setup) |
| `bds_expect` | `state`: any of `initialized`, `loaded_from_save`, `destroyed`, `ref_count`, `state`, `cursor`, `developers`, `highest_stage`, `last_build` (`{type: value}`), `special_buildings`, `decline_plan`, `planned_growth` (count), `grow_budget`, `cr_min`, `cr_max`, `cr_per_resident`, `bias`, `stage_multiplier`, the four stored cheats, `cell_counts`, `has_map`, `map_id`, `node_id`, `supply`, `demand`, `marks` (`{type: [[x, z]]}`), `steps` (generator steps), `rng_state`, `catalogue_loaded`, `building_count`, `funds`, `day` | the BDS, process and city state |
| `bds_expect_map` | `cells`: `[[x, z, value]]` | building map cells |
| `bds_expect_refs` | `developers?`, `occupants?` (`{"id": count}`) | host reference counts |
| `bds_expect_events` / `bds_expect_event_count` / `bds_clear_events` | see the shared conventions | host calls and draws |
| `bds_save` | `slot`, `expect?` | `Write` |
| `bds_expect_record` | `slot`, `hex?`, `length?`, `at?` (`[{offset, hex}]`) | the record bytes |
| `bds_patch` | `slot`, `offset` + `hex`, or `truncate` | edits a saved record |
| `bds_load` | `slot` or `hex`, `expect?`, `consumed?` | `Read` into the current BDS |

Building development events: `draw` (`n`, `value`, `stepped`), `static_cleanup`, `post` (`type`, `data2`),
`subscribe` / `unsubscribe` (`types`), `register_command` (`id`, `name`), `unregister_command` (`id`), `add_agent`
(`level`, `name`, `flags`), `remove_agent`, `register_cheat` (`id`, `name`), `unregister_cheat` (`id`),
`acquire_services`, `release_services`, `create_grid` (`instance`, `tract_size`, `id`), `create_node` (`id`),
`active_demand_changed` (`node`, `key`, `data2`), `create_developer`, `developer_init` (`dev`, `type`),
`developer_shutdown`, `developer_release`, `start_map_update`, `map_update_finished` (`dev`, `result`),
`tract_grow` (`dev`, `x`, `z`, `stage`, `type`, `outcomes_in`, `flag_in`, `built`), `tract_decline` (`dev`, `x`,
`z`, `stage`, `type`, `result`), `update_lot_elements` (`lot`, `had_foundation`: the low byte of the terrain-change
kind ≠ 0, [inferred]), `area_altitude` (`rect`), `dezone` (`rect`,
`flag`), `demolish_lots` (`lots`, `cost`), `set_funds` (`value`), `insert_occupant` (`occ`, `result`),
`remove_occupant`, `create_occupant`, `set_building_type` (`occ`, `type`), `load_occupant` (`occ`, `type`,
`result`), `set_bbox` (`occ`, `min`, `max`), `set_position` (`occ`, `pos`), `set_orientation` (`occ`,
`orientation`), `init_occupant`, `add_ref`, `release` (`occ`), `advisor_event` (`advisor`, `id`, `subject` (null),
`discarded`: `{x, z, lot, data1}`).

## Growth developer ops

The growth developer vectors (`gd14_…`, extras `gd00_…`) drive `fc4_core::growth_developer` over a scripted world
implementing `GrowthDeveloperHost`: the services, message server and simulator of `Init`, the date, source maps,
demand nodes (`GetDemand` / `QuerySupplyValue` / `AddToDemandValue`) and neighbour counts, the BDS's answers
(`GetMaxCapacity` by zone type, `GetBuildingProfile` purposes, `GetSpecialBuildings`, `GetStagePopulationMultiplier`,
`GetTractDeveloper`), the tract developer, zone types, lots, special-building occupants and the time-slice deadline.
Every op name starts with `gdev_`. All developers of a vector share one `GrowthStatics` (the file-local generator,
seed 0 until `gdev_statics`, and the slice step). The default city is 64 × 64 cells, date 0, wall-clock seed 1,
multiplier 1.0, no neighbours, no deadline, and a tract developer named `tract_developer`. Objects are named by
label, grids by their instance ID (`"0x02001010"`). The shared conventions of "Scalar simulator ops" apply (events,
property maps, the store).

### Environment

| op | Arguments | Effect |
|---|---|---|
| `gdev_env` | any of `cells` (`[x, z]`), `date`, `wall_seed`, `missing_services` (`["city", "bds", "occupant_manager", "zone_manager", "land_value", "traffic"]`), `no_message_server`, `no_simulator`, `no_grids`, `log_population` (log `UpdatePopulation`), `log_zone` (log `GetZoneType`), `log_supply` (default true: log demand and neighbour queries), `tract_developer` (bool), `multiplier`, `connected` (`[n1, n2]`), `demand` (`[{id, index?, supply?, active?, label?}]`; `active` answers `QueryActiveDemandValue`), `clear_demand`, `timer_checks` (the deadline answers "not yet" this many times per call; null = never), `max_capacity` (`{"zoneType": value}`), `purposes` (`{"0xType": purpose}`), `zones` (`[{x, z, w?, d?, zone}]`) | the scripted world |
| `gdev_source` | `map` (`intrinsic_land_value`, `total_land_value`, `slope`, `r_proximity` + `index` 1..3, `co_proximity` + `index` `0x3320` / `0x3330`, `park`, `pollution` + `index` 0..3, `school`, `hospital`, `crime`, `trip_length`, `commercial_traffic`, `landmark`, `transient`), `size` (tract size), `fill?`, `tracts?` (`[[x, z, v]]`), `none?` (removes it) | a source map with the element type of §4.3 |
| `gdev_statics` | `seed?` (reseeds the shared generator), `slice_step?`, `misses?`, `fresh?` (a new shared object for developers created later) | the process-wide state |
| `gdev_occupant` | `label`, `props?` (`{"0xID": value}`; `{"remove": true}` removes), `position?` (`[x, y, z]`) or `cell?` (`[x, z]`: its centre) | an occupant; `PositionToCell` is `floor(p / 16)` |
| `gdev_specials` | `list` (occupant labels) | the BDS's special-building list |
| `gdev_lot` | `label`, `building_type?`, `capacity?`, `size?` (`[w, d]`), `rect?` (`[x0, z0, x1, z1]`), `location?` (`[x, z]`) | a lot |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `gdev_create` | `label?` (`dev`), `add_ref?` (true: `GrowthDeveloper::create`, the constructor + `AddRef` of the BDS's `CreateDeveloper`; false: the bare constructor, count 0) | a constructed developer; it becomes the current one (other ops take `dev?`) |
| `gdev_init` | `exemplar?` (a shipped developer type; default an empty exemplar), `props?` (added or removed on top), `null?`, `expect?` | `Init` |
| `gdev_shutdown` | `expect?` | `Shutdown` |
| `gdev_idle` | `count?` (1), `ms?` (5), `expect?` | SimIdle messages (`DoIdle(ms)`) |
| `gdev_update` | `ms?`, `calls?` | `StartMapUpdate`, then `DoIdle` until the update finishes; `calls` checks the number of calls |
| `gdev_message` | `type` (`SimIdle`, `SimNewDay`, `ZoningChange`, `StateChanged` or an ID), `data1..4?`, `rect?`, `lot?`, `expect?` | `DoMessage` |
| `gdev_call` | `method`, `args?`, `expect?`, `expect_bits?` (the float32 bits of a float result) | the §10.1 slots (`add_ref`, `release`, `target_zone`, `developer_type` / `sub_type`, `purpose`, `wealth`, `construction_info` (`[initial zone code]`), `lists_valid`, `growth_list`, `decline_list` (`[[x, z, maxStage]]`), `growth_items`, `decline_items` (owned copies), `desirability_map`, `capacity_map`, `population_map`, `max_capacity_map` (instance IDs), `total_capacity`, `source_demand`, `source_active_demand` (`GetSourceDemand().QueryActiveDemandValue()`, null without a node), `caps`, `desirability_threshold`, `abandonment_threshold`, `monopoly_use`, `style_percentages`, `start_map_update`, `is_map_update_finished`, `estimate_desirability`, `subjective_thresholds`), the internal steps `new_day_finished`, `city_size`, `update_max_capacity` (`[rect]`), `update_population_count` (`[lot, type, add]`), `do_idle`, the pure helpers `population_of`, `desirability_fraction`, `add_curve`, `commute_add`, `phantom_term`, `round_capacity`, `split_capacity`, `decline_key_seed`, `decline_key` (with the world's wall seed), `style_fraction`, and `query_interface`, `slot` (`[n]` → `[name, pop, return]`), `draw_salt` |
| `gdev_set` | `state?` (`update_state`, `start_day`, `last_phantom_sum`, `total_capacity`, `ref_count`, `growth`, `decline`), `grid?` (`zoned`, `working`, `desirability`, `population`, `capacity`, `max_capacity`) with `fill?` / `tracts?` | pokes state (test setup) |
| `gdev_expect` | `state?`: a partial object of `initialized`, `ref_count`, `destroyed`, `holder`, `services`, `message_server`, `update_state`, `start_day`, `iterator` (`x`, `z`, `rect`, `state`; partial), `specials`, `special_cursor`, `growth`, `decline`, `last_phantom_sum`, `total_capacity`, `source_demand`, `last_salt`, `city_size`, `city_size_bits`, `grids` (partial), `rng_lcg`, `draws`, `slice_step`, `slice_misses`, and the tuning (`developer_type`, `target_zone`, `purpose`, `wealth`, `land_value_min`, `land_value_max`, `use_intrinsic`, `tract_size`, `baseline`, `growth_threshold`, `decline_threshold`, `phantom_demand`, `caps`, `monopoly_use`, `stage_count`, `curves` (the present curves in §4.6 order), `styles`, `abandonment`, `subjective_min`, `subjective_max`); `curve_eval?` (`[{curve, x, y}]`), `stage_eval?` (`[{stage, x, y, bits?}]`) | the developer and the shared statics |
| `gdev_expect_grid` | `grid`, `exists?`, `instance?`, `size?`, `counts?`, `all?`, `tracts?` (`[[x, z, v]]`), `nonzero?` | one of the six grids |
| `gdev_expect_salts` | `devs` (labels), `salts` | the developers' last salts, as a multiset, are among `salts` (each listed salt used once): which industrial developer gets which salt is a [compat-limit] and is not pinned |
| `gdev_expect_refs` / `gdev_expect_demand` | refs: `{label: count}` (references held through `AddRef` / `Release`); demand: `totals` (`{label: sum of AddToDemandValue}`) | host objects |
| `gdev_expect_events` / `gdev_expect_event_count` / `gdev_clear_events` | see the shared conventions | host calls |
| `gdev_sort` | `keys`, `order?` (`desc` / `asc`), `expect` (original indices in sorted order) | the game's `std::sort` on its own |
| `gdev_save` / `gdev_load` / `gdev_expect_save` | `slot`, `dev?`; load: `label?` (`dev`), `expect?` (the store-level result) | the class-object store; the loaded developer becomes current |
| `gdev_patch` / `gdev_expect_payload` | see the shared conventions (the developer's record) | |
| `gdev_read` | `hex` (a record with null references), `expect?` | `Read` on the current developer |

Growth developer events: `acquire_service` (`service`, `ok`), `release_service`, `acquire_message_server` (`ok`),
`release_message_server`, `subscribe` / `unsubscribe` (`types`), `agent` (`level`, `name`, `flags`), `remove_agent`,
`create_grid` (`instance`, `size`), `set_tract_size` (`which`, `grid`, `size`), `grid_shutdown` (`which`, `grid`),
`add_ref` / `release` (`object`), `get_demand` (`id`, `index`, `node`), `query_supply` (`node`), `add_to_demand`
(`node`, `value`, `bits`), `active_demand` (`node`), `connected_cities`, `all_connected_cities`, `stage_multiplier`, `tract_developer`,
`special_buildings`, `max_capacity` (`zone`, `purpose`, `wealth`), `building_purpose` (`building_type`),
`lot_building_type` (`lot`, `flag`), `lot_capacity` (`lot`, `developer_type`, `flag`), `lot_size`, `lot_rect`,
`lot_location` (`lot`), `timer_start` (`ms`), `wall_clock_seed`, and with their flags `update_population`
(`tract_developer`, `developer`, `x`, `z`, `stage`) and `zone_type` (`x`, `z`).

## Tract developer ops

The tract developer vectors (`td11_…`, extras `td00_…`) drive `fc4_core::tract_developer` with a scripted world
implementing `TractDeveloperHost`. Every op name starts with `tdev_`. Objects are tokens: developers, lots and
configurations take the `id` the vector gives; lots and filters the host creates get tokens from 65536 on. The
default world: a 64 × 64-cell city, construction resources 10⁹ (none used), no cheats, no anchor needed, every grid
at its default (zone 1, land value 0, powered 2, watered 1, fire 1, park 1, road access 1), no networks, no
surrounding lots, the shipped tuning exemplar (§4.21), services, simulator and message server present. A lot's
rectangle is computed from its location, size and facing (§4.10.1 table). `StartConstruction` puts the lot in
state 2 with the building; `AbandonBuilding` sets state 4 (synchronous, `lot-developer.md` §4.3);
`ReoccupyBuilding` sets state 3; `DemolishLots` clears the building; `CreateLotIfPossible` makes a state-1 lot whose
location is the inverse of the §4.10.1 table. With facing −1 on a free rectangle it first draws `U(4)` on the zone
developer's generator (R9, zones.md §7.7 / §9), records it in the tract developer's draw trace and faces the new lot
that way (the harness has no road model). Draws are traced by kind: R1–R8 by the core (§4.22), R9 by the host.

### Setup

| op | Arguments | Effect |
|---|---|---|
| `tdev_world` | any of `city` (`[x, z]` cells), `supply`, `used`, `cheats` (`{ignore_plumbing, ignore_power, ignore_caps, debug_growth}`), `anchor` (NeedsAnchorBuilding's answer), `construction_sets_state` (true), `no_services`, `no_simulator`, `no_message_server`, `create_fails` (CreateLotIfPossible gives no lot and no draw; the event still shows `lot: null`), `zone_seed` (the zone developer generator's state; default `0x8C09C68A`, its seed at clock 0) | the world |
| `tdev_developer` | `id`; any of `target_zone` (1), `subtype` (`0x1010`), `purpose` (1), `wealth` (1), `zone_code` (1), `land_value` (`[min, max]`, `[0, 0]`), `tract_size` (4), `shift` (2), `desirability` / `capacity` (`[[tx, tz, v]]`), `demand` (number, `"nan"` or null = no source demand; 1), `caps` (`[power, water, fire, park]`, 255 each), `monopoly_use` (−1), `styles` (`[[group, fraction]]`, ascending keys) | a growth developer (fields not given keep their value) |
| `tdev_lot` | `id`; any of `at` (location `[x, z]`), `size` (`[w, h]`, 1 × 1), `facing` (0), `slope`, `zone` (1), `state` (1), `livable`, `has_capacity` (default: a building stands), `days`, `wealth`, `historical`, `building`, `capacity`, `available` (default: the capacity), `road`, `organic` (true), `cfg_stage` (number or null), `can_create` (7); `remove: true` | a lot |
| `tdev_cfg` | `id` (token); any of `cfg_id` (the configuration ID, for `GetLotConfiguration`), `size`, `slope` (`[min, max]`, ±100), `stage` (1), `capacity` (`[min, max]`), `zones`, `wealths`, `purposes`, `styles` (absent = compatible with all), `roads` (required-road bits), `primary` (family IDs; null = no primary object) | a lot configuration (all are visible to the configuration manager, filtered by `IsLotConfigurationIncluded`) |
| `tdev_building` | `id` (type); any of `purpose` (1), `wealth` (1), `capacity` (1), `groups` (absent = every occupant group), `monopoly`, `fields` (`0x29B55F73` IDs) | a building type |
| `tdev_family` | `id`, `members` (BDS insertion order) | a building family |
| `tdev_cells` | `grid` (`zone`, `land_value`, `powered`, `watered`, `fire`, `park`, `road_access`), `default?`, `cells?` (`[[x, z, v]]`), `rects?` (`[[x1, z1, x2, z2, v]]`) | grid values |
| `tdev_networks` | `cells` (`[[x, z, [types]]]`) | network tiles and their `IsOfType` answers |
| `tdev_surrounding` | `rect?` (the lots `GetLotsSurroundingRect` appends for flag 1), `lots?` (`{"lot": [lots]}` for `GetLotsSurroundingLot`, flag 1); other flags append nothing | neighbour lists |
| `tdev_tuning` | `none: true`, or `shipped?` (true) with `props?` (a property map merged over it) and `remove?` (IDs) | the "Building Development" exemplar |
| `tdev_create` | `seed?` (the file-local generator's seed, injected; default `0x4C09C113`, the original's seed at clock 0), `crt_seed?` (1), `trace?` (true), `histogram?` (true), `shared?` (reuse the previous instance's statics) | a new tract developer |
| `tdev_rng` | `seed`, or `lcg` and `mul`; `crt?` | sets the shared generator / `rand()` state |
| `tdev_set` | any of `styles` (assigned as is), `style_index`, `years_since`, `years_between`, `single_style` | pokes the style state |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `tdev_init` / `tdev_shutdown` | `expect?` | `Init` / `Shutdown` |
| `tdev_message` | `msg`, `expect?` | `DoMessage` |
| `tdev_new_year` | `count?` (1) | `NewYear` |
| `tdev_set_active_styles` | `styles` | `SetActiveStyles` |
| `tdev_use_all_styles` | `value` | `UseAllStylesAtOnce` |
| `tdev_grow` | `dev`, `tract` (`[x, z]`), `stage`, `append?` (keep the previous outcome list), `out_flag_in?`; checks `expect` (built), `outcomes` (the whole list), `out_flag` | `Grow` |
| `tdev_decline` | `dev`, `tract`, `expect?` (the sum) | `Decline` |
| `tdev_update_population` | `dev`, `tract` | `UpdatePopulation` |
| `tdev_save` | `expect?` (hex), `ok?` | `Write` |
| `tdev_load` | `hex`, `expect?` | `Read` |
| `tdev_expect` | any of `style_index`, `years_since`, `years_between`, `single_style`, `all_styles`, `styles`, `current_style`, `desired` (`[desired, desiredMax]` of the last Grow), `chosen_cfg`, `chosen_building`, `lcg`, `mul`, `crt`, `initialised`, `configured`, `filters` (cached count), `tuning` (partial: `overbuild`, `reoccupy`, `redevelop_size_threshold`, `demolition`, `new_lot`, `occupancy_variation`, `min_vacancy`, `kick_out`, `available_styles`), `used` (resources), `lots` (existing tokens) | state |
| `tdev_expect_lot` | `lot`; any of `exists`, `state`, `facing`, `building`, `population` (`{devType: n}`), `rect` | a lot |
| `tdev_expect_hist` | `row`, `counts` (`{code: n}`; every other counter of the row is 0) | the debug histogram of the last Grow (§4.1) |
| `tdev_expect_draws` | `draws`: each `"Rn"` (kind only) or `["Rn", arg, value]` (exact); `keep?` | the draws since the last check |
| `tdev_expect_events` / `tdev_expect_event_count` / `tdev_clear_events` | see the shared conventions | host calls |
| `tdev_eval` | `fn`, `args`, `expect`: `outcome_codes`, `best [codes]`, `desired [demand, overbuild]`, `capacity_window [candidate, desiredMax, built]`, `rect [x, z, w, h, facing]`, `corner [bits, candidate]`, `roads_check [bits, candidate, requireRoads]` (21 or `[27, corner]`), `style_points [fraction]`, `style_group [[[key, fraction]], r]`, `score [candidate, zone, tuning {demolition, new_lot}, seed]` (`{score, bits, mul}`), `hash_order [values]`, `stl_sort [scores]` (sorted indices), `query_interface [iid]`, `zone_family [zone]`; a candidate is `{kind, existing, abandoned, n, facing, lots, street, road, avenue, rail}` | pure functions |

Tract developer events: `acquire_services`, `add_agent` (`level`, `message`), `acquire_message_server`,
`release_message_server`, `remove_agent`, `release_services`, `register_filter` (`wealth`, `purpose`, `filter`),
`unregister_filter` (`filter`), `claim` (`amount`, `ok`), `release` (`amount`), `needs_anchor` (`rect`, `purpose`),
`start_construction` (`lot`, `cfg`, `building`, `flag`), `abandon` (`lot`, `reason`), `reoccupy` (`lot`,
`dev_type`), `demolish` (`lot`), `delete_lot` (`lot`), `zone_draw` (`value`, R9), `create_lot` (`x`, `z`, `w`, `h`,
`facing`, `lot`), `redisplay` (`rect`), `set_facing` (`lot`, `facing`), `set_state` (`lot`, `state`),
`set_population` (`lot`, `dev_type`, `population`).

## Lot developer ops

(Being written: `ldev_…` ops for `fc4_core::lot_developer`, `docs/specs/lot-developer.md`.)

## Zones ops

(Being written: `zone_…` ops for `fc4_core::zones`, `docs/specs/zones.md`.)

## Adding a vector

1. Name it after the spec section it mainly covers, for example `s5_3_….json` (simulator), `f7_…` (foundations
   §7) or `b15_…` (budget §15), `d16_…` (demand §16), `o13_…` (ordinance §13), `w14_…` (weather §14),
   `g10_…` and `tg10_…` (simgrid §10, `fc4_core::grid`),
   `fl16_…` (flammability §16), `p11_…` (pollution §11), `c17_…` (crime §17), `a17_…` (aura §17),
   `pw15_…` (power §15), `pl14_…` (plumbing §14),
   `pr18_…` (protection §18), `fo16_…` (flora §16), `bd11_…` (building development §11),
   `gd14_…` (growth developer §14), `td11_…` (tract developer §11).
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
| 13–15 random generators, 16 uniform rounding and clamped Gaussian | `f7_13_15_random_generators` |
| §6.3 seed derivation (session seed → each generator's seed) | `f6_03_seed_derivation` |
| §14 16–17 variants | `f14_16_17_variants` |
| §14 18–19 property holders | `f14_18_19_property_holders` |
| §14 20–22 regional-city record, deal claiming, import/export | not yet (neighbours milestone) |
| §14 23 `rand()` | `f14_23_crt_rand`; the prime-step iterator that uses it: `w14_05_iterator` and the `fc4-core` unit tests (`weather::iterator`) |
| §14 24 month order | `s4_month_registration_order` (registration order within a level), `o13_10_month_order` (the ordinance-lag scenario with the real ordinance simulator and a stub budget, in both orders) |

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

## Coverage (demand §16)

| §16 item | Vector |
|---|---|
| 1 node arithmetic | `d16_01_node_arithmetic` |
| 2 active demand formula | `d16_02_active_demand` |
| 3 min/max ordering | `d16_03_min_max` |
| 4 propagation | `d16_04_propagation` |
| 5 shipped graph | `d16_05_shipped_graph_excerpt` (the shipped data cannot be vendored: the vector rebuilds the links of §4.5 that test 5 exercises) |
| 6 census | `d16_06_census` |
| 7 occupancy messages | `d16_07_occupancy` |
| 8 tax and economy | `d16_08_tax_economy` (the nation contract of §8.2 is a `fc4-core` unit test) |
| 9 cap relief | `d16_09_cap_relief` |
| 10 region | `d16_10_region` |
| 11 history | `d16_11_history` |
| 12 save round trip | `d16_12_save` |

## Coverage (ordinance §13)

| §13 item | Vector |
|---|---|
| 1 loading | `o13_01_loading` |
| 2 monthly amount | `o13_02_monthly_amount` |
| 3 availability | `o13_03_availability` |
| 4 enact/repeal accounting | `o13_04_accounting` |
| 5 messages | `o13_05_messages` |
| 6 quirks | `o13_06_quirks` |
| 7 cumulative queries | `o13_07_cumulative` |
| 8 array queries | `o13_08_arrays` |
| 9 save | `o13_09_save` |
| foundations §14 item 24 | `o13_10_month_order` |

## Coverage (weather §14)

| §14 item | Vector |
|---|---|
| 1 ambient values | `w14_01_ambient` |
| 2 season | `w14_02_season` |
| 3 wind | `w14_03_wind` |
| 4 temperature (and the §6.6 water scan and sea mask) | `w14_04_temperature` |
| 5 iterator | `w14_05_iterator` |
| 6 moisture | `w14_06_moisture` |
| 7 moisture variation (and §6.4.1 SetManualMoistureAddition) | `w14_07_variation` |
| 8 accessors | `w14_08_accessors` |
| 9 save | `w14_09_save` |
| 10 ambient and wind precision (§6.1, §6.2) | `w14_10_ambient_precision` |
| 11 table-mode iterator (§6.7) | `tg10_19_random_iterator` |
| 12 pass-2 smoothing reads pass-1 values (§6.5) | unit test `weather::simulator::tests::pass_two_reads_pass_one` |

## Coverage (simgrid §10)

| §10 item | Vector |
|---|---|
| 1 geometry | `g10_01_geometry` |
| 2–3 positions and bounds | `g10_02_03_positions_bounds` |
| 4–8 values, averages, tract size changes | `g10_04_08_values_averages` |
| 9–10 persistence | `g10_09_10_persistence` |
| 11–14 radius effect, rounding, clamping | `g10_11_14_radius_effect` |
| §7.1 huge radii and centres near the int32 limits (regression) | `g10_11b_radius_overflow` |
| 15–16 distance effect | `g10_15_16_distance_effect` |
| §7.2 radius above 255 rounds (regression) | `g10_15b_distance_rounds_cap` |
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
| 17 desirability mapping (scratch 2, desirability 4) | `lv16_17_desirability_mapping` |
| 18 desirability map smaller than the scratch area (skipped tracts; equal and coarser [choice]s) | `lv16_18_desirability_coverage` |

## Coverage (simgrid §10, `fc4_core::grid`)

| §10 item | Vector |
|---|---|
| 1 geometry | `tg10_01_geometry` |
| 2 positions | `tg10_02_positions` |
| 3 bounds | `tg10_03_in_bounds` |
| 4 cell value | `tg10_04_cell_value` |
| 5 averages | `tg10_05_averages` |
| 6 float average | `tg10_06_float_average` |
| 7 signed average | `tg10_07_signed_average` |
| 8 resize | `tg10_08_resize` |
| 9 record (and store save/load) | `tg10_09_record` |
| 10 read (and the §2 factories) | `tg10_10_read` |
| 11 radius effect | `tg10_11_radius_effect` |
| 12 radius rounding | `tg10_12_radius_rounding` |
| 13 round half even | `tg10_13_round_half_even` |
| 14 clamping and edges | `tg10_14_radius_clamp` |
| 15 distance effect | `tg10_15_distance_effect` |
| 16 land value distance | `tg10_16_distance_landvalue` |
| 17 two grids | `tg10_17_two_grids` |
| 18 scanning iterator | `tg10_18_scan_iterator` |
| §7.4 random cell iterator (table, prime-step, constructed) | `tg10_19_random_iterator` |

## Coverage (flammability §16)

| §16 item | Vector |
|---|---|
| Init, tuning (code defaults, shipped values, curve "delete" pattern) | `fl16_00_init_tuning` |
| 1 occupant raw value | `fl16_01_occupant_raw` |
| 2 tract priority | `fl16_02_tract_priority` |
| 3–4 daily value | `fl16_03_04_daily_value` |
| 5 cadence | `fl16_05_cadence` |
| 6 average | `fl16_06_average` |
| 7 hotspots | `fl16_07_hotspots` |
| 8 hotspot slot | `fl16_08_hotspot_slot` |
| 9 global multiplier | `fl16_09_global_multiplier` |
| 10–11 fire roll | `fl16_10_11_fire_roll` |
| 12–13 occupant update | `fl16_12_13_occupant_update` |
| 14 persistence (Read appends; failed reference) | `fl16_14_persistence` |
| 15 interface slots (portable part) | `fl16_15_slots` |
| 16 property conversion (`GetValSint32`) | `fl16_16_property_conversion` |
| 17 single-value curve | `fl16_17_single_value_curve` |
| 18 missing services | `fl16_18_missing_services` |
| 19 subscriptions, `0xC53D10AA` ignored | `fl16_19_subscriptions` |

## Coverage (pollution §11)

| §11 item | Vector |
|---|---|
| 1 cone | `p11_01_03_cones` |
| 2 default radii | `p11_01_03_cones` |
| 3 garbage rectangle | `p11_01_03_cones` |
| 4 skip (and the lists and budget of §4.2.3–4.2.4) | `p11_04_skip_lists_budget` |
| 5 relaxation | `p11_05_08_tract_visit` |
| 6 water stop at 1 | `p11_05_08_tract_visit` |
| 7 traffic | `p11_05_08_tract_visit` |
| 8 zone averaging | `p11_05_08_tract_visit` |
| 9 garbage production | `p11_09_12_garbage` |
| 10 Dispose waste-to-energy quirk (and export, landfill) | `p11_09_12_garbage` |
| 11 valve | `p11_09_12_garbage` |
| 12 valve closed in a cell | `p11_09_12_garbage` |
| 13 water treatment | `p11_13_15_capacities` |
| 14 efficiency | `p11_13_15_capacities` |
| 15 current capacity (and maintenance cost, §4.9.4) | `p11_13_15_capacities` |
| 16 monthly cost (and aging, usage roll, waste-to-energy output) | `p11_16_monthly` |
| 17 cadence | `p11_17_18_cycle` |
| 18 end of cycle | `p11_17_18_cycle` |
| 19 slots | `p11_19_20_slots` |
| 20 prop share | `p11_19_20_slots` |
| 21 effects | `p11_21_effects` |
| 22 persistence | `p11_22_persistence` |
| 23 deal rounding | `p11_23_deal_rounding` |
| 24 usage distribution | `p11_24_usage_distribution` |
| 25 Read with a stream error | `p11_25_read_error` |
| `_ftol` truncation (foundations §18.1) and curve patterns (foundations §4.1, §4.3) | `p11_26_trunc_curves` |
| other sections (lifecycle §4.1, disaster radiation §4.3, tuning §7.1, SimBegin) | `p11_00_lifecycle` |

## Coverage (crime §17)

| §17 item | Vector |
|---|---|
| 1 decay | `c17_01_decay` |
| 2 census buckets | `c17_02_census` |
| 3 jobless factor | `c17_03_jobless` |
| 4 criminal base | `c17_04_criminal_base` |
| 5 crimes to commit | `c17_05_crimes_to_commit` |
| 6 ordinance rounding | `c17_06_ordinance_rounding` |
| 7 attenuation (u = 1.0 commits) | `c17_07_attenuation` |
| 8 stamp and hotspot | `c17_08_stamp_hotspot` |
| 9 type selection (and §6.8 severity 0) | `c17_09_type_selection` |
| 10 mask parsing (and the §7 type list) | `c17_10_masks` |
| 11 crime-effect cone | `c17_11_crime_effect` |
| 12 average crime | `c17_12_average` |
| 13 pipeline | `c17_13_pipeline` |
| 14 RNG draw count (and the §6.1 24-bit sequences of a seeded generator) | `c17_14_rng_draws`, `c17_23_rng_seeded` |
| 15 incident array (core part; the 28-byte layout is the plugin's) | `c17_15_incident_array` |
| 16 prop requester | `c17_16_prop_requester` |
| 17 save | `c17_17_save` |
| 18 lifecycle | `c17_18_lifecycle` |
| 19 `rhe` beyond 2²² | `c17_22_rhe` (also `c17_16_prop_requester`) |
| 20 Read with a stream error | `c17_17_save` |
| 21 SelectProp, no lot | `c17_16_prop_requester` |
| 22 nearest crime sim distance | `c17_20_nearest` |
| §8 tuning | `c17_19_tuning` |
| §11.3 nearest crime sim / prop | `c17_20_nearest` |
| §6.8 arson | `c17_21_arson` |

## Coverage (aura §17)

| §17 item | Vector |
|---|---|
| Init, Shutdown, tuning (defaults, "delete" curve pattern, exemplar reload) | `a17_00_lifecycle_tuning` |
| 1 ABI (slots 6 / 7, getters, class, QueryInterface) | `a17_01_abi` |
| 2 park funding (int16 wrap, NaN), 19 no Parks department | `a17_02_park_funding` |
| 3 landmark funding (int16 wrap) | `a17_03_landmark_funding` |
| 4 tax effect | `a17_04_tax_effect` |
| 5 lot base aura | `a17_05_lot_base_aura` |
| 6 work stamping (and the time slice with an expiring clock) | `a17_06_work_stamping` |
| 7 curves on cells | `a17_07_curves_on_cells` |
| 8 long decay | `a17_08_long_decay` |
| 9 global effect | `a17_09_global_effect` |
| 10 aura and rating | `a17_10_aura_rating` |
| 11 transient | `a17_11_transient` |
| 12–13 hotspots and riot | `a17_12_13_hotspots_riot` |
| 14 cadence | `a17_14_cadence` |
| 15 occupant cones | `a17_15_occupant_cones` |
| 16 persistence (and a failed reference, foundations §2.7) | `a17_16_persistence` |
| 17 BustStrike | `a17_17_bust_strike` |
| 18 registry | `a17_18_registry` |
| 17 desirability mapping (scratch 2, desirability 4) | `lv16_17_desirability_mapping` |
| 20 empty curves | `a17_20_empty_curves` |
| 21 population mapping | `a17_21_population_mapping` |
| 22 time-slice step | `a17_22_time_slice` |
| §6 integer stores (`_ftol`, low bits) | `a17_23_integer_stores` |

## Coverage (power §15)

| §15 item | Vector |
|---|---|
| tuning (code defaults, "default" curve pattern, reload, ordinance factor) | `pw15_00_tuning` |
| budget hookup and removal, FindPowerPlants, GetPowerPlants, lots, deal status (§5, §5.1, §5.2) | `pw15_00_budget_lots_deals` |
| terrain changes, PostCityInit, pole and network removal, Shutdown (§5.2, §5.4, §5.6, §4.2) | `pw15_00_terrain_postinit` |
| int64 line-item secondary info (§5.1, §5.2) | `pw15_00_secondary_int64` |
| 1 output, 2 slot 45/46 ABI (and slots 32–34, 49–51) | `pw15_01_02_output_abi` |
| 3 single network, 4 shortfall, 5 conservation | `pw15_03_05_network_supply` |
| 6 import, 7 export | `pw15_06_07_import_export` |
| 8 flood fill, 9 wire radius, 10 centre cell | `pw15_08_10_flood_wire_centre` |
| 11 grid values (and dirty wiring, §5.3) | `pw15_11_grid_values` |
| 12 positions, 19 strikes (and identity, §1) | `pw15_12_19_positions_strikes` |
| 13 maintenance | `pw15_13_maintenance` |
| 14 ageing | `pw15_14_ageing` |
| 15 pole decay | `pw15_15_pole_decay` |
| 16 deficit, 17 unpowered % | `pw15_16_17_deficit_unpowered` |
| 18 ring | `pw15_18_ring` |
| 20 save | `pw15_20_save` |
| 21 lifecycle | `pw15_21_lifecycle` |
| 22 Nuclear Free Zone | `pw15_22_nuclear_free_zone` |
| 23 maintenance with departments, 24 FindPowerPlants with departments | `pw15_23_24_budget_departments` |
| 25 counters below zero, 26 pole removal | `pw15_25_26_counters_poles` |
| 27 terrain passes | `pw15_27_terrain` |
| 28 NaN ratio | `pw15_28_nan_ratio` |
| 29 Init after load, 30 failed Read | `pw15_29_30_init_load` |

## Coverage (plumbing §14)

| §14 item | Vector |
|---|---|
| Init, Shutdown, references, the agent and the department | `pl14_00_lifecycle` |
| Buildings, budget items, maintenance, the consumer-removal and lotless-producer quirks | `pl14_00_buildings_budget` |
| Shared seed cells (§6.2) | `pl14_00_seeding` |
| 1 defaults, 2 shipped tuning | `pl14_01_02_tuning` |
| 3 pipe flags, 4 pipe cost, 16 burst bookkeeping | `pl14_03_04_16_pipes` |
| 5 pump capacity, 6 pollution shutdown, 7 power, 8 condition curve | `pl14_05_08_production` |
| 9 basic flood, 10 shortage | `pl14_09_10_flood` |
| 11 ring geometry | `pl14_11_ring_geometry` |
| 12 export | `pl14_12_export` |
| 12 export flags per network, 0x28 (PL7) | `pl14_12_export_flags` |
| 13 import | `pl14_13_import` |
| 14 pipe funding, 15 GaussianFast | `pl14_14_15_pipe_funding` |
| 17 deal message, 18 disabled | `pl14_17_18_messages` |
| 19 grid values, 20 grid swap (and the time slice) | `pl14_19_20_grids` |
| 21 NearestWater | `pl14_21_nearest_water` |
| 22 ageing | `pl14_22_ageing` |
| 23 persistence | `pl14_23_persistence` |
| 24 SDK return channels | `pl14_24_slots` |
| 25 iterator after a pass (PL4) | `pl14_25_iterator` |
| 26 current producer in a save (PL3) | `pl14_26_current_producer` |
| 27 re-Init after Shutdown (PL2) | `pl14_27_reinit` |

## Coverage (protection §18)

| §18 item | Vector |
|---|---|
| Init, Shutdown, Release, tuning (code defaults, shipped exemplar, "default" curve pattern, reload filter) | `pr18_00_lifecycle` |
| 1 fire efficiency curve | `pr18_01_fire_efficiency_curve` |
| 2 road and strike modifiers, fundingOf | `pr18_02_efficiency_modifiers` |
| 3 jail efficiency | `pr18_03_jail_efficiency_curve` |
| 4 strike curve and the single draw | `pr18_04_strike_curve` |
| 5 chance of strike | `pr18_05_chance_of_strike` |
| 6 strike end (duration, funding, unsigned days) | `pr18_06_strike_end` |
| 7 monthly strike pass (posts, unadjusted rebuild only) | `pr18_07_monthly_strike_pass` |
| 8 jailbreak curve | `pr18_08_jailbreak_curve` |
| 9 rehabilitation | `pr18_09_rehabilitation` |
| 10 ResetUnjailedCriminals | `pr18_10_reset_unjailed` |
| 11 Recalculate and the jail lists | `pr18_11_recalculate` |
| 12 escapees, jailbreak begin and end | `pr18_12_escapees` |
| 13 crime events and ProcessMonthlyCrimeStats | `pr18_13_crime_event` |
| 14 occupant coverage | `pr18_14_occupant_coverage` |
| 15 effect scaling | `pr18_15_effect_scaling` |
| 16 dispatch masks, nearest station, dispatch cones | `pr18_16_dispatch` |
| 17 call numbers | `pr18_17_call_numbers` |
| 18 cadence | `pr18_18_cadence` |
| 19 cycle averages and the daily step | `pr18_19_cycle_averages` |
| 20 adjusted copy and funding change | `pr18_20_adjusted_copy` |
| 21 persistence (layout, versions, store round trip) | `pr18_21_persistence` |
| 22 slot ABI, QueryInterface, crime-compatible slots | `pr18_22_slot_abi` |
| 23 stuck jailbreak (capacity 0, NaN percentage) | `pr18_23_stuck_jailbreak` |
| 24 jailed percentage and the end and start tests | `pr18_24_jailed_percentage` |
| 25 percentages (float32 0.01f) | `pr18_25_percentages` |
| 26 GetMaxPossibleDispatchCount | `pr18_26_max_possible_dispatch` |
| 27 station effect partial reads | `pr18_27_effect_partial_reads` |
| 28 call-number search | `pr18_28_call_number_search` |
| 29 station statistics | `pr18_29_station_statistics` |
| 30 jail capacity without an item or property | `pr18_30_jail_capacity` |
| 31 locally-funded-jail list | `pr18_31_funded_jail_list` |

## Coverage (flora §16)

| §16 item | Vector |
|---|---|
| constructor, Init, Shutdown, tuning (shipped and code defaults), QueryInterface, reference count (§1, §3.1, §4, §7) | `fo16_00_init_tuning` |
| 1 stage, 2 negative age | `fo16_01_02_stage` |
| 3 elevation index, 4 score | `fo16_03_04_elevation_score` |
| 5 exemplar table conversion (and the record's other fields) | `fo16_05_table_conversion` |
| 6 best type (and the key-0 record) | `fo16_06_best_type` |
| 7 blocksPerMonth (and the monthly iterator), 8 wild seed scaling (and wild seeding) | `fo16_07_08_blocks_wild` |
| 9 daily, 10 daily with rate > 1 | `fo16_09_10_daily` |
| 11 DropSeed | `fo16_11_drop_seed` |
| 12 location bounds, 13 slope, 14 under water, 15 blocking | `fo16_12_15_location` |
| 16 planting altitude, 17 cluster merge | `fo16_16_17_planting` |
| 18 PlaceFlora | `fo16_18_place_flora` |
| 19 seeding, 20 seeding interval | `fo16_19_20_seeding` |
| 21 low score | `fo16_21_low_score` |
| 22 kill (and RemoveOccupant) | `fo16_22_kill` |
| 23 building insert (and networks) | `fo16_23_building_insert` |
| 24 terrain change | `fo16_24_terrain` |
| 25 cheat (and blast, exemplar-changed routing), 26 established | `fo16_25_26_cheat_established` |
| 27 persistence, 28 Init after load | `fo16_27_28_persistence` |
| 29 GetAllFloraTypes (and the other type queries) | `fo16_29_all_types` |
| 30 obliteration | `fo16_30_obliteration` |
| 31 AdvanceClusterType, 32 seed types | `fo16_31_32_clusters_seed_types` |

## Coverage (building development §11)

| §11 item | Vector |
|---|---|
| 1 developer types, 2 map keys, 3 zone fits | `bd11_01_03_types_keys_zones` |
| 4 monthly supply | `bd11_04_monthly_supply` |
| 5 claim / release | `bd11_05_construction_resources` |
| 6 grow order (seeds 12345 and 1); the multi-day draw sequence | `bd11_06_grow_order`, `bd11_06_grow_days` |
| 7 tried-tract marks (and their save / Shutdown) | `bd11_07_tried_tract_marks` |
| 8 decline plan (seed 99), 9 decline execution | `bd11_08_09_decline` |
| 10 max density | `bd11_10_max_density` |
| 11 suitability | `bd11_11_suitability` |
| 12 building map, 13 highest stage | `bd11_12_13_building_map_stage` |
| 14 record | `bd11_14_record` |
| 15 stage multiplier (and the tuning reads) | `bd11_15_stage_multiplier` |
| 16 catalogue queries | `bd11_16_catalogue` |
| Init, Shutdown, Release, QueryInterface (§1) | `bd11_00_lifecycle` |
| DoIdle / monthly state machine (§3.1, §4.1, §4.2) | `bd11_00_idle_monthly` |
| messages: cheats, special buildings, terrain unpacking, PlaceBuilding (§3.2, §4.12) | `bd11_00_messages` |
| DoTerrainChanged (§4.11) | `bd11_00_terrain_changed` |
| placing buildings (§4.9) | `bd11_00_placing` |
| Grow and decline edges (§4.3–§4.5) | `bd11_00_grow_edges` |
| the daily Grow attempt budget ([choice] replacing the 3 ms timer, §4.4) | `bd11_06_grow_budget` |
| persistence beyond test 14 (§6) | `bd11_00_persistence` |
| catalogue loading, names, keys (§4.7.1–§4.7.3); hash order and resize | `bd11_00_catalogue_extras`, `bd11_00_catalogue_hash_order` |

## Coverage (growth developer §14)

| §14 item | Vector |
|---|---|
| Creation, Init, Shutdown, references, QueryInterface, slot table | `gd00_lifecycle` |
| Getters, construction info, state machine bounds | `gd00_getters` |
| Every desirability layer and EstimateDesirability against a model | `gd00_layers` |
| Special buildings (states 14 → 16) | `gd00_special_buildings` |
| Time slicing of state 18 (shared step) | `gd00_slicing` |
| The game's `std::sort` and its tie order | `gd00_sort_ties` |
| Shipped exemplars (§4.9) | `gd00_shipped_tuning` |
| Draw sequence over several developers (§7): one draw per industrial update, the shared generator's state, salts as a multiset ([compat-limit]) | `gd00_draw_sequence` |
| The BDS's developer operations, one method each (§12, BDS §1.5) | `gd00_replacement_unit` |
| 1 population | `gd14_01_population` |
| 2 max capacity | `gd14_02_max_capacity` |
| 3 built capacity | `gd14_03_built_capacity` |
| 4 stage assignment | `gd14_04_stage_assignment` |
| 5 AddCurve wrap | `gd14_05_wrap` |
| 6 commute | `gd14_06_commute` |
| 7 EstimateDesirability vs update | `gd14_07_estimate` |
| 8 lists | `gd14_08_lists` |
| 9 industrial decline order (seeded RNG) | `gd14_09_industrial_decline_rng` |
| 10 NewDay rule | `gd14_10_newday` |
| 11 phantom demand (by the rules: 39, see the vector) | `gd14_11_phantom` |
| 12 style map | `gd14_12_style_map` |
| 13 tuning defaults | `gd14_13_tuning_defaults` |
| 14 save round trip | `gd14_14_save` |
| 15 subjective thresholds | `gd14_15_subjective` |
| 16 StateChanged | `gd14_16_state_changed` |

## Coverage (tract developer §11)

| §11 item | Vector |
|---|---|
| 1 outcome codes and `best` | `td11_01_02_codes_desired` |
| 2 desired / desiredMax | `td11_01_02_codes_desired` |
| 3 pass budget (18 passes; a failed relaxed pass ends the call) | `td11_03_pass_budget` |
| 4 strict vs relaxed (rule 7) | `td11_04_strict_relaxed` |
| 5 takeover thresholds | `td11_05_takeover` |
| 6 score, residential type 2 | `td11_06_09_score` |
| 7 score, commercial | `td11_06_09_score` |
| 8 score, aggregation and subdivision lot costs | `td11_06_09_score` |
| 9 industrial random term | `td11_06_09_score` |
| 10 bounding rectangle | `td11_10_15_geometry_window_styles` |
| 11 corner test | `td11_10_15_geometry_window_styles` |
| 12 capacity window | `td11_10_15_geometry_window_styles` |
| 13 style percentages → group | `td11_10_15_geometry_window_styles` |
| 14 seeded PickBuilding draws (R3–R5), inside a full Grow | `td11_14_pick_building_draws` |
| 15 hash order tie | `td11_10_15_geometry_window_styles`, `td00_hash_growth_sort` |
| 16 seeded UpdatePopulation (R7), incl. the shipped variation 0 and tract (0, 0) (seed 1, [choice]) | `td11_16_17_update_population` |
| 17 population clamps | `td11_16_17_update_population` |
| 18 Decline (R8, multi-cell lot counted once) | `td11_18_decline` |
| 19 save record (v3 bytes, v1 / v2 loads, v4 rejected, truncated records) | `td11_19_save_record` |
| 20 style rotation | `td11_20_21_styles` |
| 21 SetActiveStyles | `td11_20_21_styles` |
| 22 subdivision geometry (facings 2 and 0, inside full Grows) | `td11_22_subdivision` |
| 23 farm minimum (14 → 0, 15 → Claim(45), Release(30); R6) | `td11_23_farm` |
| 24 IsLotSuitable order | `td11_24_lot_suitable_order` |
| extra: STLport hash_set growth and `std::sort` (introsort, ties) | `td00_hash_growth_sort` |
| extra: per-pass draw order of a full Grow with several scored candidates (R1, R2 × 3, R3, R4, R5) and a scored takeover | `td00_industrial_grow_draws` |
| extra: lifecycle, messages, QueryInterface, filters, tuning reader | `td00_lifecycle_tuning` |
| extra: aggregation, Build without a lot [quirk], resource failure | `td00_aggregation_build` |
| extra: R9 (CreateLotIfPossible with facing −1) in the per-pass order | `td00_aggregation_r9` |
| extra: kick-out, caps (building developer's power, own fire), style checks [quirk] | `td00_kickout_caps_styles` |

## Coverage (lot developer §16)

(Being written: vectors `ld16_…` and `ld00_…`.)

## Coverage (zones §15)

(Being written: vectors `zn15_…` and `zn00_…`.)

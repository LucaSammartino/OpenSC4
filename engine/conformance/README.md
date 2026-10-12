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
| `weather_create` | `seed?` (the wall-clock seed), `rng?`: `{seed}` (a real generator with that seed) or `{stub: {uniform, gaussian, int}}` (queued values; each queue repeats its last value, defaults 0.5 / 0 / 0) | a new simulator (the default generator is wall-clock seeded) |
| `weather_rng` | `seed` or `stub` as in `weather_create` | replaces the simulator's generator |

### Calls and checks

| op | Arguments | Effect |
|---|---|---|
| `weather_init` / `weather_shutdown` | `expect?` | `Init` / `Shutdown` (core parts) |
| `weather_message` | `type` (`SimNewDay`, `SimNewWeek`, `SimNewMonth`, `PostCityInit` or an ID), `date?` (`{year, month, day}`, the day message's data1), `data1?`, `data3?` (the month), `expect?` | `DoMessage` |
| `weather_call` | `method`, `args?`, `expect?` | a simulator method: `do_post_city_init`, `do_complete_simulation`, `do_monthly_simulation`, `do_weekly_simulation`, `do_daily` (`[julian]`), `new_current_ambient_values` (`[seasonMonth]`), `onshore_direction` (result `[x, z]`), `simulate_cell_temperature`, `simulate_moisture` (`[x0, z0, x1, z1]` cells), `update_moisture` (world units), `generate_moisture_variation`, `rebuild_water`, `set_manual_moisture_addition`, `cell_width_x/z`, `cell_count_x/z`, `position_to_cell` (`[result, cx, cz]`), `cell_to_position`, `cell_to_standard_city_cell`, `standard_city_cell_to_cell`, `current_season`, `current_season_month`, `moisture_value`, `moisture_type`, `moisture_map` (the grid), `humidity`, `ambient_wind` / `wind_at_cell` (`{speed, v}`) |
| `weather_set` | `state` (scalar fields as in `weather_expect`; `*_fill` fills a grid; `sea_mask_cells`, `moisture_cells`, `moisture_base_cells`, `temperature_cells`: `[{x, z, w?, d?, v?}]`) | pokes state (test setup) |
| `weather_expect` | `state` (any §3 field in snake case, plus `water_rects`, `iterator` (`state`: `running`/`exhausted`, `mode`, `rect`, `count`, `index`, `current`, `step`) and `rng` (`lcg`, `mul`, `has_cached`, `cached`); `iterator` and `rng` match partially) | the simulator state |
| `weather_expect_grid` | `grid` (`temperature`, `moisture`, `moisture_base`, `sea_mask`) and any of `size`, `rows`, `all`, `cells` (`[{x, z, w?, d?, v}]`), `rect` (`[x0, z0, x1, z1)`) with `inside` / `outside`, `range` (`[lo, hi]`), `min`, `max` | grid contents |
| `weather_iterator` | `rect` (`[x0, z0, x1, z1]`, inclusive), `shuffle?` (default true), `crt_seed?` (default 1), `rand_calls?`, `step?`, `count?`, `current?`, `first?` (`[[x, z], …]`), `full_pass?` | a free-standing prime-step iterator (§6.7) |
| `weather_save` / `weather_load` / `weather_expect_save` | `slot`; load: `rng?` (the loaded object's generator before Read), `expect?` | the class-object store; a load makes the loaded simulator current |
| `weather_patch` / `weather_expect_payload` | see the shared conventions (the class is always the weather simulator) | |

Weather events: `post` (`type`: `SeasonChanged`, `WeatherCycleCompleted`, `MoistureMapUpdated` or `other`;
`data2`), and with a stub generator `rng` (`kind`: `uniform`, `gaussian`, `int` (with `lo`, `hi`); `value`).
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
| `flam_env` | any of `cells` (`[countX, countZ]`, default 64 × 64), `occupants` (`[{id, type (building, prop, flora or an ID), flam (an integer, a variant object, or null = no property), preview?, lot?, cell or rect ([x0, z0, x1, z1]), pos? ([x, y, z])}]`, added to the listing), `unlist` (ids no longer listed in their cells; still known by id), `forget` (ids removed completely), `lots` (`[{id, condition, building?}]`), `random_lot` (id or null), `factor` (the ordinance factor 0x28F42AA0, null = none; default 1), `season` (null = no weather; default 0), `unwatered` / `unwatered_all`, `zoned` / `zoned_all`, `tutorial`, `fire_manager` (default true) | the fake world |
| `flam_tuning` | `shipped` (the §8 exemplar), `props` (a property map; merged over the shipped values with `shipped`, else alone), `remove` (property IDs removed afterwards), or `none` (no exemplar) | the tuning exemplar the next `Init` reads |
| `flam_create` | `rng?` (`{r: [...]}` queued `RzRandom` doubles, repeating 50 when empty, or `{process_clock}` the real generator with that seed; default the queue), `crt_seed?` | a new simulator |
| `flam_rng` | as `rng` in `flam_create` | replaces the simulator's generator |

### Calls and checks

| op | Arguments | Effect / check |
|---|---|---|
| `flam_init` / `flam_shutdown` | `expect?` | `Init` / `Shutdown` (portable parts) |
| `flam_message` | `type` (`SimNewDay`, `SimBegin`, `InsertOccupant`, `RemoveOccupant`, `ChangeOccupant`, `SeasonChange`, `OrdinanceUpdated`, `ConditionChanged`, `ExemplarChanged` or an ID), `occupant?`, `lot?`, `expect?` | `DoMessage` |
| `flam_days` | `count`, `ends?` (the days, 1-based in this op, on which SimCycleComplete was posted) | `count` SimNewDay messages |
| `flam_call` | `method`, `args?`, `expect?` | pure functions: `occupant_raw_value` (`[p, m]`), `daily_value` (`[r, g, watered, noWater]`), `global_multiplier` (`[factor, season, summer]`, null = missing), `compute_hotspots` (`[candidates, radius, minWeight]`), `query_interface` (`[iid]`); simulator methods: `occupant_raw`, `tract_raw`, `occupant_update` (`[id, add]`), `compute_global_multiplier`, `is_cell_flammable`, `hotspot` (`[v, x, z]` or false), `average`, `flammability_grid` (its instance ID or null), `daily`, `end_of_cycle`, `compute_hotspot_list` (the new list), `fire_roll`, `curve` (`[x]`, null without a curve), `add_ref`, `release` |
| `flam_set` | any of `raw`, `adjusted` (cells), `raw_fill`, `adjusted_fill`, `state` (`global_multiplier`, `average`, `zoned_sum`, `zoned_count`, `tracts_per_day`, `candidates`, `hotspots` (`[[x, z, weight]]`), tuning fields, `curve` (flat pairs or null)) | pokes state (test setup) |
| `flam_expect` | `state`: any of `initialized`, `ref_count`, `destroyed`, `count_x/z`, `average`, `global_multiplier`, `zoned_sum`, `zoned_count`, `candidates`, `hotspots`, `tracts_per_day`, the tuning fields, `has_curve`, `has_adjusted`, `has_raw`, `iterator` (`state`, `mode`, `dirty`, `rect`, `count`, `index`, `current`, `step`; matches partially) | the simulator state |
| `flam_expect_grid` | `grid` (`raw`, `adjusted`) and any of `counts`, `tract_size`, `instance`, `cells`, `others` / `all` (every unlisted tract) | grid contents |
| `flam_expect_rand` | `calls` | the number of `rand()` calls on the subsystem's CRT generator since the last check |
| `flam_expect_visits` | any of `all`, `cells` (`[[x, z, n]]`), `total` | daily visits (one zone-type query each) since the last check |
| `flam_expect_events` / `flam_expect_event_count` / `flam_clear_events` | see the shared conventions | host calls |
| `flam_save` / `flam_load` / `flam_expect_save` | `slot`; load: `rng?`, `expect?` (the store-level result; the simulator's Read result is ignored, foundations §3.5) | the class-object store; loaded grids are registered with the fake host |
| `flam_patch` / `flam_expect_payload` | see the shared conventions; `flam_patch` takes `class?` (default the simulator) to patch a grid record | |

Flammability events: `create_grid` (`instance`, `size`), `grid_shutdown` (`instance`), `random_lot` (`lot`),
`history` (`type`, `value`), `ignite` (`x`, `z`, `use_probability`, `teleport`, `fire_iid`), `cause` (`cause`),
`post` (`type`: `SimCycleComplete`), and with a queued generator `rng` (`value`, `a`, `b`).

## Adding a vector

1. Name it after the spec section it mainly covers, for example `s5_3_….json` (simulator), `f7_…` (foundations
   §7) or `b15_…` (budget §15), `d16_…` (demand §16), `o13_…` (ordinance §13), `w14_…` (weather §14),
   `g10_…` (simgrid §10, `fc4_core::simgrid`), `tg10_…` (simgrid §10, `fc4_core::grid`),
   `fl16_…` (flammability §16).
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
| 14 persistence | `fl16_14_persistence` |
| 15 interface slots (portable part) | `fl16_15_slots` |

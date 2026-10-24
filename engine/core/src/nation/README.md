# Nation core

`engine/core/src/nation/mod.rs` implements the portable application-owned portion of
`cSC4Nation`, from nation.md and its economy dependency demand.md §8.2/§11.2.
No plugin or C ABI wiring is included. There are no nation serialization records:
nation.md §1 explicitly excludes them, so codec round trips do not apply. Tests
instead verify deterministic loading of identical property inputs and retention
of application state through Shutdown/Init.

Use one `ProcessEconomy` per application process (`ProcessEconomy::global()` in a
future adapter). Its thread-safe cache reads tuning only once, including across
multiple nation instances and Shutdown/Init cycles. Getters use shared immutable
state, without RefCell. The filesystem supplies a `RegionCatalog` of owned region
handles. Region activation calls the region interface through `RegionActivation`.
These handles carry no borrowed COM pointer ownership inside the core.

## Owner closure

| Owner / spec section | Code item | Test / open contract |
|---|---|---|
| cSC4Nation, nation §1: application state | `Nation`, `RegionCatalog`, `ProcessEconomy` | `lifecycle_messages_and_once_per_process_tuning`; discovery remains open: NQ1 |
| cSC4Nation, nation §1: serialization | No serializer or city save record, as specified | Not applicable: no record and no codec; lifecycle test retains application state |
| cSC4Nation, nation §2: economy | `EconomyTuning::get_economy_modifier`, `EconomyDate::unsigned_months` | `demand_lookup_fallback_unsigned_date_and_rounding`; invalid difficulty/missing fallback open: NQ3 |
| cSC4Nation, nation §3: messages | `init`, `shutdown`, `do_message` | `lifecycle_messages_and_once_per_process_tuning`; tutorial filesystem effects open: NQ1 |
| cSC4Nation, nation §4: slots 0–2, 17 | COM identity, refcounts, destructor belong to future adapter | Outside portable core scope; no COM or ABI introduced |
| cSC4Nation, nation §4: slots 3–4 | `init`, `shutdown` (tuning and subscriptions) | `lifecycle_messages_and_once_per_process_tuning`; filesystem lifecycle open: NQ1 |
| cSC4Nation, nation §4: slots 5–8, 11, 15 | Directory, count, active index/region and indexed region getters | `catalog_getters_and_ordered_activation`; initial/invalid index contract open: NQ2 |
| cSC4Nation, nation §4: slot 9 | `set_active_region` | `catalog_getters_and_ordered_activation`; same-index/error contract open: NQ2 |
| cSC4Nation, nation §4: slots 10, 12–14 | Explicit Unsupported methods | `unsettled_filesystem_operations_fail_without_catalog_mutation`; open: NQ1 |
| cSC4Nation, nation §4: slot 16 | `Nation::get_economy_modifier` | `lifecycle_messages_and_once_per_process_tuning` and economy tests |
| cSC4Nation, nation §5: conformance vectors 1–2 | `engine/conformance/tests/nation.rs` | `nation_section5_vector1_shipped_difficulty_and_no_city`, `nation_section5_vector2_month_before_start_is_unsigned` |
| cSC4Nation, demand §8.2 | Demand curve lookup/default and difficulty addition | `demand_lookup_fallback_unsigned_date_and_rounding` |
| cSC4Nation, demand §11.2 | `TuningProperties`, `EconomyTuning::from_properties`, `ProcessEconomy::load_once` | `missing_empty_short_and_duplicate_tuning`, `property_indices_are_plain_addition_and_inputs_are_deterministic`, `process_tuning_and_getters_are_thread_safe`; UB replacements open: NQ3 |

## Remaining contracts and shadow feasibility

NQ1 is a closure blocker: the spec supplies filesystem method names, but no
algorithms, arguments or failure behavior. Discovery, preference matching,
addition, deletion and tutorial reset return `Unsupported`, without mutation.
In particular, reset messages reach the reset method and propagate its error.
These are explicit gaps, not claims of a complete filesystem replacement.
NQ2 records safe provisional index/lifecycle choices; NQ3 records safe errors for
the original undefined reads beyond the specified missing-data defaults.

An economy-only in-game shadow is feasible next: capture dates, city presence,
difficulty and demand ID through game interfaces, share process tuning, and
compare slot 16 outputs. It requires future capi/plugin work. A filesystem shadow
needs NQ1 settled first. This lane requests no game run because it adds no runtime
wiring and cannot emit a shadow comparison.

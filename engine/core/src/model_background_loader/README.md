# Model background loader

Portable `cSC4ModelBackgroundLoader`, from `docs/specs/model-maker.md` §15.
One mutex protects the shared worker/main state. Thread launch/sleep/wait,
native timers, the shared clockwise spiral, model-ID utility, resource manager,
occupant manager, simulator and model maker are foreign `Host` operations.
Getters return locked snapshots. Shutdown releases the lock before joining the
worker so the worker can post its exit command. `destroy(host)` is the explicit
native destructor boundary; a host adapter must call it before dropping ports.

**Open MM1** (`docs/specs/questions/model-maker.md`): sampled curves return
`Unsupported::CurveInterpolationMm1`, and time-to-Duration conversion returns
`Unsupported::ClockUnitsMm1`. Constant fallback curves work using opaque native
timer values. No interpolation or clock-unit guess is hidden in a host default.
Exemplar loading supplies decoded `Tuning` through a trait, including all points;
the core preserves sampled curves and refuses to evaluate them.

| Spec section | Code item | Spec case / test and boundary |
|---|---|---|
| 1–14 Model-maker owner | Separate `model_maker` module | See its closure table; no background-loader operations specified here |
| 15.1 Lifetime | `init`, `shutdown`, `destroy`, `Binding`, `Event` | `m24_radius_m25_pause_dedup_and_lifetime`: init once, wake-before-wait, reference-release order |
| 15.2 Tuning | `Tuning`, `Curve`, `native_delay_duration` | `mm1_is_explicit_and_vector_growth_copies_prefix_values_and_suffix`; **open MM1 Unsupported** for interpolation and physical time units |
| 15.3 Radius/tasks | `expand_load_rect`, `start`, `refocus`, `cancel`, `clear_resources` | M24 `m24_radius_m25_pause_dedup_and_lifetime`; `screen_width_factors_preserve_all_five_spec_cases`; running-state/refocus in M26 test; `worker_retains_unique_resources_and_posts_completion_in_order` |
| 15.4 Worker | `worker_slice`, `continue_task`, `load_resources` | `worker_retains_unique_resources_and_posts_completion_in_order`; `query_disabled_costs_nothing_loaded_masks_skip_resources_and_pause_stops_walk`; sampled curves stop at **open MM1** |
| 15.5 Main thread | `process_communications`, `process_cells`, `pause`, `resume` | M25 `m24_radius_m25_pause_dedup_and_lifetime`; M26 `m26_partial_main_pass_keeps_commands_and_refocus_keeps_running_state`; sampled curves stop at **open MM1** |
| 15.6 Containers | `Task`, `VecDeque`, `Commands::grow_insert` | `mm1_is_explicit_and_vector_growth_copies_prefix_values_and_suffix`: prefix/copies/suffix and atEnd grow paths; native allocation addresses are not reproduced |
| 19 Conformance | M24/M25/M26 tests above | M27 is a model-maker lifecycle/view case, pending there |
| 20 Inventory | Semantic methods above | No C ABI, plugin registration or native thread loop exported |

Only type-0 tasks can be constructed by this API, matching all specified callers.
Resource getters borrow; acquire/load results and occupant lists carry one owned
reference. A resource pointer already retained in the set is released immediately;
the set's one remaining reference is released at restart/cancel/shutdown.

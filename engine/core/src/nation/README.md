# Nation core

Portable `cSC4Nation` owner from nation.md §§1–8 and demand.md §11.2.
`NationHost` supplies platform/game interfaces; `NationRegion` supplies region
operations, including the region factory (§6), and `TutorialCity` supplies the
regional-city interface. Returned region/city handles are owned and release on
Drop. Getters borrow them without taking or releasing COM references. Getters
use immutable shared state; no RefCell is involved. This object has no city save
record, so no codec is applicable.

Use one `ProcessDirectories` per process (or `ProcessDirectories::global()`). Its
OnceLock stores the two directory strings and consumes StaticInit even without
an app. Tuning belongs to each nation, is destroyed at Shutdown, and is reread at
the next Init. Difficulty survives Shutdown and resize-to-three retains existing
elements when an explicitly empty ID list skips difficulty loading.

## Owner closure

| Owner / nation section | Implementation | Coverage |
|---|---|---|
| §1 application state, no serialization | `Nation`, owned regions, process directories | construction/lifecycle conformance vectors; no save record |
| §2 / demand §11.2 economy and defaults | `EconomyTuning`, `EconomyDate` | core economy tests; `nation.rs` §5 vectors; `nation_owner.rs` NQ3 vectors |
| §3 messages | `do_message`, `init`, `shutdown` | ignored city messages, reset dispatch, three subscribe/unsubscribe calls |
| §4 COM identity/refcounts | Future game adapter | Outside portable core; no C ABI/plugin change |
| §§6.1–6.2 discovery and copying | `discover_regions`, `copy_region_files`, `copy_new_user_regions`, `ProcessDirectories` | ordered entries, Downloads exclusion, recursive suffixes, overwrite/writable, localized templates, starter fallback |
| §6.2 initialization/preferences | `init`, `get_index_of_current_region_setting_in_preferences` | NUL/length/case/64-byte prefix, no-op repeated Init, probe failures and tutorial candidate |
| §6.3 cleanup/reinit | `shutdown`, `EconomyTuning::reload` | ordered unsubscription, active Shutdown, catalog Drop, changed tuning and retained process guard |
| §7 getters/selection | Borrowed indexed getters, `set_active_region` | constructor index 0, no-op same index, invalid-index no effects, ignored callback failure |
| §7 add/delete | `add_new_region`, `delete_region` | null factory safety, both neighbors, selection before deletion, ordered erase and active adjustment, LastRegion |
| §8 tutorial reset | `reset_tutorial_cities` | app/city guards, retained/recreated/unlisted directories, default first, exports/read failures, move/shutdown/drop, registry flags, screen normal/early exits |

NQ1–NQ3 are answered and implemented. No open nation contract remains in this
portable owner. Replacement safety errors are distinct from original bool/void
returns; ordinary region, copy, export and move failures never trigger rollback.
Region factories and regional-city parsing remain the responsibility of their
respective interfaces, rather than duplicated nation algorithms.

## Integration boundary

This module has no C ABI or GZCOM adapter and is not installed as a replacement
or shadow in the game. A future adapter must implement the interfaces using the
game's services, wire the process directory owner, and compare behavior in-game.
Synthetic conformance is measured here; game shadow/ref-warning results are not.

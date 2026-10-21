| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| devperf2 (UpdatePopulation 7.7-9.8x -> ~2.1-2.9x of the original's 0.36 us/call; the tract Grow listing's clones gone; shadows exact) | `devperf2` / (engine lane) worktree `.claude/worktrees/devperf2` | **ready** — no C ABI change, `FC4_ABI_VERSION` untouched. LotsInCellRect walks the game's cell map (no GameList, no per-lot AddRef); the core gets the lots one by one through the host trait (the per-call token Vec gone); the Grow listing's per-pass Vec clones gone (mem::take + a `*_pass` helper); ListExistingLots memoised from the pass setup. Measured (perf_profile = 1, 72 turbo months): UpdatePopulation **3.52 -> 0.8-1.0 us/call** (original 0.36, so 2.1-2.9x), Grow 349 -> 267-339 us/call, days/s 51.7 -> up to 65.5. Exact: UpdatePopulation forwarder 908424 compared calls **0 mismatches**; tract self-check 74/74 roundtrip=same; development shadow 854 lines 0 mismatches, catalogue 744/744, suitability 72/72. Gate: cargo 3535/0, clippy clean, ctest 62/62 (runs 195-197, labels devperf2-*). The two remaining costs (batched per-lot reads, "find the first unmarked item") each need a C ABI addition — a batch that already bumps the ABI could take them. | (1) A future ABI-bumping batch takes the two slots above; (2) FC4LotDevHost's LotsInCellRect has the same GameList walk — check its callers' flags first; (3) perf_profile is ±20% noisy: compare per-month medians on quiet months, not run totals. |

## What the lane did

Closed glue27's open item 5 (HANDOFF open item 5): the growth developers' map-update state calls the
tract developer's `UpdatePopulation` once per desirability tract (12288 a month), and ours cost 7.7-9.8x
the original's call. Four changes, all behaviour-preserving, no C ABI change (`FC4_ABI_VERSION`
untouched, no exports added or removed):

1. **`FC4TractHost.cpp` `LotsInCellRect`** (the tract host's slot 7; the core only ever asks
   `originOnly = true`, tract-developer.md §4.18): the lots are now read straight from the game's cell
   map through `GetLot(x, z, true)` (slot 6, origin-only, borrowed, no AddRef), z outer, x inner, in
   the original's order, instead of building the game's `GameList` (a TractNodeAllocator node and one
   AddRef'd reference per lot, released again at `TractReleaseHeld`, plus a reference-audit event
   pair per lot). The rect is clamped by the loops' bounds alone - `GetLot` answers null outside the
   city (zones.md §8.2), so the original's clamp to the city gives the same lots. Same set, same
   order, nothing held. The walk needs no game-list allocator, so `ListsAvailable` no longer gates it
   (the `listsRefused` counter can only go down, and only in the broken-allocator error path).
2. **Core host trait** `TractDeveloperHost::lots_in_cell_rect` hands the lots over one by one
   (`&mut dyn FnMut(u64)`) instead of returning a `Vec<u64>`; the capi `CHost` adapts the C callback
   straight into the closure (the per-call token vector and its reallocations are gone; the C ABI
   callback table is unchanged, so no host had to change). `update_population` computes the
   desirability / sub-type / wealth reads and the per-tract generator lazily when the first lot
   arrives: an empty tract short-circuits exactly as §4.18 says (no reads, no generator), a non-empty
   one reads in the same order as before. The host and the statics are borrowed through the fields;
   the per-call `Rc` clones (two atomic refcount pairs) are gone.
3. **The tract `Grow` listing's per-pass `Vec<u64>` clones are gone**: `take_over_abandoned`,
   `list_candidate_lots_existing`, `list_candidate_lots_aggregation`,
   `list_candidate_lots_subdivision` and `pick_building` take the `w.lots` / `w.cfgs` vectors
   (`mem::take`, only the Vec header moves), walk the slice through a `*_pass` helper and put them
   back. The pass bodies never change the sets.
4. **The listing's memo is on from the pass setup** (`grow`): `ListExistingLots` walks the tract's
   cells through `m_get_lot` (it used to call the host unmemoised), and its answers serve the
   candidate passes of the same pass (nothing between them writes the cell map or a lot's geometry);
   `memo_on(false)` still comes before the build phase. Per pass this removes up to 16 host calls
   that the candidate passes used to repeat.

Left as glue27 left it: `developer_growth_list` still copies the whole growth list per grow attempt
(`development_simulator::grow`); removing it needs a "find the first unmarked item" host slot, which
is a C ABI change, deliberately out of scope for a batch. Also out of scope: batching the per-lot
host reads of `UpdatePopulation` (4 ABI hops per lot with capacity) - the original does those reads
in-process; a batch slot would again be a C ABI change.

## Measured (perf_profile = 1, the perf month sections' totals over 72 turbo months to 2032-01-01, Konradshohe, `enable = development, tract`)

| run | UpdatePopulation | bds:tract-Grow | days/s |
|---|---|---|---|
| devperf2-base-ours (before) | 3115 ms / 884736 = **3.52 us/call** | 3188 ms / 9132 = **349 us/call** | 51.7 |
| devperf2-fix1 (after fix 1-4) | 656 ms / 884736 = **0.741 us/call** | 2702 ms / 10116 = **267 us/call** | 65.5 |
| devperf2-fix1b (fix1 source rebuilt) | 0.951 us/call | 338.9 us/call | 19.3 |
| devperf2-fix2/3/4 (final DLL, 3 runs) | 0.95 / 0.95 / 1.05 us/call | 339 / 364 / 413 us/call | 46-53 |

- The original's UpdatePopulation is 0.36 us/call (glue27), so ours went from 7.7-9.8x to about
  **2.1-2.9x**. Run-to-run noise on this machine is larger than the deltas between the fix builds
  (the same DLL measured three times spread ±21% on growMs; a rebuild of identical source moved
  UpdatePopulation p50 0.73 -> 0.94). Read the table as "about 0.8-1.0 us/call", not as a trend.
- What remains in our call: the ABI hop chain (growth developer -> capi -> plugin, the plugin's
  `Section` critical section, the capi `dev()` dispatch), 16 `GetLot` vtable calls per tract (the
  original walks its own cell-map array without a per-cell virtual call - its layout is not
  documented, so we cannot read it directly), and ~4 ABI hops per lot with capacity. Closing either
  needs a C ABI addition.
- Grow's call count rises as the unit gets faster (9132 -> 10116 at equal pacing): the BDS's daily
  growth budget buys more attempts. The per-call medians above are from the same runs, so the 267 vs
  349 comparison is the honest one; the 413 figure is the noisiest run.

## Exactness (every run `verdict: clean`, `ref_warnings=0`, 0 problems)

- **UpdatePopulation forwarder** (`tract_shadow = 1` alone, the original is the live class): 883872
  compared calls, 0 failures, 42461 lots, **0 mismatches** (devperf2-upcomp); the final DLL again
  908424 compared, 0 failures, 43638 lots, **0 mismatches** (devperf2-upcomp2). The cell walk
  answers the original's `GetLotsInCellRect(originOnly = true)` exactly, lot by lot, in order.
- **Tract self-check** (`enable = tract` + `tract_shadow = 1`): styles and record 74/74 lines
  `roundtrip=same`, 0 mismatches, both runs. The population sample's mismatches (124 over 28416
  lots, 35 of 74 lines) are its documented drift (glue27's baseline: 155 over 7325 lots).
- **Development shadow** (differential, `development_shadow = 1` without `enable`): 854 growth-update
  lines (12 developers x 72 months), **0 mismatches**, catalogue 744/744, suitability 72/72 - on the
  pre-final and the final DLL alike (devperf2-devshadow, devperf2-final).

## Gate (qbuild.py all at 0b3271f4; the two commits after it, a0aaa06c and b95ffa81, are docs-only)

`qbuild.py all`: cargo test --workspace 3535 passed / 0 failed; clippy -D warnings 0 diagnostics;
ctest 62/62; fc4verify PASS (abi_entries.inc unchanged, 184 interfaces). rustfmt clean on the six
touched files.

## Files

- engine/plugin/FC4TractHost.cpp (`LotsInCellRect`)
- engine/core/src/tract_developer/mod.rs (the trait signature)
- engine/core/src/tract_developer/simulator.rs (`update_population`, the `*_pass` helpers, `grow`'s
  memo window, `list_existing_lots`)
- engine/capi/src/tract_developer.rs (`CHost::lots_in_cell_rect`)
- engine/core/src/tract_developer/tests.rs, engine/conformance/src/tract_developer.rs (the two test
  hosts and the conformance World)

## Runs (docs/runs/)

devperf2-upcomp-195, devperf2-perf-196, devperf2-shadows-197 (runs 195-197; queue labels
devperf2-base-ours, -fix1, -fix1b, -fix2, -fix3, -fix4, -shadows, -devshadow, -upcomp, -upcomp2,
-final).

## Next steps for a successor

1. The two remaining UpdatePopulation / Grow costs that need a C ABI addition (a batch that already
   bumps `FC4_ABI_VERSION` could take them): a batched per-lot read slot for `UpdatePopulation`
   (capacity + occupant wealth + available capacity in one call), and the "find the first unmarked
   item" slot for `developer_growth_list` (glue27's item 3).
2. The lot developer's own `LotsInCellRect` (`FC4LotDevHost.cpp`, §9) still builds the game list;
   the same walk applies there if its callers want originOnly = true only - check the two callers'
   flags first.
3. `perf_profile` measurements on this box are noisy (±20%); if a future lane compares per-call
   costs, run the same DLL at least twice and compare per-month medians on quiet months
   (`tick < 600 ms`) rather than run totals.

# Lane glue21fix2 (PL17 fixed; PL18: the game's growth pass over-releases the lot configurations)

Continues branch `glue21` in worktree `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21` (no rebase; the
restart briefs' `glue21fix2` worktree does not exist - keep working here). Commits: `0c5d5625` (slot 27
vector fix = PL17), `1b840d70` (prop token QI normalization + lotcfg set-query reference spec + diagnostics),
`65d97b40`/`ba74dce5`/`ad979728` (docs).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue21fix2 (PL17 fixed, PL18 open) | `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21`, branch `glue21`, tip `<see git log>` | in progress | see Handoff |

## Handoff
- Goal: land the `enable = propmgr` / `lotcfg` replacements game-clean (three shadows exact, ref_warnings=0,
  turbo to 2032) and flip the merge-queue entry to ready. PL17 (slot 27 `GetObjectIDArray` vector) is FIXED;
  the remaining fault is PL18: the game's growth pass releases 4 references per touched lot configuration per
  SimNewDay and deletes our wrappers out from under the catalogue.
- Verified this round (round 3, log analysis only - no code change): the round-2 ledger is CONFIRMED
  line-by-line against `build-runs/game-results/glue21fix2-isoref-4.log` (day-29 window = lines 41539-41790)
  and `...isoref-3.log` (window = lines 12030-12700). Caller RVAs (dll-base-relative, symbolized with
  `C:/Users/tlori/FreeCity4/.claude/worktrees/glue24/build-runs/symtool.exe build-x86/FreeCity4.dll <rva>`):
  `0xA891E2` = `LoadConfiguration`'s host add (creation, one per config, 1909 total), `0x7B2631` =
  `AuditedAddRef` at the slot-9 set-query insert, `0x77E671` = the creation release (add_by_key), `0xA8BAC3` =
  the scratch-map insert (`RebuildConfigurationMap`/`InsertConfig`, 1909 total), `0x7C0CFA` = the AV site
  (`std::_Vb_reference::operator bool`, vector<bool> reading 0; `edi` = the dead wrapper), frames
  `exe+0x304C1E`/`exe+0x30975F` = the game's zone/tract developer chain. The release site is the game exe ILT
  `0x00709798`.
- Measured ledger per touched config, day 29 (the first SimNewDay with configurations; the query runs inside
  the tract developer's `Grow` via the BDS's `DoDailySimulation`, right after `BudgetSimulator::DoMessage`):
  creation +1, creation release -1 (net 1 = the catalogue's), scratch-map +1 (net 2 ours), then +1 per
  matched slot-9 query (isoref-4: TWO queries of 90 matches -> 2 adds; isoref-3: five queries
  105/105/105/90/90 with duplicate log lines suppressed -> 4 adds for the 105-set), then releases through
  ILT 0x00709798 in rounds (isoref-4: 23/17/17/5 = 62 over 23 configs). Every deleted wrapper had 0 game-lot
  references (no `006BF53D` adds) and 4 releases (3 logged + the deleting one); every surviving 4-release
  config had 1-2 lot references (e.g. 11914A90 rel=4 lotrefs=2; 11916C50 rel=3 lotrefs=0 -> deleted).
- KEY new fact: the game's release count per config is INDEPENDENT of its own add count. 18813430 (isoref-3,
  105-set): 4 adds, only 2 releases, survived at 4. 11913800 (isoref-4, 90-set): 2 adds, 4 releases,
  deleted. So the game releases per its own internal model (per-pass release over the configurations it
  processed), not per what our slot 9 handed out; the deficit for a no-lot config is exactly 2 = the
  catalogue reference + the scratch-map reference (in the original the slot-20 map IS the catalogue, one
  object, and the filing lane measured a bulk +1 wave per config at load through an unmodeled path).
- Candidate fix to TEST (one change, measured accommodation - file PL19 only if the run contradicts the
  model): hand out TWO references per matched config per set query. Implement in the PLUGIN
  (`cFC4LotConfigManager.cpp`, the three `GetLotConfigurations*` methods, after the core call: a second
  `FC4_ADDREF` per token), NOT in the core's `with_refs` - the core stays spec-exact (B5.2 single-add) and
  its tests stay green, and the shadow (which runs its own core over the record, not through these plugin
  methods) is unaffected. Ledger with the fix: cat1 + map1 + 2 queries x 2 = 6; releases 4 -> 2; the next
  day repeats (+4/-4) -> stable. The 105-set configs (4-5 queries/day) then grow a few refs/day - the
  original leaks the same way (quirk 12: duplicates keep references slot 12 never releases). Known
  side effect: our own tract host (`ConfigurationsByFilter` in `FC4TractHost.cpp`) gets 2 refs per config
  and `TractReleaseHeld` releases one -> +1 per config per Grow pass, invisible to the shadows (they compare
  outcomes, not refcounts); note it in the comment. Check first that nothing else calls these three methods
  expecting exactly one ref (grep `GetLotConfigurationsBy` in engine/plugin).
- Next (round 4): (1) implement the plugin-side double-add with a comment citing PL18's measured ledger;
  (2) `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/glue21 quick`;
  (3) game run `glue21fix2-ref2-1`: Konradshohe, `turbo = 1`, `end_date = 2032-01-01`, `enable = lotcfg`
  only first (the fault is lotcfg-only; expect the SimNewDay-29 AV gone) via
  `<WT>/build-runs/game-queue/glue21fix2-ref2-1.req` + one
  `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qwait.py <WT>/build-runs/game-results/glue21fix2-ref2-1.verdict --fresh`;
  (4) then the full `enable = propmgr, lotcfg` run with the three shadows and check them exact; (5) on
  clean: re-run `glue21-gate-11`, write `<WT>/build-runs/merge-queue/glue21.json` state ready, update
  `docs/runs/glue21fix2.md` and this file, commit small.
- Facts: the isoref logs are git-ignored but still on disk in `build-runs/game-results/`. The qbuild all
  verdict may report `ninja did not finish (0/3)` on an up-to-date tree (touch a plugin .cpp first).
  Checked build: `cmake -B build-x86-checked -DCMAKE_BUILD_TYPE=RelWithDebInfo -DFC4_CHECKED=ON`; the queue
  accepts `dll = engine\build-x86-checked\FreeCity4.dll`. Game-request labels outstanding: none.
  PL15/PL16 answered, PL17 fixed, PL18 open (docs/specs/questions/prop-lotconfig.md); no PL19 yet.
  Do not run `python tools/questions.py` (tools/ is out of bounds); the coordinator regenerates.

## What was wrong (PL17, fixed at `0c5d5625`)

The game's `CreateLotObjects` (network-manager §8.6) calls lot configuration manager slot 27
`GetObjectIDArray(object, vec)` and reads the result back as a **vector**: `RandomUint32Uniform(|ids|)`
then `ids[pick]`. Our implementation filled the caller's container with `FillSet` — an STLport **hash_set**
layout — while the game passes an `SC4Vector<uint32_t>&` (SDK header `cISC4LotConfigurationManager.h`, spec
§B5.6 "clears vec"). The hash-set insert left the vector's begin pointer at 0 with a garbage end: the draw
came out ~29.7M and `ids[pick]` faulted at `pick × 4` — exactly the PL17 signature
(`edx 0x01BA4CBE × 4 = 0x06E932F8`, `ecx = 0`). Fixed by filling the caller's `GameVector<uint32_t>` (clear
+ push_back through the game allocator, the same pattern the shadow uses). A use-after-scope in the same
function (`co.ids` pointing into a dead local vector) was fixed alongside.

## The second fault the PL17 fix exposed (RTC #0 → int3)

With slot 27 working, the run survived city Init for the first time and reached the game's `CreateProp` →
our slot 32 `AddStreetProp` → core `add_new` → host `set_position` (`cFC4PropManager.cpp:416`) → RTC check
#0 "ESP not preserved across a call" (visible only in a checked build, run `glue21fix2-rtc-1`). Root cause:
`CreatePropInstance` handed the core the **cISC4PropOccupant primary** pointer (QI `0xE9793A65`), whose
vtable slot 9 is `GetOrientation` (pop 0) — calling occupant `SetPosition` (pop 4) through it desynced the
stack. This latent ABI bug predates glue21fix2; every earlier run died at slot 27 before reaching it.

Fix (`1b840d70`): `CreatePropInstance` normalizes every created prop token to the **cISC4Occupant
sub-object** (QI `0x26BB767E`, the factory's creation reference released after the QI hands the core its
own); all occupant-side host callbacks (`OccupantInit/Shutdown/Flag`, `SetPosition`, `Position`, `Hide`,
`InsertOccupant`, `RemoveOccupant`) resolve the occupant via QI with a scoped reference; prop-side callbacks
(`SetPropType`, `SetBoundingBox`, `SetPropUsage`, `SetOrientation`) QI back to the primary;
`CreateProp`/`CreateGenericProp` return the primary to the game with the creation reference transferred
(one reference owned, §A6.1 step 6).

## The remaining fault (PL18, open)

With `enable = lotcfg` (isolated runs `glue21fix2-iso-lotcfg*`, `isoref-1..4` — no propmgr, so independent
of the prop fixes), the city loads and runs to turbo, but the growth developer's SimNewDay pass faults
deterministically: AV reading 0 inside `std::vector<bool>` code reached from our
`cFC4LotConfiguration::IsCompatibleWithPurposeType` / `GetRequiredRoads` frames — the bit vectors of a
wrapper that was **deleted** while the catalogue map still held it. The full measured ledger is in the
Handoff above; the release site is the game exe ILT `0x00709798`, release rounds 23/17/17/5, and the
release count is independent of the game's own add count (the key fact this round established).

## Run ladder (this lane)

| Run | DLL | Verdict | Fault |
|---|---|---|---|
| glue21fix2-gate-1 | debug | exited -1 | RTC #0 int3 in `SetPosition` (prop token primary/occupant mismatch) |
| glue21fix2-rtc-1 | checked | corrupt | RTC #0 named: `SetPosition` ESP mismatch via `add_street_prop` from the game's `CreateProp` |
| glue21fix2-rtc-2/3 | checked | exited -1 | over-release cascade (fixed: audit the factory reference) then the lotcfg vector<bool> AV |
| glue21fix2-iso-lotcfg1/2 | debug, lotcfg only | exited -1 | same vector<bool> AV at SimNewDay 29 (independent of propmgr) |
| glue21fix2-iso-lotcfg3/4/5, isoref-1..4 | debug, lotcfg only | exited -1 | the PL18 over-release: wrappers deleted by the game's growth pass (ledger measured) |

Full run rows: docs/runs/glue21fix2.md.

## Notes for the successor

- The `qbuild all` build verdict can report `ninja did not finish (0/3)` on an up-to-date tree: the
  `fc4_capi_cargo` custom target is always dirty, `restat=1` then skips the downstream link edges and ninja
  prints only `[0/3]` — the parser reads that as unfinished. Workaround: touch any plugin .cpp (mtime) so
  real edges run, or clean-first. The build itself is fine (RC 0, DLL fresh).
- `symtool.exe` (glue24 worktree build-runs) symbolizes RVAs against `FreeCity4.pdb`; the queue copies the
  pdb next to the DLL. Compute RVA as `absolute - base` from the fault line.
- The checked build (`cmake -B build-x86-checked -DCMAKE_BUILD_TYPE=RelWithDebInfo -DFC4_CHECKED=ON`)
  turns RTC failures into named call sites — invaluable for ABI mismatches; the queue accepts
  `dll = engine\build-x86-checked\FreeCity4.dll`.
- The propmgr/lotcfg shadow steps silently skip when the city holds our own class (`IsOurObject` early
  return in `PropRunStep`/`LcmRunStep`) — expected behavior, not a regression (PL15 context).
- `refaudit_lines` ini key raises the refaudit verbose cap (default 2000; `AppCity` QI spam floods it).

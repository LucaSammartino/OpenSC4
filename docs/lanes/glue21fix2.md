# Lane glue21fix2 (PL17: the game's CreateLotObjects fault on the network-lot redevelopment path)

Continues branch `glue21` in worktree `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21` (no rebase).
Commits: `0c5d5625` (slot 27 vector fix), `1b840d70` (prop token QI normalization + lotcfg set-query reference spec + diagnostics).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue21fix2 (PL17 CreateLotObjects fix) | `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21`, branch `glue21`, tip `1b840d70` | Gate at this tip: cargo 2860/0, clippy clean, build edges PASS, ctest 58/58. **PL17 (the game's own `CreateLotObjects` AV) is FIXED and verified in game**: the old fault at `SimCity 4.exe+0x60D4B5` no longer occurs — the network-lot redevelopment path now runs through our slot 27 and creates street props via our slot 32 (verified in runs `glue21fix2-rtc-1..3`). A NEW pre-existing fault surfaced further down the timeline: with `enable = lotcfg` the growth developer's SimNewDay pass over-releases our `cFC4LotConfiguration` wrappers and frees them under the catalogue (AV reading 0 in `IsCompatibleWithPurposeType`, std::vector<bool> on a deleted wrapper), deterministic at the first SimNewDay; filed as **PL18** (needs an RE answer about the game's reference conventions). Merge-queue entry is `wip`. | (1) PL18: get the game's exact acquisition/release convention for lot configurations (the load-time bulk +1 wave per configuration and the growth pass's 4-releases-per-configuration pattern are measured in the lane runs); the FreeCity4 readings (slot 20 scratch map with one ref per config; slot 9 = §B5.2 ref per insert) do not balance against the game's releases; (2) once clean, re-run `glue21fix2-gate-1` (Konradshohe, turbo 2032, `enable = propmgr, lotcfg`, three shadows) and flip the merge-queue entry to `ready`. |

## What was wrong (PL17, measured)

The game's `CreateLotObjects` (network-manager §8.6) calls lot configuration manager slot 27
`GetObjectIDArray(object, vec)` and reads the result back as a **vector**: `RandomUint32Uniform(|ids|)`
then `ids[pick]`. Our implementation (`cFC4LotConfigManager.cpp`) filled the caller's container with
`FillSet` — an STLport **hash_set** layout — while the game passes an `SC4Vector<uint32_t>&` (SDK header
`cISC4LotConfigurationManager.h`, spec §B5.6 "clears vec", the shadow's own `GameU32s`). The hash-set
insert left the vector's begin pointer at 0 with a garbage end: the draw came out ~29.7M and
`ids[pick]` faulted at `pick × 4` — exactly the PL17 signature (`edx 0x01BA4CBE × 4 = 0x06E932F8`,
`ecx = 0`). Fixed by filling the caller's `GameVector<uint32_t>` (clear + push_back through the game
allocator, the same pattern the shadow uses). A use-after-scope in the same function (`co.ids` pointing
into a dead local vector) was fixed alongside.

## The second fault the fix exposed (RTC #0 → int3)

With slot 27 working, the run survived city Init for the first time and reached the game's
`CreateProp` → our slot 32 `AddStreetProp` → core `add_new` → host `set_position`
(`cFC4PropManager.cpp:416`) → **RTC check #0 "ESP not preserved across a call"** (visible only in a
checked build, run `glue21fix2-rtc-1`; the normal debug build just int3s inside the CRT's
`failwithmessage`). Root cause: `CreatePropInstance` handed the core the **cISC4PropOccupant primary**
pointer (QI `0xE9793A65`), whose vtable slot 9 is `GetOrientation` (pop 0) — calling occupant
`SetPosition` (pop 4) through it desynced the stack. This latent ABI bug predates glue21fix2; every
earlier run died at slot 27 before reaching it.

Fix (`1b840d70`): `CreatePropInstance` normalizes every created prop token to the **cISC4Occupant
sub-object** (QI `0x26BB767E`, the factory's creation reference released after the QI hands the core
its own); all occupant-side host callbacks (`OccupantInit/Shutdown/Flag`, `SetPosition`, `Position`,
`Hide`, `InsertOccupant`, `RemoveOccupant`) resolve the occupant via QI with a scoped reference;
prop-side callbacks (`SetPropType`, `SetBoundingBox`, `SetPropUsage`, `SetOrientation`) QI back to the
primary; `CreateProp`/`CreateGenericProp` return the primary to the game with the creation reference
transferred (one reference owned, §A6.1 step 6).

## The remaining fault (PL18, open)

With `enable = lotcfg` (isolated runs `glue21fix2-iso-lotcfg*`, `isoref-1..4` — no propmgr, so
independent of the prop fixes), the city loads and runs to turbo, but the growth developer's SimNewDay
pass faults deterministically: AV reading 0 inside `cFC4LotConfiguration::IsCompatibleWithPurposeType`
(line 191) — the `std::vector<bool> purposes_` of a wrapper that was **deleted** while the catalogue
map still held it. Measured ref ledger per configuration: creation +1 (catalogue's, via core
`add_configuration`), +1 raw per `GetLotConfigurationMap` rebuild (our slot-20 scratch map), +1 per
set-query insert (§B5.2, kept on duplicates per quirk 12 — this pair was missing before and is now
spec-exact); then the game's growth pass (release site `SimCity 4.exe` ILT `0x00709798`, frames near
`exe+0x30975F`/`0x304C1E`) **releases four references per configuration** in one burst and the wrapper
deletes. The game evidently acquires references at load through a path we do not model (a bulk +1 wave
of one ref per configuration taken by raw `AddRef` inside our own DLL — site not yet named; refaudit
was flooded by `AppCity` QI lines) and releases more than any spec-consistent accounting allows.
Instrumentation added: per-op ref tracing (removed), the deleting-release releaser chain (kept),
in-image first-chance faults now log registers + return-address stack walk (kept, `FC4Diag.cpp`).

## Run ladder (this lane)

| Run | DLL | Verdict | Fault |
|---|---|---|---|
| glue21fix2-gate-1 | debug | exited -1 | RTC #0 int3 in `SetPosition` (prop token primary/occupant mismatch) |
| glue21fix2-rtc-1 | checked | corrupt | RTC #0 named: `SetPosition` ESP mismatch via `add_street_prop` from the game's `CreateProp` |
| glue21fix2-rtc-2/3 | checked | exited -1 | over-release cascade (fixed: audit the factory reference) then the lotcfg vector<bool> AV |
| glue21fix2-iso-lotcfg1/2 | debug, lotcfg only | exited -1 | same vector<bool> AV at SimNewDay 29 (independent of propmgr) |
| glue21fix2-iso-lotcfg3/4/5, isoref-1..4 | debug, lotcfg only | exited -1 | the PL18 over-release: wrappers deleted by the game's growth pass |

## Notes for the successor

- The `qbuild all` build verdict can report `ninja did not finish (0/3)` on an up-to-date tree: the
  `fc4_capi_cargo` custom target is always dirty, `restat=1` then skips the downstream link edges and
  ninja prints only `[0/3]` — the parser reads that as unfinished. Workaround: touch any plugin .cpp
  (mtime) so real edges run, or clean-first. The build itself is fine (RC 0, DLL fresh).
- `symtool.exe` (glue24 worktree build-runs) symbolizes RVAs against `FreeCity4.pdb`; the queue copies
  the pdb next to the DLL. Compute RVA as `absolute - base` from the fault line.
- The checked build (`cmake -B build-x86-checked -DCMAKE_BUILD_TYPE=RelWithDebInfo -DFC4_CHECKED=ON`)
  turns RTC failures into named call sites — invaluable for ABI mismatches; the queue accepts
  `dll = engine\build-x86-checked\FreeCity4.dll`.
- The propmgr/lotcfg shadow steps silently skip when the city holds our own class (`IsOurObject` early
  return in `PropRunStep`/`LcmRunStep`) — expected behavior, not a regression (PL15 context).

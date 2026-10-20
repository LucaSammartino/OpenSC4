# Lane glue21fix2 (PL17: the game's CreateLotObjects fault on the network-lot redevelopment path)

Continues branch `glue21` in worktree `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21` (no rebase).
Commits: `0c5d5625` (slot 27 vector fix), `1b840d70` (prop token QI normalization + lotcfg set-query reference spec + diagnostics), `65d97b40` (docs).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue21fix2 (PL17 CreateLotObjects fix) | `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21`, branch `glue21`,  tip `ba74dce5` | in progress | see Handoff |

## Handoff
- Goal: PL18 (the lotcfg configuration-lifetime fault) after PL17 was fixed. Branch glue21, worktree
  C:/Users/tlori/FreeCity4/.claude/worktrees/glue21 (this file lives there; the restart brief's glue21fix2
  worktree does not exist - keep working here), tip at stop: ba74dce5.
- Done this round (analysis only, no code change committed): the isoref logs re-read and the ledger closed.
  Site labels (RVA in FreeCity4.dll, base varies per run): 0xA91E2 = catalogue/creation add path; 0x7B2631 =
  set-query insert AddRef (slots 9/10/11); 0x77E671 = the release paired with it (our development host's
  density walk, balanced +1/-1 at load); 0xA3BAC3 = the scratch-map rebuild (GetLotConfigurationMap, +1 per
  config); game exe: 0x006BF53D = lot Init takes 1 ref per placed lot's configuration (736 adds, 166 configs,
  never released while the lot lives); 0x00709798 (ILT) = the growth pass's Release.
- MEASURED (isoref-4, day 29; isoref-3, days 29-30): per SimNewDay the game makes TWO slot-9 queries
  (90 matches each in isoref-4; 105/105/90 log lines in isoref-3 across two days, duplicate log lines
  suppressed - identical text). Slot 9 adds exactly ONE reference per matched config per call (the
  double-add hypothesis is WRONG; the 210-add segments are two back-to-back calls).
  The game then releases through ILT 0x709798, in rounds (isoref-4: 23, 17, 17, 5 = 62 releases over 23
  configs): every released config got exactly TWO game acquisitions that day; the 17 four-release configs
  got releases 1-4 (12 of them had no lot reference and hit 0 -> deleted; 5 had 1-2 lot references and
  survived); the 6 one-release configs were released once and survived.
- The deficit for a no-lot config is EXACTLY 2: the game's 4 releases eat its 2 own acquisitions PLUS our
  catalogue reference and our scratch-map reference. In the original the slot-20 map IS the catalogue
  (one object), so the original's equivalent release pattern leaves the catalogue's reference intact;
  the original must also supply the game 2 more refs per developed config from a path we do not model
  (the filing lane's "bulk +1 wave at load" note; no per-config game adds are visible at load in our runs
  beyond the 166 lot-held configs).
- Candidate fix (measured accommodation, one change at a time): make our slot 9 hand out TWO references
  per matched config per call (double the B5.2 reference). Ledger: cat1+map1+2x2=6 per matched config;
  the game releases 4 -> 2 left; next day repeats (+4/-4) -> stable. Alternatively make the scratch map
  hold no references (ownsValues=false) AND slot 9 double-adds: cat1+4-4=1 -> the catalogue survives and
  the map holds borrowed pointers (the game's own slot-20 walks never released anything in our logs, so
  borrowed may be safe - but the shadow compares the map's nodes, and the map-walk release convention is
  unverified). File PL19 if the game-side release convention needs an RE answer.
- Next: (1) implement the slot-9 double reference in cFC4LotConfigManager.cpp (all three set queries,
  keeping quirk 12) OR the map-ownsValues=false variant; (2) qbuild quick, then a game run
  glue21fix2-ref2-1 (Konradshohe, turbo 2032, enable = lotcfg; then the full enable = propmgr, lotcfg with
  the three shadows); (3) on clean: glue21fix2-gate-1 re-run, flip build-runs/merge-queue/glue21.json to
  ready (the file is git-ignored; write it in build-runs/merge-queue/); (4) update docs/runs/glue21fix2.md
  and this file; run python tools/questions.py check is NOT needed (no question file change) - but if a
  question is filed, append to docs/specs/questions/prop-lotconfig.md and run
  `python tools/questions.py index` in MAIN? NO - tools/ is out of bounds for this lane; the coordinator
  regenerates. Commit small.
- Facts: the qbuild all verdict may report "ninja did not finish (0/3)" on an up-to-date tree (touch a
  plugin .cpp first). symtool.exe: C:/Users/tlori/FreeCity4/.claude/worktrees/glue24/build-runs/symtool.exe
  <dll> <rva-hex>. Checked build: cmake -B build-x86-checked -DCMAKE_BUILD_TYPE=RelWithDebInfo
  -DFC4_CHECKED=ON; the queue accepts dll = engineuild-x86-checked\FreeCity4.dll. The full analysis is
  in build-runs/pl18-analysis.txt (this worktree). Game-request labels outstanding: none.

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
of one ref per configuration taken by a raw `AddRef` inside our own DLL — site not yet named; refaudit
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

Full run rows: docs/runs/glue21fix2.md.

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
- `refaudit_lines` ini key raises the refaudit verbose cap (default 2000; `AppCity` QI spam floods it).

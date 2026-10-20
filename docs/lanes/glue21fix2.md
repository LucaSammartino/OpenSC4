# Lane glue21fix2 (PL17: the game's CreateLotObjects fault on the network-lot redevelopment path)

Continues branch `glue21` in worktree `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21` (no rebase).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue21fix2 (PL17 CreateLotObjects fix) | `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21`, branch `glue21`, tip `1c637e99`+ | in progress | see Handoff |

## Handoff
- Goal: fix open question PL17 — AV at `SimCity 4.exe+0x60D4B5` inside the game's own `CreateLotObjects`
  (network-lot redevelopment, city-detail §3.4) after civic Init, before PostCityInit.
- Baseline: `qbuild all` PASS (cargo 2860/0, clippy clean, 144/144 edges, ctest 58/58).
- **Diagnosis (root cause found):** fault address `0x06E932F8` = `edx 0x01BA4CBE × 4` with `ecx=0` — the game's
  `CreateLotObjects` reading `ids[pick]` on an `SC4Vector<uint32_t>` whose begin pointer is 0. Our slot 27
  `cFC4LotConfigManager::GetObjectIDArray` (cFC4LotConfigManager.cpp:791) fills the caller's container with
  `FillSet`, which writes an STLport **hash_set** layout (functors word at +0), but the game passes an
  `SC4Vector<uint32_t>&` (SDK header line 72; spec §B5.6 says "clears vec"). Our hash-set insert wrote the
  functors field (=begin) as 0 and buckets into +4.. → begin=null, garbage end → huge `RandomUint32Uniform`
  draw → `ids[pick]` faults. Also: `co.ids` pointed at a dead local `std::vector` (use-after-scope) before
  the core call ran.
- Fix applied: slot 27 now treats `ids` as `GameVector<uint32_t>` (clear + push_back via game allocator, same
  as the shadow's `GameU32s`/`AsSdk` pattern) and the cfg-object struct is built with a properly scoped id
  buffer; helper renamed to `FillGameVector`, `FillSet` retained for the actual set slots (9/13/14/15).
- Next: qbuild quick → fmt → commit → game run `glue21fix2-gate-1` (Konradshohe, turbo 2032, shadows ON,
  ini `enable = propmgr, lotcfg`) → verdict clean + ref_warnings=0 + shadows exact → re-run `glue21-gate-11`
  style label → flip merge-queue entry to ready.
- Facts: game queue labels use `glue21fix2-*`; one request outstanding max. LotDevObjectOf(o, ids) needs the
  ids vector alive while `co` is used. `F32 pick` come from generator `0x00B44870`. The network lot manager is
  NOT replaced — the game's own class runs CreateLotObjects; only our slot 27/32 are the touchpoints.
- Questions: PL17 filed open in docs/specs/questions/prop-lotconfig.md (this lane closes it).

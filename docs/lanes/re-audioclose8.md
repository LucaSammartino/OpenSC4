| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audioclose8: the last bodies of ten audio owners (AEMS, ambience layer, group layer, emanation models, SFX) and the claim of the listener, SEM manager and sound bank, audio-owner-closure.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-audiofilters8) | **ready** (docs only). Ten owner rows claimed: specified +26,749 bytes (+140 functions); AU48 and AA14 archived | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The 23 functions these owners still lacked were read in the disassembly; the rest of each owner was checked against
the existing text (audio.md §3.4, §10.7–§10.7.1, §18.4, §19.2; audio-owner-contracts.md §17–§19; audio-ambience.md
§8.1–§8.7). Written on 2026-10-25.

### What changed

- New docs/specs/audio-owner-closure.md: AEMS RemoveAllBanks and the view thunks; the ambience layer's small members,
  `_RemoveLayers`, `_RemoveAllFilters`, `_StaticRemoveAllFilters` and destructor helper; the group layer's removal and
  Release; the emanation models' deleting destructors; `SFX::Listener`.
- docs/specs/audio-ambience.md §8.3: **correction**: every filter-list entry holds a reference (`_AddFilter` AddRefs
  before appending; `_RemoveAllFilters` releases one per entry).
- AU48 and AA14 archived: both were answered in the specs (audio.md §10.7.1, audio-ambience.md §8.1.1) but still open
  in the question files. QUESTIONS.md regenerated.

### Coverage

- tools/coverage-map.tsv: rows above `cSC4Audio*` for the AEMS event system, sound bank, ambience layer, listener, SEM
  manager and `cSC4AudioEmanationModel*`; `cGroupLayer` and `SFX` filled.
- Specified bytes 5,031,862 → 5,058,611 (+26,749, 90.1% → 90.6%, `cov_local.py` with owners-c1).

### Left open

`tSound` (AU49/AU50: linked volume and pan, distance and simulation pause, Resume) and `cSourceFileDebugInfo` are a
separate lane. AA10, AA12, AA13, AU42–AU46, AU49, AU50 stay open.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelmaker: the background loader and the model-making outline, folded into model-maker.md (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-budgetwin) | **ready** (docs and RE tables only). model-maker.md §14 (making models, outline) and §15 (the background loader `cSC4ModelBackgroundLoader`: lifetime, tuning with the shipped variants, radius, start / refocus / cancel, the worker thread, the main-thread commands, pause and resume) added; §3.2, §5, §10.1, §10.2 and §20 linked; `cSC4ModelBackgroundLoader` claimed as specified (the model maker stays unclaimed, its remainder is MM2); new questions/model-maker.md (MM1, MM2); 85 names (13 new, 72 renamed; one held-lane name corrected). Specified coverage +7,750 B / +21 functions | MM2 (the model-loading remainder), MM1 (curve units) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export and the SDK headers. Written on 2026-10-19 as a separate `model-maker.md` and held back when `main` gained
its own model-maker.md (three rounds on `cSC4ModelMaker`, parked with model loading as the remainder); upstream's
sections are more detailed than the held lane's on the model maker itself, so this fold adds what upstream did not
have: the background loader and the per-type outline of `AddModels` / `AddModel`.

### What changed (docs/specs/model-maker.md)

- §15 (new): the background loader. Read again in the bytes for the fold: `Init` (`0x0048E900`: the speed, the held
  services, the city cell counts, the thread start), `Shutdown` (`0x0048CC90`: stop, wake, wait, the simulator's
  resume, releases), the refocus `0x0048CB40` and `CancelAllLoadTasks` (`0x0048CD60`), `ExpandLoadRect`
  (`0x0048CA40`: truncation, then an unsigned max with the minimum), and the main-thread commands (`0x0048E170`, jump
  table `0x0048E298`). Command 1 (task done) clears the model maker's `+0xA4`, which the held lane missed; a
  `ProcessLoadedCells` that stops with cells left keeps the command list for the next tick; commands 3 and 4 are the
  simulator's `HiddenPause` / `HiddenResume` (the held lane's MM2).
- §14 (new, outline): `AddModels` by occupant type (loader, masks), `AddModel` (fallback key, size threshold,
  placeholder, instance flags). Both stay in the §20 remainder (MM2).
- §3.2, §5 (`0xCAD61E00`, `0xEAD61E16`, `0x86AD10EE`), §10.1, §10.2: links to §15; the two loader setters are named
  `Pause` and `Resume`.
- §19 cases M24–M28; §20: AddModels / AddModel marked as outlined, thirteen names filled in, the two setters moved
  to the loader owner.
- Kept upstream where it differs from the held lane: the vtables (primary `0x00A8EDF8`, save `0x00A8EDE0`; the held
  lane called the save one the primary and put `DoMessage` in slot 9), the save record with its five references
  (which explains the held lane's MM4 size puzzle), the deferral operation codes (the held lane's MM3), and
  `0x0048EAE0` being a model-maker method despite its loader export name.

### Names and coverage

- re/symbols/model-maker.tsv: 85 names (13 new: `GetGZCLSID`, `QueryInterface`, `Write`, the destructor and its
  deleting wrapper, the release thunk, `ApplyStateEffect`, the inserted / removed / deferral handlers, the loader's
  `Pause`, `Resume` and `PostCommand`; 72 replace auto names). Correction: `0x005BFE60` was named
  `cSC4ModelBackgroundLoader::BeginLoadTask` (auto name kept by the held lane); it is a shared `std::vector` insert
  that `PostCommand`, `cSC4RegionalCity::ExtractLocalDeals`, `cSC4WinMapView::AddHighlightedOccupant` and
  `cSC4LightingManager::GetDecalShadowParams` call, so it now carries a library name. The `names_windows.tsv` delta
  (−72 +85) is identical to `merge_names.py`'s; no non-auto upstream name is replaced.
- tools/coverage-map.tsv: `cSC4ModelBackgroundLoader` → model-maker.md (it fell to `cSC4Model*`). The held lane also
  claimed `cSC4ModelMaker` and `cSC4ModelMakerUtility`; neither is claimed here (upstream's §20 remainder, and
  graphics-formats.md §4.2 covers only two of the utility's functions).

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: after re-budgetwin 3,470,432 bytes in 15,324
functions; with this lane 3,478,182 bytes in 15,345 functions (+7,750 B, +21 functions). The denominator drops by one
function and 227 bytes (5,584,352 B) because `0x005BFE60` now has a library name. `docs/coverage-report.md` was not
regenerated (the tool accepts only the GOG exe).

### Questions

- New file questions/model-maker.md: MM1 (the units and interpolation of the loader's curves), MM2 (the
  model-loading remainder, outlined in §14).
- The held lane's MM2–MM4 were never filed upstream; they are settled (the simulator's hidden pause; upstream §8's
  operation codes; upstream §4's record with its references), and its MM1 is narrowed to the curves (the memory unit
  is upstream §7.1's MiB).

### Not covered

MM2's bodies; `cSC4ModelMakerUtility` beyond graphics-formats.md §4.2.

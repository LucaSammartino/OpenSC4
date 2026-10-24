| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-ov13 (overlay capture boundary and inactive headers) | Codex / `re-overlays-ov13` (RE lane; detached HEAD) | ready; OV13/OV14 answered in `ae4a83951`; eight paper vectors; doccheck, questions check and diff check pass; no ABI changes or game runs | Coordinator: land docs; retain observation-only coverage until original input recording and live-record runs exist |

## Report

OV13 is answered **DIFFERS** in overlay-manager §3.8.1: the native owner has no supported capture-registration/history-export API. Terrain-view initialization supplies a concrete instrumentation boundary before its four direct Init calls; interception of virtual adds alone misses Init and its initial resources/settings. A new table defines original event ordering, caller model pointee layouts, lighting/terrain state, resource identity/dimension inputs, transient image uploads/restores and the inherited FP environment. It distinguishes inputs from native expected results and nested helper operations. This is a recorder requirement, not a claim that a recorder now exists.

OV14 is answered **MATCHES** in §2.6: inactive dynamic shift/nx/nz words have no guaranteed values. Preserve them diagnostically without active static-grid constraints or pointer arithmetic. Report static cells N/A while still enumerating all live arenas; static storage retained by repeat Init remains inactive.

Eight new §9.2 paper vectors cover poisoned dynamic headers, retained static storage, invalid active static grids, late attachment, missed direct Init, borrowed model buffers, failed/nested creation and transient upload bytes. No executable conformance files or engine changes were made. Both original questions moved to `docs/specs/questions/answered/overlay-manager.md` with dated answers; the regenerated index changes overlay-manager open/answered counts from 8/7 to 6/9. Existing unrelated questions remain unchanged.

### Evidence and validation

- Export queried first with main-checkout `tools/re_query.py`; used export instructions with bytes and existing owner/lighting/texture specs, with a bounded decomp read. No REA was called (the task states its session has no binary open).
- Dynamic-header evidence: constructor `0x0073BD85–0x0073BD91` zeros index pointers only; BuildCellIndex `0x0073CB14–0x0073CB35` writes dimensions/shift; Init `0x0073CCF1–0x0073CCF8` skips it for dynamic managers.
- Capture evidence: direct terrain-view construction/Init `0x0075C5B2–0x0075C6E7`; model service forwarding and quality choice `0x0073C14E–0x0073C1D6`; descriptor numeric widths checked in `0x007D8C87–0x007D8D05`; upload path `0x00809B90–0x00809CC6` and texture-bindings §§1.5/1.12. No new arithmetic formulas inferred from pseudocode.
- `python tools/doccheck.py check`: PASS. `python tools/questions.py check`: PASS. `git diff --check`: PASS after removing an initial trailing blank line.
- Cargo/clippy/CTest and game runs: n/a for this text-only lane. No new shadow exactness claim. Existing owner run numbers are not this lane's validation.
- The requested Owner closure table / `open:` rows are absent from glue-overlays-shadow5 on this base and main. Read its Coverage limits section naming OV13/OV14 and the question entries instead.
- q.py failed to launch Bash (CreateFileMapping error 5); bounded PowerShell calls worked. Git commits printed a denied packed-refs lock attempt but completed on detached HEAD; no refs/config changes or Git maintenance were attempted.
- No engine/plugin/capi reads or edits, ABI exports/rows, coverage-map edits, or shared handoff edits.

### Remaining work

Coordinator can land these documentation answers. Full-owner coverage still requires an instrumented original-input recorder before Init, independently reproducible lighting/resources/uploads, and measured samples with live records; post-load observation remains the permitted subset. These are implementation/measurement prerequisites, not unanswered OV13/OV14 rules.

## Handoff

- Complete: `b04f4a0db` initial handoff; `ae4a83951` specification answers, eight vectors, archive moves and generated index.
- All required documentation checks passed. No ABI proposals, game requests or newly filed questions.
- Next: supervisor publishes detached HEAD to reserved lane branch; coordinator consumes ready queue entry.
- Queue: `build-runs/merge-queue/re-overlays-ov13.json`, kind `re`, state `ready`, actual full HEAD tip.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-ov13-r1-manifest.json`.

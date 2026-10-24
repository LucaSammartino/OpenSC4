| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lots-lr15apply12 (LR15 scoped owner inventories) | `Codex` / `glue-lots-lr15apply12` (engine lane, detached HEAD) | Ready measured unit/texture work: cargo 7377/0; clippy clean; CTest 81/81. Runs 552/553 clean, ref_warnings=0, to 2032; lotunit and lotbasetexture each 73/73 per run; lotwall 1/73 per run (answered LR15/T88), unverified. ABI unchanged. | (1) Credit scoped unit/texture evidence; (2) coordinator append their two keys to the current single regress shadows string; (3) retain wall mismatch under LR15, default off and unverified. |

## Report

Applied LR15 §14.2 and its behavioural fixtures T80–T88. Baseline a150cd2db already included
train199's answer and `glue-lots-shadow12`; no merge was needed. Engine implementation: dd547e00c.

- Added independent default-off `lotunit_shadow`, `lotbasetexture_shadow`, `lotwall_shadow`.
  Existing master-lot/inline-trip scope and counts remain separate.
- Units: distinct borrowed GetUnit pointers from the complete master-lot walk; no Release of
  getter results, no recursion or all-allocation census. Ordered off-master member references
  preserve their identities. Empty/off-master-only units are outside this rooted scope.
- Textures/walls: separate slot-39 full inclusive (0,0)…(191,191) type-filtered queries,
  maxCount=-1. Native allocator-A sentinel; slot 48 releases list elements/nodes, then sentinel
  is freed. Serializable QI has a separate audited acquisition/release. Malformed, duplicate/null,
  unavailable and capture/read failures remain errors; successful empty is distinct.
- Rust isolated owner codecs preserve identities, floats, all texture bytes and saved frames.
  Wall hash-induced Read mask growth deliberately remains a mismatch. No native Read/Init,
  simulation mutation, owner replacement or normalization was introduced.
- Added Rust codec and C++ inventory/list/failure discriminators. Config acceptance and engine
  README additions document the keys. New transport is private static-link glue; public ABI unchanged.

## Validation

Focused lotrec_lr15: **4/4 PASS**; owned-file fmt: **3 files PASS**, no differences.
One `qbuild all` on stable final engine source: **PASS**. Cargo **7377 passed / 0 failed**
(49 result lines), clippy **0 diagnostics**, native build **262/262 edges**, CTest **81/81**.
Native job `qbuild-20261024-172851-build-ctest` completed PASS (551.8s); same operation collected
to terminal, no duplicate verify. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-build.log`, `fc4verify-ctest.log`. DLL SHA256 prefix **af7bdefc**, 21,244,416 bytes.

Initial Bash wrapper CreateFileMapping failure (before Cargo) remains in
`build-runs/q/20261024-160002-35584.log`; direct PowerShell focused Cargo passed in
`build-runs/lr15-focused.log`. Initial relative fmt paths failed; absolute owned paths passed.
These environment diagnostics remain separate from the passing source gate.

## House-city results

Both Konradshohe runs were clean, with foreign_exceptions=0, watchdog_stalls=0, rtc=0,
overruns=0 and **ref_warnings=0**. All three inventories were complete and nonempty in 73/73
observations, with zero capture/reader errors.

```text
glue-lots-lr15-house2032-t1-20261024 293/365
glue-lots-lr15-house2032-n2-20261024 293/365
```

Run [552](../runs/glue-lots-lr15-house2032-t1-20261024-620.md): turbo, reached 2032-01-10 in 33.453s.
Run [553](../runs/glue-lots-lr15-house2032-n2-20261024-621.md): normal, reached 2032-01-01 in 188.047s.
Each run: lotrec **73/73**, lottrip **73/73**, lotunit **73/73**, lotbasetexture **73/73**,
lotwall **1/73** clean lines. New unit/texture scopes: **146/146 per run**; existing
master-lot/inline-trip scopes remain **146/146 per run**.

Turbo unit instances **4596/4596** (61..64 per observation), textures **51101/51101** (685..757).
Normal units **4650/4650** (62..64), textures **51561/51561** (693..757).
Final normal units **63/63**, textures **693/693**, walls **45/86** (41 differences);
final turbo units **64/64**, textures **685/685**, walls **45/85** (40 differences).
Run 553 preserves the final new-owner native lines verbatim.

Walls are measured but **unverified**: turbo 6496 selected instances, 2784 differences;
normal 6595 selected, 2723 differences. Both inventories complete 73/73 with zero errors.
Each run's 576 bounded diagnostics has first offset **40** (edge mask), with core masks
including native bits. This is the **answered LR15 §14.2/T88** discriminator: Read ORs nonzero
hash bits into the mask; normalization is explicitly forbidden. Normal speed confirms the same
failure. No wall exactness, all-allocation or replacement credit is claimed.

Evidence: `build-runs/game-results/<label>.{log,verdict}`, event-driven single wait and bounded
accounting in `build-runs/lr15-game-evidence.py` and `lr15-detail-evidence.py`. Forbidden
`tools/coverage.py` was not run; coordinator credits the independently measured scopes after review.

## Questions and next steps

No new question. Wall differences are already covered by **LR15** in
`docs/specs/questions/answered/lot-records.md`; no duplicate open question was filed.
Existing LR6/LR7 were untouched. READY covers the passing implementation and measured
master-rooted-unit/registered-texture scopes; wall remains default off and unverified.

Coordinator: credit these exact scopes and append only `lotunit_shadow` and
`lotbasetexture_shadow` to the current existing single `shadows = ...` regress profile string.
This lane cannot read/edit forbidden `tools/`, so that profile and coverage tables were untouched.
No game/native requests remain outstanding. No engine source changed after the passing gate.

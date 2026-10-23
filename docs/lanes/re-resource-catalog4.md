| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-resource-catalog4: native catalog and key picker contracts | Codex; re-resource-catalog4 (detached worktree) | in progress | Round 2: native catalog, selection and picker contracts; retain historical R1 denial. |

## Handoff
- R2 admission: copied `build-runs/evidence-run4/coordinator-resource-catalog-r2-host-proof.json` records canonical host full guard exit 0; own R2 manifest matches lane, RE role and reserved path. Source clean at pinned 98e396bb. No sandbox admission rerun or live-session repair.
- R2 binary SHA256 independently matches export metadata: 3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b. Next: trace native owner instructions and publish first substantive spec.
- R2 milestone: first substantive `docs/specs/resource-catalog.md` now covers exact-group row filtering, duplicate/default selection, refresh failure, picker word permutation and close-result separation; linked from prior exclusion. Next: registration filters, key-input notifications and shutdown reference audit, then questions/checklist.
- The following R1 failure and preliminary evidence are historical, not a retrospective success or behavior publication.
- Goal: first native behavior publication for catalog/base/picker/key-input owners, docs only.
- Done: reused reserved detached worktree; read lane/operating rules and existing scope exclusion; queried export and instructions only; no behavior publication.
- STOP: required full guard `tools/lanes.py --repo C:/Users/tlori/FreeCity4 check re-resource-catalog4 --resume --kind re --deps re-editors4 --touches "resource and exemplar editor"` returned exit 1 on 2026-10-23.
- Guard: `REA-DOWN C:\Users\tlori\AppData\Roaming\npm\node_modules\rea-agents\scripts\rea.mjs does not exist`; also `dep re-editors4: no such branch or commit`, `resume: branch re-resource-catalog4 does not exist`, and expected branch/worktree mismatch (reserved worktree is intentionally detached).
- Next: coordinator resolves required guard/configuration and metadata expectations; do not bypass guard or retry known live NoApp. Resume only with authorization under a passing guard.
- Export metadata: program SimCity 4.exe; SHA256 3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b; image base 00400000. No independent binary hash comparison completed.
- Preliminary identity evidence (not a published contract): factory 00664280 allocates 0x114 bytes then calls catalog constructor 00686450; failure returns zero. Factory 00664750 allocates 0x12c bytes, constructs catalog at allocation+4, installs vtables 00aad0d8 / 00aad0b4 / 00aace58, constructs variant at +0x118; failure returns zero.
- Catalog QI 006855e0 recognizes 7a368ee3, writes catalog pointer, calls AddRef, returns AL=1; other IDs delegate to cGZWin QI with this+4. Picker QI 006759b0 recognizes 06c5ff7b and AddRefs before writing picker pointer; fallback calls catalog QI with this+4. Class registration IDs still need primary confirmation.
- Preliminary row evidence: 00685ae0 chooses 32-byte type/group record by unsigned-checked index at catalog+0xf4; clears widget only after valid record and widget checks; asks global resource service for type enumeration; filters key second dword by exact selected group; appends rows with raw key pointers and instance fallback text `0x%08X` (00685ba7). Full key-word identity and widget sorting remain to verify.
- 00685c3a requests widget operation +0x20c; do not claim ordering until primitive identified. Subsequent loop defaults selection to row zero and changes to every full three-dword match of requested key, then setViewKey. Duplicates therefore need explicit audit, as do enumeration reference effects (no release identified in this body).
- GetSelectedKey 006857e0 obtains widget selected coordinates and item data; null data returns AL=0 without output writes; nonnull copies three 32-bit words and returns AL=1. RefreshKeyList 00685cf0 initializes a zero key, calls virtual GetSelectedKey, updates only when AL succeeds.
- Local decomp files for catalog/picker contain many failed bodies (field123_0x90 type error); instructions are needed. No live query attempted, repaired, or required as successful prerequisite.
- Validation: no build/tests/game runs (docs-only lane); no question/index changes, ABI proposals, tracked spec changes, or merge queue. Guard denial ends this round.
- No engine/tests, native execution, live-session repair, ABI edits or subagents authorized.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-resource-catalog4-r1-manifest.json.

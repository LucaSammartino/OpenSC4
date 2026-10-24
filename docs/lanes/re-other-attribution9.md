| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-other-attribution9: unowned function attribution | Codex; re-other-attribution9 (detached worktree) | ready; all 155 final-catch-all functions reviewed | Review 58 proposed rows and the gAdviceMessageBox propagation correction. |

## Report

Completed [the ranked ledger](../specs/attribution/other-owners.tsv): **155 functions, 12,549 bytes (12.549 decimal KB)**, covering every current final `*` catch-all function. **High confidence: 33 functions / 3,731 bytes (3.731 KB); medium: 122 / 8,818 bytes (8.818 KB).** [The evidence audit](../specs/attribution/other-attribution-audit.md) records source addresses, confidence rules, folded/dependency limitations, and the map application procedure. [The proposed map file](../specs/attribution/proposed-map-rows.tsv) has 58 exact owner patterns; no coverage-map edits or ABI proposals were made.

The brief's 1.21 MB residual and 600 KB target are stale. The final catch-all now contains only 12,549 bytes. All three catch-alls contain 48,740 bytes / 448 functions; separately, the unattributed set contains 85,977 bytes / 831 functions. This lane does not claim prior lanes' attribution as new work. The main and worktree coverage scripts/maps were both run: their complete final-catch-all rows agree, despite unrelated map-file differences.

### Bytes by proposed subsystem

KB below means 1,000 bytes. These totals are proposals, not measured implementation or spec coverage.

| Subsystem | Bytes | KB |
|---|---:|---:|
| widget toolkit | 2,243 | 2.243 |
| audio | 1,015 | 1.015 |
| 3D scene and models | 966 | 0.966 |
| framework services (RZ) | 811 | 0.811 |
| tree diagnostics | 768 | 0.768 |
| effects | 712 | 0.712 |
| windows and panels | 706 | 0.706 |
| city tools and input | 701 | 0.701 |
| network rules | 686 | 0.686 |
| automata (managers, records) | 631 | 0.631 |
| advisors | 580 | 0.580 |
| Lua host | 446 | 0.446 |
| sim grids | 404 | 0.404 |
| occupants and props | 383 | 0.383 |
| network manager | 355 | 0.355 |
| 2D drawing and fonts | 241 | 0.241 |
| tutorial | 226 | 0.226 |
| web and network | 172 | 0.172 |
| container helpers | 109 | 0.109 |
| persistence and resources | 81 | 0.081 |
| port developers | 74 | 0.074 |
| timers | 74 | 0.074 |
| framework services (GZ) | 62 | 0.062 |
| crime | 54 | 0.054 |
| terrain | 39 | 0.039 |
| application | 5 | 0.005 |
| cheats | 5 | 0.005 |
| **Total** | **12,549** | **12.549** |

### Specification gaps and review points

Largest owners without an owner-level spec: `cDumpSubTreeVisitor` 768 bytes; `cWinBMPViewGVExtra` 706; `cSTEChangeTerrainUI` 701; bridge-local `cParser` 686; `nsGZWinUtils` 682; `RZ` 639; `nsTexUtil` 625. Existing mentions of helpers do not warrant owner-level spec credit. All proposed spec cells remain `-`; this attribution ledger is not a behavior specification.

`004179A0` is free function `gAdviceMessageBox`, currently absorbed into `cADVString` by the coverage tool's `free-neighbour` rule. The ledger proposes its actual free-function owner; inserting that proposed map row alone will not override the propagation rule. Both owners map to advisors. Unnamed exclusive-reach bodies and template dependencies retain medium confidence; their class allocation does not assert a recovered method identity.

### Validation and limits

`python tools/doccheck.py check`: **PASS**. `git diff --check`: **PASS**. Local TSV validation: **PASS**, verifying all 155 addresses, exact exported byte sizes, uniqueness, complete baseline membership, confidence counts, and one-to-one coverage of the 58 proposed owner keys by map patterns. The first validator attempt stopped because it required a high-confidence transferred name even for a vtable-proven entry; the corrected validator accepts that independent evidence and passed. A subsequent full vtable query corrected `00735100` to slot 2 of `00AB38A4` before final validation.

Cargo, clippy, ctest: n/a (documentation only). Game runs and shadow comparisons: n/a. Questions filed/answered: none. No REA calls or session errors; export/decomp sources sufficed. The q.py Bash wrapper failed with a Windows CreateFileMapping error; bounded native PowerShell commands were used. No engine files were read or edited directly, no ABI exports changed, and no behavior implementation was attempted.

## Handoff

- Milestones: initial record `a1c10b51e`; full attribution/evidence draft `0a43b1792`; final report includes corrected slot citation and passing validation.
- Ready for coordinator review of the 58 proposed rows; insert accepted rows before catch-alls with core/shadow/replace `-`.
- Resolve `004179A0`'s free-neighbour propagation separately if retaining its free-function owner in coverage output.
- Evidence inventories: `build-runs/owners.tsv`, `owners-worktree.tsv`; reports `coverage.md`, `coverage-worktree.md`; validation helper `inspect-attribution.py` (all gitignored).
- No pending game requests, ABI rows or questions. Remaining separate catch-alls and unattributed functions were not included in this final-asterisk audit.

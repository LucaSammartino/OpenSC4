| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nettools-ng9 (network drag and host inputs) | `Codex` / `re-nettools-ng9` (RE lane, detached HEAD) | READY partial: NG8 and NT35 answered in c99d94e82; nine drag and three observation-boundary vectors; doccheck and questions check PASS; ABI unchanged; no new game runs | (1) implement NG8 public occupant observations and repeat narrow-drag matrix; (2) resolve existing NT36 occupied-cell/subclass/model boundary |

## Report

NG8 is answered in [network-geometry §3.3.1](../specs/network-geometry.md#331-underground-extension-and-the-observation-boundary-ng8).
The former sentence incorrectly included slot 67 and described only a diagonal finish extension.
The width-one path probes both ends, including wholly straight drags, through slots 64/65.
Underground selection uses an unfiltered standard-city-cell occupant enumeration, own-layer QI,
then a second enumeration for underground connections whose slot 38 matches the layer class ID.
The spec gives argument/storage/ownership rules, candidate geometry, counter changes, bounds,
singleton handling and nine conformance vectors. Reserved cells do not veto this draw extension.

Evidence: export instruction bytes for 0x00659470, 0x006584D0, 0x006585E0,
0x00639790 and 0x006392F0; scoped main-checkout decomp for the network and underground tools;
existing occupant-manager, network-occupants and underground-occupants interface contracts.
No REA call was needed; the configured session has no binary open. No engine files were read.
The initial broad decomp search was slow; subsequent searches were scoped to individual class files.
The Bash q.py wrapper failed with Windows CreateFileMapping access denied; direct PowerShell
queries and gates completed. No game executable or decomp material was copied into the spec.

NT35 is answered in [network-tools §8.1.4](../specs/network-tools.md#814-live-inventory-cannot-be-extracted-from-the-tool-getters-nt35).
There is no public tool getter for the retained solved inventory or its piece/plane/support inputs.
Three boundary vectors distinguish coordinate copies, supplied NT34 constraints and unavailable
live reconstructed price. Public city prices do not uniquely recover a fixture; the measured
110/113, 170/192, 90/93 and 55/61 differences remain explicitly unresolved by this answer.

Both questions moved, with their provenance and answers, to the corresponding answered archives;
`tools/questions.py index` regenerated QUESTIONS.md. No new question was filed. NT36 is unchanged
and remains open. This round stopped new RE at the required two-thirds accounting threshold
(104739/150000 at the milestone commit), so it delivers partial closure rather than a guessed
occupied-cell/model integration contract.

Validation: `python tools/questions.py check` PASS (`questions: ok`);
`python tools/doccheck.py check` PASS (`doccheck check: ok`). Logs:
`build-runs/questions-re-nettools-ng9.log`, `build-runs/doccheck-re-nettools-ng9.log`.
The first whitespace check found a blank line at the now-empty NG8 open file's EOF; removed
in the report commit and checked again. Cargo, clippy and CTest are n/a for this prose-only change.
No C ABI exports, ABI tables or ABI version changed; no proposed ABI-row file is needed.

No new game runs were requested. Existing glue-nettools-shadow12 runs 512/513 remain clean
to 2032-01-01 with ref_warnings=0, wide shadow 73/73 and narrow shadow 0/73. The type-4
cells 12/10, straight 0/0 and total 7/6 mismatch is retained. Those results predate this spec;
neither NG8 nor NT35 confers new shadow exactness, placement, model or whole-owner credit.

## Handoff

- Done: c99d94e82 answers NG8 and NT35; questions index and documentation gates pass.
- Next RE task: read existing NT36 in docs/specs/questions/network-tools.md, then trace
  occupied-cell classification/reuse and power-line/bridge model ownership/call boundaries.
- Start with network-tools §§8.3.1.1/8.3.2 and the named power-line/bridge specs; do not
  infer the native 65/974 inventories from totals or treat missing observations as empty cells.
- Engine follow-up: use §3.3.1 public occupant observations and repeat the same draw matrix.
- No outstanding game request, proposed ABI rows or uncommitted implementation work.

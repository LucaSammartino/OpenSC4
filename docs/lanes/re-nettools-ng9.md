| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nettools-ng9 (network drag and host inputs) | `Codex` / `re-nettools-ng9` (RE lane, detached HEAD) | in progress: NG8 and NT35 answered in c99d94e82; resuming NT36 from 68f9906d2; ABI unchanged; no new game runs | (1) resolve NT36 occupied-cell/subclass/model boundary; (2) run documentation gates and report |

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

The first round committed NG8 and NT35 in c99d94e82 and its partial report in 68f9906d2.
This resumed round answers NT36 in network-tools §8.3.3: ordered occupied-cell classification,
last accepted pole/wire selection, manager/QI signatures, interface pointer distinctions, allocator-A
query-list ownership, and separate pole/wire/model host responsibilities. Seven conformance vectors
cover list order, capabilities, pole coordinate filtering, reuse and bridge publication failure.
Slot 40's last arguments are clarified as null filter and unlimited count, not an occupant flags mask.
Original private constructors and slot-54 cell records are not a general model callback ABI.
All three questions are in their answered archives; QUESTIONS.md is regenerated. No question filed.

Resumed evidence: re_query func/callees and instruction bytes for 0x0064FD00, 0x00650140,
0x00652980, 0x0062EEB0, 0x00624210 and 0x00649210; scoped power-line/network-tool decomp;
existing power-line-tool, network-bridges, occupant-manager and ABI contracts. No REA call needed.

Validation: `python tools/questions.py check` PASS (`questions: ok`);
`python tools/doccheck.py check` PASS (`doccheck check: ok`). Logs:
`build-runs/questions-re-nettools-ng9-r2.log`, `build-runs/doccheck-re-nettools-ng9-r2.log`.
The first whitespace check found a blank line at the now-empty NG8 open file's EOF; removed
in the report commit and checked again. Cargo, clippy and CTest are n/a for this prose-only change.
No C ABI exports, ABI tables or ABI version changed; no proposed ABI-row file is needed.

No new game runs were requested. Existing glue-nettools-shadow12 runs 512/513 remain clean
to 2032-01-01 with ref_warnings=0, wide shadow 73/73 and narrow shadow 0/73. The type-4
cells 12/10, straight 0/0 and total 7/6 mismatch is retained. Those results predate this spec;
None of NG8, NT35 or NT36 confers new shadow exactness, placement, model or whole-owner credit.
NT36 retains powerline valid 13/0 and price 65/0, bridge valid 17/0 and price 974/0;
10 rays and 40 lot reads with prefix 13 and selection complete=0 do not cover the missing host.

## Handoff

- Done: c99d94e82 answers NG8 and NT35; questions index and documentation gates pass.
- Done this round: NT36 answer and seven vectors; moved to answered archive; both gates pass.
- Next: commit the final lane report and write the merge-queue JSON with actual full HEAD.
- Engine follow-up: use §3.3.1 public occupant observations and repeat the same draw matrix.
- Engine follow-up: supply §8.3.3 occupied-cell observations and subclass/model effects, then
  compare fresh placement/cost results; keep current differences visible and default-off.
- No outstanding game request, proposed ABI rows or uncommitted implementation work.

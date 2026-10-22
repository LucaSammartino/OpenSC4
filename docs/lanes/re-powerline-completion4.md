| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-powerline-completion4 (PowerLineTool public owner contracts) | `re-powerline-completion4` / (RE spec lane) | continue: guard stopped queue publication; bounded docs committed; owner PARKED | Coordinator publication only; no further RE round; retain NT18/NT22/NT24 and withhold owner coverage. |

## Handoff

- Goal: final third round completed bounded docs; whole owner remains PARKED. No fourth RE round.
- Commits: f19c79f9 builder; 3dc0b77a zone/type-5 Attempt; 510f273a numeric curves/containers;
  c5e838ab final spec/question/report publication. Preserves 27cc9cf9 and 67f996cf.
- Validated: diff 27cc9cf9 --check passes; questions index regenerated; 46 unique entries / 16156 B;
  all prior 48 vectors preserved plus 12 = 60 unique. Cargo/clippy/ctest/game/shadow n/a; no ABI change.
- NT21 and NT23 answered in docs/specs/questions/answered/network-tools.md. NT18/NT22/NT24 parked
  in open questions; full exact evidence/bytes and remaining invariants in final report below.
- Exact next step: coordinator publishes detached commits to reserved branch and queues bounded docs.
  Do not restart RE or promote whole-owner coverage. No further research is authorised this round.
- Queue attempt was blocked before execution by context guard at 150k; no new queue file was written.
  Any existing ignored queue file is from a prior round and must not be mistaken for this tip.
- Final committed spec/report tip before this handoff: c5e838ab. Actual final tip is this handoff commit.
- Native comparison: sample count trunc(max(minimumCount,length/sampleDistance)), defaults 5 and 8.
- Main checkout read-only; no engine/tools/ABI-table edits. Export/decomp fallback; no REA retry.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-powerline-completion4-r3-manifest.json.
- Harness requires structured state continue solely for budget stop; this is not a request for round 4.

## Final report — 2026-10-22, round 3 of 3

**PARK: the whole owner remains incomplete.** Bounded documentation is ready to publish; no
whole-owner coverage, replacement or host-hook-removal credit. No fourth round, restart, merge,
push or ref update. Preserved `27cc9cf9`, prior `67f996cf` and all earlier lane work.

Substantive commits:

- `f19c79f9`: complete native instruction trace of 0064E3E0 (1338 B), record gating, geometry
  dispatch and optional wire-reference lifetime (§5.3).
- `3dc0b77a`: zone entry/query/pole/removal cleanup (§6.4), and all 1438 B of type-5 regional
  Attempt 00628BD0 (§5.4). No failure-code mutation or preview/consent UI inside Attempt.
- `510f273a`: control-point helper 0064A6C0 (431 B), cubic sampler 006493B0 (1068 B), property
  defaults/reads 0064A170 (245 B), nested geometry/list copies, and 13 containers / 1530 B (§§5.5,6.5).
  Native comparison proves N=truncate(max(minimumCount,length/sampleDistance)); default minimum 5,
  sample distance 8. Main geometry copies A then N−1 samples; extra geometry has 3 samples without A.
- Final publication: NT21 and NT23 moved with original question text to answered/network-tools.md;
  12 new independent conformance vectors, preserving the prior 48 (60 unique total).

### Exact parked obligations

- **NT18:** construction/reachability of nonempty +0x398 groups. Containing consumers are
  DeterminePolePositions 00650840 (3345 B) and PlacePoles 00651560 (2884 B); this does not mean all
  6229 B are unproved. §6.6 lists every literal +398/+39C access in 0064xxxx–0065xxxx: constructor,
  destructor and PlacePoles only. No direct producer, and no zone-update access. A producer through
  an alias, adjusted base or external/inherited call remains unidentified; its byte count is unknown.
- **NT22:** the data invariant making backward split `(k,count,-1)` at 00652828 safe. UpdateOnZoneChange
  006522F0 (1658 B) is traced; its local endpoint/list/count/match guards do not guarantee an outside
  clear cell at or before k. FindNewPolePosition 006520C0 (552 B), especially 006520E1–00652120,
  has no lower-bound check. Reachable connection-cell/rectangle construction is the remaining evidence,
  not another unread orchestration branch. No observed crash or safe clamp is invented.
- **NT24:** source/counting basis of the brief's 49 functions. Measured inventory stays **46 unique
  attributed entries / 16156 B**, unchanged. All 13 listed container bodies now have contracts.
  No three extra function addresses or residual body-byte total can be assigned without that source.

### Validation and publication

- Native export instructions with bytes establish widths, branches and operation order. Original PE
  confirms float32 3C23D70A/3F000000/3F800000 and float64 1/3/6; defaults verified in native stores,
  not copied from decompiler register attribution. Local export/decomp fallback only; no REA retry.
- Final checks: whitespace diff, question-index regeneration, inventory count/byte sum and vector-ID
  uniqueness. Cargo/clippy/ctest, game and shadow are n/a for docs-only work; no game runs requested.
- No engine, tools, ABI table, export or game-file edits; no ABI proposal. Evidence scratch remains in
  build-runs/power-owner.tsv, power-owner.py, inspect-power.py and rd.py. Bash wrapper startup denied
  E_ACCESSDENIED; bounded PowerShell used. Commits succeeded despite packed-refs.lock warnings.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-powerline-completion4-r3-manifest.json.
- Next: coordinator may publish these bounded docs. Keep the owner parked and residual questions open.

## Prior round-2 report (historical; residuals superseded above)

Whole owner remains **incomplete**. This round supplies reviewable primary-evidence contracts, not
implementation coverage, host-hook removal or replacement credit. Preserved `67f996cf`, earlier
`0e36dec5`/`b00920bc`, and all ignored inventory/evidence. No engine, tool, ABI table or export edits.

Milestones:

- `2c750974`: fixed both trailing-blank-line warnings first; interception eligibility, forced override
  limits, ordered descriptor equality, coordinate-OR duplicate predicates and endpoint reference transfer.
- `42301fa2`: zone/building/road breaks, strict last-clear closure boundary, native zone-release
  asymmetry, saved endpoint detachment and append-only repair queues.
- `25bad428`: classifier's literal third zone probe at (z,z), last-qualifying overwrite order,
  last-match pole/wire probes, QI ownership, enable predicate, reference-copy helper and raw pool allocator.
  Restored original NT19/NT20/NO24 question wording in answered files.
- `63c050e8`: all FindNewPolePosition branches, unused ninth argument, borrowed optional counterpart,
  insertion/release/draw order. Corrected adjacency groups to dragged-cell indices and emission off-pivot.
- `9f712744`: zone-wire endpoint-overlap deduplication, first-match split search and literal relocation
  calls `(k,count,+1)` then `(k,count,-1)`; documented backward-walk boundary uncertainty.

## Whole-owner checklist

All 46 unique attributed entries, 16,156 body bytes, remain in power-line-tool.md §9 and
`build-runs/power-owner.tsv`. The brief's 49-function count is still unreconciled; byte total matches.
The following groups enumerate the complete measured inventory; a contract in one group grants no
completion credit to the owner or its dependencies.

| Functions | Contract / residual |
|---|---|
| 00626990, 0064F0D0, 006503C0, 00650560, 00650580, 006505D0, 00650630 | Prior lifecycle/defaults/resources contracts retained §§1,7 |
| 0064F0F0 | Existing cost contract retained |
| 0064EFE0, 0064F2B0, 0064F3F0, 0064F570 | Position/sentinel, slot82, draft undo and failure-string contracts retained §§2–5 |
| 0064F140, 0064F880 | Resource enumeration retained §7 |
| 0064F170, 0064F1D0, 0064F230, 0064FB40, 00650760 | Record/pivot/link contracts; ordered equality, coordinate OR and duplicate link append verified |
| 0064FED0, 00650140 | Reserved walk and create/reuse contracts retained |
| 0064FD00, 0064FFE0, 0064FBF0 | Classifier and complete probe selection/reference contracts added §6.1 |
| 00650840, 00651560, 00652980 | Determine/Place branch contracts extended §§4–5; wrapper retained; external geometry/group-source obligations remain NT18/NT21 |
| 006520C0, 006522F0 | Relocation contract added §6.2; zone-wire branch §6.3; zone entry/pole branch and final cleanup remain NT22 |
| 00649910, 0064A2E0, 0064B6E0, 0064E3E0 | Enable/raw pool/reference-copy specified; 1338-byte connection geometry remains NT21 |
| 0064F5D0, 0064F610, 0064F670, 0064F710, 0064F770, 0064F920, 0064F9A0, 0064FA60, 0064FAF0, 00650250, 006502B0, 006502F0, 00650480 | 13 container helpers / 1530 B: full copy/erase/destruction ownership remains NT24 |

## Questions and exact next evidence

No new question IDs; updated public residuals in `docs/specs/questions/network-tools.md`:

- NT18: caller branches now specified; source/reachability of +0x398 groups unproved. Do not attribute
  construction to zone update without evidence. Full connection body is separately NT21.
- NT21: finish 0064E3E0, 1338 B, including numeric geometry and optional wire-vector lifetime;
  enable 00649910 and list-copy 0064B6E0 are now proved. Decomp body is at main
  `decomp/_unnamed/region_00640000.c:4840`; it misattributes registers, so use primary instructions.
- NT22: prove 006522F0–006525B9 entry/query/pole branch and 00652883–00652979 final removal/cleanup.
  Establish the invariant permitting the second split call's `(k,count,-1)` scan to find a clear outside
  cell before indexing below zero. FindNewPolePosition has no lower-bound guard.
- NT23: full type-5 AttemptNeighborConnections 00628BD0 endpoint/direction, consent/failure, vector and
  preview behavior remains unproved. Wrapper Attempt/Place pairing is already answered by NT20.
- NT24: 13 container helpers and count discrepancy above. Classifier/probes/pool obligations resolved.

NT19/NT20/NO24 remain answered; index regenerated with `python tools/questions.py index`. No answer
was closed merely because one of its dependent branches was specified. This authorized round ends here;
no self-restart, merge, push, ref update or coverage promotion.

## Validation and evidence

- Native export instruction bytes and bounded decomp reads were the evidence source. Numeric adjacency
  constants independently read from original PE: 00A80AA8=4F800000 (2^32), 00A81228=3F800000 (1.0).
- Inventory recheck: 46 rows, 46 unique addresses, sum 16156 bytes. Vector table: 48 rows, with labels
  reconciled to 48 unique IDs (23 added this round). These are independent host-observation contracts,
  not executed game traces or implementation fixtures; no constructed-only coverage claim.
- `git diff 67f996cf --check` passes; question index regenerated. Cargo/clippy/ctest and game/shadow n/a
  for this documentation-only round. No ABI proposals or exports changed.
- Prior REA attempt failed `target_unavailable` / no application open; local export/decomp fallback
  retained, no REA retry needed. q.py Bash startup again denied E_ACCESSDENIED; bounded PowerShell used.
- Scratch evidence remains under this worktree's build-runs/: power-owner.py, power-owner.tsv and
  inspect-power.py. Main checkout read-only. Detached commits succeed despite packed-refs.lock warning.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-powerline-completion4-r2-manifest.json`;
  one guarded round, context 150000 / max calls 100. WIP queue identifies the actual detached HEAD.

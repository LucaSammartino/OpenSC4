| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio9: cSC4Audio owner contracts | Codex; re-audio9 detached worktree | ready: documentation increment; full behavior remains partial | Land docs; retain audio spec coverage part; resolve AU40–AU44. |

## Report

The changes are ready to land as a documentation increment. They do **not** finish the brief's
full behavioral specification of every owner. Proposed coverage-map `audio` spec-column value:
**`part`**. No coverage-map edit was made. A complete inventory is not evidence of complete
behavioral coverage; the unresolved contracts below remain material for implementation.

### Changes and measured facts

- Queried the Windows export first, then checked named-owner inventory and targeted main-checkout
  decompilation. The named inventory has **15 cSC4Audio owners / 198 named entries**, including folded
  names; these are not unique method counts or a byte-coverage measurement.
- Added [audio-owner-contracts.md](../specs/audio-owner-contracts.md), linked as the §§17–21 supplement
  to [audio.md](../specs/audio.md). It inventories every named owner and links existing event-handler,
  sound-manager, AEMS, ambience, director and UI framework contracts instead of duplicating them.
- Added listener construction/interface observations, camera/city/tuning dispatch, position-message
  fields, strict squared-distance gating and the instruction-verified zoom multiply order.
- Corrected audio §13: listener **sends** `0x8A4A69C7`; its camera inputs are `0xA6B79602` and
  `0xA6B79612`. Linked this producer contract from [audio-ambience.md](../specs/audio-ambience.md) §7.
- Filled the 3D emanation model's missing primary/secondary vtables and identifier `0x7DCDED3D`.
  Added shared model slots and SEM registration/teardown observations. The export's `Init` label
  for `0x004ED750` conflicts with its teardown effects and must not dictate replacement behavior.
- Recovered all **19 primary scape slots**, plus service and secondary table boundaries, directly
  from image pointers. Added lifecycle, selected key-failure rules and stop-event IO. Model arithmetic
  and the full scape instance algorithm remain explicitly open.
- Added preferences interface/method inventory, master-volume/message IO, 30-entry persistence bounds,
  source/notification acceptance and instruction-verified slider arithmetic. Added **15 prose
  conformance vectors AU9-01–AU9-15**; no engine vectors or source were edited.

### Validation and evidence limits

- `python tools/doccheck.py check`: **PASS**, including final tracked spec content.
- `python tools/questions.py index` regenerated the index; `python tools/questions.py check`: **PASS**.
- `git diff --check HEAD~1 HEAD` at spec commit `1f349c27a`: **PASS**.
- Local `build-runs/check-audio-docs.py`: **PASS**, checking relative-link paths, 15 vector IDs and
  19 sequential ABI proposal slots. Its first run falsely recognized inline-code expressions in the
  pre-existing spec as links; after excluding inline code it passed. The first result was a checker
  limitation, not a broken-link repair. This check does not verify anchors or behavior.
- Cargo, clippy and ctest: **n/a**, documentation only. Game runs: **none**; no shadow/replace verdict.
- REA was not called: the brief states no binary is open in that session. Used the local export,
  targeted decomp files and executable read-only. Corrupt numeric pseudocode was not treated as a
  settled algorithm. No claim of full 63 KB owner coverage or game verification is made.
- No engine, plugin, capi, tools/coverage-map.tsv, docs/abi, README or coordinator handoff edits.

### Questions and ABI proposal

Filed **AU40–AU44** in [questions/audio.md](../specs/questions/audio.md): listener geometry/failures;
SEM/model arithmetic and registration rollback; complete scape operations; preferences construction,
controls/song persistence; complete disaster-fire handler. No pre-existing open question was answered
or moved. AA10/AA12/AA13 were left unchanged.

Proposed ABI rows: **`build-runs/abi-rows-re-audio9.tsv`** in this worktree, 19 primary scape rows in
the existing vtables.tsv column format. Targets/slots are measured; unverified argument/return columns
are intentionally blank. It is an ignored proposal artifact, not an applied ABI-table change.
**No C ABI exports changed.**

### Commits and next steps

- `09409ee7f`: initial lane inventory/handoff.
- `1f349c27a`: owner supplement, message correction, model facts, vectors and questions.
- Resolve AU42 and AU44 before treating scape/fire integration as implementable in full; then AU41,
  AU40 and AU43. Extend executable conformance vectors in an engine lane only from settled rules.
- Coordinator may land these docs and review/apply ABI proposals; retain coverage `part`.

## Handoff

- Documentation increment complete; queued for landing, not a full audio completion claim.
- Spec milestone: 1f349c27a; initial lane record: 09409ee7f.
- Evidence scripts and ABI proposal remain under this worktree's ignored build-runs/.
- Next RE work: AU42 scape operations and AU44 fire handler; exact ranges are in questions/audio.md.
- Remaining: AU40 listener geometry/failures, AU41 SEM/model arithmetic, AU43 preferences/song controls.
- Recommended audio spec coverage stays part. No game request is outstanding.

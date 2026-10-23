| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv5: QV12/QV13 owner contracts | Codex / re-map-view-qv5 (detached HEAD) | ready: doccheck passed; QV12/QV13 answered | Implement map-view §§1–3, 5–7 and vectors 5–16; other map-view questions remain separate. |

## Report

Documentation-only RE lane. No engine, plugin, capi, ABI tables, coverage map or tools changed.
The reserved worktree was reused on detached HEAD as instructed.

### Changes

- `18cb63ea`: native selection/legend contracts. Selection failures are not transactional; missing
  exemplars leave the old identity after teardown. Custom creation success calls no-argument initialization;
  false initialization/creation and child-interface failures do not impose a general selection abort.
  Buttons continue to the requested legend/panels. Reset keeps the same-view guard.
- `72c53ae2`: hotspot effects, highlight colours and twelve conformance vectors (5–16). Corrected the legend
  title to scalar u32 and specified the native unequal-list boundary, including its unchecked equal-count
  read. Moved QV12/QV13 to `docs/specs/questions/answered/map-view.md` and regenerated the questions index.
- Final clarification: `Update` is void; a normally returning custom update result does not gate its common
  clock-store tail. Individual early exits remain QV9, rather than an invented generic error policy.

### Evidence and measured results

Used the original Windows export first through `re_query.py`, bounded main-checkout decompilation reads,
and byte-bearing instruction pages for numeric/branch/argument verification. No REA call was needed.
Checked constants directly in the read-only executable: double 0.5/16; float 0.5/1/2/128/12582912.

- Crime slot 8 returns byte strength and signed cell coordinates. Map ignores strength and places the
  radius-100, null-occupant record at centred-cell world coordinates with terrain altitude.
- A nonempty name requests effects through manager slot 7. Successful hotspot polls attempt a temporary
  scale-100 effect, followed by a retained scale-200 effect; both use transform flag bytes 2,3 and
  Start(false). Repeated hotspots stop/replace the retained effect and mark changed even without movement.
- EF shades building footprints; EE draws coverage rings and ignores its source alpha byte. Neither
  parameterizes the named 3D effect. Geometry, blend arithmetic and creation/acquisition failures are in §5.
- Evidence anchors: selection `0x007A49B0`, legend `0x007A04F0`, reset `0x007A5610`, mode button
  `0x007A0F30`, hotspot `0x007A1C21`–`0x007A1DDC`, common effect helper `0x0079F620`, reconciliation
  `0x007A1390`, bitmap decoration `0x007A45F6`–`0x007A481E`.
- `python tools/doccheck.py check`: PASS. Questions index regenerated successfully. Final whitespace check
  run after removing an extra EOF blank line flagged in the earlier check.
- Cargo/clippy/CTest: n/a, specification-only change. Game/shadow runs: n/a; no replacement was wired or run.
- Git commits succeeded despite a shared `packed-refs.lock` permission diagnostic; no shared refs/config
  were modified by the lane. The actual detached HEAD is used for the queue tip.

### Questions and limits

Answered QV12 and QV13; filed no new questions. QV1–QV11 remain open in their existing scopes, including
general building-set comparison, reset semantics, malformed property/ramp domains and early-exit clocks.
Invalid required pointers and the legend's unchecked read are identified as native unsafe boundaries;
the spec does not invent deterministic fallback text or recoverable native errors for them.
The owner README and implementation were not edited; implementers can now replace the QV12/QV13 boundaries
using the spec and conformance vectors. Full native owner closure is not claimed by this documentation lane.

## Handoff

- Goal: answer map-view QV12 and QV13 using original executable evidence; documentation only.
- Done: `18cb63ea`, `72c53ae2`; both answers archived; spec vectors 5–16; doccheck passed.
- Next: coordinator lands the queued documentation tip; implementation lane consumes these contracts.
- Worktree is detached by coordinator design; commit HEAD normally, do not switch refs.
- Source evidence is in main checkout; edits only in this worktree; never inspect engine implementation.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv5-r1-manifest.json`.

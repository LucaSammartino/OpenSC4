| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-answers5d: network and underground owner answers | Codex / `re-netocc-answers5d` (RE, detached HEAD) | ready: NO36 and prior NO41 answered; 6 new prose vectors this round; doccheck/questions/diff checks passed; ABI unchanged | Land verified answers; NO33 NO40 NO43 UG5 remain open for further native research |

## Report

Round 2 answers **NO36** and publishes another verified subset of **UG5**. The result is READY
to land as spec work, as requested even with other questions open; it does not claim complete pole
or underground owner closure. Prior **NO41** remains answered in network §3.6.3/test 51
(`2869269f9`): opaque base +268, adjusted descriptor +26C and connection auxiliary fields.

### Changes and evidence

- `c617504ae`: network §6.2.4 completes pylon MySim pixel composition, number-label rasterization,
  resource failure behavior, main/label billboard geometry, transform rounding and cached bounds.
  Corrected +150 to call-number texture ID; MySim uses main +148. Added prose vectors 56–59,
  archived NO36 with DIFFERS and regenerated the question index.
- `bf981d41e`: network §3.6.4 and underground §4 specify all four connection message decisions,
  interface-query/reference lifetime, equal-pointer reattachment, own-primary-pointer fallback,
  lower-height updates, linked removal/Shutdown, context +4-word comparison and the −0x24C
  bounds-wrapper adjustment. Added prose vectors 60–61 and a UG5 progress entry.
- Retained earlier pylon Draw/messages (`cb51c38e8`, tests 52–53) and pole occupant bounds
  (`9162e3b21`, tests 54–55). These combine with the previous setters/accessors to answer NO36.
- Native sources: main export queries first, bounded main decomp excerpts, then instruction bytes
  for numeric decisions at 005F12D0/005F1D00/005F20A0 and 00656C80/00654140/00654160.
  Resource constants read from the PE at AA4F58/AA51AC/AA51B0. No decompiled text or instructions
  appear in the specs. REA was not called: the session has no binary open.

### Validation and limits

`python tools/doccheck.py check`, `python tools/questions.py check`, and `git diff --check` passed.
This round adds **6 prose conformance vectors**, not executable implementation tests. Cargo, clippy,
CTest and game runs are n/a for this RE-only text change; shadow exactness n/a. ABI exports unchanged;
no proposed ABI rows. No engine reads/edits, plugin, capi or tools/coverage-map.tsv edits. The closure
table lives in forbidden engine paths; core-netocc-close5's lane report and the owner question entries
were used as the requirements instead. Source and spec-only verification cannot certify implementation.

No new questions filed. **NO33, NO40, NO43 and UG5 stay open**, rather than receiving guessed
geometry or owner-result callbacks. NO36 was archived; NO41 was already archived. Research ended
after measured context 102701 (that figure includes writing/validation); remaining allowance was
used for specification, checks and delivery. Commits succeeded despite an incidental packed-refs.lock
permission message; detached HEAD is retained and the supervisor publishes the reserved branch.

## Handoff
- Delivery: READY for answered questions and verified partial contracts; no further work in this round.
- Done: NO36 c617504ae; connection message subset bf981d41e; prior NO41 2869269f9.
- Next research: NO40/UG5 CreatePieceModel/CreateGeometry then Draw and generated transform/bounds behavior.
- Underground source: main decomp/cSC4UndergroundOccupant.c 1076–1837; connection 1132–2168.
- Then NO33/NO43: pole model/resource/transform/Pick/wire/Draw and diagonal-floor arithmetic order.
- Pole source: main decomp/cSC4PowerPoleOccupant.c 495–832, 1003–1166, 1632–1784.
- Avoid stale export names: 00648360 is a layer query; 00654160 is attached Refresh; 00654140 adjusts −24C.
- Ignored helper build-runs/evidence_constants.py reads PE constants only; no helper or RE output committed.
- No pending game requests, ABI proposals, implementation checks or engine reads.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-netocc-answers5d-r2-manifest.json.

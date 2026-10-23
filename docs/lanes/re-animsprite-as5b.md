| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-animsprite-as5b: animated sprite remaining contracts | Codex / `re-animsprite-as5b` (RE, detached worktree) | Partial spec ready: AS2/AS10 answered; doccheck passed; full owner closure remains unproved. | Resolve AS1 controller producer, AS3 exact inventory and AS6/AS7 native numeric qualification. |

## Report

Answered **2 of 6 requested questions**, AS2 and AS10. Their complete question
entries and dated DIFFERS answers moved to
`docs/specs/questions/answered/animated-sprite.md`; the question index was
regenerated. AS1/AS3/AS6/AS7 remain open with new dated partial evidence. The
delivery is ready as partial documentation, not complete animated-sprite owner
closure. Previous AS4/AS5/AS11 answers were not redone.

### Published behavior and measured results

- **AS2:** new §7.1 and `docs/specs/animated-sprite-data.md` enumerate all 1,084
  shipped animation model TGIs and all 469 literal exemplar-property reference
  occurrences found across 452 exemplar records (389 distinct referenced models).
  All animation headers and 2,253 tables occur in SimCity_1.dat. All 9,345
  exemplars/cohorts in the eight selected installed DATs parsed without errors.
  The other 695 models are not labeled unused: inherited/family/state/script
  references and runtime instantiation were not measured. The full identifier
  table records exact keys/property IDs; named examples and package identity
  are in §7.1. No binary/resource payload was committed.
- **AS2 naming:** the Mac associations for 007FD7D0/007FED50 are tier D,
  score 0.5, one supporting match. They establish tentative label provenance,
  not Mac behavior. The proved Windows dispatch stays authoritative.
- **AS10:** §3.1.5 publishes LookAt's mixed transient/binary32 difference,
  reciprocal, normalized-component and cross-product boundaries, checked against
  instruction bytes at 007FE257–007FE370. §4.1.1 distinguishes hardware FPATAN,
  the classification-only binary64 asin copy, the final binary32 angle stores,
  and the CRT helper's temporary `(CW & 0x0300) | 0x007F` mode. It preserves PC
  while forcing nearest RC internally; startup PC53 does not prove renderer mode.
- **AS1 partial:** §4.3 reconciles serialized-stream, borrowed-block and holder
  offsets. Block +0x1C is entry count; holder +0x1C copies block +0. The mutable
  owner controller +0x34 is separate. Reviewed creation/load paths do not install
  it; no native allocation/installation producer was found or invented.
- **AS3 partial:** independently read the sprite 23-slot and shared 19-slot
  vtables from the PE; they agree with the existing published surfaces. Broad
  adjacent export intervals contain 72 functions/11,081 bytes and include other
  owners and pool helpers. This is not the requested 60/10,482 reconciliation.
- **AS6/AS7 partial:** §4.1.1 retains the runtime/foreign-call CW provenance
  boundary; §6 distinguishes Pick's x87 binary32 X/Y copies from integer Z
  copies. No malformed/nonfinite/trap results or native bitwise goldens claimed.
- **Conformance:** §10 adds proposed UNEXECUTED LookAt reciprocal-split and
  asin CW-transform vectors plus a measured static census vector. These are
  independent arithmetic/identifier facts, not native owner-call results.

### Validation and limits

`python tools/questions.py index` succeeded. Final
`python tools/doccheck.py check`: **passed**. Cargo, clippy and CTest are n/a
for this documentation-only lane. No game runs, game requests, shadow comparison,
C ABI changes or proposed ABI rows. No engine/plugin/capi, coverage-map, tools,
main-checkout or shared-ref edits. The blanket no-engine-read rule took precedence
over the brief's request to read the engine module README; its closure limitations
were read from `docs/lanes/core-animsprite5.md` instead.

All RE used the local export, metadata and read-only installed DATs; no REA
operation was needed. Ignored scripts in this worktree's build-runs/ reproduce
the census and PE/export inventory. The first census command printed excessive
detail and was immediately bounded; the corrected complete census and table
generation both terminated successfully. Git's first commit emitted a
packed-refs.lock permission warning but created `8c272406`; no shared ref was
changed. No asynchronous process or outstanding game label remains.

The research phase ended at **96,047 / 150,000 measured context units**, below
the two-thirds research cap. Remaining questions were kept open instead of
being relabeled on incomplete evidence. Further work needs a proved controller
producer, the source owner-accounting roster, and runtime CW/foreign-primitive
qualification with independently obtained numeric evidence.

## Handoff

- Goal: remaining controller/data/inventory/numeric questions; documentation only.
- Prior lane: AS4/AS5/AS11 answered; do not redo. Its handoff read.
- Done: `8c272406` AS2 static census/AS10 boundaries; final report commit adds AS1/AS3/AS6/AS7 partial evidence.
- Measured: 1,084 models/2,253 tables, all SimCity_1.dat; 469 direct refs/452 exemplars/389 models.
- Evidence scripts: ignored build-runs/census-as5b.py, census-table-as5b.py, inventory-as5b.py.
- Next: find the native producer installing controller +0x34; do not use block +0x1C as sequence starts.
- AS3: get the authoritative 60/10,482 roster; broad address intervals are not an owner count.
- LookAt dx/q/bx/by/rx/rz have retained transients; asin helper preserves PC and forces nearest RC internally.
- No renderer CW capture; startup PC53 request is not an owner-entry-mode proof.
- Restrictions: never read engine; closure evidence comes from core-animsprite5 lane report.
- No game runs or ABI changes planned. Main checkout is read-only.

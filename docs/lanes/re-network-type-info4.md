| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-network-type-info4: owned static TypeInfo lifecycle | Codex / re-network-type-info4 | ready | Land bounded spec; TI1–TI4 remain explicitly unresolved. |

## Report

Published [network-type-info.md](../specs/network-type-info.md) as the dedicated owned lifecycle contract,
with stable links and corrected facts in network-occupants §8 and network-geometry §2.3. This is new defaults,
lifetime and branch evidence, not credit for repeating the previously established loader/property labels.

Measured from canonical export instructions and PE bytes:

- Thirteen forward static constructions and reverse exit destructions; untouched scalars start in PE zero-fill,
  while five derived fields have explicit nonzero constructor words. Constructor defaults are not reload defaults.
- Clear deletes evaluator pointers backward, bridge elements forward, and support nodes by bucket/chain order;
  backing storage, loaded byte, placement vectors/scalars and derived fields survive Clear.
- Reload processes each record fully in road/one-way/dirt/rail/light-rail/monorail/street/highway/ground-highway/
  avenue/pipe/subway/power-line order. No local conditional branch tests any loader result. No all-record staging
  or rollback. LoadComplete is a byte store; a later reload never clears that byte. Power line skips content loaders.
- False whole-exemplar acquisition leaves P untouched but the wrapper still derives and publishes. Admitted
  resources with missing angle properties can reconvert retained radians. Absent vectors differ from explicit
  empty vectors and successful short vectors are not padded or validated for equal lengths.
- Corrected P+0x60 fallback to one zero word per P+0x54 entry; P+0x6C fallback to one 10.0 word per entry;
  0xE974EAA7 destination to P+0xA0; 0x4A27124F default to zero; documented 0x4A275748 and the cross-field
  0xEA86E3B5 failure write to P+0x8C. Corrected diagonal arithmetic to continue from the resident x87 intermediate.

### Evidence and inventory

The export executable SHA-256 and independent original-file hash match
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`; original Windows file version is
1.1.641.0. Canonical re_query/export instructions are primary; local decompilation was navigation only. No REA
session action was required or attempted. A standard-library PE inspection verified the zero-filled array range
and constants 0x3C8EFA35, 0x3F3504F3 and 0x41800000.

Spec §7 individually qualifies 18 relevant outer/lifecycle/helper bodies: ten specified within the normal-return
primitive envelope and eight partial, with foreign content bodies explicitly unknown here. Sizes are individual
export measurements, never a total coverage claim. The historical 20-row/4,836-byte reach attribution is not
owner completion; evaluator factory 0x00622A90 is expressly excluded. Generic suffix-copy behavior and foreign
bridge/support/evaluator/parser/model algorithms receive no new algorithm credit.

Admission proof was copied at `build-runs/evidence-run4/coordinator-network-type-info-first-host-proof.json`.
Verified current own manifest identity: re-network-type-info4, kind re, round 1, this worktree. Runtime guard
status was within budget at each milestone. No global host/spawncheck was rerun in the worker.

### Validation and limits

27 independently derived literal/trace vectors in spec §6 are **UNEXECUTED**. They include constructor/Clear,
first/later missing resources, ordered reference destruction and retention, duplicate/repeated calls, absent/empty/
unequal vectors, explicit/default fields, angle words and primitive-boundary failures. No implementation output
was consulted; no native execution or green proof is claimed.

Questions index regenerated; questions check, doccheck check/links and git diff --check passed. Final checks run
again with this report before enqueueing. Cargo, clippy, ctest, builds and games: n/a (docs-only RE scope).
ABI exports unchanged; no ABI proposal file. No engine/glue/maps/tools/generated-coverage/coordinator edits.

### Questions and next steps

Filed TI1–TI4 in [questions/network-type-info.md](../specs/questions/network-type-info.md): present-variant
coercion/output semantics, nonreturning allocation/parser failures and unwind behavior, x87 control/FPTAN exact
derived bits, and foreign support-key duplicates. These are explicit bounds, not prerequisites to the published
normal-return owner contract. No prior question entries were answered or moved.

Land the documentation tip through the normal queue. Implementers may use the supported envelope and proposed
vectors, preserving the distinction between static arithmetic and executed evidence. Follow-up RE beyond that
envelope requires separately scoped work; this round makes no request for additional allowance.

Milestones: a07d4b77 initial substantive contract; 626e550f independent vectors, body map, questions and prior-spec
corrections. Final tip is the report commit named in the queue and structured result; detached HEAD is intentional.

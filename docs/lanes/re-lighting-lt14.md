| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lighting-lt14: LT14/LT15 behavior | Codex / `re-lighting-lt14` (RE lane, detached worktree) | ready; answers `c72a586e7`; doccheck/questions/whitespace PASS; no ABI changes or game runs | Land with glue-lighting-shadow9 question reconciliation; whole-owner capture remains an implementation task. |

## Report

Resolved LT14 and LT15 in lighting.md §§1.4, 1.6 and 2.4, with conformance vectors 23–27.
LT14 has a negative snapshot answer: the lighting owner has no serialization interface, and current
clock/view/settings cannot reconstruct its historical state. The spec gives the independent ordered
input record needed for replay, source interfaces and dependency boundaries, initialization and shared
property history, tint/reset calls, settings, view and terrain notifications, delivery-time reads,
query-specific inputs and safe comparison scope. A complete public capture facility is not claimed.
The unidentified public meaning/accessor of the raw suppression byte remains LT1, with numeric
environment/NaN questions LT12/LT13 unchanged. Missing inputs must limit coverage rather than be seeded
from native lighting outputs. Answering the contract does not establish whole-owner runtime exactness.

The normal time driver reads visual hour (clock slot 12), then visual minute (slot 15), at post-city-init
and VisualTimeTick delivery. Its argument is one binary32 store of hour + minute × binary32 0x3C888889;
seconds, saved game time and posted message payload are not the driver's argument source. Checked
instruction bytes at 007E6E40–007E6ED9 and 007F5551–007F556A; read the constant directly at 00A94124.
The event-history vectors cover stale fade gradients, cached month and retained process-shared texture.

LT15 is a transcription correction: installed GOG SimCity_1.dat exemplar
6534284A/A9189CF0/00000001 property 085489C8 is u32 **69D3A23B = 1775477307**, not 69D3BC3B.
Verified independently with main tools/dbpf.py and checked the unsigned property read and unchanged
integer getter copy in the instruction bytes. Vector 23 also specifies missing-property retention
and per-holder override isolation. The glue lane's r1 0/46 literal-table comparison is preserved as
failed evidence; this lane neither reran nor relabeled that result.

The named glue owner-closure table was absent from its current lane file. Read its available Handoff
open-scope row and the exact LT14/LT15 entries from the reserved glue worktree instead. Neither entry
exists in this base's or main's open questions file; imported the original entries directly into the
answered archive, with date/lane/rule markers, and regenerated QUESTIONS.md. On landing with the glue
branch, remove any reintroduced open LT14/LT15 entries and regenerate the index. Locally the index
has lighting 6 open / 9 answered, next LT16; no local open question was silently deleted.

Validation: tools/questions.py index and check PASS; tools/doccheck.py check PASS; git diff --check
PASS. Export/decomp and installed data were sufficient; no REA query or fallback error. No engine,
plugin, capi, ABI table, coverage map, tooling source, game file or external artifact changed. No new
questions, ABI proposals, C ABI exports, cargo/clippy/CTest runs or game requests. Work is spec text
only; helper/evidence scripts remain ignored in build-runs.

## Handoff
- Done: initial handoff d8bbeacfb; answers, five vectors and question index c72a586e7.
- Next: coordinator lands actual detached HEAD on reserved re-lighting-lt14 branch.
- Reconcile open LT14/LT15 from glue-lighting-shadow9 if that branch lands before/after this one.
- No game request outstanding from this lane; no implementation or whole-owner verification claim.

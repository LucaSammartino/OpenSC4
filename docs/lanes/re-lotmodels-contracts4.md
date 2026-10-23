| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotmodels-contracts4: LM1–LM8 native contracts | Codex / `re-lotmodels-contracts4` (RE) | ready: bounded finite LM4/owned LM7 publication 5a7e8f0c,1257ad67; LM4/LM7 answered; questions/diff/doc checks pass; no runtime claims | Coordinator review; retain LM1/LM2/LM8 partials; no automatic additional round. |

## Report

Same original RE family, round 2, admitted from the supplied own lane/re/worktree/round2 manifest and copied
`build-runs/coordinator-lotmodels-r2-host-proof.json`. Runtime own role/round/not-parked state and guard
were checked. No global-ref admission, REA preflight, server/session action or additional round was attempted.
The initial detached `87fc8b73` tree was clean. Its retained findings and `81d9d85b`/`dd213dcc`, including
the previous 11 vectors and LM3/LM5/LM6 answers, remain intact; the historical uncommitted claim was stale.

`5a7e8f0c` publishes substantive LM4 branches in lot-models §3.2. `1257ad67` publishes the completed
bounded LM4/LM7 text, independent ledgers and question moves. Primary evidence is the Windows image
SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, matching the main export
summary, imagebase00400000 and 2,193,251 instructions. re_query instruction bytes/data were primary;
decompilation was navigation only. No REA query was necessary or attempted.

LM4 now specifies four side ranges/inner/across cells, externally controlled x87 arithmetic, sampling
and neighbour-test order, strict eligibility, all three ordered emit branches, equality/crossing rules,
UV origin, raw light outputs and bounds. Small offsets affect bounds only; along-edge bounds are inset.
Crossings are not interpolated. Bounds use raw heights without seam overshoot. Rebuilds retain an old
record tail; the stored draw length is not the emitted-segment byte. The valid finite/allocation/service
domain is explicit. §8.1 crosses four literal sides with eleven height cases and supplies UV, colour,
bounds, empty/narrow/tie/neighbour and reuse ledgers.

LM7 now specifies reload/recolour/binding/pass order and returns, exact initialized value/mask words,
conditional driver-call order, force effects, combiner initialization/decoder tuple and names supported
by the existing public driver contract. Mode255 is sentinel; explicit0 tints. All RGB uses light
component0. Mode3 fallback shifts unsigned alpha before multiplying stored k; geometry alpha remains
original. Force8 is after pass filtering. The flat DrawPrims citation is corrected to 005E0424–005E0433.
§8.2 includes explicit mode/pass/failure/cache/fallback traces and packed-bit counterexamples.

Validation: `python tools/questions.py index` regenerated the index; `questions.py check` passed;
`git diff --check` passed. Ignored `build-runs/lm-r2-doccheck.py` checked four documents, 18 local file
links and the lane-row format. Ignored `build-runs/lm4-r2-literals.py` executed exact rational scalar
arithmetic for binary32 seam/UV/bounds/environment words and integer masks only. This is analytic
calculation, not execution of a native builder, device or implementation. Proposed native vectors remain
**UNEXECUTED**. No cargo/clippy/ctest/build/game/shadow work was applicable or performed.

LM4 and LM7 moved to `docs/specs/questions/answered/lot-models.md` marked ANSWERED 2026-10-23 DIFFERS.
LM1 (upstream authors/global slot46/other mode bytes), LM2 (complete inherited/lifetime inventory and
failure paths), and LM8 (unordered/overflow/invalid numeric domains) remain open. No new question IDs.
No native/runtime/default/full-owner/byte-coverage promotion, ABI exports/rows/version changes or
engine/tests/private implementation reads. No proposed ABI file. The specific brief forbids queue writing,
so no merge-queue file was created. Commits succeeded despite a packed-refs.lock permission diagnostic;
shared refs/config/hooks were not modified. Next step is coordinator review of this bounded publication.

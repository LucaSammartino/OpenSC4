| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-sprites-contracts4: animated-sprite native contracts | `re-sprites-contracts4` (RE docs lane, detached round 3) | DOCS READY: finite AS6 schedules; 72 rational cases, links/questions/diff passed; AS8 answered DIFFERS; ABI unchanged; no native/game runs | Coordinator review/land bounded docs; retain AS1/2/3/6/7/9 residuals and untouched AS4/5. No automatic fourth round. |

## Report

The original final round adds substantive finite SetOrientation contracts to
[animated-sprite.md](../specs/animated-sprite.md), §§3.1.1–3.1.4. **DOCS READY** means this bounded
publication can land; it does not qualify the full owner, implementation, native runtime, default mode,
whole-call floating environment or coverage. R1/R2 specs and the historical report below remain retained.

| Milestone | Commit | Result |
|---|---|---|
| First complete partial-mask schedule | 58d1593f | Mask-1 A operations, binary32 versus transient boundaries, snapshots, write order and state |
| All finite branches | 22e43e92 | Shared/composite A/B/D schedules, full-mask comparison, masks 0–7/reset/flags/counter/return table; obsolete LookAt contradiction removed |
| Analytic vectors and reconciliation | c073101f | Conditional R3-M/C/L/Z; 72 exact-rational cases; AS8 DIFFERS archive; AS6/AS9 partial notes and questions index |

### Evidence and measured results

- Canonical re_query func/asm --bytes was used first; the complete 007FC4B0–007FCC69 instruction CFG was read in 56-line pages. No REA calls/session/target repair or decompiled output was needed.
- Main executable FileVersion is **1.1.641.0**, SHA256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, matching the canonical Windows export summary, image base 00400000.
- Native jump table 007FCC6C confirms mask entries 007FC4F9/007FC616/007FC649/007FC852/007FC885/007FC999/007FCBA3. Corrected the earlier non-boundary A/D range start 007FC882 to 007FC885.
- All trig here is hardware FCOS/FSIN; callees query reports **0 calls**. There is no foreign parameter conversion or unknown foreign return format to assume. Primitive results remain conditional hardware transients; they are not host sin/cos, assumed binary32 or assumed double returns.
- Each arithmetic operation, actual B32 store/reload, retained transient, matrix snapshot and write order is specified. B retains a different row's sine product than A/D. Composite stages consume the preceding stage's stored matrix; omitted angles are never set to zero in a full-mask formula.
- Finite mask table covers both initial flag-4 states and counter 255: single-axis counters 0/1, two-axis 1/2, mask 7 counter 0, mask 0 counter 255 with complete bit preservation. Unrelated flags, translation, scale, dirty bytes and cached counter remain untouched; all return 0.
- R3-M derives all 16 mask/flag cases by independent signed permutations at PC24/53/64 RN. R3-C independently derives cancellation by exact integer/dyadic products under all four RCs. R3-L distinguishes retained sine, its later B-stored negative and wrong in-place sources. R3-Z derives conditional signed-zero results.
- Chosen primitive pairs are expressly **conditional stage examples**, with no angle-reachability or native injectable API claim. R2 geometric/radical vectors remain tolerance/proposed UNEXECUTED; no host-trig exact bits were substituted.
- Small setter asm --bytes recheck at 007FB340/007FB3B0/007FB410 supports AS8 DIFFERS with an explicit completing-comparison boundary, including non-closed alpha acceptance, sentinel/disable rules, raw index narrowing, returns and unrelated-bit preservation. No complete lifetime/interface inventory re-audit was attempted.

### Validation and questions

- `build-runs/check-sprite-r3-rational.py`: **72/72 conditional analytic cases passed** (48 mask/flag/PC, 12 cancellation/RC, 6 large/small, 6 signed-zero). This is rational/sign analysis, not original or implementation execution.
- `build-runs/check-sprite-r3-doc.py`: **8 local links passed**, four schedule/vector sections present, obsolete geometric contradiction absent and AS8 archive consistent. `python tools/questions.py index`, `check` and `git diff --check` passed.
- AS8 moved to [answered/animated-sprite.md](../specs/questions/answered/animated-sprite.md), dated 2026-10-23 and marked DIFFERS. AS6 and AS9 retain precise PARTIAL notes in [questions/animated-sprite.md](../specs/questions/animated-sprite.md). Index now records **8 open / 1 answered**; no new question IDs.
- Cargo/clippy/ctest/game/shadow: **n/a**, RE docs role. No engine/tests/private implementation reads; no native/game/build/RPC/process execution, code/ABI/version/map/generated-coverage edits or ABI proposals. Helpers and commit messages stay ignored in own build-runs.
- Commits succeeded on detached HEAD. Git again printed packed-refs.lock permission errors while returning success and creating the listed commits; no shared ref/config repair was attempted. Prior R1/R2 limitations and failure records below remain historical evidence.

### Authority, publication and residuals

Own new manifest `C:/Users/tlori/FreeCity4/build-runs/run4-re-sprites-contracts4-r3-manifest.json`
was verified as lane re-sprites-contracts4, kind re, round **3**, correct worktree and base **7167dba3**.
Copied `build-runs/evidence-run4/coordinator-sprites-final-host-proof.json` and eligibility note establish
full admission/dependency exits 0, expected round 3, original rounds 2/limit 3 and parked false. Runtime
STOP/park/cap/role constraints remain mandatory; measured codexguard status remained within budget.
Historical R2 manifest hashes/global detached refs were not used as new admission requirements.

The round-3 instruction explicitly supersedes the old R2 blanket queue prohibition: the **own** normal
`build-runs/merge-queue/re-sprites-contracts4.json` is permitted/required after the clean final commit,
kind docs, ABI false, no game runs, actual detached HEAD tip. No coordinator/refill/other-lane/shared-state
queue or branch/ref mutation is authorized or performed.

Remaining: AS1 sequence-controller provenance; AS2 usage/names; AS3 exact 60-function/10,482-byte and
generic lifetime/interface reconciliation; untouched AS4/5 qualifications; AS6 whole-call PC/RC provenance,
native angle/numeric qualification and controller boundary; AS7 malformed/nonfinite/whole-call mode;
AS9 inventory reconciliation. Incoming PC/RC and actual hardware trig transients remain parameters of
the local proof. No ambient/default mode, global CRT/trig/callback preservation or startup/thread proof
is claimed. Next action: coordinator review and landing of bounded docs; no automatic fourth round,
budget reset, new owner family, merge/rebase/push or further admission request.

## Previous round-2 report (retained evidence)

Round 2 adds bounded instruction-derived AS6/AS7 text to docs/specs/animated-sprite.md. It does not promote
the complete owner, implementation, runtime, defaults, numeric mode or coverage to verified status.

| Milestone | Commit | Result |
|---|---|---|
| AS7 quad/cache/draw/pick | c588e64e | Exact finite corner equations, box reset/reflected-maximum rule, translation/cache/state-call order |
| AS6 rotation/LookAt | 23a5eb2d | X/Y/Z sign/order, sentinel omission/reset/counters, target/local −Z rule, conditional numeric stores |
| Bounded inventory/questions | 831b3966 | Five owner bodies separated from two thunks/foreign support; fallback/box fixtures; partial questions retained |

### Evidence and measured results

- Canonical re_query/export/asm --bytes was used before supporting local decomp. No REA calls or target/session actions were needed.
- Main bin/SimCity4.exe SHA256 is `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, matching ghidra/export/windows/export-summary.txt; image base 00400000.
- AS6 primary ranges: 007FC4B0–007FCC69 and 007FE250–007FE38D. Two distinct noncommuting fixtures and a mixed −100 fixture use independent literal/radical expectations, not host trig. PC/RC remains inherited and conditional.
- AS7 primary ranges: 007FE400–007FEA84, 007FEA90–007FED4C, 007FD710–007FD7C6; fallback camera support 007FF340–007FF3BE. Lower corners alter Y only; box includes origin/top corners/reflected maximum and resets each refresh.
- Draw skips the boolean highlight-pass argument or signed zoom<2, not stored highlight. Full zoom/rotation compare against cached bytes; lookup uses low bytes. Same-zoom camera/mpp/magnification changes alone do not invalidate. Draw calls no-argument SetModelTransform; no forced render-state restore.
- Fade threshold is binary32(alpha×double 0.5), raw constant checked as 3FE0000000000000. Fade sentinel/threshold constants checked as C2C80000/C3480000/3F7D70A4; retained setter quirks remain in §4.4.
- Corrected unfinished base-constructor citation to 00801CF0–00801D67; checked its default-store range and sprite constructor overrides, GetSortValue/WorldBounds/null-resource body, and the dynamic-displacement Pick/Draw thunks. Full lifetime/interface inventory was not re-audited.
- Export-size inventory: five newly traced bodies total 4,849 bytes, two adjustment thunks total 16 bytes. These are evidence extents, not algorithm-byte/implementation credit. No subtraction from 60/10,482 or complete attribution claim.
- R1 69ee90df/d45820c2/c3132eeb findings were retained. Coarse clock/SetFrame/fade/LookAt/draw/summary prose was reconciled with the detailed rules; successful-prefix load rollback and non-rollback of frame count remain explicit.

### Validation

- Independent exact-rational Q1–Q5 analysis, local-link/citation checks, questions check, and git diff --check passed. The ignored helper is build-runs/check-sprite-r2-doc.py.
- Proposed **UNEXECUTED** vectors: R2-R1/R2-R2 (noncommuting rotations), R2-R3 (sentinels/LookAt), R2-D1 (conditional half ties), R2-Q1–Q5 (raw geometry/shrink/cache/fallback/box omission). Rational analysis is not native execution; R1 vectors were not renumbered.
- Questions index regenerated; generated contents unchanged. No whole question moved to answered. AS6/AS7 partial evidence and exact residuals are recorded in docs/specs/questions/animated-sprite.md; AS1/AS2/AS3–AS9 remain open.
- Cargo/clippy/ctest: n/a, RE documentation role. No engine/tests/private implementation reads, code/ABI/version/map/coverage changes, game/native runs, ABI proposals or shared-state edits.
- Git commits succeeded on detached HEAD. Git printed a packed-refs.lock permission error during each commit, but returned success and created the commits; no ref/config repair was attempted.

### Authority and residuals

The own isolated manifest is C:/Users/tlori/FreeCity4/build-runs/run4-re-sprites-contracts4-r2-manifest.json:
lane re-sprites-contracts4, kind re, round 2, correct worktree, base c3132eeb. Copied host admission is
build-runs/coordinator-sprites-r2-host-proof.json (full guard/dependency ancestor exits 0, unparked, expected
round 2); runtime budget/STOP guard remained active. The historical user-pause STOP is not current authority.

WIP residuals: AS1 controller provenance (+0x34 remains null in constructor/load), AS3 full 60-function/10,482-byte
inventory and complete lifetime/interface reconciliation, exhaustive partial-mask arithmetic-store transcription,
whole-call PC/RC provenance, and malformed/nonfinite input outcomes beyond the stated branches. AS2 shipped
usage/names remain excluded. No blanket AS1/AS3–AS9 completion or native/default/full-owner claim is made.

Next action belongs to the coordinator: review the bounded spec commits with these residuals. No automatic
third/fourth round, reset, self-restart or further RE admission is requested. No merge-queue file was written:
the task-specific queue prohibition overrides the generic lane finish template.

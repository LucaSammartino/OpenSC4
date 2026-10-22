| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lighting-contracts4: LT10 native projection contract | Codex / `re-lighting-contracts4` (RE docs, detached supervisor checkout) | ready; LT10 answered; arithmetic/doc checks PASS; ABI unchanged; cargo/clippy/ctest and game n/a | Implementers reconcile §5.8 and vectors 18–22; LT1–LT4 remain outside this round. |

## Report (2026-10-22, round 2)

Resolved only the residual LT10 projection question from detached base `9a236cfd`. The lighting spec
now specifies the complete native entry in §5.8.1–§5.8.4: model-frame direction, basis arithmetic,
mixed-precision accumulation/cofactors/coefficients/lifting, inverse fields, all flag-byte cases,
composition spills, and column-major sixteen-float output. The individual checklist marks this entry
proved; the existing owner/helper inventory and six earlier answered contracts are retained.

The direction uses a flag-4 conditional transpose and the negated manager direction, without translation,
scale or normalization. Composition uses the entire byte being nonzero. Bit 2 selects reciprocal scale
on the nine linear output components, after inverse translation preparation; constants are not scaled
again. Unknown nonzero flag bits still enable composition. Register sums and binary32 memory sums are
distinct, and the first RHS product stays unspilled while the second spills. U and V have different
coefficient, lift and translation stores. A generic symmetric least-squares solve is insufficient.

Four new independent specification vectors (19–22) distinguish these contracts:

- Direction flags 0/2 versus 4/6 give d=(0,2,1) versus (0,2,−1), with exact basis values.
- Six output cases for flags 0, 1, 2, 4, 6, 128 distinguish the whole-byte gate, transpose, inverse
  translation and reciprocal scale, using a triangle with an independently fixed affine fit.
- Product rounding changes a noncollinear triangle's determinant from exact 1 to native −1; native
  returns false with untouched output. All arithmetic in the discriminator is exact before stated spills.
- Identical UV columns give translation constants **−16777215 for U** and **−16777216 for V**, because
  only the U translation dot product rounds before adding its fitted constant.

LT10 is marked **ANSWERED / DIFFERS** in `docs/specs/questions/answered/lighting.md`; its active entry
was removed and `docs/specs/QUESTIONS.md` regenerated. LT1–LT4 remain outside this brief. No new question,
ABI proposal or required foreign primitive output was introduced. This resolves a spec gap, not portable
implementation coverage or measured runtime/shadow exactness.

## Evidence and checks

Primary evidence: all **713 instructions / 2,964 bytes** of `007D8C80–007D9813`, the attributed basis
helper `0080B9F0–0080BAC6` and owner getter `007D6B50–007D6B65`, from the local original Windows export.
Every numeric stage cites instruction addresses in the spec. PE data independently confirms binary32
0/1 constants and the binary64 0.01 threshold. Decompilation was navigation only; no Mac-derived behavior,
REA call, executable invocation, game run, network request or original-method provider was used.

Scratch checks in ignored `build-runs/` passed: `trace-lt10-rounding.py` symbolically audits native
accumulation/cofactor/lift/composition stacks; `check-lt10-r2.py` checks independent rational arithmetic,
flag matrices and PE constants; `check-lighting-docs.py` checks inventory/accounting and question/vector
consistency. Question-index regeneration is idempotent and `git diff --check` passes. Cargo, clippy,
ctest, game and shadow checks are n/a for this docs-only change. No measured game verdict is claimed.

Substantive commits: `4d1e3140` direction proof/vector; `69675480` complete arithmetic, flags, layout,
vectors and LT10 reconciliation. Only spec/question/index/lane documentation is tracked in this round.
No engine files were read or edited; tools/maps/ABI tables are unchanged. The reserved detached checkout
was retained. Git emitted a packed-refs lock warning, but commits succeeded and HEAD was verified.
The q.py Bash wrapper failed with E_ACCESSDENIED; capped PowerShell/Python queries succeeded instead.

## Next steps

Implementers can reconcile their portable partial projection with §5.8 and vectors 18–22. Runtime
conformance and any later owner-coverage claim require implementation-side evidence. No further RE
round, self-restart, branch/ref update, merge or push is requested by this completed LT10 task.

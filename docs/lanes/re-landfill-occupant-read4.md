| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landfill-occupant-read4: occupant nested read and position return contracts | Codex / `re-landfill-occupant-read4` (RE docs lane; detached HEAD) | ready; evidence `2b3b575f`; LG32/LG33 answered; questions/literal/docs/diff checks pass; cargo/clippy/ctest n/a; no new game runs or C ABI changes | 1. Coordinator docs review and pending ABI annotation review. 2. Preserve LG31 and native differences; no automatic extra round. |

## Result

First original research round complete, analysis/docs only. Pinned starting commit: `b8158f9a`
(`7a081829`'s parent; main advanced concurrently). Scope was LG33 occupant/base nested-read propagation
and LG32 concrete inherited-interface position return. Evidence milestone: `2b3b575f`; initial handoff: `7a081829`.
No engine source read or edited, no native replay, code-address calls, implementation expected-value changes,
comparator changes, shared ABI edits, C ABI exports or coverage/state changes. No extra research round requested.

## Published inventory

- [landfill §6.4](../specs/landfill.md): exact four-layer call/order/return/error contract, early-exit and
  partial-write state, no resynchronisation, literal 39/57 misalignment explanation, historical DIFF retained.
- [landfill §7.3](../specs/landfill.md): actual occupant slot 9 body and caller obligations; typed position/bounds
  effects separated from residual EAX/AL and SDK Boolean observation.
- [foundations §10.3](../specs/foundations.md): retain strict exemplar-to-plain propagation; explicitly identify
  occupant-base exception and zero-initialised partial key processing before stream-error gating.
- [lot-records §2.2–§2.3](../specs/lot-records.md): qualify common nested-read conventions and record the common
  base's ignored holder result without inventing a derived-class rule.
- [literal vectors](../specs/landfill-occupant-read-vectors.md): 18 proposed rows: 8 complete/version inputs,
  8 primitive-error/exact/truncated inputs (including repeated controls), 2 position/return inputs. Includes full
  independent 57-byte input, initial state, ordered primitive calls, exact consumed offsets/error, retained and
  modified state, raw f32 words and partial expectations. Five shorter-prefix cases and pre-existing sticky error
  are separately marked as limited by unread stack destinations. Proposed state is not new native acceptance.
- [open questions](../specs/questions/landfill.md): only LG31 remains, verbatim.
  [Answered archive](../specs/questions/answered/landfill.md): LG32 and LG33 moved with original measurement text
  intact and individual MATCHES/DIFFERS answers. Uninitialised/reused-temporary cases marked PARTIAL.
- `docs/specs/QUESTIONS.md`: required index regenerated; landfill open 3 → 1, answered 36 → 38;
  total open 440 → 438, answered 1239 → 1241. No other index rows changed.

## Primary findings

LG33: landfill tests base AL (`0x00477588`–`0x0047758F`); base ignores exemplar AL and reads stream error
(`0x005E8DD8`–`0x005E8DED`); exemplar tests plain AL (`0x005FF454`–`0x005FF456`). Exemplar version 1
consumes only bytes 16–17 without setting stream error. Base therefore returns true; landfill reads its 21-byte
tail starting at 18 and returns true at cursor 39, error 0. Static state is tile 2, orientation 0, position words
`00000000/00000000/00100000`, empty false; no new native state scoring. Plain bad version instead resumes at 20
and ends at 41. Outer/base wrong versions stop at 2/4. Stream error prevents the landfill tail when base returns
false; a tail error is tested only after empty assignment and bounds update. Key processing can clear a prior
parent even on a short key because its words start zero and the error check comes afterwards.

LG32: slot 9 at table `0x00A8CDA0` targets `0x00477600`; original PE data independently checked. The body
copies float32 words, computes float32 bounds, and leaves EAX = z input bits from `0x0047760F` through ret 4
at `0x00477657`. For `(88.5,-4,120.25)`, EAX `42F08000`, AL `00` explains the SDK false. Treat as void,
do not require true. Creation caller `0x006BC8F4` proceeds to Init without testing a return. Table's ported
PowerLine name is not the concrete owner: landfill constructor installs it at `0x0047728A`. No general return
claim for other occupant bodies. Literal constants 8/16 and their arithmetic widths/order were instruction/data checked.

## Validation and historical observations

- `python tools/questions.py index`: generated required local index; `questions.py check`: PASS.
- `python build-runs/check-landfill-occupant-docs.py`: PASS for 57 bytes, misaligned words/cursors,
  typed float outputs, links/fences in all four changed specs; checks LG31 unchanged and original LG32/LG33
  observations preserved verbatim in the archive.
- `git diff b8158f9a HEAD --check`: PASS at evidence tip; final report checked before commit. Scope inventory
  against that pinned base contains exactly the eight docs files listed above (including this lane report).
  Worktree clean at
  completion. Cargo/clippy/ctest n/a for RE documentation. Broad instruction-doc checks were not run because they
  read unrelated or role-excluded documents. Passing metadata/literal checks do not establish native acceptance.
- No new game labels. Historical `vlfr4-r2-on-reload-2032-20261022a` and
  `vlfr4-r2-normal-diagnostic-20261022b` remain as filed: S0 position/bounds/57-byte agreement, SDK position false
  unscored; LG33 native true/39/57/error0 vs Rust false; outer/base/final-byte rejection agrees; normal clean/ref0.
  Own-output reload 4/4 and scalar/record comparison 8/9 do not resolve nested propagation; no rescoring.
- Source access: export queried first, then paged `asm --bytes`; only named class decompilation slices consulted.
  REA reported `target_unavailable`: “No app is open. Ask the user which app to investigate, then call open_binary
  with its local path.” Export/decomp fallback used; no app opened or retry loop. q.py Bash wrapper returned
  `Bash/Service/CreateInstance/E_ACCESSDENIED`; capped PowerShell/Python used instead.

## Pending review and limits

`build-runs/abi-rows-re-landfill-occupant-read4.tsv` contains exactly one pending replacement annotation in
`docs/abi/vtables.tsv`'s format (table `00a8cda0`, slot 9, target `00477600`). It retains extractor vocabulary
`eax-or-void`, adds concrete method/return notes and high confidence; no shared table edited. This is an ABI
documentation refinement, not a C export change. Coordinator should review and reconcile this row separately.

Unread version/count/byte temporaries prevent deterministic expectations in several primitive-failure cases;
no promise of finite failed-count processing or portable empty value is made. Underlying stream atomicity is
the existing foundations contract, not a fresh proof of every stream transport. Numeric worked vectors assume
round-to-nearest; no runtime FPU environment was measured. Nonzero partial parent keys require a controlled
resource fixture for an exact lookup outcome. LG31 public zone lifecycle and unrelated unknowns stay excluded.

Next: coordinator reviews/lands docs and pending ABI annotation. Any implementation or new native acceptance
work requires a separately assigned lane; do not silently fit existing verification to these deductions.

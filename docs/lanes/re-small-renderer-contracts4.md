| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-small-renderer-contracts4: native renderer owner contracts | Codex / `re-small-renderer-contracts4` (detached worktree) | wip: PARKED final round 3/3; TB4/GT4/FF3 answered; checks passed | Preserve worktree/evidence; FF2 exact residual below. No fourth round or blanket owner credit. |

## Report

Coordinator publication review (2026-10-22): accept the already committed TB4/GT4/FF3 answers and bounded inventories for the train, after checking the published scope and clean tree. FF2 remains open exactly as reported. This is publication of existing findings, with no additional RE, whole-family credit, fourth round or worktree removal.

Final round 3/3 resolved the texture/glyph inventory residuals and flame sampler numeric boundaries. Committed changes are confined to specs, exact question files/index and this lane report. No engine, tool, map, ABI-table or game changes.

### Commits and native results

- `2a6bf9f4`: substantive area proof and exact texture 58/4883 reconciliation.
- `1ebf0d2e`: glyph 43/4475, private tree insertion, flame placement stores, historical flame 16/3513 membership; TB4/GT4/FF3 answers.
- `396c9083`: complete texture address table, seven fire adjustors, native PC24 qualification and distinguishing exponent-range vector.
- Prior rounds retained: `bea42a5f` lifecycle; `a2e1f920` material/PA4; `b0bdd07a` glyph lifetime/flame geometry/FF4; `f9d0d03d` prior report. Earlier answers were not repeated.

Texture-bindings §3.2: **53 class bodies/4749 + five free functions/134 = 58/4883**. The free functions are four class/create helpers and NextPowerOfTwo. Folded entries/9 bytes and named data belong to different scopes. Every class-body address/size is enumerated. Prior material release/copy/assignment and manager slots 6/7 contracts remain intact.

Glyph-textures §3.4: manager **30/3938**, glyph info **8/324**, string info **5/213**, total **43/4475**. This includes two private tree helpers and four remote thunks. Two of those thunks (0080BCE0/0080BCF0) actually jump to ordinance QI/Release and enter through neighbor attribution; they are not manager methods. Duplicate tree insertion preserves the first value/count and takes no helper-level glyph AddRef; font IDs compare unsigned.

Fire-flames §§2.2–2.3: exact area subtraction origin/asymmetric spills, cumulative float totals, separate normal spills and full barycentric/normal-offset/matrix/scale/translation store sequence. Native startup **0045236C → 005F85E0 → 008929E0** sets normal main-thread precision to **24 significant bits**. Each arithmetic instruction rounds accordingly, but x87 retains a wider exponent until explicit float-store conversion. Caller/thread exceptions are cross-referenced to foundations §18; no unconditional double or 80-bit-significand contract is claimed.

Independent arithmetic vectors:
- Twice-areas 2²⁴,1,1 give stored cumulative totals 16777216 three times; one final rounding gives 16777218.
- At PC24, a third matrix-row result of 2¹²⁹ overflows its float copy, but retained-register scale 0.25 yields finite final z=2¹²⁷. Reloading that copy leaves infinity.
- At explicitly PC53, a cancellation vector gives z=0.5 versus 0 with float reloads. Normal PC24 gives 0 for that same vector.
- Duplicate glyph keys and font IDs 0x7FFFFFFF/0x80000000 distinguish insertion/reference and unsigned-order rules.

Fire-flames §4.4: **16/3513 = three flame bodies/555 + thirteen fn:GetPointsOnModel reachability helpers/2958**, confirmed in two archived owner TSVs. That scope excludes the actual sampler and flame constructor. Current attribution differs (fire occupant 35/6518 plus flame 3/555). Seven QI/deleting-destructor/read/write adjustors are independently proved and do not turn destruction into Shutdown.

### Questions and exact parked residual

Answered **TB4** in texture-bindings §§3.1–3.2; **GT4** in glyph-textures §§1.1.1/3.1/3.3–3.4; **FF3** in fire-flames §§2.1–2.3. Entries moved to the corresponding docs/specs/questions/answered files and index regenerated. No new IDs. Prior TB5–TB7, GT3/GT5–GT6, FF4 and PA4–PA8 answers preserved; no repeated path-owner work.

**FF2 remains open/PARKED after final round 3/3 in docs/specs/questions/fire-flames.md.** Inventory scope is reconciled. Complete operand/byte contracts remain for twelve non-triangle reachability helpers: 0046FE80/59, 0046FEC0/249, 004702B0/114, 00470330/119, 004703B0/61, 004703F0/54, 00470430/678, 004706E0/233, 004707D0/220, 004708B0/667, 00471070/45, 004710A0/41; also complete inherited-occupant shared-slot membership beyond the seven proved wrappers. Exact scope is preserved in fire-flames §4.4. The 21 lifecycle/visual bodies and sampler/triangle proofs are published; no whole-family completion claim.

Pre-existing FF1, TB1/TB2 and GT1/GT2 are outside this round. Preserve this worktree and evidence. **No fourth round, restart, merge or push.** Queue is wip because FF2 is incomplete; supported documentation can be reviewed independently.

### Validation and evidence

Passed: questions index/check; full-round diff whitespace check; independent arithmetic vectors; all three published address-table unique memberships/counts/byte sums; duplicate archived flame totals. Cargo/clippy/ctest/game/shadow: n/a, docs only. C ABI exports unchanged; no ABI proposal file.

Primary native sources: main ghidra/export/windows instructions/body sizes, selected decomp ranges, re/names_windows.tsv and docs/abi tables. Main build-runs/coverage-owners-base.tsv and coverage-owners-occprops.tsv establish historical membership only; numeric behavior is proved from instructions. REA was not needed/called; no error/fallback.

Ignored local evidence: build-runs/audit-renderer-scope.py, scope-{texture,glyph,flame,entry}.tsv, check-round3-vectors.py, prior inventory TSVs/scripts. No scratch code committed. Git emitted a packed-refs.lock permission diagnostic during housekeeping, but detached commits succeeded; no shared-ref/config repair attempted.

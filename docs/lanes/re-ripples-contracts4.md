| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ripples-contracts4: exact ripple and selection contracts | Codex / `re-ripples-contracts4` (RE spec lane; reserved detached worktree) | continue: guard context limit; WR4-WR6, WR9-WR10 answered; WR7-WR8 partial; docs checks passed; ABI unchanged; no game runs | 1. Coordinator reviews committed bounded result and residuals. 2. No merge-queue file written. |

## Handoff
- Goal: exact native WR4-WR10 behavior contracts, spec text only.
- Done: 3b970a37 WR5; 33ace687 WR4/WR6/WR9/WR10; c7cfb5ae partial WR7/WR8; 613177cf final report/inventory and signed IDIV correction.
- Exact next step: coordinator reviews remaining WR7/WR8 obligations in report below; do not assume another round is authorized.
- WR7 residual: dynamic plain black RGB initialization, hash allocation/rehash submission history, NaN height operand selection.
- WR8 residual: complete inherited/public API/signatures/attribution, Cleanup/destructor/refcount, tuning absence/type/ownership/reset, texture acquisition failure.
- Native address-window inventory 69 bodies/15276 bytes is not the brief's 65/15308 owner attribution; no whole-owner credit.
- Native findings and vectors: docs/specs/water-ripples.md; five dated answers moved to questions/answered/water-ripples.md; WR7/WR8 remain open with partial notes.
- REA failed target_unavailable/No app is open; export, decomp and executable-data fallback used.
- Last successful checks: questions index generated, git diff --check passed; no engine tests/game runs or ABI changes.
- Guard denied final queue command at context 150k; it never executed. No merge-queue file was written. Stop with continue as instructed.

## Report (2026-10-22)
- Goal: resolve WR4-WR10 from native export/decomp/instructions; spec text only.
- Done: 3b970a37 answers WR5; native WR4/WR6/WR9/WR10 contracts now in §§1.2,1.5,2.2, with vectors 12-15.
- Done: 33ace687 answers WR4/WR6/WR9/WR10; WR7 drawing subset and WR8 direct lifecycle facts added, questions remain open with narrowed residuals.
- Remaining: WR7/WR8 native proof below; this guarded round is finished, no restart requested.
- Evidence roots: C:/Users/tlori/FreeCity4/{ghidra/export/windows,decomp,bin/SimCity4.exe}.
- Questions: docs/specs/questions/water-ripples.md; target docs/specs/water-ripples.md.
- Constraints: no engine reads, no ABI table edits, no runtime/coverage claims, no subagents.
- q.py Bash wrapper fails with WSL E_ACCESSDENIED; use bounded PowerShell commands directly.
- REA batch 0074b570 failed target_unavailable (No app is open); no session changes, export fallback.
- Native scan: ripple range 0075fb20..00762100 = 43 bodies/9374 B; selection 007505c0..00751c80 = 24/5635 B; reconcile owner attribution later.
- WR5 differences: partial outer survives inner spacing rejection; lower edge equal accepted; 1002 total pairs maximum; fade denominator configured count.
- New differences: period bounds [122499,227499); raw 64-bit RNG product participates; single frame step/discard overshoot; strict >2M delay; all clear-mark overloads refresh preference; packed b3=low colour byte.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-ripples-contracts4-r1-manifest.json.

### Result and evidence

Substantive commits: `3b970a37` (WR5), `33ace687` (WR4/WR6/WR9/WR10), `c7cfb5ae` (WR7/WR8 bounded facts).
Changed only public ripple spec, its question/answer files, and this lane report. Questions index regenerated
after each milestone; it produced no tracked content change. No ABI proposal, runtime/default/coverage claim,
engine read, external communication or game run. `git diff --check` passed and tracked status was clean after
the third milestone. Final citation audit corrected AddShore conversion to signed IDIV, not unsigned division.

Primary evidence: local Windows export/instructions and executable data; local decomp read by function ranges.
REA `batch_decompile([0074b570])` returned `target_unavailable`, "No app is open"; no REA session changes or
additional tools. The export instructions expose both trace callees despite the incomplete pseudocode.

Independent distinguishing vectors are spec §6 items 9–15: pending outer with later inner; duplicate first u;
1002-pair split; short-ramp denominator/byte conversion; equal-water trace start rejection; one-frame overshoot;
native [122499,227499) periods; out-of-range key aliases/packed b3; area and strict-delay boundaries/counter wrap.
These are derived native contract vectors, not measured game outputs or comparisons against implementation.

### Exact bounded native-body inventory

This is an **address-window inventory**, not a complete owner attribution: 69 bodies / 15276 bytes, comprising
44 / 9438 bytes in `0075FAE0 <= address < 00762100` and 25 / 5838 bytes in
`007504F0 <= address < 00751C80`. Export `functions.tsv` body_bytes supplies the lengths. The task's
65 / 15308 owner attribution has not been reconciled; shared/inherited/folded functions can lie outside these
windows and templates inside them are not necessarily owner methods. No whole-owner completion claim.

The following inventory enumerates every body in those windows. A = a contract/evidence path was examined and
its relevant proved behavior or explicit residual is in the spec; R = storage/helper path not fully established.
**A is not a claim that every instruction or every input case of that body is proved.** In particular the
tuning, Init/failure, lifecycle and plain-colour obligations remain open. There are 22 A ripple bodies / 7508
bytes and 20 A selection bodies / 5265 bytes; 27 R bodies / 2503 bytes, in addition to the stated residual paths
inside A bodies. These counts cannot be used as implemented or completely proved owner-byte counts.

| Ripple body VA | Bytes | Native role / status |
|---|---:|---|
| 0075FAE0 | 64 | QueryInterface; A, inherited interface path still WR8 |
| 0075FB20 | 19 | process tuning reset callback; A, §2.4 |
| 0075FB40 | 106 | rectangle/strip contact; A, §1.5 |
| 0075FBB0 | 130 | Shutdown; A, §2.4 |
| 0075FC40 | 550 | spline positions; A, §1.3 |
| 0075FE70 | 110 | strip draw; A, §1.4 |
| 0075FEE0 | 384 | frame/period/alpha preparation; A, §§1.3/1.5 |
| 00760060 | 719 | tuning; A, numeric period proved, absence/type residual WR8 |
| 00760330 | 328 | Init; A, direct facts §2.4, failure residual WR8 |
| 00760480 | 294 | vertex-vector assignment; R |
| 007605B0 | 123 | strip copy construction; R |
| 00760630 | 115 | strip-list copy/assignment; R |
| 007606B0 | 59 | remove last strip; R, call-site role in §1.3 |
| 007606F0 | 80 | clear strip list; R |
| 00760740 | 74 | strip-list destruction; R |
| 00760790 | 87 | shore destruction; R |
| 007607F0 | 216 | remove intersecting strips; A, §1.5 |
| 007608D0 | 85 | contact-rectangle storage growth; R |
| 00760930 | 127 | contact-rectangle assignment; R |
| 007609B0 | 1112 | grouped contact/render bounds; A, §1.4 |
| 00760E10 | 193 | terrain edit; A, §1.5 |
| 00760EE0 | 292 | StartNewRipple; A, normal path §1.3, failure WR8 |
| 00761010 | 303 | shore bounds; A, §1.4 |
| 00761140 | 1099 | full candidate traversal; A, §1.3 |
| 00761590 | 149 | shore copy construction; R |
| 00761630 | 173 | shore-vector replacement; R |
| 007616E0 | 47 | shore copy range; R |
| 00761710 | 67 | shore destroy range; R |
| 00761760 | 42 | shore-vector destruction; R |
| 00761790 | 129 | constructor; A, §2.4 |
| 00761820 | 8 | base-adjustment thunk; R |
| 00761830 | 8 | deleting-destructor adjustment thunk; R |
| 00761840 | 8 | message-interface adjustment thunk; R |
| 00761850 | 8 | message-interface adjustment thunk; R |
| 00761860 | 8 | message-interface adjustment thunk; R |
| 00761870 | 52 | shore-vector clear; R |
| 007618B0 | 276 | shore-vector allocation/insertion; R |
| 007619D0 | 40 | deleting destruction; R |
| 00761A00 | 363 | index-to-control-point copy; A, §1.2 |
| 00761B70 | 383 | seed enumeration/visit clearing; A, §1.2 |
| 00761CF0 | 381 | rebuild and alpha parameters; A, §1.3 |
| 00761E70 | 264 | tick scheduling/frame arithmetic; A, §1.5 |
| 00761F80 | 107 | message dispatch; A, §1.5 |
| 00761FF0 | 256 | lazy build/state/zoom/frustum; A, §§1.4/1.5 |

| Selection body VA | Bytes | Native role / status |
|---|---:|---|
| 007504F0 | 203 | rotation/mirror UV mapping; A, §2.3 |
| 007505C0 | 57 | Shutdown; A, §2.4 |
| 00750600 | 406 | four render states; A, §2.3 |
| 007507A0 | 117 | float4 comparison shared helper; R |
| 00750820 | 311 | plain corner positions; A, §2.3, NaN residual |
| 00750960 | 319 | textured corner positions; A, §2.3, NaN residual |
| 00750AA0 | 188 | Init; A, §2.4 |
| 00750B60 | 83 | textured record copy; R |
| 00750BC0 | 111 | textured table find/insert; R |
| 00750C30 | 102 | plain table find/insert; R |
| 00750CA0 | 67 | ClearCurrentSelections; A, §2.2 |
| 00750CF0 | 713 | textured draw; A, §2.3, ordering residual |
| 00750FC0 | 1324 | plain draw; A, §2.3, dynamic palette/ordering residual |
| 007514F0 | 76 | Update/depth restore; A, §2.3 |
| 00751540 | 160 | textured table insertion; R |
| 007515E0 | 97 | plain insertion/update; A, §2.2, table allocation residual |
| 00751650 | 126 | textured insertion/update; A, §2.2, table allocation residual |
| 007516D0 | 162 | list marking; A, §2.2 |
| 00751780 | 388 | signed inclusive rectangle marking; A, §2.2 |
| 00751910 | 136 | plain cell marking; A, §2.2 |
| 007519A0 | 208 | palette textured marking; A, §2.2 |
| 00751A70 | 321 | packed-colour textured marking; A, §2.2 |
| 00751BC0 | 101 | constructor; A, §2.4 |
| 00751C30 | 8 | deleting-destructor thunk; A, direct path only |
| 00751C40 | 54 | deleting destructor; A, direct path §2.4, base/reference residual |

Additional distinguishing native bodies outside those windows: trace wrapper `0074B570` 104 bytes, forward
`0074AD20` 453, reverse `0074AB60` 426 (983 bytes); RNG `00913696` 27, bounded `009136B1` 25, range
`009136CA` 21 (73 bytes). Data citations are in the spec; no decompiled output or assembly listings committed.

### Remaining work and limits

- WR7: finish dynamic plain black RGB source, native hash allocation/rehash history sufficient to reproduce
  submission order, NaN height operand selection. Normal finite geometry, UVs, indices, state arguments and
  binding cache behavior are now specified, but full render equivalence remains unclaimed.
- WR8: complete owner/public/inherited API signatures and attribution, Cleanup/destructor/refcount routes,
  tuning absent-property/type/array ownership/process-reset defaults, resource acquisition failure. Direct
  constructors/Init/Shutdown differ materially from provisional safe core policies; do not promote by inference.
- WR1-WR3 remain separately open. No additional question IDs filed. Five entries moved intact to
  `docs/specs/questions/answered/water-ripples.md` with dated MATCHES/DIFFERS rule and address references.
- This is one guarded round; queue state WIP so the coordinator can review/publish bounded evidence. No merge,
  restart, push, ABI changes, C API export changes, ABI proposals or engine validation were performed.

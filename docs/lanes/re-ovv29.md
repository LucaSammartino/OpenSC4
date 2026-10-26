| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ovv29 / overlay owner contracts (kind docs) | Codex / .claude/worktrees/re-ovv29 / reserved branch re-ovv29 (detached HEAD) | ready; content commits 533ce1bea, 2ee2427d3; doccheck/questions/diff checks pass; native inventory 54/54, slots 44/44; game n/a; shadow n/a; C ABI unchanged; OVV4 narrowed, OVV6/NOV2/NOV3 answered | 1. Review/land docs. 2. Import proposed vtable rows if desired. 3. Observe the OVV4 inline-reset watchpoint if that operation is required. |

- overlay-views §§2.3, 3.2–3.5 publish native mutations, the 54-address checklist, full primary/secondary interfaces and a lifetime case.
- network-overlay-views §§1.1–1.2 prove loader-zero pulse, folded depth/Shutdown and full-u32 inline filter mutation; first Update uses pulse 5.
- network-overlay-views §§2.1–2.2 prove hover/pressed cell producers, marker basis/0x38-byte state, model submission order and observed Shutdown discipline.
- OVV4, OVV6 moved to docs/specs/questions/answered/overlay-views.md; NOV2, NOV3 to docs/specs/questions/answered/network-overlay-views.md; index regenerated.
- OVV4 is bounded: no disable entry in the recovered owner/tables; an external inline reset remains a precise runtime observation, not a claimed absence.
- Observe a live P+0x44 nonconstructor 1→0 write, writer PC/stack, then a normal-only Update with surviving nonempty cells and prefer-opaque=false (overlay-views §2.3).
- Worked colour/pass, pulse/state, interaction/marker and reference-lifetime cases are static and UNEXECUTED. No owner/runtime/replacement promotion is claimed.
- Gate: python tools/doccheck.py check PASS; python tools/questions.py check PASS; git diff HEAD~2 --check PASS before report commit.
- Evidence check: build-runs/ovv29-check.py PASS (54-address ledger: 49 named + 5 unnamed; original vtable words 44/44; pulse loader zero-fill).
- Earlier diff check caught two trailing blank lines after archival; corrected before the final passing check and amended content commit.
- Proposed docs/abi/vtables.tsv-format rows: build-runs/abi-rows-re-ovv29.tsv (44 rows). docs/abi/ was not edited; no C ABI exports changed.
- Sources: local read-only export queries, decomp navigation and original PE data; REA was not needed.
- No engine/ files read or edited. Cargo/clippy/ctest and game runs n/a (docs-only lane).

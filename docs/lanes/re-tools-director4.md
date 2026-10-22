| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tools-director4: tools COM director and service boundaries | `re-tools-director4` / (RE spec lane, detached worktree) | ready docs, partial owner: inventory70/31293 B; D/S boundaries36/3320 B; decoder gaps33/27952 B; artifact1/21 B; 25 vectors; TD3 answered; TD1/TD2/TD4 open; docs audits PASS; no ABI/game runs | 1. Review TD4 attribution correction. 2. Resolve TD1/TD2 before whole-owner credit. No further round requested. |

## Report (2026-10-22, round 1)

Added docs/specs/tools-com-director.md, scoped to cGZToolsCOMDirector and its two
registered service boundaries. Read re-application4's report to avoid duplicate
cSC4App/automata claims. The brief's 31293B is a byte total, not a class-name suffix.

Exact current attribution: 70 unique bodies / 31,293 B, all listed in the spec.
Checklist D=6/294 B, S=30/3026 B, U=33/27952 B, artifact=1/21 B. D/S document
boundaries and dependencies, not complete nested decoder arithmetic. Whole-owner
and whole-application credit are withheld. No coverage-map edits are proposed.

Specified: singleton construction/reentrancy/unwind/exit cleanup; QI faces and
native pointer adjustments; reference counts; both class factories and registrations
(3CEC2B47/9CEC2B58); enumeration callbacks/ignored results; InitializeCOM and unusual
CanUnloadNow; all nine lifecycle hooks (constant true, no side effects); every
registered-service slot; buffer/file/decoder ownership; open failure retaining active;
unchecked allocation-null decisions; creation ignoring open failure/leaking on failed
QI; ID3 skip rules; PCM delivery boundary; native header tables/parser; input errors;
instruction-verified integer/float operation order and uninitialized format field.

Evidence: main checkout executable, exported function metadata and instructions;
named director pseudocode used for navigation only. Vtables and constant tables
were read from PE bytes. Largest decoder bodies were inspected, with remaining
algorithm/rounding contracts explicitly open. No implementation/test output is
contract evidence. REA was not needed; no external/network activity.

TD4: alleged 00420036/21 B is a false numeric-table reference. Native consumers
00A68D6F and 00A6935B-00A693B0 read signed 16-bit entries54/66 at 00AF5218;
their packed dword accidentally points inside another instruction's immediate.
Coordinator review requested; excluding artifact gives69 candidates/31,272 B.
No attribution/export/tool/map changes were made.

Questions: TD1 (33 U bodies: internal header parser, bit unpacking, float/MMX
transforms and exact output rounding); TD2 (interface names and invalid _ftol2
conversions); TD4 (attribution correction) in questions/tools-com-director.md.
TD3 answered DIFFERS: native unwind clears singleton flag and destroys completed
base, permitting retry; rule in section2, archive answered/tools-com-director.md.
QUESTIONS.md regenerated with three open entries and one answered entry.

Validation: exact inventory audit PASS, vector audit PASS (TD-V1..TD-V25 unique
and contiguous), questions.py check PASS, git diff --check PASS. These are document
consistency checks; the25 independent vectors were not run against the game or an
implementation. Cargo/clippy/ctest n/a; no game/shadow/replacement claims.
Scratch evidence/audits are ignored in this worktree's build-runs/. No ABI proposals
or ABI/export/version changes; no engine/tools/maps edits or engine reads.

Milestones: f164d515 (inventory and director contract), 3b3a4120 (service slots and
TD3 answer), final report commit (header/submission rules,25 vectors,TD4,index,audits).
Detached commits are for the supervisor to publish; no ref/branch changes, merge,
push, restart or subagents. Remaining work belongs to coordinator prioritization.

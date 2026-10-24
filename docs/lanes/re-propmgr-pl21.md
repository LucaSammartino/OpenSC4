| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-propmgr-pl21: prop list persistence | `re-propmgr-pl21` / (RE lane, detached HEAD) | ready: PL21 answered; behavior commit `14f012eda`; doccheck passed; no ABI change; no new game runs or shadow exactness credit | (1) Land spec; (2) investigate live/snapshot/list-input differences without excluding timed props. |

## Report

Answered PL21 in prop-lotconfig §A7.2 and §A8; added quirk 20 and §T.17's six prose conformance
vectors PL21-G/U/C/R/W/S. Moved the complete question to the answered archive with the 2026-10-24
DIFFERS disposition and regenerated QUESTIONS.md: prop-lotconfig now has 7 open and 10 answered
entries; total index counts are 599 open and 1423 answered. No new questions or proposed ABI rows.

There is no private per-lot membership collection or timed-prop omission from the save path. Slot 23
passes its caller's list to slot 24. A fresh query draws from the occupant grid, with usage 2 and spatial
tests. The occupant-manager writer references every grid node. The prop-manager record itself has
only version and manager references, so identical bytes there do not establish identical live occupants.
An existing caller entry can survive without grid membership; a later insertion can produce a query
entry absent from an earlier snapshot. These possibilities are explicit conformance cases, not a diagnosis
of the supplied intermittent game failure. No timing/persistence whitelist is justified.

Evidence was queried from the main checkout using re_query.py before reading the relevant decomp
sections. Instructions with bytes verified the slot-23 list forwarding (`0x004A1FDA`–`0x004A1FFB`),
slot-24 grid call and positional pruning (`0x004A3290`–`0x004A3372`), usage-2 filter path
(`0x004A2210`–`0x004A2304`), prop-manager writer (`0x004A2430`–`0x004A2468`) and unconditional grid-node
reference writing (`0x0049EDD0`–`0x0049EE27`). The local read-only PE helper
`build-runs/filter-vtable.py` resolved filter vtable `0x00A8F8E4` slot 3 to `0x004A2210`; it and all
raw source material remain uncommitted. Related contracts checked: occupant-manager §8.2/8.4/13 and
lot-records §10.1. No REA call was needed; no binary was opened through REA.

Validation: `python tools/questions.py index` succeeded; `python tools/doccheck.py check` passed;
`git diff --check HEAD^ HEAD` passed for the behavior commit. Cargo, clippy and CTest are n/a for
this docs-only lane. No engine, plugin, capi, ABI table or coverage-map file was read or changed.
The requested owner closure table resides under engine, which the brief expressly forbids reading;
the supplied PL21 closure requirement and docs/lanes/glue21.md and glue-proplot-shadow5.md reports
were used instead. The Bash wrapper failed with CreateFileMapping error 5; native PowerShell queries
completed successfully. Git printed a packed-refs.lock permission error but successfully created
behavior commit `14f012eda`; no shared refs/config were changed.

No new game runs were requested or performed. Prior reported evidence remains unchanged: glue21-r4-3
219/220, r4-4 220/220 and r4-5 357/358 prop shadows; the brief's train169 run 380 failure and the
glue-proplot-shadow5 turbo failure are not explained or reclassified by this work. The next engine/glue
investigation should compare caller-list input and referenced occupant/grid state at a consistent
observation, preserving every mismatch. This lane closes the behavior question, not that regression.

## Handoff

- Goal completed: PL21 membership contract and six vectors committed in `14f012eda`.
- Checks: questions index, doccheck and behavior-commit whitespace check passed.
- Next: coordinator lands this docs-only result, then resumes the unexplained shadow investigation.
- Facts: no omitted timed-prop class; no new game evidence; no ABI changes or exports.

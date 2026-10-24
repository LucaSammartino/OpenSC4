| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotconf-glue22: capacity and cost contracts | Codex, `re-lotconf-glue22` (reserved detached worktree) | ready; GLUE22 answered; doccheck PASS | Implement capacity host from §4.1; keep cost/occupancy scope explicit |

## Report

Resolved **GLUE22** in [lot-configuration.md](../specs/lot-configuration.md) §§4.1–4.2, with five prose
conformance vectors (15–19). The building-capacity provider is `cISC4BuildingDevelopmentSimulator`,
IID `0x07813EAE`, slot 39 (`+0x9C`, `0x0069CA60`): thiscall with one uint32 building type, pop 4,
EAX borrowed profile address, capacity uint16 at +8. Documented city acquisition, QueryInterface reference
ownership, profile lifetime, unknown-type insertion, slot 40's nullable alternative and configuration
getters' AX return width. Family expansion includes direct valid types even when its return count is 0.

The configuration has no cost getter among its 34 slots. Profile plop/budget costs are per building type;
there is no configuration-family cost aggregation contract. Native profile inputs can measure the manager's
capacity reduction, while independent expected profile capacities must be derived from resource properties.
Actual occupancy requires a lot instance. No previously unmeasured field receives exactness credit here.

Evidence: original export queried first with `re_query.py`, then bounded reads of the main checkout's
manager decompilation, existing building-development spec and building-profile ABI layout. Widths,
calling convention and ownership checked against byte-bearing instruction queries at `0x0048AEE7`–
`0x0048AF04`, `0x0069CA60`, `0x006985C0`, `0x006197E0`, `0x006952A7`, `0x004798E0`, `0x004798F0`,
`0x0069BF29`, `0x0069BDB0`–`0x0069BDCB` and `0x0069BE20`–`0x0069BE3F`; complete configuration
vtable enumerated. No REA invocation needed. No ABI rows proposed or exports changed.

Validation: `python tools/questions.py index` regenerated the questions index;
`python tools/doccheck.py check` **PASS**; `git diff --check` **PASS**. Cargo, clippy and CTest **n/a**
(spec text only). No game runs requested or performed. The landed shadow report's 73/73 exact defined-field
result is prior evidence only; capacity, costs and occupancy receive no new dynamic measurement in this lane.

Question moved from [open](../specs/questions/lot-configuration.md) to
[answered](../specs/questions/answered/lot-configuration.md), marked MATCHES. No new questions filed.
Specification milestone: `5c84d8059` (initial handoff `206aa3502`).

## Handoff

- Complete: GLUE22 spec and question closure committed at `5c84d8059`; validation passes.
- Next: coordinator lands this RE result and routes §4.1 to an implementation lane for the capacity host.
- Continue independent expected capacity derivation from resources; do not copy native profile results into expected values and claim independent derivation.
- No open questions, ABI proposals, game requests or other pending work in this lane.

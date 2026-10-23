| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-poles5: utility owner specifications | `Codex` / `re-netocc-poles5` (RE) | ready: documentation delivery only; NO44 answered, NO36/NO40/UG5 advanced; six conformance cases; doccheck/questions PASS; no ABI or game runs | 1. Land specification additions. 2. Complete NO33/NO43 and remaining NO36/NO40/UG5 before owner closure. |

## Report

This delivery closes **one of six requested questions**, NO44. It does not close the pole/utility owner.
The remaining five questions stay in their open files; partial progress is explicitly marked there.
No engine, plugin, capi, ABI tables or coverage files were changed.

### Published behavior

- `a1302a47`: network §6.1 and test 45 settle pylon construction: UI=1, dispatch state=0,
  dispatch type uninitialized. The base constructor also does not initialize that word. NO44 moved
  to `docs/specs/questions/answered/network-occupants.md`; the question index was regenerated.
- `04c0c750`: network §6.2.2/tests 46–48 specify cached bounds, transform counter/mask semantics,
  texture-ID sorting, UI-gated box picking with strict distance replacement, ordinary texture tables
  and the retained stale texture ID after a failed replacement.
- The same commit adds network §3.6.2/tests 49–50: unattached connection floor/wall generation,
  attached early exits, four endpoint pairs, append versus clear behavior, RGB/alpha/UV rules and
  the quirk that both wall-bottom V values use endpoint A's drop. Underground §4 links this contract.

### Evidence and validation

Read the owner closure README and core-netocc-poles5 report first. Queried the original export before
reading selected main-checkout decomp functions; verified new numeric operations against instruction
bytes. Texture tables were read from the local executable at the cited addresses. No REA call was
needed. The stale `0x00648360` transform label is disproved by its instructions: it queries the pipe
class IID then `0x4A10AFC2`, as underground §4 already specifies. Do not use it as transform evidence.

`python tools/doccheck.py check`: **PASS**. `python tools/questions.py check`: **PASS**.
Six new prose conformance cases (45–50), not executable vectors; executable engine files were outside
this RE lane's write scope. Cargo/clippy/CTest and game/shadow: **n/a**. No ABI exports or proposed ABI rows.
The first diff whitespace check found one surplus EOF blank line after moving NO44; removed before final check.

q.py failed at Bash startup with Win32 error 5; bounded PowerShell calls worked. Commits succeeded despite
a shared `packed-refs.lock` permission diagnostic; no refs/config were changed manually. Research stopped
after the guard measured 94,738/150,000 context, reserving the remaining allowance for publication/checks/report.

### Remaining work

- **NO33/NO43:** complete pole connection/cell-walk, model/resource/transform/bounds/Pick/Draw owner rules;
  diagonal floor operation order and wire rendering still need verified specification. Existing partial
  rules in §§4.1–4.7.1 remain the only published contracts; exploratory reads here do not close them.
- **NO36:** full RebuildModel/billboard construction, MySim and call-label pixel/geometry composition,
  Draw and DoMessage. The new accessors and ordinary texture selection can be implemented independently.
- **NO40:** descriptor-dependent connection piece/geometry dispatch and underground base geometry,
  ratios, transforms, boxes and Draw. §3.6.2 covers standalone builders, not that dispatch.
- **UG5:** same descriptor-dependent connection geometry plus Draw, transform/view decisions, bounds
  and message handling. UG5 is actually filed in `questions/underground-occupants.md`, not the network file.

## Handoff
- Documentation delivery is ready to land, but full requested owner closure is incomplete.
- Commits: a1302a47 (NO44), 04c0c750 (partial NO36/NO40/UG5); final report/check cleanup follows.
- Next RE pass: start with remaining work above; retain all five open questions until every named algorithm is specified.
- Sources: C:/Users/tlori/FreeCity4 export/decomp; do not read engine implementation.
- Worktree is detached by supervisor design; commit normally, never switch/update refs.

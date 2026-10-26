| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vc30 / VC29-PC Lua simulation precision (kind docs) | Codex / re-vc30 / reserved branch re-vc30; detached HEAD | ready; behavior commit 104f57c37; doccheck check passed; diff check passed; cargo/clippy/ctest n/a; game/shadow n/a; VC29-PC answered; ABI/seams none | 1. Capture the broad-run affected calls' inputs, PC and active Lua/modal stack, plus PC after outer return. 2. Reproduce invocation precision while retaining native float-store boundaries. |

VC29-PC was imported from views-cities29; all existing main questions were retained.
Foundations §§18.1–18.4 now distinguish the 24-bit baseline from live precision during native re-entry.
Lua-host §4.5 and regional-city, lot-developer and residential-education-health agree on the boundary.
The original scheduler does not assert 24-bit. Lua's outer guard restores PC only after the full call returns.
Instruction evidence includes the actual SC4 framework override (0x004735F0, table 0x00A8C9B0), not just its base.
Worked PC-24/PC-53 cases cover regional balance, float-sink slope and newcomer/grid/global EQ/HQ results.
Numeric examples were independently calculated; they are static conformance expectations, not native reruns.
VC29-PC moved to docs/specs/questions/answered/foundations.md, marked ANSWERED 2026-10-26 (re-vc30): DIFFERS.
Questions index regenerated. doccheck check: ok; git diff --check: passed.
No engine files, ABI exports, precision policy, tolerances or game settings changed. No REA was needed.
The broad-run mismatch attribution remains unmeasured; foundations §18.4 states the precise runtime trace needed.
No new question or ABI proposal file. No new game run or shadow-equivalence claim.

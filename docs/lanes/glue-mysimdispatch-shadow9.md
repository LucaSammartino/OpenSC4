| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysimdispatch-shadow9: dispatch replay round 2 | Codex / `glue-mysimdispatch-shadow9` (engine, detached HEAD) | in progress | Implement specified contracts and measure house to 2032. |

## Handoff
- Goal: implement MS18/MS20 dispatch contracts and extend independent dispatch replay.
- Prior milestone: cddc2899c constructor/header 73/73 exact; live owner uncompared.
- Started merge of re-mysims-ms20; resolved additive CMake/config conflicts and removed obsolete open MS18/MS20 entries.
- Merged answered contracts in 16354d1ef; MS18 §12.29 supplies 21 prose vectors, not executable engine vectors.
- Added core route-ready ordinary/same-mode/FE/delay decisions, stuck ordering, deterministic model lookup, retry-false quirk and constructor defaults. Focused dispatch suite passed 33/33 (42 unequal pairs inside one test).
- Failed initial compile was private valid() visibility; fixed. q.py cannot launch Git Bash (CreateFileMapping error); direct cargo works.
- Added idle snap/recovery and callbacks 1/2/9 plus mission exit; executable conformance vectors use the public record input boundary. First integration run failed 3 old expectations; corrected them to new MS18 stuck and mask ordering. Corrected integration passed 31/31 before further idle tests.
- Plugin now predicts 249 assigned constructor payload bytes independently; unassigned constructor words are excluded by spec, not measured differences. Live headers remain bounded; no claim of whole-owner replay.
- Filed MS21 for exact route partition/index/snap rules; failed-leg skip and callback-5 progression remain unsupported. MS20 explicitly rules out complete public read-only input observation; a controlled/instrumented input boundary is required for live whole-owner credit.
- Corrected and expanded integration now passes 38/38, including MS18-13/14/15 FE timing and MS18-16 first-vehicle rejection. Removed duplicated core test copies; executable new vectors live only in conformance include.
- Full qbuild all first attempt launched on 140680449; cargo test-discovery failed because tests/mysim_dispatch_ms18.inc.rs was auto-discovered as a crate (dot invalid). Moved the included module into tests/support/mysim_dispatch_ms18.rs. Must preserve failed logs and rerun full only after collecting original operation to terminal.
- Original full operation: exec session 10266, now waiting for native build/CTest queue label qbuild-20261024-100651-build-ctest. Rust/clippy failed solely on auto-discovered dotted crate; evidence preserved in build-runs/r2-failed-discovery/. Do not duplicate. No round-2 game request yet.
- Review diagnostic before corrected full: constructor helper currently counts 249 bytes but §7.2.3 does not explicitly promise auxiliary +10/+FC/+100 constructor values. Tighten it to the first 229 bytes (four location payloads), excluding byte4 and unassigned scalar ranges: 208 predicted bytes. Do not assume native total payload length or auxiliary reference shape. Update C++ count/logs, capi test and report; optionally ask MS22 for omitted auxiliary defaults.
- Next: collect original full, apply constructor-scope diagnostic, full qbuild all on corrected inputs, then unique house game request through 2032 (no qwait). Native queue may still be waiting behind shared games.
- Clean-room prohibition on running tools/ still applies; questions index regeneration delegated to coordinator (incoming index already has answered entries).
- No round-2 game request outstanding; unique labels required, wait via same exec session without qwait.

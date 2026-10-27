| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lot29 / lot-records live lot ABI; kind docs | Codex / re-lot29 worktree / detached HEAD, reserved re-lot29 | ready; docs commits 2270757b8, cc64eaaa2, 0298840d1; main merged c262eaf9e; doccheck PASS; native ABI audit 107/107; cargo/clippy/ctest n/a; game n/a; questions none; C ABI exports unchanged | 1. Land docs. 2. replace-lot29 consumes lot-records §13. |

- Resumed the committed spec; no earlier re-lot29 lane file existed in this worktree.
- Merged main as requested; edited only docs/specs/lot-records.md and this report.
- §13 covers every entry: primary 70, Serializable 6, property holder 25, traffic source 6.
- Each row specifies slot/offset, arguments and cleanup, return channel, ownership, effects and caller subsystems; U identifies unused live-lot entries.
- Restricted QI identity, all view adjustments, borrowed casts, property identity, destruction and whole-lot stream thunks are explicit.
- GetLandValue returns the inclusive cell-byte sum in EDX:EAX; it is not an ST0 getter or a monthly native call.
- The residential worked month separates the land-value grid pipeline from development reads and includes population/occupancy effects.
- Corrected the worked occupancy result after population 90/100: native f32 intermediate plus integer truncation produces f32 0.89.
- Measured static audit: all 107 table offsets and executable targets match; native stack cleanup matches every row (109 target/thunk functions checked).
- Gate: python tools/doccheck.py check PASS; git diff --check PASS. No runtime or shadow run requested or claimed.
- Evidence used the read-only export, decompilation and instruction bytes. REA was not needed.
- ABI row proposals: none. Questions filed/answered this round: none; existing LR6/LR7 are outside the live-lot interface task.
- Nothing remains unspecified in §13's four live-lot views. This is a specification result, not replacement registration or runtime verification.
- Ready queue: build-runs/merge-queue/re-lot29.json, kind docs; supervisor publishes the detached tip to the reserved lane branch.

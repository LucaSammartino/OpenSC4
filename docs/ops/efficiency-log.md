# Efficiency log

How much coverage each dollar and each hour buys, day by day, and which change moved it. The headline metric is
**$ per implemented coverage point**. Next to it: $ per verified point, once forge or lean lanes make verification a
large part of the day's work.

## How to add a day

1. Download the day's CSV from OpenRouter (Activity → Export) into Downloads.
2. Take the start-of-day and end-of-day `implemented %` from `python tools/coverage.py` (or from the HANDOFF notes).
3. Run `python tools/spend.py <csv> --coverage-delta <end - start>` and paste the printed row into the table below.
4. In "Changes in effect", note what changed since the previous row, so a jump can be attributed to its cause.

Claude-model days (no OpenRouter CSV) take the same columns from the session transcripts' `message.usage`. Mark the
row `(claude)`.

## Daily

| Date | $ | Calls | Median ctx | p90 ctx | Cache hit | $ in misses | $ in >400k ctx | Median out | Providers | Model hours | Impl. +pts | $/pt |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 2026-10-20 | 184.71 | 11,979 | 430k | 704k | 95.7% | 25.10 (406) | 135.84 | 101 | 13 | 20.4 | 4.0 | $46 |

Column notes: *ctx* = prompt tokens per call. *$ in misses* = calls that re-read more than 100k tokens uncached (count
in brackets). *$ in >400k ctx* = calls with a prompt above 400k. *Median out* = completion tokens per call.
*Model hours* = summed generation time.

## Changes in effect

| From | Change | Expected effect | Measured |
|---|---|---|---|
| 2026-10-20 (baseline) | z-ai/glm-5.3-flash via OpenRouter; lanes start at 172k ctx, no compaction, 13 providers | - | $46/pt; 85% of $ is cached re-reads, 0.5% output |
| 2026-10-20 evening | OpenRouter: provider pinned to DigitalOcean, auto top-up on | "$ in misses" near 0, providers ≤ 2, no 402/403 lane deaths | pending (next CSV) |
| 2026-10-20 (`0e1ebcca`) | Lean lane harness: `fc4-lane` agent types with a minimal tool list, 150k context budget with restart from `docs/lanes/<lane>.md`, quiet build/search wrappers, blocking `qwait` | lane first-call ctx 172k → <30k; median ctx < 150k; ~60% fewer tokens per lane | probe (subscription route): first call 7,356 tokens vs 39,063 for a default subagent. The GLM route is still unmeasured: check the first lane after the restart |
| 2026-10-20 (`b8628bfe`) | Forge: scripted spec→class generator with no agent loop; class tests in the pack | 20-50× lower $ per class | pilot: History $0.027 (2 calls), Parks $0.086 (6 calls), both pass their class tests, Parks clean in game; $0.41 total over 44 calls. A lane costs about $5 for a class like History. Claude-code backend overhead: 1,276 tokens |
| 2026-10-20 (`445ed0ac`) | Build/gate: sccache, `fc4verify quick` lane checks, `land` requires a parsed gate record | faster lane checks; no unbuildable landings | fresh worktree build + ctest 304 s → 168 s; train gate 444 s → 193 s; one-line plugin edit check 172 s → 3 s |

## Baseline facts (2026-10-20 audit)

- Cost ≈ calls × context. Cached input is $0.030/M on GLM, a quarter of the $0.12/M uncached price; on Claude a cached
  token costs 10% of an uncached one.
- Lane agents: 10,630 calls, 448k average ctx, first call 172k in every lane (inline MCP tool schemas: REA ~170 tools,
  Chrome, computer-use, browser), maximum 974k. 10,258 Bash calls vs 13 REA calls.
- About 58% of lane tokens were spent before the lane's first file write, or in runs that never wrote (detection is
  approximate).
- Coordinator session: 579M tokens, ending the day at 742k ctx.
- Lanes spent 21h waiting on the model and 5h running tools. Median latency is 2.7 s/call under 250k ctx and 4.5 s
  over 500k.
- At $46/pt, the remaining 74.7 implemented points would cost about $3,400, about 19 days like this one.

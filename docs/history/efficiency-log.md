# Efficiency log

How much coverage each dollar and each hour buys, run by run, and which change moved it. The headline metric is
**$ per implemented coverage point**, with $ per specified point next to it for RE-heavy runs. Count a point for the
run that **produced** it (the lane ran and finished), not the run that landed it. A finished lane that waits for the
next session would otherwise flatter the second run.

## How to add a run

1. Download the CSV from OpenRouter (Activity → Export) into Downloads. Timestamps are UTC.
2. Find the run's window: the coordinator transcript's first and last `timestamp` in
   `~/.claude/projects/C--Users-tlori-FreeCity4/<session>.jsonl`.
3. Take `implemented %` (and specified/verified) on main at both ends:
   `git show $(git rev-list --first-parent -1 --before="<end>" main):docs/coverage-report.md`.
4. Run `python tools/spend.py <csv> --since "<start>" --until "<end>" --label "<run>" --coverage-delta <pts>`, and
   paste the row below. `--model <substr>` keeps one model, for model comparisons from the same export.
5. Per-lane cost: split the run's $ in proportion to each lane's context tokens. Lane transcripts are under
   `<session>/subagents/workflows/*/agent-*.jsonl`, with the lane name in the matching `.meta.json` `description`.
6. In "Changes in effect", note what changed since the previous run, so a jump can be traced to its cause.

## Runs

| Run | $ | Calls | Median ctx | p90 ctx | Cache hit | $ in misses | $ in >400k ctx | Median out | Providers | Model hours | Impl. +pts | $/pt |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| run 1: 2026-10-20 09:06-18:10, GLM, before the changes | 184.71 | 11,979 | 430k | 704k | 95.7% | 25.10 (406) | 135.84 | 101 | 13 | 20.4 | 4.1 | $45 |
| run 2: 2026-10-20 20:31 - 10-21 08:24, GLM, lean lanes | 136.87 | 17,967 | 247k | 569k | 97.3% | 6.24 (208) | 59.91 | 93 | 15 | 28.1 | 0.7 | $196 |

Column notes: *ctx* = prompt tokens per call. *$ in misses* = calls that re-read more than 100k tokens uncached (count
in brackets). *$ in >400k ctx* = calls with a prompt above 400k. *Median out* = completion tokens per call.
*Model hours* = summed generation time.

**Points, corrected 2026-10-21.**
- Run 1 was first logged as +4.0 from 21.3%. But main already read 23.1% when the run started: train6, the 21.3 → 23.1
  step, was merged by the Claude session before it. Run 1 landed +2.2 (23.1 → 25.3).
- Run 1 also finished cores27, the network-tools core. Run 2 landed it in train10, and it only counted once its map
  row was wired (`de72a3a1`, +1.9). Run 1 therefore produced 2.2 + 1.9 = **4.1 points**.
- Run 2 produced +0.7 implemented (cores28), **+2.8 specified** (32.9 → 35.7: re-netocc, re-terrain3d, re-occprops;
  $49 per specified point) and +0.7 verified.

## Run 2 against run 1

**What the changes bought (unit cost):**

| | run 1 | run 2 |
|---|---:|---:|
| $ per call | 0.0160 | 0.0076 (-52%) |
| Output tokens per $ | 20k | 55k (2.7x) |
| Lane first-call context | 172k | 7.8k |
| Lane median context | 413k | 179k |
| Coordinator median / max context | 448k / 742k | 372k / 961k |
| Coordinator share of context tokens | 11% | 15% |

**What did not hold:**
- **The provider pin.** Run 2 used 15 providers and sent 0 calls to DigitalOcean; run 1 had sent it 75% of calls. The
  price per prompt token varies about 4x between providers in this run: OpenInference $0.0077/M, Relace $0.0129/M,
  Together and InferenceNet $0.032/M (`spend.py` prints this per provider).
- **The 150k lane budget.** Only the lane start honours it. **88% of lane context tokens were spent above 150k**, with
  lanes up to 930k. The budget is a prompt instruction: the lane runs `ctx.py` after a milestone and hands off. GLM
  lanes rarely reach a milestone, so they never check.
- **Stall control.** re-answers35 made 2,789 calls (about 1,800 `re_query` runs; REA was unavailable) for about $16 and
  wrote nothing but its lane file. The rule "no file written for 45+ min → kill it" exists only as text.

**Where run 2's $136.87 went** (split by context tokens, approximate):

| Lane | ~$ | Outcome |
|---|---:|---|
| glue29 | 27.17 | landed (train17): the network-tool replacement, clean in game (run 221). No coverage point until a regress replace run includes `nettool` |
| coordinator | 20.80 | trains 12-18 merged and landed |
| mysims1 | 17.66 | **not landed**: shadow written, lane gate passes, game run never made |
| re-answers35 | 16.34 | **nothing**: 0 answers |
| glue21fix2 | 15.80 | **not landed**: PL17-PL19 chase, still blocked |
| re-terrain3d | 13.07 | landed (train14 + train18): TR21 contract, 34 ABI rows |
| cores28 | 10.00 | landed (train16): MySimPool/MySimDispatch, the run's +0.7 implemented |
| re-occprops | 6.54 | landed (train16): the occupants and props spec |
| re-netocc | 6.00 | landed (train13): the network-occupants spec, +2.1 specified. This re-did re-specs22 ($21 in run 1, lost) |
| devperf2 | 3.05 | landed (train15): UpdatePopulation 3.52 → ~0.9 us/call |
| glue24fix2 | 0.45 | landed (train12): the tick-5500 int3 disarmed |

$49.80 (36%) went to lanes that did not land. About $12 of it was spent after the user's 21:49 wind-down: three lanes
the coordinator started after its context was compacted.

**Reading:** the harness changes halved the cost of a call. The cost of a point still rose, because run 2's work was
mostly fix rounds, perf and replacement glue that the coverage metric does not count, and a third of its spend went
to lanes that never landed.

## What fills the context (run 2, measured 2026-10-21)

Each tool result is weighted by the number of later calls that re-send it, which is what the bill counts. Thinking is
excluded: it is not re-sent (context grows by 1.45× the tool args and results per call, and by 0.3× once thinking is
added). Script: split each agent transcript into tool results by tool and target, then weight each by its remaining
calls.

| Share of re-sent lane context | |
|---|---:|
| Exploration output: `re_query` 20%, grep 17%, ad-hoc bash 14% | ~50% |
| The lane's own tool-call arguments (inline heredoc scripts, whole-file writes) | ~18% |
| Engine source reads (`fc4.h` alone 4%) | ~8% |
| Specs and docs, the behaviour source | ~14% |
| Instructions and process prose (lane prompt, HANDOFF, ops docs, lane files) | ~2.6% |
| Fixed preamble (system prompt and tool schemas, 7.8k per call) | ~4% |

The coordinator differs. Its preamble is 49.8k per call: the desktop harness's tool descriptions, the skill and
plugin listings (including the unrelated teya-pm plugin), and the deferred MCP tool names. That is 11% of its tokens.
Its own tool-call arguments are about 30%, and git output about 20%.

**Reading:** instruction prose is not where the tokens go. The bill comes from command output that stays resident for
hundreds of calls. The levers are to keep output short (cap `re_query`/grep/build output in the `lane-bin` wrappers;
print counts and paths, not dumps), to keep it short-lived (the enforced context budget), and to move inline scripts
into files. Prose costs in another way: instructions that contradict each other or have gone stale.

### Output caps: what they would have saved in run 2 (estimated 2026-10-21)

`python tools/ctx_weight.py <run-2 subagents dir>` reruns the weighting over the 33 lane transcripts of run 2 (16,136
calls, 3,333M re-sent tokens). It then applies the caps to the same transcripts. The chars-to-tokens ratio is
calibrated against the transcripts' own usage records (3.94 chars/token). The categories differ from the table above
because `sed -n`/`cat`/`head` file reads are split from other ad-hoc Bash here.

| Re-sent lane context, run 2 | Share | Cap 60 lines / 4 KB saves |
|---|---:|---:|
| Bash file reads (`sed -n` ranges, `cat`, `head`; 23% of them read `docs/specs`) | 21.1% | not capped |
| `Read` tool | 16.4% | not capped |
| grep (`qgrep`, `grep`, `Grep`) | 15.0% | 7.8% of it |
| `re_query` | 14.2% | 7.4% of it |
| Bash command strings (22.5% of their chars are absolute `C:/Users/tlori/FreeCity4/...` paths) | 11.8% | - |
| Prompt, attachments and user text | 9.6% | - |
| Other ad-hoc Bash output (the `q.py` target) | 3.7% | 19% of it |
| Write/Edit arguments | 1.7% | ~0 as Edit |
| Inline heredoc scripts | 1.1% | ~0 as file + Edit |
| Fixed preamble | 3.8% | - |

| Cap on re_query, grep and ad-hoc Bash | Re-sent tokens saved |
|---|---:|
| 60 lines / 4 KB (the shipped default) | **2.9%** (98M) |
| 40 lines / 3 KB | 5.7% |
| 30 lines / 2 KB | 8.6% |

**Reading:** a per-call cap saves little, because lanes already pipe almost every command through `| head -n 28`. The
volume is thousands of small results: 5,361 grep calls and 3,785 `re_query` calls, with a median ad-hoc output of
680 chars. These are upper bounds on the saving. A capped lane reruns some queries with `--full` or pages on, and
those extra calls are not counted. Inline scripts and whole-file rewrites are about 3% together, so the file-and-Edit
rule is for hygiene, not $. The cost is in how many results stay resident and for how long. That points back to lever
1, the enforced context budget. File reads (37.5% with `Read`) are the largest category; reading spec sections by
heading instead of in 80-150-line `sed` ranges is the next thing to measure. Running lanes with the worktree as
their working directory would remove most of the absolute paths (~2.7% of the total).

## Next levers, ranked by measured $

1. **Enforce the context budget in the harness**, not the prompt: stop a lane round at N context tokens and restart
   it from its Handoff. This targets the 88% of lane tokens spent above 150k.
2. **Enforce stall kill**: stop a lane that has made M calls with no commit (re-answers35: $16, 0 output), and check
   that REA is reachable before starting an RE lane.
3. **Fix the provider routing.** The pin did not hold. Test OpenInference/Relace, which were 2.5-4x cheaper per prompt
   token in run 2, for errors and cache behaviour before pinning.
4. ~~**Collect the uncounted points**~~ done 2026-10-21 (`regresslists6`): the replace list and map keys now cover
   automata, netmgr, netlot, nettool and demolition, and the regress passed. Replaced 12.1 → **15.9%** (+3.8), for no
   lane spend. Run 2's glue29 and earlier landed cores produced most of it.
5. **Model comparisons**: run the same lane brief on each model and compare with
   `spend.py --since/--until/--model` plus the per-lane split above.
6. **A lean coordinator**: its preamble is 49.8k tokens per call in the desktop app (11% of its spend). Run a GLM
   coordinator from the `claude` CLI with only the tools it uses (Bash, Read, Edit, Write, Workflow), with no
   connectors and no unrelated plugins, and restart it after each train lands.

## Changes in effect

| From | Change | Expected effect | Measured |
|---|---|---|---|
| 2026-10-20 (baseline) | z-ai/glm-5.3-flash via OpenRouter; lanes start at 172k ctx, no compaction, 13 providers | - | run 1: $45/implemented pt (corrected from $46 on 21.3 → 25.3); 85% of $ is cached re-reads, 0.5% output |
| 2026-10-20 evening | OpenRouter: provider pinned to DigitalOcean, auto top-up on | "$ in misses" near 0, providers ≤ 2, no 402/403 lane deaths | run 2: **not in effect**. 15 providers, 0 DigitalOcean calls. Misses fell anyway ($24.90 → $6.24) |
| 2026-10-20 (`0e1ebcca`) | Lean lane harness: `fc4-lane` agent types with a minimal tool list, 150k context budget with restart from `docs/lanes/<lane>.md`, quiet build/search wrappers, blocking `qwait` | lane first-call ctx 172k → <30k; median ctx < 150k; ~60% fewer tokens per lane | run 2: first call 7.8k; lane median 413k → 179k; $/call -52%. The budget is not enforced: 88% of lane tokens above 150k |
| 2026-10-20 (`b8628bfe`) | Forge: scripted spec→class generator with no agent loop; class tests in the pack | 20-50× lower $ per class | pilot: History $0.027 (2 calls), Parks $0.086 (6 calls), both pass their class tests, Parks clean in game; $0.41 total over 44 calls. A lane costs about $5 for a class like History. Not used in run 2 |
| 2026-10-20 (`445ed0ac`) | Build/gate: sccache, `fc4verify quick` lane checks, `land` requires a parsed gate record | faster lane checks; no unbuildable landings | fresh worktree build + ctest 304 s → 168 s; train gate 444 s → 193 s; one-line plugin edit check 172 s → 3 s. Run 2: trains 12, 14-17 gated. Trains 13 and 18 and two ABI-row commits went ungated because they were treated as docs-only, but `interfaces.tsv` feeds `abi_conformance_test` |

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

# Forge pilot (2026-10-20)

`tools/forge.py` regenerates one plugin class (`engine/plugin/cFC4<X>.h/.cpp`) from its spec with direct model
calls and no agent loop. It builds one deterministic context pack, makes one generation call, then runs at most N fix
rounds that send back only the trimmed compiler errors or test failures and the files concerned. This page records
what the pilot measured. Every number below comes from `build-runs/forge/<class>.jsonl` (`python tools/forge.py
report`) and from the game queue's verdict files.

**Recommendation: GO, but narrowly.** Use forge for plugin glue classes whose core and C API already exist *and*
that have in-process ctests, with the class's test functions in the pack. On that setup, both test-covered classes
passed build and tests: History in 2 calls for $0.027, Parks in 6 calls for $0.086. The lanes' rate is about $5 and
$2.5 for the same subsystems. Do not use it, yet, for classes without a test oracle (Dispatch), and do not use
compile-only mode at all: the pilot's first History run compiled cleanly and crashed the game. Keep agents for
integration and for whatever still fails to converge.

## What was measured

Model: `z-ai/glm-5.3-flash` on OpenRouter, provider pinned to DigitalOcean, no fallbacks, `--reasoning low`.
Template class: `cFC4Aura`. Each class was regenerated into a scratch copy of `engine/` (`build-runs/forge/engine`).
The files on main were never touched.

With the class's test functions in the pack (final forge):

| Class | Result | Calls | Prompt tok (total / cached) | Output tok | $ (API-reported) | Game run (replace) |
|---|---|---:|---:|---:|---:|---|
| cFC4History | build + 2 class ctests pass after **1** fix round | 2 | 99k / 0 | 31k | **0.0267** | not rerun (same tests as the clean run below) |
| cFC4Parks | build + class ctest pass after 5 fix rounds | 6 | 325k / 242k | 154k | **0.0856** | `forge-parks-1` clean; all 24 month lines agree with the baseline on `parks=`/`zoos=` |
| cFC4Dispatch | (no class test exists; nothing to add) | | | | | |

Before that (tests only as failure lines, no test source in the pack):

| Class | Result | Calls | Prompt tok (total / cached) | Output tok | $ (API-reported) | Game run (replace) |
|---|---|---:|---:|---:|---:|---|
| cFC4History | build + 2 class ctests pass after 4 fix rounds | 5 | 197k / 161k | 32k | 0.0237 | `forge-history-2` clean, 0 ref warnings |
| cFC4Dispatch | builds after 1 fix round (it has no class ctests) | 2 | 98k / 0 | 36k | **0.0287** | `forge-dispatch-2` clean, 0 ref warnings |
| cFC4Parks | did **not** converge: 3 attempts × 6 rounds, last 2 → 1-2 test failures | 7 per attempt | ~365k / ~270k per attempt | ~84k | 0.057-0.065 per attempt | - |
| cFC4History on Claude (subscription, below) | build + ctests pass after 1 fix round | 2 | 140k / 1.4k | 18k | 1.48 notional, $0 billed | `forge-history-cc-1` clean, 0 ref warnings |

- **Pack size:** 55.7k / 53.4k / 61.1k tokens by estimate (chars / 3.6) for History, Dispatch and Parks, of which
  the test functions are 4.1k (History) and 1.4k (Parks). GLM's tokenizer measured 58.8k and 63.0k on the first
  History and Parks calls, all under the 80k cap.
- **Fix-round calls** carry only the header/ABI/contract block plus the files and failures: 35-52k tokens. From the
  second call on, DigitalOcean serves 26-54k of each prompt from cache.
- **Total pilot spend:** **$0.406** on OpenRouter for 44 calls, including the tool-development reruns and probes, plus
  $1.49 notional on the Claude subscription (4 calls). The $5 cap was never approached.

### Against the lane baseline (2026-10-20 audit, docs/ops/efficiency-log.md)

| | Lanes | Forge (GLM) |
|---|---|---|
| Context per call | ~450k average, 172k on the first call | 35-61k |
| Calls | ~1,000 per lane | 2-7 per class |
| Output per call | ~100 tok | 1.7k-26k tok |
| $ per subsystem | $46/coverage point: History $5.2, Dispatch $6.1, Parks $2.5 (or, at $0.19 per function, $8.0 / $7.6 / $3.6) | History $0.027, Dispatch $0.029, Parks $0.086 (plus $0.18 for the 3 attempts before the tests were in the pack) |

**This is not like for like.** A lane's dollars bought the Rust core, the C API, the tests and the game verification
as well as this class. Forge regenerated only the glue class, on top of an existing core, `fc4.h` and the project's
tests. Even if the glue class were only a tenth of a lane's work, the History run would still be about 20× cheaper.
The per-call economics (about 10× less context, about 150× fewer calls) are the robust result. Whether forge can also
write the core (cargo tests as the oracle) has not been measured.

**Method caveat:** the three pilot classes were also used to develop forge. The test stage, the contract fixes and
the reasoning cap were added mid-pilot, so these are not blind runs. Only one first-attempt-after-the-fixes run was
made per class.

## What the pilot taught (now built into forge)

1. **Compile-only is not a checker.** The first History build that compiled (2 calls, $0.020) never set
   `fc4_history_host.user`, so every core callback got a null context. The game run said `corrupt`
   (`foreign_exceptions=1`). Forge now runs the class's ctests after a clean compile (`plugin_load_test` loads the
   DLL and drives the class through its vtable). That suite catches this bug in 3 s with exact `CHECK failed`
   lines, and feeds the failures into the same fix rounds. `--no-tests` turns this off.
2. **GLM's reasoning counts against the output cap.** With the default effort, the first call spent all 32,000 output
   tokens on reasoning and returned no file. `--reasoning low --max-tokens 64000` fixed it: 5.5-17k reasoning tokens
   per first call.
3. **The contract must include tests and their context.** Parks' `FC4ParksTestInput` is aggregate-initialised in
   `tests/plugin_cmglue_test.inc`. The contract scan skipped `.inc` files at first, then showed the usage line without
   its locals' types. The model kept changing the struct's layout, which broke the test build.
4. **Template errors need their call site.** An error inside `xutility` is useless without MSVC's "see reference to"
   note pointing into the class. The trimmer now keeps those notes.
5. **The `<x>_shadow` switches cannot check a plugin class.** They compare the ORIGINAL class with the core, and the
   game log says they go `INACTIVE` as soon as ours is enabled. The game check is therefore the replace run
   (`enable = history, dispatch, parks`). Two runs of the same baseline DLL already differ in city counters
   (occupants from the first month, the turbo end date), so `forge.py gamediff` masks those. Even then, a game run is
   only a crash and reference check for History and Dispatch: their log lines are free-form diagnostics the spec does
   not define. Parks' month lines are field-based (`parks= zoos= funding= latch= escapes=`) and do compare.
6. **Scratch hygiene:** restoring the originals with `shutil.copy2` kept their old mtimes, so ninja silently relinked
   the forged objects into the "baseline". `forge-baseline-3` and `forge-dispatch-1` were invalid and were redone as
   `forge-baseline-4` and `forge-dispatch-2`. Forge now restores with a fresh mtime.

7. **The class's tests belong in the pack.** The tests check exact log lines (`history Init: ok=1`,
   `parks month 2000-02-01: parks=0 zoos=0`) that exist only in the tests, not in the spec. Forge now follows each
   preset's ctest names to their `--mode` flag and to the test function in `engine/tests` (`test_sources`). It puts
   those functions, numbered, in the shared block that every fix round resends, and adds the 6 test lines before
   each failing CHECK. This is clean-room input: the tests are engine-side, not exe-derived. History dropped from 4
   fix rounds to 1; Parks went from 0 of 3 attempts converging to passing on the first attempt.

## Parks

Without the test source, 3 attempts all failed: the build converged within 2-3 rounds, then the behaviour bounced
between 5, 3, 2 and 2 test failures. With it, the first build was 4 compile errors, and every later round compiled
and failed one check: `parks month 2000-02-01: parks=0 zoos=0` (the month line for a city with no parks). Round 5
fixed it; that round spent 51k reasoning tokens ($0.028), a third of the run. In the game the forged Parks agrees
with the baseline on `parks=` and `zoos=` in all 24 months. It drops the baseline's extra diagnostic fields
(`funding= latch= lot-writes= escapes= occupants=`), which no test checks.

## Claude Code backend (`--backend claude-code`)

This backend runs one headless `claude -p --output-format json` turn per call with the CLI's own login (a
subscription or a gateway), so no API key is needed. Forge replaces Claude Code's default context with:
`--system-prompt <forge rules> --tools "" --strict-mcp-config --mcp-config {"mcpServers":{}} --disable-slash-commands
--no-session-persistence --setting-sources user`, run in an empty folder outside the repo (no `CLAUDE.md`, no
memory), with the parent session's `CLAUDE_CODE_*` variables stripped. `--bare` is not used because it refuses OAuth,
i.e. a subscription login. The npm shim is resolved to the native `bin/claude.exe`, so the multi-line system prompt
never passes through cmd.exe.

| First call, "Reply with OK" | Input tokens |
|---|---:|
| Default `claude -p` (measured by the coordinator) | ~42,000 (2 + 28,799 cache read + 13,012 cache write) |
| forge's stripped call | **1,276** (2 + 1,274 cache write; the repeat call read all 1,274 from cache) |
| of which forge's own rules + prompt | ~520 |
| Remaining CLI overhead | **~750 tokens per call**; the CLI also makes a small side call to `claude-haiku-5-5` |

The OpenRouter call's overhead is about 0: GLM counted 455 prompt tokens against forge's own ~523 estimate.

On a real class (History, Opus 5.5, `--effort low`), the subscription run passed build and tests in 2 calls (one
fix round, against 4 on GLM with the same pack, before the tests were added). It also stayed closer to the original: .cpp similarity 0.39 and 22/28 methods, against
0.23 and 3/28 for GLM. **Limit:** the CLI sends the pack as one user message, so forge cannot place a cache
breakpoint after the shared block. The fix call reused only the 709-token system prompt and re-wrote 54.5k to cache.
On a subscription that costs quota, not dollars; through a paid gateway it would cost about 60× GLM's price.

## How to run it

```
python tools/forge.py setup                                    # scratch engine (copy or sync from engine/), configure
python tools/forge.py pack  cFC4History                        # the pack and its token estimate, no API call
python tools/forge.py run   cFC4History --reasoning low --max-tokens 64000 --rounds 6
python tools/forge.py run   cFC4History --backend claude-code --reasoning low --rounds 6
python tools/forge.py game  cFC4History --which forged   --with cFC4Dispatch,cFC4Parks
python tools/forge.py game  cFC4History --which baseline --with cFC4Dispatch,cFC4Parks
python tools/forge.py gamediff cFC4History <baseline-label> <forged-label>
python tools/forge.py probe --backend openrouter|claude-code   # fixed per-call overhead
python tools/forge.py report
```

Keys come from `OPENROUTER_API_KEY` / `ANTHROPIC_API_KEY` or from `~/.config/fc4-forge/<backend>.key` (a file inside
the repo is refused). They are never printed or logged. A run stops above `--max-cost` ($1.50), and forge refuses to
start once the logged total reaches `--budget` ($5).

## Next steps

1. Point forge at the Rust side (a core module plus its `capi` file, with `cargo test` and the conformance vectors as
   the oracle). That is where lanes spend most of their calls; this pilot measured only the glue.
2. Give glue classes without a class ctest (Dispatch) a generic in-process check: factory, QueryInterface, a
   Read/Write round trip, the message target at +4. Then "builds" stops being the only gate for them.
3. Hand classes that still do not converge to a lean lane with forge's last round as the starting point
   (`r<N>/` files plus `test-failures.txt`).
4. Forward use (a class that does not exist yet): the pilot took the include list and the contract from the existing
   files. A new class needs `--files/--specs/--interfaces/--seeds` and a contract written by the coordinator.

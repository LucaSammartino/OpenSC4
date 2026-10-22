export const meta = {
  name: 'fc4-lane',
  description: 'One FreeCity4 lane (engine, RE, tooling or mechanical) in its own worktree, ending in a merge-train queue entry',
  whenToUse: 'Spawn one lane. Before spawning: python tools/lanes.py slots, python tools/lanes.py check <lane> --deps ... --touches ..., python tools/lanes.py brief <subsystem> --kind engine|re (pass its output as args.brief). See docs/ops/coordinating.md.',
  phases: [
    { title: 'Gate', detail: 'before every round a small agent runs .claude/lane-bin/laneguard.py preflight: no round starts while build-runs/STOP exists, while the lane is parked, or (RE lanes) while REA is unreachable' },
    { title: 'Lane', detail: 'the lane works in its worktree, keeps its state in docs/lanes/<lane>.md, writes build-runs/merge-queue/<lane>.json, then stops; restarted fresh from that file past the context budget or after an API error; the merge train lands it' },
  ],
}

// args: { kind: 'engine'|'re'|'tool'|'mech', name, task, date, brief?, game?, deps?, fix?, model?, effort?, base?,
//         ctxBudget?, maxCalls?, maxRounds?, unguarded? }
//   kind 'mech' is mechanical work (docs rows, ABI rows, regress records, renames): it defaults to a cheaper model.
//   fix: the merge train's failure text (python tools/merge-train.py fail --lane <lane>), for a fix round.
//   ctxBudget: context tokens per round (default 150000). Past it the lane writes its Handoff and returns state
//     'continue'; the loop below starts a fresh agent from that file. maxRounds: cap on rounds (default 3, the user's per-lane cap; lane_launch.py passes 1 per launch and counts rounds in its ledger).
//   maxCalls: the progress watchdog, tool calls allowed since the worktree's last commit (default 150).
//
// Enforcement (docs/ops/coordinating.md "Lane guard"): the budget, the watchdog, the STOP flag and parking are not
// left to the model. .claude/settings.json runs .claude/lane-bin/laneguard.py before every tool call of an
// fc4-lane / fc4-lane-re agent labelled lane:<name>; it reads the limits from the "lane guard:" line of LANE below
// and denies calls past them. This script has no file access, so before each round a small gate agent runs
// `laneguard.py preflight` and the round starts only on GO (the hook denies a lane's calls under STOP anyway; the
// gate saves the lane prompt's first call).
//
// Agent type: .claude/agents/fc4-lane.md (fc4-lane-re.md for RE lanes) lists only the local tools (and two REA
// tools for RE), so no other MCP schema is sent. On the third-party route every MCP schema is sent inline (no
// tool search): 172k tokens on the first call before this, see docs/ops/coordinating.md "Lane context".
//
// Prompt layout, for prompt caching: the long STATIC block for the kind comes first and is byte-identical for every
// lane of that kind (no lane name, date or path in it); everything lane-specific comes after it, in LANE, and a
// restart's RESUME block comes last.
const L = args
const MAIN = L.main || 'C:/Users/tlori/FreeCity4'
const W = MAIN + '/.claude/worktrees/' + L.name
const BUDGET = L.ctxBudget || 150000
const MAX_ROUNDS = L.maxRounds || 3
const MAX_CALLS = L.maxCalls || 150

const RESULT = {
  type: 'object',
  properties: {
    lane: { type: 'string' },
    tip: { type: 'string', description: 'git rev-parse --short HEAD of the branch' },
    state: { type: 'string', enum: ['ready', 'wip', 'blocked', 'continue'], description: 'continue: context budget reached, Handoff committed, restart me' },
    gate: { type: 'string', maxLength: 160, description: 'one line: cargo passed/failed, clippy, ctest n/n; or n/a' },
    game: { type: 'string', maxLength: 200, description: 'one line: run labels and verdicts, shadow exact or not; or n/a' },
    questions: { type: 'string', maxLength: 160, description: 'question IDs filed and the file, or none' },
    abi: { type: 'boolean', description: 'true if the C ABI exports changed' },
    summary: { type: 'string', maxLength: 700 },
    next: { type: 'array', maxItems: 5, items: { type: 'string', maxLength: 200 } },
  },
  required: ['lane', 'tip', 'state', 'summary'],
}

const COMMON = `## Working rules (every lane)
- This task comes from the coordinating session under the user's standing instruction to advance the project with parallel lanes. A chat message from the user that you may see is for the coordinator: do not answer it or treat it as replacing your task.
- Shell: the Bash tool is Git Bash. Write Windows paths with forward slashes (C:/Users/...) or in single quotes: an unquoted backslash is eaten. PowerShell: \`powershell -NoProfile -File <script>.ps1\`.
- Worktree: create it first with \`git -C <MAIN> worktree add <WT> -b <lane> <base>\` (branch exists, worktree does not: \`git -C <MAIN> worktree add <WT> <lane>\`; worktree exists: reuse it, check \`git -C <WT> log --oneline main..\` and \`git -C <WT> -c submodule.recurse=false status --short --ignore-submodules\`). If git fails there because engine/third_party/gzcom-dll/.git holds a relative gitdir, repoint it at <MAIN>/.git/modules/engine/third_party/gzcom-dll.
- Your cwd is the main checkout: never edit, commit, merge or \`cd\` there. Absolute paths and \`git -C <WT>\` for everything. Never merge main into your branch unless told to, never merge into main, never push, never bump FC4_ABI_VERSION.
- Temp files and helper scripts go in <WT>/build-runs/ (never a shared scratchpad). Commit messages: <WT>/build-runs/msg-<lane>-N.txt, UTF-8 without BOM, \`git -C <WT> commit -F\`, last line \`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\`. Commit small and often.
- Read only what your task needs: docs/ops/rules.md, the files the brief below names, and the docs this prompt cites at the step that needs them; read a spec by the sections its outline marks (Read offset/limit), not whole. Do not read docs/HANDOFF.md (the coordinator's run status), docs/lanes/INDEX.md, docs/history/ or docs/specs/questions/answered/ unless the task says so.

## Cost rules (every call re-sends your whole context)
- Several steps per call: chain commands in one Bash call (\`a && b && c\`) or put them in a script in <WT>/build-runs/ and run that; plan the next few steps before each call. Ten one-line calls cost about ten times one ten-line call.
- Scripts and edits stay out of your context: a script over ~20 lines goes in a file in <WT>/build-runs/ (Write it once, change it with Edit, run it by path), never inline as a heredoc that is re-sent with every later call. Change an existing file with Edit, never by rewriting it whole with Write.
- Quiet output by default. Lane tools in <MAIN>/.claude/lane-bin/ (they hold no RE content; run them, no need to read them):
  - \`python <MAIN>/.claude/lane-bin/qgrep.py <regex> <path>...\` (capped search: 40 lines, 8 per file, 160 chars, 4 KB; \`--count\` for one total, \`-l\` for counts per file, \`-C N\` for context, \`--full\` only when you need every hit). With the Grep tool always set head_limit to 40 or less.
  - \`python <MAIN>/.claude/lane-bin/qwait.py <file> [--fresh]\`: waits in ONE call (9 minutes at most; call again on "timeout") and prints the first lines. Never poll in a loop of tool calls.
  - Any other command: \`python <MAIN>/.claude/lane-bin/q.py --wt <WT> "<command line>"\` runs it in <WT> (relative paths work), keeps the full output in <WT>/build-runs/q/ and prints the last 40 lines / 4 KB and the exit code (\`--head\` for the first lines); grep that log for more. Or pipe through \`| head -n 40\`. Never print a whole log, spec, diff or decompiled file.
- Stateless: your conversation can be cut at any time (context budget, API error) and a fresh agent continues from your files alone. Right after the worktree exists, write <WT>/docs/lanes/<lane>.md: the table row (format docs/lanes/README.md, State at stop "in progress"), then a \`## Handoff\` section of at most 25 lines: goal; done (milestones with commit hashes); next (the exact next step); facts a successor needs (measured numbers, game-request labels outstanding, paths, decisions taken). Update and commit it after every milestone (a commit of real work, a gate result, a game verdict, a spec section). Then, in the same call, run \`python <MAIN>/.claude/lane-bin/ctx.py <lane> --budget <budget>\`: on OVER, make sure the Handoff is committed, write no merge-queue file, and return state "continue" with the tip and a one-line summary.
- Finish: (1) commit everything (no WIP at the tip if ready); (2) rewrite docs/lanes/<lane>.md: the one table row in the format of docs/lanes/README.md, then below it your full report (what changed, measured results, questions, next steps) in place of the Handoff section; commit it; (3) write <WT>/build-runs/merge-queue/<lane>.json as docs/lanes/README.md "How a lane lands" says (lane, branch, tip, kind, abi, game_runs, summary, state ready|wip); (4) return the short structured result. The coordinator reads only that result; details belong in docs/lanes/<lane>.md.`

const ENGINE = `You are a clean-room ENGINE IMPLEMENTER lane on FreeCity4 (OpenSC4), a GPL-3.0 reimplementation of SimCity 4 Deluxe (GOG 1.1.641): Rust core engine/core, C ABI engine/capi, C++ GZCOM plugin engine/plugin loaded by the real game.

## Clean-room rules (these do not bend)
- Never open, list, grep or read: decomp/, decomp-mac/, external/, ghidra/ (any part), re/, tools/, docs/subsystems/, docs/sim-engine.md, any game executable — in either checkout. Do not run scripts from tools/.
- You may read: docs/specs/ (your behaviour source), docs/abi/, docs/HANDOFF.md, docs/ops/, docs/harness.md, docs/manual-checks.md, docs/ARCHITECTURE.md, CONTRIBUTING.md, README.md, engine/ (incl. the SDK headers in engine/third_party/gzcom-dll), and in the main checkout only build-runs/coord-verify-lane.ps1. You may run the lane tools in .claude/lane-bin/.
- Spec silent or ambiguous: implement the most literal reading, mark it with a code comment, and file a numbered question by appending it to docs/specs/questions/<spec>.md (the spec's ID prefix, next free number per docs/specs/QUESTIONS.md; see docs/specs/questions/README.md). That is the only file under docs/specs/ you may edit. Never guess from other sources. Never commit game files or decompiled output.

## Build and test
- Always through the quiet wrapper: while you work, \`python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> quick\` (cargo test -p the crates you touched, compile-only their dependents, then the sccache-backed CMake build); before you report, \`qbuild.py --wt <WT> all\` (cargo test --workspace, clippy -D warnings, then the CMake build + full ctest via fc4verify). Run \`all\` once first to record a baseline. Full logs stay in <WT>/build-runs (qbuild-<step>.log, fc4verify-<step>.log); the console gets only counts, exit lines and the first errors. Grep a log for more; never print it.
- \`python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> fmt <files>\`: rustfmt --check --edition 2021 on the .rs files you touched (add no diffs, no mass reformat).
- engine/tests/abi_entries.inc is generated at build time: never edit or merge it.

## Glue rules (docs/ops/lessons.md has the why)
- Never Release a pointer you did not AddRef or create (§5.1b): FC4_RELEASE_CREATED for temporaries the game creates for you, FC4_ACQUIRED/FC4_RELEASE for references you take.
- The game calls getters from a second thread during OnTick (§5.1): no RefCell for state getters read. Read runs before Init: never size saved state from the city in Read. Other subsystems only through the game's interfaces.
- Shared files (CMakeLists.txt, FC4Config.*, FC4Subsystems.cpp, fc4.h, capi/src/lib.rs, plugin_load_test.cpp, FC4TestSeam.*, README): edits additive, grouped, at the end of their lists.

## Testing in the real game (docs/harness.md)
- Build the plugin (verify leaves engine/build-x86/FreeCity4.dll), write <WT>/build-runs/game-queue/<name>.req, then wait in one call: \`python <MAIN>/.claude/lane-bin/qwait.py <WT>/build-runs/game-results/<label>.verdict\` (labels are unique), and grep <label>.log for your lines. At most one request outstanding (6 slots shared by all lanes). Prefer \`sim_turbo = 1\` with an end_date; use \`script = <file>\` for in-city actions, never the UI. failed-start / harness-error: resend.
- Before you report: verdict clean, ref_warnings=0, your shadow exact or every difference a numbered question. If a shadow differs only under turbo, also run once at normal speed.
- Do not start the game yourself, do not touch Documents/SimCity 4/.`

const RE = `You are a SPEC WRITER (RE side) lane on FreeCity4 (OpenSC4), a clean-room reimplementation of SimCity 4 Deluxe (GOG 1.1.641). You read the original program and write behaviour specs in docs/specs/ for implementers who never see it. Your output is spec text, never code.

## Where things are (main checkout)
- bin/SimCity4.exe (working copy), decomp/ (gitignored, one .c file per class), ghidra/export/windows (the export), re/ (names, snapshot), docs/abi/ (interface tables). Read them from the main checkout by absolute path; edit only in your worktree.
- Query the export first with \`python <MAIN>/tools/re_query.py\` (func, asm [--max N --skip N], callers, callees, xrefs [--from], strings, search, vtable; lists capped at --limit 40, asm at 56 lines per page, every call at 60 lines / 4 KB; \`--count\` for counts only, \`--full\` lifts the cap): read-only, lock-free, ~0.15 s, safe in parallel; several queries per Bash call. Search decomp/ with qgrep.py, then Read the function by offset/limit.
- REA: you have only mcp__rea__procedure_pseudo_code and mcp__rea__batch_decompile, for pseudocode the export and decomp/ lack. All lanes share ONE REA Ghidra session: expect ~3 minutes if you are the first to query; on an error fall back to re_query.py and decomp/ and say so in your report (docs/re-workspace.md "Querying in parallel lanes").
- Ghidra pseudocode is unreliable on some functions: for anything numeric (float vs double, operation order, widths) verify against the instructions (re_query.py asm --bytes) and cite addresses.

## Rules
- Behaviour in prose, tables and worked examples. Never paste decompiled code or assembly listings into specs; short address citations and constants are fine. Never commit decomp/ output, game files or anything from external/.
- Never read or edit engine/. Do not edit docs/HANDOFF.md or README.md.
- Questions: each lives in docs/specs/questions/<spec>.md. Answering one: write the rule into the named spec section, then move the entry to docs/specs/questions/answered/<spec>.md marked \`**ANSWERED <date> (<lane>):** MATCHES|DIFFERS — <rule>\`. Then \`python <WT>/tools/questions.py index\`.
- Proposed docs/abi rows: write them to <WT>/build-runs/abi-rows-<lane>.tsv in those files' formats and name the file in docs/lanes/<lane>.md; do not edit docs/abi/.`

const TOOL = `You are a coordinator-side TOOLING lane on FreeCity4 (OpenSC4). You may read and change tools/, docs/ and build-runs scripts; engine/ only if the task says so. Exe facts: \`python <MAIN>/tools/re_query.py\`; waiting: \`python <MAIN>/tools/lanes.py wait\`. Tools are standard-library Python 3 or PowerShell 5.1, read results not exit codes, and are tested for real before you report (say what you ran and what it printed).`

const MECH = `You are a MECHANICAL lane on FreeCity4 (OpenSC4): the task is a well-defined edit (docs rows, ABI rows, regress records, renames, regenerating generated files). Do exactly the task, verify it with the check the task names (or a diff review), and do not redesign anything. If the task turns out to need judgment, stop with state "blocked" and say why in one line.`

const STATIC = { engine: ENGINE, re: RE, tool: TOOL, mech: MECH }[L.kind]
if (!STATIC) throw new Error(`unknown kind ${L.kind}`)
const AGENT_TYPE = L.kind === 're' ? 'fc4-lane-re' : 'fc4-lane'

const LANE = `## Your lane
- lane: ${L.name}; worktree <WT> = ${W}; main checkout <MAIN> = ${MAIN}; base: ${L.base || 'main'}; today: ${L.date || 'see the system date'}; context budget <budget> = ${BUDGET}.
- lane guard: budget=${BUDGET} calls=${MAX_CALLS}. The harness enforces both: past the budget, or after ${MAX_CALLS} tool calls since your worktree's last commit, or when the coordinator sets STOP, every tool call except committing your lane file is denied and the denial says what to return. Commit after every milestone so the call count restarts.
${L.deps ? `- Depends on (already on main): ${L.deps}\n` : ''}${L.kind === 'engine' && L.game ? '- This lane changes the plugin: the in-game checks above are required before state "ready".\n' : ''}
${L.brief ? `## Brief (generated by tools/lanes.py brief: the files you need)\n${L.brief}\n` : ''}
## Task
${L.task}
${L.fix ? `\n## FIX ROUND\nThe merge train sent this lane back. Fix exactly this, rerun the affected checks, then Finish again:\n${L.fix}\n` : ''}`

const prompt = `${STATIC}\n\n${COMMON}\n\n${LANE}`
const model = L.model || (L.kind === 'mech' ? 'sonnet' : undefined)

// A restart starts a fresh agent with the same prompt plus RESUME: its state is the worktree and the Handoff, never
// the previous conversation. Why a round ended: 'continue' = the lane measured its context past the budget (ctx.py)
// and handed off; null = agent() gave up (an API error after retries, or a skip by the user: two nulls in a row stop
// the loop, so a deliberate skip is not fought for long).
const resume = (round, why) => `
## RESTART (round ${round} of at most ${MAX_ROUNDS})
The previous round ${why}. You are a fresh agent: the worktree exists, do not recreate it. First, in ONE Bash call: \`sed -n '/^## Handoff/,$p' ${W}/docs/lanes/${L.name}.md | head -n 40; git -C ${W} log --oneline -15 main..; git -C ${W} -c submodule.recurse=false status --short --ignore-submodules | head -n 30\`. Trust those over the task text for what is done. Uncommitted changes are the previous round's unfinished edits: review them (git diff, capped) before continuing. Do not redo finished milestones; continue from the Handoff's "next".`

// Agent types are read when a session starts, from its cwd's .claude/agents/: a session started before the files
// existed (or in another checkout) does not know them, and agent() throws "agent type ... not found". The lane guard
// acts only on the fc4-lane agent types, so a default subagent would run unguarded: refuse, unless args.unguarded is
// true (then run as a default workflow subagent, every MCP schema again on the third-party route, and say so).
let agentType = AGENT_TYPE
const run = async (p, opts) => {
  if (agentType) {
    try {
      return await agent(p, { ...opts, agentType })
    } catch (e) {
      if (!/not found/.test(String(e && e.message))) throw e
      if (!L.unguarded) {
        throw new Error(`${L.name}: agent type ${agentType} unknown to this session, so the lane would run without the lane guard. Start a new coordinator session in the main checkout (agent types and .claude/settings.json are read at session start), or pass args.unguarded: true`)
      }
      log(`${L.name}: agent type ${agentType} unknown to this session; args.unguarded is set, running as a default subagent with every tool and WITHOUT the lane guard`)
      agentType = undefined
    }
  }
  return agent(p, opts)
}

// The gate: one small agent per round runs the pre-flight and reports its one line. GO starts the round; STOP,
// PARK (the last two results blocked on the same question) or REA-DOWN (RE lanes) end the workflow without one.
const GATE = {
  type: 'object',
  properties: {
    verdict: { type: 'string', enum: ['GO', 'STOP', 'PARK', 'REA-DOWN', 'ERROR'] },
    line: { type: 'string', maxLength: 400, description: 'the command output, verbatim' },
  },
  required: ['verdict', 'line'],
}
const gateCmd = `python ${MAIN}/.claude/lane-bin/laneguard.py preflight ${L.name} --kind ${L.kind}`
const gate = async (round) => {
  const g = await run(`Run exactly this one Bash command and nothing else, then return its output. verdict is the first word of the output (GO, STOP, PARK or REA-DOWN), or ERROR if the command failed to run; line is the output verbatim.

${gateCmd}`,
    { label: `gate:${L.name}#${round}`, phase: 'Gate', schema: GATE, model, effort: 'low' })
  if (!g) return { verdict: 'ERROR', line: 'the gate agent returned nothing (an API error or a skip)' }
  return g
}
const notStarted = (round, g) => ({
  lane: L.name, tip: '', state: 'blocked',
  summary: `round ${round} not started: ${g.verdict} ${g.line}`.slice(0, 700),
  next: g.verdict === 'STOP' ? ['python tools/lanes.py go clears the STOP flag']
    : g.verdict === 'PARK' ? [`answer the open question, then python tools/lanes.py unpark ${L.name}`]
    : g.verdict === 'REA-DOWN' ? ['reconnect REA (/mcp) in the coordinator session; python tools/lanes.py check <lane> --kind re']
    : [`rerun the pre-flight by hand: ${gateCmd}`],
})

let res = null
let nulls = 0
let why = ''
for (let round = 1; round <= MAX_ROUNDS; round++) {
  phase('Gate')
  const g = await gate(round)
  if (g.verdict !== 'GO') {
    log(`${L.name}: round ${round} not started - ${g.verdict}: ${(g.line || '').slice(0, 200)}`)
    res = round === 1 ? notStarted(round, g) : { ...(res || {}), ...notStarted(round, g), tip: (res && res.tip) || '' }
    break
  }
  phase('Lane')
  const label = round === 1 ? `lane:${L.name}` : `lane:${L.name}#${round}`
  const r = await run(round === 1 ? prompt : prompt + resume(round, why),
    { label, phase: 'Lane', schema: RESULT, model, effort: L.effort })
  if (!r) {
    nulls++
    log(`${L.name}: round ${round} returned nothing (API error or skipped)${nulls >= 2 ? '; second in a row, stopping' : '; restarting from the Handoff'}`)
    if (nulls >= 2) break
    why = 'died without a result (an API error)'
    continue
  }
  nulls = 0
  res = r
  if (r.state !== 'continue') break
  log(`${L.name}: round ${round} handed off at ${r.tip} (context budget): ${(r.summary || '').slice(0, 100)}`)
  why = `reached its context budget and handed off at ${r.tip}: ${(r.summary || '').slice(0, 300)}`
  if (round === MAX_ROUNDS) log(`${L.name}: round cap ${MAX_ROUNDS} reached with state continue; restart by hand`)
}
if (res) log(`${L.name}: ${res.state} ${res.tip || ''} ${(res.summary || '').slice(0, 120)}`)
return res

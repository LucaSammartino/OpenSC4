export const meta = {
  name: 'fc4-lane',
  description: 'One FreeCity4 lane (engine, RE, tooling or mechanical) in its own worktree, ending in a merge-train queue entry',
  whenToUse: 'Spawn one lane. Before spawning: python tools/lanes.py slots, python tools/lanes.py check <lane> --deps ... --touches ..., python tools/lanes.py brief <subsystem> --kind engine|re (pass its output as args.brief). See docs/ops/coordinating.md.',
  phases: [
    { title: 'Lane', detail: 'the lane works in its worktree, writes docs/lanes/<lane>.md and build-runs/merge-queue/<lane>.json, then stops; the merge train lands it' },
  ],
}

// args: { kind: 'engine'|'re'|'tool'|'mech', name, task, date, brief?, game?, deps?, fix?, model?, effort?, base? }
//   kind 'mech' is mechanical work (docs rows, ABI rows, regress records, renames): it defaults to a cheaper model.
//   fix: the merge train's failure text (python tools/merge-train.py fail --lane <lane>), for a fix round.
//
// Prompt layout, for prompt caching: the long STATIC block for the kind comes first and is byte-identical for every
// lane of that kind (no lane name, date or path in it); everything lane-specific comes after it, in LANE.
const L = args
const MAIN = 'C:\\Users\\tlori\\FreeCity4'
const W = MAIN + '\\.claude\\worktrees\\' + L.name

const RESULT = {
  type: 'object',
  properties: {
    lane: { type: 'string' },
    tip: { type: 'string', description: 'git rev-parse --short HEAD of the branch' },
    state: { type: 'string', enum: ['ready', 'wip', 'blocked'] },
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
- Worktree: create it first with \`git -C <MAIN> worktree add <WT> -b <lane> <base>\` (branch exists, worktree does not: \`git -C <MAIN> worktree add <WT> <lane>\`; worktree exists: reuse it, check \`git -C <WT> log --oneline main..\` and \`git -C <WT> -c submodule.recurse=false status --short --ignore-submodules\`). If git fails there because engine/third_party/gzcom-dll/.git holds a relative gitdir, repoint it at <MAIN>/.git/modules/engine/third_party/gzcom-dll.
- Your cwd is the main checkout: never edit, commit, merge or \`cd\` there. Absolute paths and \`git -C <WT>\` for everything. Never merge main into your branch unless told to, never merge into main, never push, never bump FC4_ABI_VERSION.
- Temp files and helper scripts go in <WT>\\build-runs\\ (never a shared scratchpad). Commit messages: <WT>\\build-runs\\msg-<lane>-N.txt, UTF-8 without BOM, \`git -C <WT> commit -F\`, last line \`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\`. Commit small and often.
- Read only what your task needs: docs/HANDOFF.md (the short index) and the files the brief below names; read a spec by the sections its outline marks (offset/limit), not whole. Do not read docs/lanes/INDEX.md, docs/history/ or docs/specs/questions/answered/ unless the task says so.
- Waiting: never poll in a loop of tool calls. Wait in ONE call that blocks, e.g. PowerShell \`$f='<file>'; $n=0; while (-not (Test-Path $f) -and $n -lt 108) { Start-Sleep 5; $n++ }; if (Test-Path $f) { Get-Content $f -TotalCount 20 } else { 'timeout' }\` (9 minutes at most; call again on 'timeout'). Read long outputs from files with grep for the verdict lines, never whole.
- Finish: (1) commit everything (no WIP at the tip if ready); (2) write docs/lanes/<lane>.md: the one table row in the format of docs/lanes/README.md, then below it your full report (what changed, measured results, questions, next steps); commit it; (3) write <WT>\\build-runs\\merge-queue\\<lane>.json as docs/merge-train.md says (lane, branch, tip, kind, abi, game_runs, summary, state ready|wip); (4) return the short structured result. The coordinator reads only that result; details belong in docs/lanes/<lane>.md.`

const ENGINE = `You are a clean-room ENGINE IMPLEMENTER lane on FreeCity4 (OpenSC4), a GPL-3.0 reimplementation of SimCity 4 Deluxe (GOG 1.1.641): Rust core engine/core, C ABI engine/capi, C++ GZCOM plugin engine/plugin loaded by the real game.

## Clean-room rules (these do not bend)
- Never open, list, grep or read: decomp/, decomp-mac/, external/, ghidra/ (any part), re/, tools/, docs/subsystems/, docs/sim-engine.md, any game executable — in either checkout. Do not run scripts from tools/.
- You may read: docs/specs/ (your behaviour source), docs/abi/, docs/HANDOFF.md, docs/ops/, docs/harness.md, docs/manual-checks.md, docs/ARCHITECTURE.md, docs/PROGRESS.md, CONTRIBUTING.md, README.md, engine/ (incl. the SDK headers in engine/third_party/gzcom-dll), and in the main checkout only build-runs\\coord-verify-lane.ps1.
- Spec silent or ambiguous: implement the most literal reading, mark it with a code comment, and file a numbered question by appending it to docs/specs/questions/<spec>.md (the spec's ID prefix, next free number per docs/specs/QUESTIONS.md; see docs/specs/questions/README.md). That is the only file under docs/specs/ you may edit. Never guess from other sources. Never commit game files or decompiled output.

## Build and test
- Copy <MAIN>\\build-runs\\coord-verify-lane.ps1 to <WT>\\build-runs\\ first. Record a baseline before changing anything.
- Cargo from engine/ only (never the worktree root): set CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0 (disk is shared). \`cargo test --workspace --target i686-pc-windows-msvc\` to a file, sum every \`test result:\` line; \`cargo clippy --workspace --all-targets -- -D warnings\` (main is clean); \`rustfmt --check --edition 2021\` on the .rs files you touched (add no diffs, no mass reformat).
- Plugin + ctest: \`powershell -ExecutionPolicy Bypass -File <WT>\\build-runs\\coord-verify-lane.ps1 -Engine <WT>\\engine\` to a file. It exits 0 even when cmake fails: read its \`build exit=\` and \`ctest exit=\` lines and the ctest summary (all "Not Run" = build failed). Its output is UTF-16: strip NULs before grepping.
- engine/tests/abi_entries.inc is generated at build time: never edit or merge it.

## Glue rules (docs/ops/lessons.md has the why)
- Never Release a pointer you did not AddRef or create (§5.1b): FC4_RELEASE_CREATED for temporaries the game creates for you, FC4_ACQUIRED/FC4_RELEASE for references you take.
- The game calls getters from a second thread during OnTick (§5.1): no RefCell for state getters read. Read runs before Init: never size saved state from the city in Read. Other subsystems only through the game's interfaces.
- Shared files (CMakeLists.txt, FC4Config.*, FC4Subsystems.cpp, fc4.h, capi/src/lib.rs, plugin_load_test.cpp, FC4TestSeam.*, README): edits additive, grouped, at the end of their lists.

## Testing in the real game (docs/harness.md)
- Build the plugin (coord-verify leaves engine\\build-x86\\FreeCity4.dll), write <WT>\\build-runs\\game-queue\\<name>.req, then wait in one blocking call (see Waiting) for build-runs\\game-results\\<label>.verdict and read <label>.log with grep. At most one request outstanding (6 slots shared by all lanes). Prefer \`sim_turbo = 1\` with an end_date; use \`script = <file>\` for in-city actions, never the UI. failed-start / harness-error: resend.
- Before you report: verdict clean, ref_warnings=0, your shadow exact or every difference a numbered question. If a shadow differs only under turbo, also run once at normal speed.
- Do not start the game yourself, do not touch Documents\\SimCity 4\\.`

const RE = `You are a SPEC WRITER (RE side) lane on FreeCity4 (OpenSC4), a clean-room reimplementation of SimCity 4 Deluxe (GOG 1.1.641). You read the original program and write behaviour specs in docs/specs/ for implementers who never see it. Your output is spec text, never code.

## Where things are (main checkout)
- bin\\SimCity4.exe (working copy), decomp\\ (gitignored), ghidra\\export\\windows (the export), re\\ (names, snapshot), docs\\abi\\ (interface tables). Read them from the main checkout by absolute path; edit only in your worktree.
- Query the export first with \`python <MAIN>\\tools\\re_query.py\` (func, asm, callers, callees, xrefs [--from], strings, search, vtable; --limit N): read-only, lock-free, ~0.15 s, safe in parallel. Use the REA MCP tools only for pseudocode, types/prototypes or analysis the export lacks. All lanes share ONE REA Ghidra session: never call close_binary, never open_binary on another file, expect ~3 minutes if you are the first to query. On "Ghidra client is already started": close_binary once, retry once, then fall back to re_query.py and say so in your report (docs/re-workspace.md "Querying in parallel lanes").
- Ghidra pseudocode is unreliable on some functions: for anything numeric (float vs double, operation order, widths) verify against the instruction bytes and cite addresses.

## Rules
- Behaviour in prose, tables and worked examples. Never paste decompiled code or assembly listings into specs; short address citations and constants are fine. Never commit decomp/ output, game files or anything from external/.
- Never read or edit engine/. Do not edit docs/HANDOFF.md or README.md.
- Questions: each lives in docs/specs/questions/<spec>.md. Answering one: write the rule into the named spec section, then move the entry to docs/specs/questions/answered/<spec>.md marked \`**ANSWERED <date> (<lane>):** MATCHES|DIFFERS — <rule>\`. Then \`python <WT>\\tools\\questions.py index\`.
- Proposed docs/abi rows: write them to <WT>\\build-runs\\abi-rows-<lane>.tsv in those files' formats and name the file in docs/lanes/<lane>.md; do not edit docs/abi/.`

const TOOL = `You are a coordinator-side TOOLING lane on FreeCity4 (OpenSC4). You may read and change tools/, docs/ and build-runs scripts; engine/ only if the task says so. Exe facts: \`python <MAIN>\\tools\\re_query.py\`; waiting: \`python <MAIN>\\tools\\lanes.py wait\`. Tools are standard-library Python 3 or PowerShell 5.1, read results not exit codes, and are tested for real before you report (say what you ran and what it printed).`

const MECH = `You are a MECHANICAL lane on FreeCity4 (OpenSC4): the task is a well-defined edit (docs rows, ABI rows, regress records, renames, regenerating generated files). Do exactly the task, verify it with the check the task names (or a diff review), and do not redesign anything. If the task turns out to need judgment, stop with state "blocked" and say why in one line.`

const STATIC = { engine: ENGINE, re: RE, tool: TOOL, mech: MECH }[L.kind]
if (!STATIC) throw new Error(`unknown kind ${L.kind}`)

const LANE = `## Your lane
- lane: ${L.name}; worktree <WT> = ${W}; main checkout <MAIN> = ${MAIN}; base: ${L.base || 'main'}; today: ${L.date || 'see the system date'}.
${L.deps ? `- Depends on (already on main): ${L.deps}\n` : ''}${L.kind === 'engine' && L.game ? '- This lane changes the plugin: the in-game checks above are required before state "ready".\n' : ''}
${L.brief ? `## Brief (generated by tools/lanes.py brief: the files you need)\n${L.brief}\n` : ''}
## Task
${L.task}
${L.fix ? `\n## FIX ROUND\nThe merge train sent this lane back. Fix exactly this, rerun the affected checks, then Finish again:\n${L.fix}\n` : ''}`

const prompt = `${STATIC}\n\n${COMMON}\n\n${LANE}`
const model = L.model || (L.kind === 'mech' ? 'sonnet' : undefined)

phase('Lane')
const res = await agent(prompt, { label: `lane:${L.name}`, phase: 'Lane', schema: RESULT, model, effort: L.effort })
if (res) log(`${L.name}: ${res.state} ${res.tip || ''} ${(res.summary || '').slice(0, 120)}`)
return res

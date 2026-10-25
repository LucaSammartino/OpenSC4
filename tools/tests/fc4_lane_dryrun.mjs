// Dry run of .claude/workflows/fc4-lane.js with a mock agent(): no model is called and no lane is started.
//
//   node tools/tests/fc4_lane_dryrun.mjs <scenario.json>
//
// scenario: { args: {...workflow args}, guardMain: <fixture main checkout>, lanes: [<result of round 1>, ...],
//             setStopAfterRound?: n, unknownAgentType?: true (agent() throws "not found" for an agentType, as in a
//             session started before .claude/agents/ existed) }
// A gate agent (label gate:*) is simulated faithfully: the Bash command in its prompt is run for real against this
// checkout's .claude/lane-bin/laneguard.py with FC4_LANEGUARD_MAIN=guardMain, and its first word becomes the verdict.
// A lane agent (label lane:*) returns the next entry of `lanes`. Prints one JSON object: {result, agents, logs}.
import { closeSync, openSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = dirname(fileURLToPath(import.meta.url))
const REPO = resolve(HERE, '..', '..')
const sc = JSON.parse(readFileSync(process.argv[2], 'utf8'))
const src = readFileSync(resolve(REPO, '.claude/workflows/fc4-lane.js'), 'utf8').replace('export const meta', 'const meta')
const agents = []
const logs = []
let laneRound = 0

async function agent(prompt, opts) {
  if (sc.unknownAgentType && opts.agentType) throw new Error(`agent type '${opts.agentType}' not found`)
  agents.push(opts.label)
  if (opts.label.startsWith('gate:')) {
    const m = prompt.match(/python (\S+)\/\.claude\/lane-bin\/laneguard\.py (preflight .*)$/m)
    if (!m) throw new Error('gate prompt has no preflight command')
    // File-backed stdout avoids Node's denied pipe creation in Windows lane sandboxes.
    // The preflight still runs for real; a nonzero STOP/PARK exit is a normal verdict.
    const output = resolve(sc.guardMain, 'build-runs', `${opts.label.replace(/[^a-z0-9-]/gi, '_')}.out`)
    const fd = openSync(output, 'w')
    try {
      execFileSync(process.env.FC4_TEST_PYTHON || 'python',
        [resolve(REPO, '.claude/lane-bin/laneguard.py'), ...m[2].split(/\s+/), '--no-handshake'],
        { env: { ...process.env, FC4_LANEGUARD_MAIN: sc.guardMain }, stdio: ['ignore', fd, 'inherit'], windowsHide: true })
    } catch (e) {
      if (!Number.isInteger(e.status)) throw e
    } finally {
      closeSync(fd)
    }
    const line = readFileSync(output, 'utf8').trim()
    rmSync(output)
    const w = line.split(/\s+/)[0]
    return { verdict: ['GO', 'STOP', 'PARK', 'REA-DOWN'].includes(w) ? w : 'ERROR', line }
  }
  const r = sc.lanes[laneRound++]
  if (sc.setStopAfterRound && laneRound === sc.setStopAfterRound) {
    writeFileSync(resolve(sc.guardMain, 'build-runs', 'STOP'), JSON.stringify({ reason: 'set mid-run', since: 'now' }))
  }
  return r === undefined ? null : r
}

const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor
const body = new AsyncFunction('args', 'agent', 'log', 'phase', src)
const result = await body(sc.args, agent, (m) => logs.push(m), () => {})
process.stdout.write(JSON.stringify({ result, agents, logs }))

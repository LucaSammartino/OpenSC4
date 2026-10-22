// Render the existing canonical workflow's prompt/schema without executing a workflow or calling a model.
import {readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname, resolve} from 'node:path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const request = JSON.parse(readFileSync(0, 'utf8'));
let source = readFileSync(resolve(root, '.claude/workflows/fc4-lane.js'), 'utf8');
const marker = '// A restart starts a fresh agent';
if (!source.includes(marker)) throw new Error('Workflow prompt boundary changed; update renderer tests');
source = source.split(marker)[0].replace('export const meta', 'const meta');
const rendered = new Function('args', source + '\nreturn {prompt, schema: RESULT};')(request.args);
if (request.provider === 'codex') {
  // Provider-specific transport instructions only; the behaviour brief and clean-room rules are shared.
  rendered.prompt = rendered.prompt
    .replace('- Shell: the Bash tool is Git Bash. Write Windows paths with forward slashes (C:/Users/...) or in single quotes: an unquoted backslash is eaten. PowerShell: `powershell -NoProfile -File <script>.ps1`.',
      '- Shell: exec_command uses PowerShell on Windows. Use forward-slash paths. For Bash syntax, run the quiet q.py wrapper. Use apply_patch for edits; cap shell output and avoid long inline scripts.')
    .replace('- Your cwd is the main checkout: never edit, commit, merge or `cd` there.',
      '- Your cwd is your lane worktree. Never edit, commit, merge or `cd` in the main checkout.')
    .replace('Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>',
      'Co-Authored-By: Codex <noreply@openai.com>')
    .replace('ctx.py <lane> --budget <budget>', 'codexguard.py status --manifest <MANIFEST>')
    .replaceAll('--fresh', '');
  rendered.prompt += '\n\n## Codex transport\nThe coordinator has created and reserved this worktree. Reuse it. Use only local shell/edit tools and the explicitly configured REA tools. Do not spawn agents, workflows or model CLI processes. The shared lane guard enforces every supported tool call; when denied, write only the permitted handoff and commit it, then return the required structured result. No tools or MCP servers may be added.\n';
  rendered.prompt += `- <MANIFEST> = ${request.manifest}\n`;
  const worktree = `${request.args.main || 'C:/Users/tlori/FreeCity4'}/.claude/worktrees/${request.args.name}`;
  rendered.prompt += `- Before any other tool, run exactly this single shell command to initialize measured context accounting: git -C "${worktree}" rev-parse --show-toplevel\n`;
  // Codex strict JSON schemas require every property in required, and no extra properties.
  rendered.schema.required = Object.keys(rendered.schema.properties);
  rendered.schema.additionalProperties = false;
}
process.stdout.write(JSON.stringify(rendered));

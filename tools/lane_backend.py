"""One reserved lane round, using Claude Workflow or Codex subscription CLI.

This is the worker transport. lane_launch.py owns scheduling and round reservations.
It never changes providers automatically, resumes a model conversation, or merges code.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import tomllib

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / '.claude/lane-bin'))
import codexguard
import laneguard

NOWIN = subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0


def write_json(path, value):
    laneguard.write_json(str(path), value)


def render(args, provider, manifest):
    result = subprocess.run(['node', str(REPO/'tools/lane_prompt.mjs')],
        input=json.dumps(dict(args=args, provider=provider, manifest=str(manifest).replace('\\', '/'))),
        capture_output=True, text=True, encoding='utf-8', timeout=20, creationflags=NOWIN)
    if result.returncode:
        raise RuntimeError('Prompt renderer failed: ' + result.stderr[-500:])
    return json.loads(result.stdout)


def codex_environment():
    # Account auth remains in its original credential store. Never copy/print auth.json or accept API fallback.
    return {k: v for k, v in os.environ.items()
            if k not in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL', 'OPENAI_API_BASE')
            and (not k.startswith('CODEX_') or k == 'CODEX_HOME')}


def codex_defaults():
    home = Path(os.environ.get('CODEX_HOME', str(Path.home()/'.codex')))
    p = home/'config.toml'
    d = tomllib.loads(p.read_text(encoding='utf-8')) if p.is_file() else {}
    return {k: d[k] for k in ('model', 'model_reasoning_effort') if k in d}


def codex_command(m, schema, final, env):
    cli = shutil.which('codex')
    if not cli:
        raise RuntimeError('Codex CLI is not installed')
    login = subprocess.run([cli, 'login', 'status'], capture_output=True, text=True,
                           env=env, timeout=20, creationflags=NOWIN)
    if login.returncode or 'Logged in using ChatGPT' not in login.stdout + login.stderr:
        raise RuntimeError('Codex requires ChatGPT subscription login; run codex login. API fallback is disabled.')
    guard_script = (REPO/'.claude/lane-bin/codexguard.py').as_posix()
    # Forward slashes are required here: Windows hook commands are interpreted by a shell.
    command = f'python "{guard_script}" hook --manifest "{Path(m["manifest"]).as_posix()}"'
    hook = '[{hooks=[{type="command",command=' + json.dumps(command) + ',timeout=15}]}]'
    args = [cli, '--no-daemon', '--dangerously-bypass-hook-trust', '--enable', 'hooks',
            '-c', 'hooks.PreToolUse=' + hook, '-c', 'hooks.SessionStart=' + hook,
            '-c', 'forced_login_method="chatgpt"', '-c', 'model_provider="openai"',
            '-c', 'features.multi_agent=false', '-c', 'web_search="disabled"',
            '-c', 'approval_policy="never"', '-c', 'tool_output_token_limit=2000']
    options = codex_defaults() | m.get('codex', {})
    for key in ('model', 'model_reasoning_effort'):
        if options.get(key):
            args += ['-c', key + '=' + json.dumps(options[key])]
    if m['kind'] == 're':
        cfg = laneguard.rea_config()
        if not cfg:
            raise RuntimeError('REA configuration is missing')
        # The same real server as Claude, with only the two lane-approved tools.
        for key in ('command', 'args'):
            args += ['-c', 'mcp_servers.rea.' + key + '=' + json.dumps(cfg.get(key, []))]
        for key, value in (cfg.get('env') or {}).items():
            args += ['-c', 'mcp_servers.rea.env.' + json.dumps(key) + '=' + json.dumps(value)]
        args += ['-c', 'mcp_servers.rea.enabled_tools=["procedure_pseudo_code","batch_decompile"]']
    args += ['exec', '--ignore-user-config', '--strict-config', '--json',
             '--sandbox', options.get('sandbox', 'workspace-write'), '-C', m['worktree'],
             '--add-dir', str(Path(m['main'])/'build-runs'),
             '--add-dir', str(Path(m['main'])/'.git'),
             '--output-schema', str(schema), '-o', str(final), '-']
    return args


def validate_result(result, m):
    if not isinstance(result, dict) or result.get('lane') != m['lane']:
        raise ValueError('Wrong or missing lane in provider result')
    if result.get('state') not in ('ready', 'wip', 'blocked', 'continue'):
        raise ValueError('Invalid lane result state')
    if not isinstance(result.get('summary'), str) or len(result['summary']) > 700:
        raise ValueError('Invalid lane summary')
    if result['state'] == 'ready':
        tip = result.get('tip', '')
        if not re.fullmatch(r'[0-9a-f]{7,40}', tip):
            raise ValueError('Ready result requires a real commit')
        head = laneguard.worktree_head(m['worktree'])
        if not head.startswith(tip):
            raise ValueError('Ready result tip differs from worktree HEAD')
        dirty = subprocess.run(['git', '-C', m['worktree'], 'status', '--porcelain', '--untracked-files=no',
                                '--ignore-submodules'], capture_output=True, text=True, creationflags=NOWIN)
        if dirty.returncode or dirty.stdout.strip():
            raise ValueError('Ready result has uncommitted tracked changes')
    return result


def run_codex(m):
    manifest = Path(m['manifest'])
    rendered = render(m['args'], 'codex', manifest)
    prompt, schema = manifest.with_suffix('.prompt.txt'), manifest.with_suffix('.schema.json')
    final, events = manifest.with_suffix('.final.json'), manifest.with_suffix('.events.jsonl')
    prompt.write_text(rendered['prompt'], encoding='utf-8')
    write_json(schema, rendered['schema'])
    env = codex_environment()
    command = codex_command(m, schema, final, env)
    with prompt.open('rb') as inp, events.open('wb') as out:
        proc = subprocess.Popen(command, cwd=m['worktree'], stdin=inp, stdout=out,
                                env=env, creationflags=NOWIN)
        while proc.poll() is None:
            if manifest.with_suffix('.fatal.json').exists():
                proc.terminate()
                proc.wait(timeout=20)
                raise RuntimeError('Codex hook failed; round stopped. See manifest fatal diagnostic.')
            time.sleep(.2)
    status = laneguard.read_json(m['status_path'], {})
    if not status.get('session_id') or not status.get('tool_calls'):
        raise RuntimeError('Codex guard handshake/tool coverage missing; result rejected')
    usage, errors = {}, []
    for line in events.read_text(encoding='utf-8').splitlines():
        try:
            e = json.loads(line)
        except ValueError:
            continue
        if e.get('type') == 'turn.completed':
            usage = e.get('usage', {})
        if e.get('type') in ('turn.failed', 'error'):
            errors.append(str(e)[:800])
    if proc.returncode or errors:
        raise RuntimeError('Codex round failed: ' + '; '.join(errors or [str(proc.returncode)]))
    result = validate_result(json.loads(final.read_text(encoding='utf-8')), m)
    codexguard.record_result(m, result)
    return dict(type='result', subtype='success', provider='codex', model=status.get('model'),
                is_error=False, result=json.dumps(result), structured_output=result,
                session_id=status['session_id'], usage=usage)


def run_claude(m):
    candidate = Path(os.environ.get('APPDATA', ''))/'npm/node_modules/@anthropic-ai/claude-code/bin/claude.exe'
    cli = m.get('claude_cli') or (str(candidate) if candidate.is_file() else shutil.which('claude'))
    if not cli:
        raise RuntimeError('Claude Code CLI is not installed')
    prompt = ('Invoke the saved guarded Workflow fc4-lane exactly ONCE with these args. '
              'Never direct Agent, never unguarded fallback. Wait for TaskOutput block=true until done '
              '(repeat on timeout); return the lane result. Do not spawn any other lane or restart this round. Args: '
              + json.dumps(m['args']))
    allowed = ['Workflow', 'TaskOutput', 'Bash', 'Read', 'Edit', 'Write', 'Grep', 'Glob']
    if m['kind'] == 're':
        allowed += ['mcp__rea__procedure_pseudo_code', 'mcp__rea__batch_decompile']
    env = dict(os.environ, CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS='0')
    raw = Path(m['manifest']).with_suffix('.claude.json')
    with raw.open('wb') as out:
        p = subprocess.run([cli, '-p', '--permission-mode', 'dontAsk', '--allowedTools', *allowed,
                            '--output-format', 'json'], input=prompt.encode('utf-8'), stdout=out,
                           cwd=m['main'], env=env, creationflags=NOWIN)
    result = json.loads(raw.read_text(encoding='utf-8'))
    result['provider'] = 'claude'
    if p.returncode:
        result['is_error'] = True
    return result


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--manifest', required=True)
    a = ap.parse_args()
    m = json.loads(Path(a.manifest).read_text(encoding='utf-8'))
    try:
        # The launcher commits the reservation immediately after Popen; never run before it appears.
        for _ in range(50):
            state = laneguard.read_json(m['state'], {})
            entry = state.get('lanes', {}).get(m['lane'], {})
            if entry.get('pid') == os.getpid() and entry.get('rounds') == m['round']:
                break
            time.sleep(.1)
        else:
            raise RuntimeError('No matching shared-ledger round reservation')
        code, line = laneguard.preflight(m['main'], m['lane'], m['kind'])
        if code:
            raise RuntimeError('Round not started: ' + line)
        result = run_codex(m) if m['provider'] == 'codex' else run_claude(m)
    except Exception as e:
        result = dict(type='result', subtype='error', provider=m['provider'], is_error=True,
                      result=f'{type(e).__name__}: {e}')
    write_json(m['result'], result)
    return 1 if result.get('is_error') else 0


if __name__ == '__main__':
    sys.exit(main())

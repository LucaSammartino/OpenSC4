"""One reserved lane round, using Claude Workflow or Codex subscription CLI.

This is the worker transport. lane_launch.py owns scheduling and round reservations.
It never changes providers automatically or merges code. Transport retries resume the same conversation.
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
    env = {k: v for k, v in os.environ.items()
            if k not in ('OPENAI_API_KEY', 'CODEX_API_KEY', 'OPENAI_BASE_URL', 'OPENAI_API_BASE')
            and (not k.startswith('CODEX_') or k == 'CODEX_HOME')}
    return env


def git_output(m, *args):
    return subprocess.check_output(['git', '-C', m['worktree'], *args],
        text=True, encoding='utf-8', stderr=subprocess.PIPE, creationflags=NOWIN).strip()


def prepare_codex_git(m):
    """Commit on a detached HEAD; only the supervisor publishes the lane's branch."""
    if not re.fullmatch(r'[a-z0-9][a-z0-9_-]*', m['lane']):
        raise ValueError('Invalid Git lane name')
    ref = 'refs/heads/' + m['lane']
    if git_output(m, 'symbolic-ref', '-q', 'HEAD') != ref:
        raise ValueError('Codex worktree must start on its own lane branch')
    common = Path(git_output(m, 'rev-parse', '--path-format=absolute', '--git-common-dir')).resolve()
    metadata = Path(git_output(m, 'rev-parse', '--absolute-git-dir')).resolve()
    if common != (Path(m['main'])/'.git').resolve() or not metadata.is_relative_to(common/'worktrees'):
        raise ValueError('Codex requires a linked lane worktree in the registered repository')
    base = git_output(m, 'rev-parse', 'HEAD')
    m.update(git_ref=ref, git_base=base, git_common=str(common), git_dir=str(metadata))
    # Persist recovery information before changing HEAD; no checkout/files are changed.
    write_json(m['manifest'], m)
    git_output(m, 'checkout', '--detach', '--quiet')


def publish_codex_git(m):
    """Compare-and-swap only this lane's ref, retaining detached commits on conflicts."""
    ref, base = m['git_ref'], m['git_base']
    if ref != 'refs/heads/' + m['lane']:
        raise ValueError('Git publication ref differs from the registered lane')
    head = git_output(m, 'rev-parse', 'HEAD')
    current = git_output(m, 'rev-parse', '--verify', ref)
    symbolic = subprocess.run(['git', '-C', m['worktree'], 'symbolic-ref', '-q', 'HEAD'],
        capture_output=True, text=True, creationflags=NOWIN)
    if symbolic.returncode == 0 and symbolic.stdout.strip() != ref:
        raise ValueError('Refusing to publish another checked-out branch')
    git_output(m, 'merge-base', '--is-ancestor', base, head)
    if current not in (base, head):
        raise ValueError('Lane branch moved concurrently; detached work preserved')
    if current != head:
        git_output(m, 'update-ref', ref, head, base)
    git_output(m, 'symbolic-ref', 'HEAD', ref)
    m['git_published'] = head
    write_json(m['manifest'], m)


def codex_git_permissions(m):
    common, metadata = Path(m['git_common']).resolve(), Path(m['git_dir']).resolve()
    if common != (Path(m['main'])/'.git').resolve() or not metadata.is_relative_to(common/'worktrees'):
        raise ValueError('Invalid Codex Git permission roots')
    # A whole TOML table preserves dots in path keys. Shared refs/config/hooks stay read-only.
    # Do not add an explicit read-only common-dir carveout: Codex exec's Windows
    # runtime cannot represent writable descendants of one. The inherited root
    # read policy already keeps common refs/config/hooks read-only.
    rules = {metadata.as_posix(): 'write', (common/'objects').as_posix(): 'write'}
    table = '{' + ','.join(json.dumps(k)+'='+json.dumps(v) for k, v in rules.items()) + '}'
    return ['-c', 'default_permissions="fc4-lane"',
            '-c', 'permissions.fc4-lane.extends=":workspace"',
            '-c', 'permissions.fc4-lane.filesystem=' + table]


def codex_defaults():
    home = Path(os.environ.get('CODEX_HOME', str(Path.home()/'.codex')))
    p = home/'config.toml'
    d = tomllib.loads(p.read_text(encoding='utf-8')) if p.is_file() else {}
    result = {k: d[k] for k in ('model', 'model_reasoning_effort') if k in d}
    # Pinned, not read from config.toml: the elevated sandbox logs on as the
    # CodexSandboxOffline account for every command and never closes the logon,
    # so a swarm leaks tens of thousands of lsass sessions (2026-10-23).
    result['windows.sandbox'] = 'unelevated'
    return result


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
            '-c', 'approval_policy="never"', '-c', 'tool_output_token_limit=2000',
            '-c', 'shell_environment_policy.set={GIT_CONFIG_COUNT="2",GIT_CONFIG_KEY_0="gc.auto",'
                  'GIT_CONFIG_VALUE_0="0",GIT_CONFIG_KEY_1="maintenance.auto",GIT_CONFIG_VALUE_1="false",'
                  # 3 jobs per lane's cargo: memory, not CPU, is the limit (2026-10-24, 13 lanes: ~25 rustc held
                  # 53-60 GB, commit 98-99%, and the paged-out games stalled the regress) while CPU averaged 32%
                  'CARGO_BUILD_JOBS="3"}']
    options = codex_defaults() | m.get('codex', {})
    if options.get('sandbox', 'workspace-write') != 'workspace-write':
        raise ValueError('Codex lanes require the scoped workspace permission profile')
    if options.get('windows.sandbox') == 'elevated':
        raise ValueError('The elevated Windows sandbox leaks logon sessions; use unelevated')
    args += codex_git_permissions(m)
    for key in ('model', 'model_reasoning_effort', 'windows.sandbox'):
        if options.get(key):
            args += ['-c', key + '=' + json.dumps(options[key])]
    tier = m.get('args', {}).get('serviceTier', options.get('service_tier'))
    if tier is not None:
        if tier not in ('fast', 'default'):
            raise ValueError('Codex service_tier must be fast or default')
        args += ['-c', 'service_tier=' + json.dumps(tier)]
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
             '-C', m['worktree'],
             '--add-dir', str(Path(m['main'])/'build-runs'),
             '--output-schema', str(schema), '-o', str(final), '-']
    return args


def codex_resume_command(command, session_id):
    # CLI 0.163: cwd/add-dir belong to exec, not its resume subcommand.
    index = command.index('exec')
    parent, options = command[:index + 1], command[index + 1:-1]
    for flag in ('-C', '--add-dir'):
        while flag in options:
            i = options.index(flag)
            parent += options[i:i + 2]
            del options[i:i + 2]
    return parent + ['resume', *options, session_id, '-']


TRANSPORT_ERROR = re.compile(
    r'Reconnecting|workspace routing discovery|stream disconnected|request timed out|'
    r'No such host is known|error sending request', re.I)
RESUME_PROMPT = ('The connection dropped. Continue the same task from where you stopped; '
                 're-check git status first.')


class CodexRoundError(RuntimeError):
    def __init__(self, message, usage):
        super().__init__(message)
        self.usage = dict(usage)


def codex_events(path, offset, usage):
    errors, session_id = [], None
    with path.open('rb') as stream:
        stream.seek(offset)
        for line in stream:
            try:
                event = json.loads(line)
            except (ValueError, UnicodeError):
                continue
            if event.get('type') == 'thread.started':
                session_id = event.get('thread_id')
            if event.get('type') == 'turn.completed':
                for key, value in (event.get('usage') or {}).items():
                    if isinstance(value, (int, float)):
                        usage[key] = usage.get(key, 0) + value
            if event.get('type') in ('turn.failed', 'error'):
                message = event.get('message') or event.get('error') or event
                if isinstance(message, dict):
                    message = message.get('message', message)
                errors.append(str(message))
    return errors, session_id


def stop_codex(proc):
    if os.name == 'nt':
        subprocess.run(['taskkill', '/PID', str(proc.pid), '/T', '/F'],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, creationflags=NOWIN)
    else:
        proc.terminate()
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()


def update_codex_status(m, **fields):
    path = Path(m['status_path'])
    with codexguard.locked(path.with_suffix('.lock')):
        status = laneguard.read_json(str(path), {})
        status.update(fields)
        write_json(path, status)
    return status


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


def run_codex(m, *, clock=None, sleep=None):
    clock, sleep = clock or time.time, sleep or time.sleep
    _, hard_limit = codexguard.round_limits(m)
    manifest = Path(m['manifest'])
    rendered = render(m['args'], 'codex', manifest)
    prompt, schema = manifest.with_suffix('.prompt.txt'), manifest.with_suffix('.schema.json')
    final, events = manifest.with_suffix('.final.json'), manifest.with_suffix('.events.jsonl')
    prompt.write_text(rendered['prompt'], encoding='utf-8')
    write_json(schema, rendered['schema'])
    env = codex_environment()
    command = codex_command(m, schema, final, env)
    status = update_codex_status(m, round_started=clock(), paused_seconds=0)
    usage, resumes = {}, []
    session_id = status.get('session_id')
    events.write_bytes(b'')
    for attempt in range(4):
        if final.exists():
            final.replace(final.with_suffix(f'.before-attempt-{attempt}.json'))
        offset = events.stat().st_size
        timed_out, fatal = False, False
        with prompt.open('rb') as inp, events.open('ab') as out:
            # Children inherit below-normal priority so the gate and game queue win the CPU.
            below = subprocess.BELOW_NORMAL_PRIORITY_CLASS if os.name == 'nt' else 0
            proc = subprocess.Popen(command, cwd=m['worktree'], stdin=inp, stdout=out,
                                    env=env, creationflags=NOWIN | below)
            try:
                while proc.poll() is None:
                    fatal = manifest.with_suffix('.fatal.json').exists()
                    timed_out = codexguard.round_elapsed(status, clock()) >= hard_limit
                    if fatal or timed_out:
                        stop_codex(proc)
                        break
                    sleep(.2)
            finally:
                if proc.poll() is None:
                    stop_codex(proc)
        errors, thread_id = codex_events(events, offset, usage)
        if fatal or manifest.with_suffix('.fatal.json').exists():
            raise CodexRoundError('Codex hook failed; round stopped. See manifest fatal diagnostic.', usage)
        if timed_out or codexguard.round_elapsed(status, clock()) >= hard_limit:
            status = update_codex_status(m, round_timed_out=True)
            result = dict(lane=m['lane'], tip=laneguard.worktree_head(m['worktree']), state='continue',
                          gate='n/a', game='n/a', questions='none', abi=False,
                          summary='Round time budget reached; committed work preserved.',
                          next=['Continue from git status and the lane notes.'])
            codexguard.record_result(m, result)
            return dict(type='result', subtype='success', provider='codex', is_error=False,
                        result=json.dumps(result), structured_output=result, usage=usage,
                        session_id=status.get('session_id'), model=status.get('model'))
        status = laneguard.read_json(m['status_path'], {})
        observed_session = status.get('session_id') or thread_id
        if (session_id and observed_session and session_id != observed_session) or (
                thread_id and observed_session != thread_id):
            raise CodexRoundError('Codex thread differs from the guard session', usage)
        session_id = session_id or observed_session
        if not proc.returncode and not errors:
            break
        if attempt == 3 or not session_id or not errors or not all(TRANSPORT_ERROR.search(e) for e in errors):
            raise CodexRoundError('Codex round failed: ' + '; '.join(errors or [str(proc.returncode)]), usage)
        wait_started = clock()
        resumes.append(dict(time=laneguard.now_iso(), attempt=attempt + 1, error='; '.join(errors)))
        status = update_codex_status(m, resumes=resumes, resume_wait_started=wait_started)
        sleep((60, 180, 600)[attempt])
        status = update_codex_status(m, paused_seconds=status.get('paused_seconds', 0) + clock() - wait_started,
                                     resume_wait_started=None)
        if attempt == 0:
            command = codex_resume_command(command, session_id)
            prompt = manifest.with_suffix('.resume.prompt.txt')
        prompt.write_text(RESUME_PROMPT, encoding='utf-8')
    if not status.get('session_id') or not status.get('tool_calls'):
        raise CodexRoundError('Codex guard handshake/tool coverage missing; result rejected', usage)
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
        if m['provider'] == 'codex':
            prepare_codex_git(m)
        result = run_codex(m) if m['provider'] == 'codex' else run_claude(m)
    except Exception as e:
        result = dict(type='result', subtype='error', provider=m['provider'], is_error=True,
                      result=f'{type(e).__name__}: {e}')
        if isinstance(e, CodexRoundError):
            result['usage'] = e.usage
    if m['provider'] == 'codex' and m.get('git_base'):
        try:
            publish_codex_git(m)
        except Exception as e:
            result = dict(type='result', subtype='error', provider='codex', is_error=True,
                          result=f'Git publication failed; detached work preserved: {type(e).__name__}: {e}')
    write_json(m['result'], result)
    return 1 if result.get('is_error') else 0


if __name__ == '__main__':
    sys.exit(main())

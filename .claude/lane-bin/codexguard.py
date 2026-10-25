#!/usr/bin/env python3
"""Codex transport adapter for laneguard's shared policy. No model or project logic here."""
import argparse
import contextlib
import json
import math
import os
from pathlib import Path
import re
import sys
import time

import laneguard as guard


@contextlib.contextmanager
def locked(path):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a+b') as f:
        if f.seek(0, 2) == 0:
            f.write(b'0')
            f.flush()
        if os.name == 'nt':
            import msvcrt
            while True:
                try:
                    f.seek(0)
                    msvcrt.locking(f.fileno(), msvcrt.LK_NBLCK, 1)
                    break
                except OSError:
                    time.sleep(.02)
        else:
            import fcntl
            fcntl.flock(f, fcntl.LOCK_EX)
        try:
            yield
        finally:
            if os.name == 'nt':
                f.seek(0)
                msvcrt.locking(f.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(f, fcntl.LOCK_UN)


def context_tokens(transcript):
    """Latest model input, including cached input ONCE, never the cumulative billed total.

    Rollout format is version-sensitive: missing/malformed telemetry blocks work, rather than
    silently removing the budget. A partial final JSONL record can be ignored.
    """
    with open(transcript, 'rb') as f:
        f.seek(0, 2)
        f.seek(max(0, f.tell() - 8 * 1024 * 1024))
        lines = f.read().splitlines()
    for line in reversed(lines):
        try:
            record = json.loads(line)
        except (ValueError, UnicodeError):
            continue
        payload = record.get('payload', {})
        if record.get('type') == 'event_msg' and payload.get('type') == 'token_count':
            info = payload.get('info') or {}
            value = (info.get('last_token_usage') or {}).get('input_tokens')
            if isinstance(value, int) and value > 0:
                return value
    raise ValueError('Codex context telemetry missing; no tool execution until usage is available')


def patch_paths(patch, wt):
    if not isinstance(patch, str) or not patch.startswith('*** Begin Patch'):
        return []
    paths = re.findall(r'^\*\*\* (?:Add File|Update File|Delete File|Move to): (.+)$', patch, re.M)
    return [str((Path(wt) / p.strip()).resolve()) for p in paths]


def handoff_allowed(tool, tin, wt, lane):
    if tool == 'apply_patch':
        paths = patch_paths(tin.get('command', ''), wt)
        return bool(paths) and all(guard.file_allowed(p, wt, lane) for p in paths)
    return guard.call_allowed_when_tripped(tool, tin, wt, lane)


def validate_manifest(m):
    if m.get('provider') != 'codex' or not re.fullmatch(r'[a-z0-9][a-z0-9_-]*', m.get('lane', '')):
        raise ValueError('Invalid Codex lane registration')
    if Path(m['worktree']).resolve() != Path(guard.worktree(m['main'], m['lane'])).resolve():
        raise ValueError('Lane registration worktree mismatch')
    if not re.fullmatch(r'codex-[a-f0-9]{32}', m.get('guard_id', '')):
        raise ValueError('Invalid guard identity')
    if m['budget'] <= 0 or m['max_calls'] <= 0:
        raise ValueError('Invalid guard limits')


def bootstrap_command(m):
    # Codex persists its first usage event after its first tool call. Before that, permit only
    # this read-only identity probe. No project work runs without measured context telemetry.
    return f'git -C "{Path(m["worktree"]).as_posix()}" rev-parse --show-toplevel'


ROUND_MESSAGE = ('Round time budget reached: commit what you have, write the merge-queue entry '
                 '(ready if the gate already passed, otherwise continue) and return the result now.')


def round_limits(m):
    minutes = m.get('args', {}).get('roundMinutes', 40)
    if isinstance(minutes, bool) or not isinstance(minutes, (int, float)) or not math.isfinite(minutes) or minutes <= 0:
        raise ValueError('roundMinutes must be a positive finite number')
    # SPEC-GAP: overrides move both thresholds, keeping five minutes on either side of the budget.
    return max(0, minutes - 5) * 60, (minutes + 5) * 60


def round_elapsed(status, now):
    paused = status.get('paused_seconds', 0)
    if status.get('resume_wait_started') is not None:
        paused += max(0, now - status['resume_wait_started'])
    return max(0, now - status.get('round_started', now) - paused)


def round_handoff_allowed(tool, tin, wt, lane):
    if tool in ('Read', 'Grep', 'Glob'):
        return True
    if tool == 'Bash':
        command = tin.get('command', '')
        if guard.bash_allowed(command, wt):
            return True
        # Shell readers are needed by Codex's exec tool; reject evaluation, redirection and rg subprocess flags.
        if re.search(r'[;&<>`$()\r\n]', command) or re.search(r'--(pre|hostname-bin)\b', command):
            return False
        parts = command.split('|')
        return bool(re.match(r'^\s*(Get-Content|rg|cat|head|tail)\s+', parts[0])) and all(
            re.fullmatch(r'\s*(Select-Object\s+[-\w\s]+|(?:head|tail)\s+-n\s*\d+)\s*', p)
            for p in parts[1:])
    queue = Path(wt) / 'build-runs' / 'merge-queue' / (lane + '.json')
    if tool == 'apply_patch':
        paths = patch_paths(tin.get('command', ''), wt)
        return bool(paths) and all(Path(p) == queue.resolve() for p in paths)
    if tool in ('Write', 'Edit'):
        return Path(tin.get('file_path', '')).resolve() == queue.resolve()
    return False


def handle(data, manifest, *, clock=None):
    clock = clock or time.time
    validate_manifest(manifest)
    soft_limit, hard_limit = round_limits(manifest)
    m = manifest
    status_path = Path(m['status_path'])
    with locked(status_path.with_suffix('.lock')):
        status = guard.read_json(str(status_path), {})
        event = data.get('hook_event_name')
        if event == 'SessionStart':
            if Path(data['cwd']).resolve() != Path(m['worktree']).resolve():
                raise ValueError('Codex started outside its reserved worktree')
            if status.get('session_id') not in (None, data['session_id']):
                raise ValueError('A lane round must not start a second Codex session')
            status.setdefault('started', guard.now_iso())
            status.setdefault('round_started', clock())
            status.update(session_id=data['session_id'], model=data.get('model'),
                          transcript=data.get('transcript_path'))
            guard.write_json(str(status_path), status)
            return 'allow', ''
        if event != 'PreToolUse':
            return 'allow', ''
        if not status.get('session_id') or data.get('session_id') != status['session_id']:
            raise ValueError('Missing or mismatched SessionStart guard handshake')
        if data.get('transcript_path') != status['transcript']:
            raise ValueError('Codex transcript changed within a lane round')
        elapsed = round_elapsed(status, clock())
        if elapsed >= hard_limit:
            return 'deny', ROUND_MESSAGE
        tool = data.get('tool_name', '')
        tin = data.get('tool_input') or {}
        if elapsed >= soft_limit and not round_handoff_allowed(tool, tin, m['worktree'], m['lane']):
            return 'deny', ROUND_MESSAGE
        tool_id = data.get('tool_use_id')
        if tool_id and tool_id == status.get('last_tool_id'):
            return tuple(status['last_decision'])
        if tool in ('spawn_agent', 'Agent', 'Workflow'):
            decision = ('deny', 'Lanes cannot spawn workers; only the guarded coordinator reserves rounds.')
        else:
            try:
                tokens = context_tokens(status['transcript'])
            except (OSError, ValueError):
                if status.get('bootstrap_done'):
                    raise
                if tool != 'Bash' or tin.get('command', '').strip() != bootstrap_command(m):
                    return ('deny', 'LANE GUARD - context handshake required. Run exactly this single shell '
                            'command first: ' + bootstrap_command(m))
                tokens = 0
                status['bootstrap_done'] = True
            def usage(path):
                if tokens:
                    status['context_tokens'] = tokens
                return tokens
            decision = guard.guard_call(m['main'], m['lane'], m['guard_id'],
                f"lane:{m['lane']}#{m['round']}", tool, tin, status['transcript'],
                limits=(m['budget'], m['max_calls']), context_reader=usage,
                allowed_when_tripped=handoff_allowed)
        status.update(last_tool_id=tool_id, last_decision=list(decision), last_hook=guard.now_iso(),
                      tool_calls=status.get('tool_calls', 0) + 1)
        guard.write_json(str(status_path), status)
        return decision


def record_result(m, result):
    """Same rounds ledger as Claude; switching providers never removes parking evidence."""
    validate_manifest(m)
    path = Path(guard.rounds_path(m['main'], m['lane']))
    with locked(path.with_suffix('.lock')):
        if any(r.get('agent_id') == m['guard_id'] for r in guard.read_rounds(m['main'], m['lane'])):
            return
        agent = guard.read_json(str(Path(guard.guard_dir(m['main']))/'agents'/f"{m['guard_id']}.json"), {})
        trip = agent.get('trip')
        if trip in ('stop', 'watchdog', 'park'):
            result['state'] = 'blocked'
        elif trip == 'budget':
            result['state'] = 'continue'
        guard.append_line(str(path), json.dumps(dict(ts=guard.now_iso(), agent_id=m['guard_id'],
            label=f"lane:{m['lane']}#{m['round']}", provider='codex', guard=trip,
            **{k: result.get(k) for k in ('state', 'tip', 'questions', 'summary')})))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('command', choices=('hook', 'status'))
    ap.add_argument('--manifest', required=True)
    a = ap.parse_args()
    try:
        m = json.loads(Path(a.manifest).read_text(encoding='utf-8'))
        if a.command == 'status':
            s = guard.read_json(m['status_path'], {})
            value = s.get('context_tokens')
            print(f"context {value if value is not None else 'unavailable'} / {m['budget']}; "
                  f"{'OVER' if value and value > m['budget'] else 'within budget'}")
            return 0
        decision, message = handle(json.load(sys.stdin), m)
        if decision == 'deny':
            guard.emit_deny(message)
    except Exception as e:
        # Codex treats hook process errors as nonblocking: emit a supported denial on every error.
        guard.emit_deny(f'LANE GUARD - Codex adapter error: {type(e).__name__}: {e}. Stop and report blocked.')
        try:
            p = Path(a.manifest).with_suffix('.fatal.json')
            guard.write_json(str(p), {'error': str(e), 'ts': guard.now_iso()})
        except OSError:
            pass
    return 0


if __name__ == '__main__':
    sys.exit(main())

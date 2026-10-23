"""Refill reviewed briefs through lane_launch. Explicit provider; no automatic account/provider failover."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

import lane_launch as launcher
import lane_evidence
import lane_supervision as supervision

LIMIT_RE = re.compile(r'rate_limit_error|usage_limit|rate limit (?:reached|exceeded)|'
                      r'usage limit (?:reached|exceeded)|quota exceeded|insufficient credits|'
                      r'out of (?:extra )?usage|you.{0,16}(?:hit|reached).{0,24}limit', re.I)


def provider_error(result):
    return bool(result.get('is_error') or LIMIT_RE.search(json.dumps(result)))


def spec_deps(spec):
    deps = spec['args'].get('deps', [])
    return deps.split(',') if isinstance(deps, str) else deps


def unmet_deps(root, deps):
    """The deps that are not ancestors of main yet; a brief launches only when there are none."""
    return [d for d in deps if subprocess.run(['git','merge-base','--is-ancestor',d,'main'],cwd=root,
                                              capture_output=True,creationflags=launcher.backend.NOWIN).returncode]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--provider', choices=('claude', 'codex'))
    ap.add_argument('--root', type=Path, default=Path(launcher.backend.laneguard.main_root()))
    ap.add_argument('--queue', nargs='+', type=Path)
    ap.add_argument('--once', action='store_true')
    a = ap.parse_args()
    root = a.root.resolve()
    runs = root/'build-runs'
    state_path, queue_path = runs/'run4-coordinator-state.json', runs/'run4-refill-queue.json'
    read = launcher.backend.laneguard.read_json
    write = launcher.backend.write_json
    def locked():
        return launcher.backend.codexguard.locked(runs/'run4-coordinator.lock')
    def log(s):
        with (runs/'run4-refill.log').open('a', encoding='utf-8') as f:
            f.write(time.strftime('%H:%M:%S ') + s + '\n')
    if a.queue:
        with locked():
            q = read(str(queue_path), [])
            for path in a.queue:
                spec = read(str(path), None)
                if not spec or spec['args'].get('maxRounds', 1) != 1:
                    raise ValueError('A queued brief must reserve exactly one round')
                lane_evidence.preflight(root, spec, path)
                name = spec['args']['name']
                if any(e['name'] == name and e['status'] in ('pending','launching') for e in q):
                    raise ValueError('Already queued: ' + name)
                q.append(dict(name=name, spec=str(path.resolve()), status='pending'))
            write(queue_path, q)
        print('queued', len(a.queue))
        return
    try:
        with supervision.ownership(runs/'run4-refill-owner.lock'):
            return dispatch(a, root, runs, state_path, queue_path, read, write, locked, log)
    except (supervision.SupervisionError, OSError, ValueError) as exc:
        print(f'ERROR: refiller ownership/publication failed: {exc}', flush=True)
        return 1


def dispatch(a, root, runs, state_path, queue_path, read, write, locked, log):
    """Only entered with the exclusive dispatcher owner lock held through exit.

    Retain markers on every exit/crash: an unlocked or dead identity is unhealthy.
    Never deleting markers also prevents an old exit from deleting a newer owner.
    """
    with locked():
        state = read(str(state_path), {})
    provider = a.provider or state.get('provider', 'claude')
    supervision.publish_dispatcher(root, provider)
    seen = {e.get('result') for e in state.get('lanes', {}).values()
            if Path(e.get('result', '')).is_file() and read(e['result']) is not None}
    log('START reviewed briefs; provider=' + provider + '; all shared guards retained')
    while True:
        if (runs/'STOP').exists():
            log('STOP set; refiller exits')
            return
        if launcher.provider_pause(root, provider):
            log('PAUSE ' + provider + '; audit provider availability before resuming')
            return
        launcher.launch(root, state_path)
        with locked():
            state, q = read(str(state_path), {}), read(str(queue_path), [])
        for entry in state.get('lanes', {}).values():
            path = entry.get('result', '')
            result = read(path, None) if Path(path).is_file() else None
            if not isinstance(result, dict) or path in seen:
                continue
            seen.add(path)
            if provider_error(result):
                source = entry.get('provider', result.get('provider', 'claude'))
                write(runs/'provider-pauses'/f'{source}.json',
                      dict(provider=source, reason='provider error or usage/rate limit', result=path))
                log('PAUSE provider result ' + Path(path).name)
                return
        live = [n for n,e in state.get('lanes', {}).items() if launcher.alive(e.get('pid'))]
        glue = sum(state['lanes'][n].get('role') == 'glue' for n in live)
        if len(live) < state.get('concurrency', 7):
            for entry in q:
                if entry['status'] != 'pending':
                    continue
                spec = read(entry['spec'])
                role = spec.get('lane_role', 're' if spec['args']['kind'] == 're' else 'verify')
                if role == 'glue' and glue >= state.get('glue_limit', 3):
                    continue
                if unmet_deps(root, spec_deps(spec)):
                    continue
                selected = spec.get('provider', provider)
                if launcher.provider_pause(root, selected):
                    continue
                p = subprocess.run([sys.executable,str(root/'tools/lane_launch.py'),entry['spec'],
                                    '--root',str(root),'--provider',selected],cwd=root,capture_output=True,
                                   text=True,encoding='utf-8',errors='replace',creationflags=launcher.backend.NOWIN)
                message = (p.stdout+'\n'+p.stderr).strip()
                if p.returncode == 0 and 'LAUNCHED' in message:
                    decision = 'launched'
                elif re.search(r'at cap|cpu busy|low disk|exceeds85|live lane cap|glue lane cap',message,re.I):
                    decision = 'pending'
                elif 'STOP' in message:
                    log('STOP denied launch; exits')
                    return
                else:
                    decision = 'blocked'
                log(decision.upper() + ' ' + entry['name'] + ' ' + message.replace('\n',' ')[-700:])
                with locked():
                    current = read(str(queue_path), [])
                    for item in current:
                        if item['name'] == entry['name'] and item['status'] == 'pending':
                            item.update(status=decision, last=message[-1200:], provider=selected)
                            break
                    write(queue_path,current)
                break
        if a.once:
            return
        time.sleep(10)


if __name__ == '__main__':
    sys.exit(main())

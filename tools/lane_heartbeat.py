"""Read-only lane heartbeat. Counts registered live processes, never worktree activity.

Liveness is not progress: a live worker that has not committed for STALL_HOURS, or a ready merge-queue entry that has
waited LAND_LAG_HOURS, is an alert of its own (an idle lane must not go unnoticed for days)."""
import argparse
from collections import Counter
from datetime import datetime
import json
import os
from pathlib import Path
import subprocess
import time

import lane_launch

STALL_HOURS = 2.0      # a live worker with no commit (or launch) this long is stalled
LAND_LAG_HOURS = 2.0   # a ready queue entry waiting this long means the train is not being run


def summarize(state, queue, train, *, is_alive, paused=False, stopped=False,
              refill_pid=None, progress=None, ready=None, now=None):
    """progress: {lane: epoch of its last commit or launch}; ready: {lane: epoch its ready queue entry was written}."""
    lanes = state.get('lanes', {})
    live = sorted(name for name, entry in lanes.items() if is_alive(entry.get('pid')))
    target = state.get('concurrency', 7)
    pending = [entry['name'] for entry in queue if entry.get('status') == 'pending']
    current = train.get('current') or {}
    gate = current.get('gate') or {}
    regress = current.get('regress') or {}
    refill_alive = is_alive(refill_pid)
    alerts = []
    if not stopped and not paused:
        if len(live) < target:
            alerts.append('REFILL: live workers below target')
            if not pending:
                alerts.append('REVIEW QUEUE: no pending briefs')
        if not refill_alive:
            alerts.append('REFILLER: process is not running')
    if paused:
        alerts.append('PROVIDER PAUSED: verify availability before resuming')
    if stopped:
        alerts.append('STOP: no spawning')
    now = time.time() if now is None else now
    stalled = sorted(n for n in live if (progress or {}).get(n) and now - progress[n] > STALL_HOURS * 3600)
    if stalled:
        alerts.append(f'STALLED: no commit for {STALL_HOURS:g}h+ from ' + ', '.join(
            f'{n} ({(now - progress[n]) / 3600:.1f}h)' for n in stalled))
    waiting = sorted(n for n, at in (ready or {}).items() if now - at > LAND_LAG_HOURS * 3600)
    if waiting:
        alerts.append(f'LAND LAG: {len(waiting)} ready queue entr{"y" if len(waiting) == 1 else "ies"} waiting '
                      f'{LAND_LAG_HOURS:g}h+ (oldest {(now - min(ready[n] for n in waiting)) / 3600:.1f}h): python tools/merge-train.py status, then land or fix them')
    if gate.get('ok') and regress.get('ok'):
        alerts.append('LAND: gate and regress passed; coordinator review required')
    return dict(live=len(live), target=target, lanes=live,
                roles=dict(Counter(lanes[name].get('role', 'unknown') for name in live)),
                glue_limit=state.get('glue_limit', 3), pending=pending,
                queue_states=dict(Counter(entry.get('status', 'unknown') for entry in queue)),
                rounds=state.get('rounds'), provider=state.get('provider', 'claude'),
                provider_paused=paused, stop=stopped, refiller_alive=refill_alive,
                train=current.get('id'), gate_ok=gate.get('ok'),
                regress=regress.get('verdict'), stalled=stalled, land_lag=waiting, alerts=alerts)


def read_json(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def last_progress(root, name, entry):
    """Epoch of the lane worktree's last commit, or of its launch (manifest written), whichever is newer."""
    times = []
    manifest = entry.get('manifest')
    if manifest and Path(manifest).exists():
        times.append(Path(manifest).stat().st_mtime)
    wt = root / '.claude' / 'worktrees' / name
    if wt.is_dir():
        p = subprocess.run(['git', '-C', str(wt), 'log', '-1', '--format=%ct', 'HEAD'],
                           capture_output=True, text=True)
        if p.returncode == 0 and p.stdout.strip().isdigit():
            times.append(float(p.stdout.strip()))
    return max(times) if times else None


def ready_entries(root):
    """{lane: mtime} of merge-queue files in state ready (main checkout and every worktree; .merged files skipped)."""
    out = {}
    dirs = [root / 'build-runs' / 'merge-queue'] + sorted((root / '.claude' / 'worktrees').glob('*/build-runs/merge-queue'))
    for d in dirs:
        for f in sorted(d.glob('*.json')) if d.is_dir() else []:
            if f.name.endswith('.merged.json'):
                continue
            try:
                rec = read_json(f)
            except (OSError, ValueError):
                continue
            if rec.get('state', 'ready') == 'ready':
                lane = rec.get('lane') or f.stem
                out[lane] = min(out.get(lane, f.stat().st_mtime), f.stat().st_mtime)
    return out


def snapshot(root):
    runs = root / 'build-runs'
    state = read_json(runs / 'run4-coordinator-state.json')
    pid_path = runs / 'run4-refill.pid'
    refill_pid = int(pid_path.read_text().strip()) if pid_path.exists() else None
    lanes = state.get('lanes', {})
    progress = {n: last_progress(root, n, e) for n, e in lanes.items() if lane_launch.alive(e.get('pid'))}
    value = summarize(state, read_json(runs / 'run4-refill-queue.json'),
                      read_json(runs / 'merge-train/state.json'),
                      is_alive=lane_launch.alive, refill_pid=refill_pid,
                      paused=bool(lane_launch.provider_pause(root, state.get('provider', 'claude'))),
                      stopped=(runs / 'STOP').exists(),
                      progress={n: v for n, v in progress.items() if v}, ready=ready_entries(root))
    value['at'] = datetime.now().astimezone().isoformat(timespec='seconds')
    return value


def report(root):
    value = snapshot(root)
    runs = root / 'build-runs'
    output = runs / 'run4-heartbeat.json'
    temporary = output.with_name(output.name + f'.{os.getpid()}.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')
    temporary.replace(output)
    message = (f'{value["at"]} HEARTBEAT live {value["live"]}/{value["target"]}, '
               f'glue {value["roles"].get("glue", 0)}/{value["glue_limit"]}, '
               f'pending {len(value["pending"])}, rounds {value["rounds"]}; '
               f'train {value["train"]} gate={value["gate_ok"]} regress={value["regress"]}; '
               f'pause={value["provider_paused"]} STOP={value["stop"]}; ' + '; '.join(value['alerts']))
    with (runs / 'run4-heartbeat.log').open('a', encoding='utf-8') as log:
        log.write(message + '\n')
    print(message, flush=True)
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--once', action='store_true', help='For a scheduler or coordinator poll')
    parser.add_argument('--interval', type=int, default=300)
    args = parser.parse_args()
    if args.interval < 1:
        parser.error('--interval must be positive')
    while True:
        value = report(args.root.resolve())
        if args.once or value['stop']:
            return
        time.sleep(args.interval)


if __name__ == '__main__':
    main()

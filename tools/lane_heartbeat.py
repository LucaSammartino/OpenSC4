"""Read-only lane heartbeat. Counts registered live processes, never worktree activity."""
import argparse
from collections import Counter
from datetime import datetime
import json
import os
from pathlib import Path
import time

import lane_launch


def summarize(state, queue, train, *, is_alive, paused=False, stopped=False,
              refill_pid=None):
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
    if gate.get('ok') and regress.get('ok'):
        alerts.append('LAND: gate and regress passed; coordinator review required')
    return dict(live=len(live), target=target, lanes=live,
                roles=dict(Counter(lanes[name].get('role', 'unknown') for name in live)),
                glue_limit=state.get('glue_limit', 3), pending=pending,
                queue_states=dict(Counter(entry.get('status', 'unknown') for entry in queue)),
                rounds=state.get('rounds'), provider=state.get('provider', 'claude'),
                provider_paused=paused, stop=stopped, refiller_alive=refill_alive,
                train=current.get('id'), gate_ok=gate.get('ok'),
                regress=regress.get('verdict'), alerts=alerts)


def read_json(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def snapshot(root):
    runs = root / 'build-runs'
    state = read_json(runs / 'run4-coordinator-state.json')
    pid_path = runs / 'run4-refill.pid'
    refill_pid = int(pid_path.read_text().strip()) if pid_path.exists() else None
    value = summarize(state, read_json(runs / 'run4-refill-queue.json'),
                      read_json(runs / 'merge-train/state.json'),
                      is_alive=lane_launch.alive, refill_pid=refill_pid,
                      paused=bool(lane_launch.provider_pause(root, state.get('provider', 'claude'))),
                      stopped=(runs / 'STOP').exists())
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

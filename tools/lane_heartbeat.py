"""Read-only lane heartbeat. Counts registered live processes, never worktree activity.

Liveness is not progress: a live worker that has not committed for STALL_HOURS, or a ready merge-queue entry that has
waited LAND_LAG_HOURS, is an alert of its own (an idle lane must not go unnoticed for days).

--wait-until train|backlog|<file> waits only at full capacity. All modes return
4 for underfill/backlog shortage, 5 for STOP/provider pause, and 1 for supervision errors.
Wait additionally returns 0 event, 2 timeout, 3 usage. Only local supervision artifacts are written."""
import argparse
from collections import Counter
from datetime import datetime
import json
import os
from pathlib import Path
import subprocess
import sys
import time

import lane_launch
import lane_refill
import lane_supervision as supervision

WAIT_EVENT, WAIT_TIMEOUT, WAIT_USAGE, WAIT_BACKLOG, WAIT_HALT = 0, 2, 3, 4, 5
WAIT_ERROR = 1
MAX_WAIT = 3600
READ_ATTEMPTS, READ_SECONDS, READ_DELAY = 5, 2.0, 0.25

STALL_HOURS = 2.0      # a live worker with no commit (or launch) this long is stalled
LAND_LAG_HOURS = 2.0   # a ready queue entry waiting this long means the train is not being run


def summarize(state, queue, train, *, is_alive, paused=False, stopped=False,
              refill_pid=None, progress=None, ready=None, now=None, waiting_on=None):
    """progress: {lane: epoch of its last commit or launch}; ready: {lane: epoch its ready queue entry was written};
    waiting_on(entry): the deps a pending brief still waits on (empty: the refiller can launch it)."""
    lanes = state.get('lanes', {})
    live, counted = [], set()
    for name, entry in sorted(lanes.items()):
        pid = entry.get('pid')
        if (type(pid) is int and pid > 0 and pid not in counted and
                entry.get('role') != 'helper' and entry.get('status') not in ('pending', 'draft') and is_alive(pid)):
            live.append(name)
            counted.add(pid)
    target = state.get('concurrency', 9)
    pending = [entry for entry in queue if entry.get('status') == 'pending']
    waiting = {entry['name']: list(waiting_on(entry)) if waiting_on else [] for entry in pending}
    waiting = {name: deps for name, deps in waiting.items() if deps}
    launchable = [entry['name'] for entry in pending if entry['name'] not in waiting]
    open_slots = max(target - len(live), 0)
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
        if len(launchable) < open_slots:
            alerts.append(f'BACKLOG LOW: {len(launchable)} launchable < {open_slots} open slots')
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
    lagging = sorted(n for n, at in (ready or {}).items() if now - at > LAND_LAG_HOURS * 3600)
    if lagging:
        alerts.append(f'LAND LAG: {len(lagging)} ready queue entr{"y" if len(lagging) == 1 else "ies"} waiting '
                      f'{LAND_LAG_HOURS:g}h+ (oldest {(now - min(ready[n] for n in lagging)) / 3600:.1f}h): python tools/merge-train.py status, then land or fix them')
    if gate.get('ok') and regress.get('ok'):
        alerts.append('LAND: gate and regress passed; coordinator review required')
    return dict(live=len(live), target=target, lanes=live,
                roles=dict(Counter(lanes[name].get('role', 'unknown') for name in live)),
                glue_limit=state.get('glue_limit', 3), pending=[entry['name'] for entry in pending],
                launchable=launchable, waiting=waiting, open_slots=open_slots,
                queue_states=dict(Counter(entry.get('status', 'unknown') for entry in queue)),
                rounds=state.get('rounds'), provider=state.get('provider', 'claude'),
                provider_paused=paused, stop=stopped, refiller_alive=refill_alive,
                train=current.get('id'), gate_ok=gate.get('ok'),
                regress=regress.get('verdict'), stalled=stalled, land_lag=lagging, alerts=alerts)


def read_json(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


class CriticalReadError(RuntimeError):
    """No valid critical state was obtained; never substitute a healthy default."""
    def __init__(self, message, *, timed_out=False):
        super().__init__(message)
        self.timed_out = timed_out


class RecoveryHalt(RuntimeError):
    pass


class CriticalReader:
    """One bounded policy for coordinator/queue/train reads, including train freshness stat."""
    def __init__(self, *, sleep=time.sleep, now=time.monotonic, deadline=None, check=None):
        self.sleep, self.now, self.deadline, self.check = sleep, now, deadline, check

    def __call__(self, path, expected=dict, *, fresh_since=None):
        end = self.now() + READ_SECONDS
        if self.deadline is not None:
            end = min(end, self.deadline)
        last = None
        attempts = 0
        for _ in range(READ_ATTEMPTS):
            if last is not None:
                if self.check:
                    self.check()
                if self.now() >= end:
                    break
            attempts += 1
            try:
                value = read_json(path)
                if not isinstance(value, expected):
                    raise ValueError(f'expected JSON {expected.__name__}, got {type(value).__name__}')
                modified = path.stat().st_mtime if fresh_since is not None else None
                return None if fresh_since is not None and modified < fresh_since else value
            except (OSError, ValueError) as exc:
                last = exc
            if self.check:
                self.check()
            left = end - self.now()
            if attempts == READ_ATTEMPTS or left <= 0:
                break
            self.sleep(min(READ_DELAY, left))
        raise CriticalReadError(f'cannot read critical JSON {path} after {attempts} attempts '
                                f'(retry/deadline exhausted): {type(last).__name__}: {last}',
                                timed_out=self.deadline is not None and self.now() >= self.deadline) from last


def recovery_halt(root):
    """Check flags afresh between read attempts, without recursively retrying coordinator state.

    An unreadable coordinator cannot identify a provider; it never supplies a cached healthy value.
    The critical read still has to succeed or report its persistent failure.
    """
    runs = root / 'build-runs'
    if (runs / 'STOP').exists():
        raise RecoveryHalt('HALT: STOP set; no spawning')
    try:
        state = read_json(runs / 'run4-coordinator-state.json')
    except (OSError, ValueError):
        return
    if isinstance(state, dict):
        provider = state.get('provider', 'claude')
        if lane_launch.provider_pause(root, provider):
            raise RecoveryHalt(f'HALT: provider {provider} paused')


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


def deps_checker(root):
    """waiting_on for summarize: the refiller's own deps rule, one git call per distinct dep."""
    landed = {}

    def waiting_on(entry):
        try:
            deps = lane_refill.spec_deps(read_json(Path(entry['spec'])))
        except (KeyError, OSError, ValueError, AttributeError):
            return ['(unreadable spec)']
        for dep in deps:
            if dep not in landed:
                landed[dep] = not lane_refill.unmet_deps(root, [dep])
        return [dep for dep in deps if not landed[dep]]
    return waiting_on


def snapshot(root, *, reader=None):
    return supervision.capacity(root, lambda: _snapshot(root, reader=reader))


def _snapshot(root, *, reader=None):
    reader = reader or CriticalReader()
    runs = root / 'build-runs'
    state = reader(runs / 'run4-coordinator-state.json')
    refiller = supervision.dispatcher(root, state.get('provider', 'claude'))
    lanes = state.get('lanes', {})
    progress = {n: last_progress(root, n, e) for n, e in lanes.items() if lane_launch.alive(e.get('pid'))}
    value = summarize(state, reader(runs / 'run4-refill-queue.json', list),
                      reader(runs / 'merge-train/state.json'),
                      is_alive=lane_launch.alive,
                      paused=bool(lane_launch.provider_pause(root, state.get('provider', 'claude'))),
                      stopped=(runs / 'STOP').exists(),
                      progress={n: v for n, v in progress.items() if v}, ready=ready_entries(root),
                      waiting_on=deps_checker(root))
    value['at'] = datetime.now().astimezone().isoformat(timespec='seconds')
    value['refiller'] = refiller
    value['refiller_alive'] = refiller['verified']
    value['alerts'] = [a for a in value['alerts'] if not a.startswith('REFILLER:')]
    if not refiller['verified']:
        value['alerts'].append('REFILLER UNKNOWN: ' + refiller['reason'])
    return value


def backlog(value):
    text = f'pending {len(value["launchable"])} launchable / {len(value["waiting"])} waiting'
    if value['waiting']:
        text += ' [' + '; '.join(f'{name}: {",".join(deps)}' for name, deps in value['waiting'].items()) + ']'
    return text


def capacity_line(value):
    incident = value.get('capacity_incident', {})
    age = incident.get('age_seconds')
    age_text = f'{age:g}s' if age is not None else 'UNKNOWN (clock/history unverifiable)'
    return (f'CAPACITY: {value["open_slots"]} actual workers missing; '
            f'launchable deficit {max(value["open_slots"] - len(value["launchable"]), 0)}; '
            f'observed age {age_text}; live {value["live"]}/{value["target"]}, {backlog(value)}')


def action(value):
    if value['stop']:
        return WAIT_HALT, 'HALT: STOP set; no spawning'
    if value['provider_paused']:
        return WAIT_HALT, f'HALT: provider {value["provider"]} paused; no spawning'
    if value['live'] < value['target'] or len(value['launchable']) < value['open_slots']:
        return WAIT_BACKLOG, capacity_line(value)
    return None


def report(root):
    value = snapshot(root)
    runs = root / 'build-runs'
    output = runs / 'run4-heartbeat.json'
    supervision.atomic_json(output, value)
    message = (f'{value["at"]} HEARTBEAT live {value["live"]}/{value["target"]}, '
               f'glue {value["roles"].get("glue", 0)}/{value["glue_limit"]}, '
               f'{capacity_line(value)}, rounds {value["rounds"]}; '
               f'train {value["train"]} gate={value["gate_ok"]} regress={value["regress"]}; '
               f'pause={value["provider_paused"]} STOP={value["stop"]}; ' + '; '.join(value['alerts']))
    with (runs / 'run4-heartbeat.log').open('a', encoding='utf-8') as log:
        log.write(message + '\n')
    print(message, flush=True)
    return value


def train_event(train, train_id):
    """A line once the train needs the coordinator: gate failed, regress finished, or no longer current."""
    current = train.get('current') or {}
    if current.get('id') != train_id:
        return f'TRAIN {train_id}: no longer current (landed or cleared)'
    gate, regress = current.get('gate'), current.get('regress')
    if gate is not None and not gate.get('ok'):
        return f'TRAIN {train_id}: gate FAIL'
    if regress is not None and regress.get('verdict') != 'RUNNING':
        verdict = regress.get('verdict') or ('PASS' if regress.get('ok') else 'FAIL')
        skipped = f' (skipped: {regress["skipped"]})' if regress.get('skipped') else ''
        return f'TRAIN {train_id}: gate ok, regress {verdict}{skipped}'
    return None


def _wait_until(root, event, *, timeout, poll, fresh, snap, sleep, now):
    """(exit code, one line): the named event, BACKLOG LOW, STOP/pause or the timeout, whichever comes first."""
    start = now()
    reader = CriticalReader(sleep=sleep, now=now, deadline=start + timeout,
                            check=lambda: recovery_halt(root))
    train_path = root / 'build-runs' / 'merge-train' / 'state.json'

    def read_train():
        # --fresh: an unchanged file holds an old verdict, but must still be valid JSON.
        return reader(train_path, fresh_since=start if fresh else None)
    train_id = None
    if event == 'train':
        def happened():
            nonlocal train_id
            if train_id is None:
                train_id = (reader(train_path).get('current') or {}).get('id')
                if not train_id:
                    return None
            train = read_train()
            return train_event(train, train_id) if train is not None else None
    elif event == 'backlog':
        def happened():
            return None
    else:
        path = Path(event) if Path(event).is_absolute() else root / event

        def happened():
            try:
                modified = path.stat().st_mtime
            except OSError:
                return None
            return None if fresh and modified < start - 2 else f'FILE {path}: present'
    while True:
        value = snap(root) if snap is not None else snapshot(root, reader=reader)
        decision = action(value)
        if decision:
            return decision
        line = happened()
        if line:
            return WAIT_EVENT, line
        if event == 'train' and train_id is None:
            return WAIT_USAGE, f'WAIT: no current train in {train_path}'
        left = start + timeout - now()
        if left <= 0:
            return WAIT_TIMEOUT, (f'TIMEOUT after {timeout:g}s waiting for {event}; '
                                  f'live {value["live"]}/{value["target"]}, {backlog(value)}')
        sleep(min(poll, left))


def wait_until(root, event, *, timeout=540, poll=10, fresh=False, snap=None,
               sleep=time.sleep, now=time.time):
    try:
        return _wait_until(root, event, timeout=timeout, poll=poll, fresh=fresh,
                           snap=snap, sleep=sleep, now=now)
    except RecoveryHalt as exc:
        return WAIT_HALT, str(exc)
    except CriticalReadError as exc:
        try:
            recovery_halt(root)
        except RecoveryHalt as halt:
            return WAIT_HALT, str(halt)
        if exc.timed_out:
            return WAIT_TIMEOUT, f'TIMEOUT after {timeout:g}s waiting for {event}; {exc}'
        return WAIT_ERROR, f'ERROR: {exc}'
    except (supervision.SupervisionError, OSError) as exc:
        try:
            recovery_halt(root)
        except RecoveryHalt as halt:
            return WAIT_HALT, str(halt)
        return WAIT_ERROR, f'ERROR: {exc}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--once', action='store_true', help='For a scheduler or coordinator poll')
    parser.add_argument('--interval', type=int, default=300)
    parser.add_argument('--wait-until', metavar='EVENT',
                        help='train (current train gate fail / regress verdict), backlog, or a file path')
    parser.add_argument('--timeout', type=float, default=540, help=f'--wait-until seconds (max {MAX_WAIT})')
    parser.add_argument('--poll', type=float, default=10, help='--wait-until seconds between checks')
    parser.add_argument('--fresh', action='store_true',
                        help='--wait-until: ignore a train verdict or file already there when the wait starts')
    args = parser.parse_args()
    if args.interval < 1:
        parser.error('--interval must be positive')
    if args.wait_until is not None:
        if not 0 < args.timeout <= MAX_WAIT or args.poll <= 0:
            print(f'WAIT: --timeout must be in (0, {MAX_WAIT}] and --poll positive', flush=True)
            sys.exit(WAIT_USAGE)
        code, line = wait_until(args.root.resolve(), args.wait_until, timeout=args.timeout,
                                poll=args.poll, fresh=args.fresh)
        print(line, flush=True)
        sys.exit(code)
    while True:
        try:
            value = report(args.root.resolve())
        except (CriticalReadError, supervision.SupervisionError, OSError) as exc:
            try:
                recovery_halt(args.root.resolve())
            except RecoveryHalt as halt:
                print(str(halt), flush=True)
                sys.exit(WAIT_HALT)
            print(f'ERROR: {exc}', flush=True)
            sys.exit(WAIT_ERROR)
        decision = action(value)
        if decision:
            print(decision[1], flush=True)
            sys.exit(decision[0])
        if args.once:
            return
        time.sleep(args.interval)


if __name__ == '__main__':
    main()

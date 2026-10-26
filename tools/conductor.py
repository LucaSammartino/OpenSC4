"""The coordinator's run loop: keep Codex lanes busy, resume rounds lost to outages, and wake the coordinator.

Usage: python tools/conductor.py [--run build-runs/run7] [--state build-runs/run6-state.json] [--wake 20]

Blocks until something needs the coordinator, then prints one heartbeat block and exits:
  0  a lane round finished and needs review (its pre-review checks are printed)
  2  a lane stalled, live lanes fell below target with nothing pending, or the wake interval passed
  3  a resource alarm (disk >= 85% or commit charge >= 95%)
Meanwhile it, without waking anyone:
  - resumes a finished round whose state is `continue`, or that died on a transient Codex error (refunding the round),
  - launches specs listed in <run>/pending.txt (one spec path per line, first launchable first) into free slots,
  - appends a metrics row to <run>/metrics.tsv every --beat minutes (live lanes counted by live pid, never by state).
The run directory holds target.txt (live-lane target), brief.md (printed at every wake: the mandate and the rules),
reviewed.json (result-file times already reviewed), pending.txt, metrics.tsv and heartbeat.log.
A round counts as finished when its result file is newer than reviewed.json says: the state clears a lane's pid on
exit, so a pid alone misses rounds (run 5).
"""
import argparse
import json
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TRANSIENT = ('at capacity', 'stream disconnected', 'No such host', 'Reconnecting', 'request timed out',
             'Codex hook failed')
FINAL = (" FINAL ROUND: start nothing new; finish and commit what is in flight, run the gate once, and write the "
         "merge-queue entry with state ready and the full 40-char tip so the finished work lands.")
STALL_MINUTES = 45  # no event-log growth for this long (a contended fc4_core test build took 34 min in run 5)
# Paths a lane of that kind must not change (clean room and the user's coverage definitions).
FORBIDDEN = {'engine': ('decomp/', 'ghidra/', 're/', 'tools/', 'bin/'),
             'any': ('tools/coverage-map.tsv',)}


def sh(args, cwd=ROOT, timeout=120):
    r = subprocess.run(args, cwd=cwd, capture_output=True, text=True, encoding='utf-8', errors='replace',
                       timeout=timeout)
    return (r.stdout or '') + (r.stderr or '')


def alive_pids():
    out = sh(['tasklist', '/fo', 'csv', '/nh'])
    return {ln.split('","')[1] for ln in out.splitlines() if ln.count('","') >= 2}


def mtime(p):
    p = Path(p)
    return p.stat().st_mtime if p.exists() else 0


def load(p, default):
    try:
        return json.loads(Path(p).read_text(encoding='utf-8-sig'))
    except (OSError, ValueError):
        return default


def result_of(entry):
    """The lane's final result as a dict when it parses, else the raw text (a Codex error message, say)."""
    raw = load(entry['result'], {}).get('result')
    if isinstance(raw, str):
        try:
            return json.loads(raw)
        except ValueError:
            return raw
    return raw


def classify(res):
    """'continue', 'transient' or 'review' for a finished round's result."""
    if isinstance(res, dict):
        return 'continue' if res.get('state') == 'continue' else 'review'
    if isinstance(res, str) and any(t in res for t in TRANSIENT):
        return 'transient'
    return 'review'


class Run:
    def __init__(self, a):
        self.dir = ROOT / a.run
        self.dir.mkdir(parents=True, exist_ok=True)
        self.state = ROOT / a.state
        self.a = a
        self.reviewed_path = self.dir / 'reviewed.json'
        self.reviewed = load(self.reviewed_path, None)
        if self.reviewed is None:  # first start: everything on disk now counts as reviewed
            self.reviewed = {n: mtime(e['result']) for n, e in self.lanes().items()}
            self.save_reviewed()

    def target(self):
        t = self.dir / 'target.txt'
        return int(t.read_text(encoding='utf-8-sig').strip()) if t.exists() else 13

    def lanes(self):
        """Lanes of the state file with a result path, minus those in <run>/ignore.txt (finished or abandoned)."""
        s = load(self.state, {'lanes': {}})
        ign = set((self.dir / 'ignore.txt').read_text().split()) if (self.dir / 'ignore.txt').exists() else set()
        return {n: e for n, e in s['lanes'].items() if e.get('result') and n not in ign}

    def kind(self, lane):
        spec = self.spec_for(lane)
        return load(spec, {}).get('args', {}).get('kind', '') if spec else ''

    def save_reviewed(self):
        on_disk = load(self.reviewed_path, {}) or {}
        self.reviewed_path.write_text(json.dumps({**on_disk, **self.reviewed}, indent=0), encoding='utf-8')

    def spec_for(self, lane):
        for d in (self.dir / 'specs', self.dir):
            p = d / f'{lane}.spec.json'
            if p.exists():
                return p
        return None

    def launch(self, spec):
        out = sh([sys.executable, 'tools/lane_launch.py', '--provider', 'codex', '--state', str(self.state), str(spec)])
        last = out.strip().splitlines()[-1:] or ['']
        return last[0]

    def resume(self, lane, entry, note):
        """Launch the lane's next round from its base spec with a resume note (the lane reads its own handoff)."""
        base = self.spec_for(lane)
        rounds = entry.get('rounds', 1)
        if base is None or rounds >= entry.get('round_limit', 3):
            return None
        d = load(base, None)
        d['resume'] = True
        final = FINAL if rounds + 1 == entry.get('round_limit', 3) else ''
        d['args']['task'] = f"Resume: continue from docs/lanes/{lane}.md and your commits. {note}{final} " + d['args']['task']
        out = base.parent / f'{lane}-r{rounds + 1}.spec.json'
        out.write_text(json.dumps(d, indent=1), encoding='utf-8')
        wt = ROOT / '.claude/worktrees' / lane
        if sh(['git', '-C', str(wt), 'rev-list', '--count', 'main..HEAD']).strip() == '0':
            # a round lost before its first commit leaves the branch "already merged"; the launcher refuses that
            sh(['git', '-C', str(wt), 'commit', '-q', '--allow-empty', '-m',
                f'{lane}: lane start (round {rounds} lost before any commit)'])
        last = self.launch(out)
        if last.startswith('LAUNCHED'):
            self.use_note(lane)
        return last

    def refund(self, lane):
        """A round lost to a Codex outage is not lane work: give it back (cap 9 rounds)."""
        sys.path.insert(0, str(ROOT / 'tools'))
        import lane_backend
        with lane_backend.codexguard.locked(ROOT / 'build-runs/run4-coordinator.lock'):
            st = load(self.state, None)
            ent = st['lanes'][lane]
            if ent.get('round_limit', 3) < 9:
                ent['round_limit'] = ent.get('round_limit', 3) + 1
                self.state.write_text(json.dumps(st, indent=1), encoding='utf-8')

    def take_note(self, lane):
        """The coordinator's review note for the lane's next round (notes/<lane>-next.txt); resume marks it used
        only once the round launched, so a refused launch keeps it for the next try."""
        p = self.dir / 'notes' / f'{lane}-next.txt'
        if not p.exists():
            return ''
        return ' '.join(p.read_text(encoding='utf-8').split()) + ' '

    def use_note(self, lane):
        p = self.dir / 'notes' / f'{lane}-next.txt'
        if p.exists():
            p.replace(p.with_name(f'{lane}-next.used.txt'))

    GLOBAL_REFUSAL = re.compile(r'live lane cap|lane cap reached|exceeds ?85|run total round cap|provider paused|'
                                r'spawn cap|too many|glue lane|active \d+ */ *cap', re.I)

    def fill_from_pending(self):
        p = self.dir / 'pending.txt'
        specs = [l.strip() for l in p.read_text(encoding='utf-8').splitlines() if l.strip()] if p.exists() else []
        for spec in list(specs):
            last = self.launch(spec) if Path(spec).exists() else 'missing spec'
            self.log(f'PENDING {Path(spec).stem}: {last}')
            if last.startswith('LAUNCHED') or 'already running' in last or last == 'missing spec':
                specs.remove(spec)
                p.write_text(''.join(s + '\n' for s in specs), encoding='utf-8')
                if last.startswith('LAUNCHED'):
                    return True
                continue
            if self.GLOBAL_REFUSAL.search(last):
                return False  # blocked for everyone (lane cap, disk, round cap): try again later
            # refused for this spec only (touches overlap, unmet dependency, parked lane): it stays in pending.txt
            # and the pass goes on, so one blocked spec cannot hold the free slots empty behind it
        return False

    def log(self, line):
        line = f'{time.strftime("%H:%M:%S")} {line}'
        print(line, flush=True)
        with open(self.dir / 'heartbeat.log', 'a', encoding='utf-8') as f:
            f.write(line + '\n')


def idle_minutes(entry):
    stem = str(Path(entry['result'])).replace('-result.json', '')
    m = mtime(stem + '-manifest.events.jsonl')
    return (time.time() - m) / 60 if m else 0


def commit_charge():
    out = sh(['powershell', '-NoProfile', '-Command',
              '$o=Get-CimInstance Win32_OperatingSystem; "{0} {1}" -f $o.TotalVirtualMemorySize,$o.FreeVirtualMemory'])
    try:
        total, free = (int(x) for x in out.split()[:2])
        return 100 * (total - free) / total
    except ValueError:
        return -1.0


def game_queue():
    roots = [ROOT] + [p for p in (ROOT / '.claude/worktrees').glob('*') if p.is_dir()]
    waiting = sum(len(list((r / 'build-runs/game-queue').glob('*.req'))) for r in roots)
    running = sum(len(list((r / 'build-runs/game-results').glob('*.running'))) for r in roots)
    return waiting, running


def prereview(lane, entry, kind):
    """Mechanical checks the coordinator would otherwise do by hand before reading a finished lane."""
    wt = ROOT / '.claude/worktrees' / lane
    res = result_of(entry)
    lines = []
    if isinstance(res, dict):
        lines.append(f"  state={res.get('state')} tip={res.get('tip')} gate={res.get('gate')}")
        lines.append(f"  summary: {str(res.get('summary'))[:600]}")
    else:
        lines.append(f'  raw result: {str(res)[:400]}')
    head = sh(['git', '-C', str(wt), 'rev-parse', 'HEAD']).strip()
    files = [f for f in sh(['git', '-C', str(wt), 'diff', '--name-only', 'main...HEAD']).split() if f]
    bad = [f for f in files if f.startswith(FORBIDDEN['any']) or (kind == 'engine' and f.startswith(FORBIDDEN['engine']))]
    queue = load(wt / 'build-runs/merge-queue' / f'{lane}.json', {})
    lines.append(f'  commits={sh(["git", "-C", str(wt), "rev-list", "--count", "main..HEAD"]).strip()} '
                 f'files={len(files)} queue={queue.get("state", "none")} '
                 f'queue_tip_matches={bool(queue.get("tip")) and head.startswith(queue.get("tip", "")[:9])}')
    if bad:
        lines.append(f'  FORBIDDEN PATHS: {" ".join(bad)}')
    dirty = sh(['git', '-C', str(wt), 'status', '--short', '--ignore-submodules=all']).strip()
    if dirty:
        lines.append(f'  dirty: {len(dirty.splitlines())} paths')
    return lines


def heartbeat(run, live, extra=()):
    disk = shutil.disk_usage('C:/')
    disk_pct = 100 * disk.used / disk.total
    commit = commit_charge()
    waiting, running = game_queue()
    row = [time.strftime('%Y-%m-%d %H:%M'), str(len(live)), str(run.target()), f'{disk_pct:.1f}', f'{commit:.1f}',
           str(waiting), str(running), sh(['git', 'rev-parse', '--short', 'main']).strip()]
    m = run.dir / 'metrics.tsv'
    if not m.exists():
        m.write_text('time\tlive\ttarget\tdisk_pct\tcommit_pct\tgames_waiting\tgames_running\tmain\n', encoding='utf-8')
    with open(m, 'a', encoding='utf-8') as f:
        f.write('\t'.join(row) + '\n')
    return disk_pct, commit, waiting, running


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--run', default='build-runs/run7')
    ap.add_argument('--state', default='build-runs/run6-state.json')
    ap.add_argument('--wake', type=float, default=20, help='minutes until a routine wake')
    ap.add_argument('--beat', type=float, default=15, help='minutes between metrics rows')
    a = ap.parse_args()
    run = Run(a)
    start = last_beat = time.time()
    while True:
        lanes, pids = run.lanes(), alive_pids()
        live = {n: e for n, e in lanes.items() if e.get('pid') and str(e['pid']) in pids}
        done = [n for n, e in lanes.items() if mtime(e['result']) > run.reviewed.get(n, 0) + 1]
        review = []
        for n in done:
            run.reviewed[n] = mtime(lanes[n]['result'])
            kind = classify(result_of(lanes[n]))
            if kind == 'transient':
                run.refund(n)
                lanes[n] = run.lanes()[n]
            if kind in ('continue', 'transient'):
                note = ('Your previous round failed on a transient Codex error; continue from your last commit.'
                        if kind == 'transient' else 'Your previous round ended in state continue; carry on with its next steps.')
                last = run.resume(n, lanes[n], note + ' ' + run.take_note(n))
                run.log(f'AUTO-RESUMED {n} ({kind}): {last}')
                if last and last.startswith('LAUNCHED'):
                    continue
            review.append(n)
        if done:
            run.save_reviewed()
            # a resumed lane has a new pid: count live lanes again, or it reads as a free slot
            lanes, pids = run.lanes(), alive_pids()
            live = {n: e for n, e in lanes.items() if e.get('pid') and str(e['pid']) in pids}
        stalled = [n for n, e in live.items() if idle_minutes(e) > STALL_MINUTES]
        under = len(live) < run.target() and time.time() - start > 10
        if under and not review and not stalled and run.fill_from_pending():
            continue
        beat_due = time.time() - last_beat > a.beat * 60
        wake_due = time.time() - start > a.wake * 60
        if beat_due or review or stalled or wake_due or (under and time.time() - start > 120):
            disk, commit, waiting, running = heartbeat(run, live)
            last_beat = time.time()
            alarm = disk >= 85 or commit >= 95
            if not (review or stalled or wake_due or alarm or under):
                time.sleep(30)
                continue
            print(f'===== HEARTBEAT {time.strftime("%Y-%m-%d %H:%M")} =====')
            brief = run.dir / 'brief.md'
            if brief.exists():
                print(brief.read_text(encoding='utf-8').strip())
            print(f'live {len(live)}/{run.target()}  disk {disk:.1f}%  commit {commit:.1f}%  '
                  f'games waiting {waiting} running {running}  main {sh(["git", "rev-parse", "--short", "main"]).strip()}')
            for n, e in sorted(live.items()):
                print(f"  {n}: r{e.get('rounds')}/{e.get('round_limit', 3)} idle {idle_minutes(e):.0f}m")
            pend = run.dir / 'pending.txt'
            if pend.exists() and pend.read_text().strip():
                print('pending:', ' '.join(Path(s).stem for s in pend.read_text().split()))
            for n in review:
                print(f'REVIEW {n}')
                print('\n'.join(prereview(n, lanes[n], run.kind(n))))
            if stalled:
                print('STALLED:', ' '.join(stalled))
            if alarm:
                print(f'ALARM: disk {disk:.1f}% commit {commit:.1f}%')
                sys.exit(3)
            if under and not review:
                print('UNDERFILLED: nothing launchable in pending.txt')
            sys.exit(0 if review else 2)
        time.sleep(30)


if __name__ == '__main__':
    main()

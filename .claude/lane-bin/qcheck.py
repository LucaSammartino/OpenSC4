#!/usr/bin/env python3
"""Replace-mode check for engine lanes: your owners switched on in the real game against the original's references
under the fixed schedule (docs/specs/simulator.md 5.8). It runs the coordinator's check from the main checkout on
<WT>/engine/build-x86/FreeCity4.dll; build first (qbuild.py all).

Usage: python qcheck.py --wt <worktree> --replace <keys> [--scenarios cover|<name,name>] [--jobs 4]
  cover (the default): the scenarios that reach your owners' code. One line per scenario: exact, or where it first
  differs: the first delivery whose rand() state differs (day, delivery number, the reference's target vtable), the
  first DIGEST date and fields, and the save records that differ (how many only shift object IDs, from which ID and
  class, and the first record whose content differs, with its owner key and class), then the run directory (its
  game.log has your plugin's lines; end.sc4 is the save).
The full output goes to <WT>/build-runs/qcheck-<n>.log. Exit 0 all exact, 1 a difference, 2 an error. One qcheck at
a time per lane; the game queue runs it beside the other lanes' games.
"""
import argparse
import itertools
import os
import subprocess
import sys

MAIN = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--wt', required=True)
    ap.add_argument('--replace', required=True)
    ap.add_argument('--scenarios', default='cover')
    ap.add_argument('--jobs', type=int, default=4)
    a = ap.parse_args()
    dll = os.path.join(a.wt, 'engine', 'build-x86', 'FreeCity4.dll')
    if not os.path.isfile(dll):
        print(f'qcheck: no {dll}; build first (qbuild.py --wt <WT> all)')
        return 2
    logs = os.path.join(a.wt, 'build-runs')
    os.makedirs(logs, exist_ok=True)
    log = next(p for p in (os.path.join(logs, f'qcheck-{n}.log') for n in itertools.count(1)) if not os.path.exists(p))
    command = [sys.executable, os.path.join(MAIN, 'tools', 'check.py'), 'run', '--dll', a.wt, '--replace', a.replace,
               '--scenarios', a.scenarios, '--jobs', str(max(1, min(a.jobs, 6)))]
    p = subprocess.run(command, cwd=MAIN, capture_output=True, text=True, encoding='utf-8', errors='replace')
    out = p.stdout + p.stderr
    with open(log, 'w', encoding='utf-8') as f:
        f.write(out)
    lines = out.splitlines()
    for line in lines[:60]:
        print(line[:600])
    if len(lines) > 60:
        print(f'... {len(lines) - 60} more lines in {log}')
    print(f'exit {p.returncode}; full output {log}')
    return p.returncode


if __name__ == '__main__':
    sys.exit(main())

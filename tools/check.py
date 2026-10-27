#!/usr/bin/env python3
"""Our owners in replace mode against the fixed-schedule references (docs/NEXT_STEPS.md step 2).

  python tools/check.py record [--scenarios SEL] [--jobs 12]
      the pinned reference DLL's result for each selected scenario that has none (tools/golden.py record)
  python tools/check.py run --dll <worktree or DLL> --replace <keys> [--scenarios SEL] [--jobs 12] [--no-bisect]
      each scenario with those owners switched on: exact, or the first differing delivery, day, record and
      owner; on a difference with several owners, each owner alone, to name the ones that differ
  python tools/check.py run --dll <worktree or DLL> --replace each [--jobs 12]
      every owner alone on each scenario of its cover; per owner, how many are exact; the table is written to
      build-runs/golden/owner-check.tsv

SEL is `cover` (the default: for each owner, the scenarios of its greedy reach cover in
build-runs/golden/owner-scenarios.tsv, written by `tools/coverage.py --owner-scenarios`), `all`, a set from
engine/tests/scenarios/suite.tsv, or scenario names separated by commas. `--replace none` switches on no owner of
ours: the candidate must then match the references exactly (a drift check of the harness and the schedule).
Lua and clockless scenarios are not run. Exit code: 0 all exact, 1 a difference, 2 an error. Standard library only.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import csv
import datetime as dt
import json
from pathlib import Path
import re
import subprocess
import sys

REPO = Path(__file__).resolve().parents[1]
GOLDEN = Path(__file__).with_name('golden.py')
SECONDS = 900  # a run's watchdog; not part of a fixed-schedule key


def scenarios():
    with (REPO / 'engine/tests/scenarios/suite.tsv').open(encoding='utf-8-sig', newline='') as stream:
        return {row['name']: row for row in csv.DictReader(stream, delimiter='\t')}


def owner_covers():
    path = REPO / 'build-runs/golden/owner-scenarios.tsv'
    if not path.is_file():
        raise ValueError(f'no {path}; run: python tools/coverage.py --owner-scenarios {path}')
    covers = {}
    with path.open(encoding='utf-8', newline='') as stream:
        for row in csv.DictReader((line for line in stream if not line.startswith('#')), delimiter='\t'):
            covers.setdefault(row['key'], []).append(row['scenario'])
    return covers


def select(text, keys, rows):
    if text == 'cover':
        covers = owner_covers()
        names = {name for key in (keys or covers) for name in covers.get(key, [])}
    elif text == 'all':
        names = set(rows)
    elif any(text in row['sets'].split(',') for row in rows.values()):
        names = {name for name, row in rows.items() if text in row['sets'].split(',')}
    else:
        names = set(filter(None, text.split(',')))
    unknown = names - set(rows)
    if unknown:
        raise ValueError(f'unknown scenarios: {sorted(unknown)}')
    # Not run: Lua scripts, and clockless cities (no day comes, so no run reaches a dated save).
    skipped = {name for name in names if rows[name]['script'].lower().endswith('.lua')
               or 'clockless' in rows[name]['sets'].split(',')}
    return sorted(names - skipped), sorted(skipped)


def golden(command, row, *extra):
    args = [sys.executable, str(GOLDEN), command, '--schedule', 'fixed', '--seconds', str(SECONDS),
            '--region', row['region'], '--city', row['city'], '--until', row['end']]
    if row['script'] != '-':
        args += ['--script', row['script']]
    process = subprocess.run(args + list(extra), cwd=REPO, capture_output=True, text=True)
    return process.returncode, process.stdout + process.stderr


def record(args):
    rows = scenarios()
    names, lua = select(args.scenarios, None, rows)
    print(f'{len(names)} scenarios' + (f'; not run (Lua or clockless): {", ".join(lua)}' if lua else ''), flush=True)
    failed = 0

    def one(name):
        code, out = golden('record', rows[name])
        state = ('error: ' + (re.findall(r'golden: (.*)', out) or [out.strip()[-200:]])[-1] if code else
                 'cached' if re.search(r'^cached:', out, re.M) else 'recorded')
        return name, code, state

    with ThreadPoolExecutor(args.jobs) as pool:
        for name, code, state in pool.map(one, names):
            failed += code != 0
            print(f'{name}: {state}', flush=True)
    print(f'{len(names) - failed} of {len(names)} scenarios have a reference')
    return 2 if failed else 0


def record_owners():
    """Save record type (a class ID) -> (replace key, class name), from the subsystem registry."""
    plugin = REPO / 'engine/plugin'
    constants = {}
    for header in plugin.glob('*.h'):
        for name, value in re.findall(r'\b(kFC4_CLSID_\w+)\s*=\s*(0x[0-9A-Fa-f]+)', header.read_text(encoding='utf-8')):
            constants[name] = int(value, 16)
    text = (plugin / 'FC4Subsystems.cpp').read_text(encoding='utf-8')
    arrays = {name: re.findall(r'\{\s*(0x[0-9A-Fa-f]+|kFC4_CLSID_\w+)\s*,\s*"(\w+)"', body)
              for name, body in re.findall(r'const ClassEntry (k\w+Classes)\[\] = \{(.*?)\n\};', text, re.S)}
    owners = {}
    for key, array in re.findall(r'\{"([a-z][a-z0-9_]*)",\s*[^,]+,\s*(k\w+Classes)\b', text):
        for clsid, name in arrays.get(array, []):
            value = constants.get(clsid) if clsid.startswith('kFC4') else int(clsid, 16)
            if value is not None:
                owners.setdefault(value, (key, name))
    return owners


def digests(log):
    """The schedule's per-delivery digest lines: (julian, delivery) -> (rand, generators, target vtable, message)."""
    found = {}
    for match in re.finditer(r'schedule: digest (\d+)\.(\d+) rand (\S+) generators (\S+) heap \S+ \S+ target vt (\S+) '
                             r'msg (\S+)', Path(log).read_text(encoding='utf-8-sig', errors='replace')):
        found[(int(match[1]), int(match[2]))] = match.group(3, 4, 5, 6)
    return found


def first_delivery(reference_log, candidate_log):
    expected, actual = digests(reference_log), digests(candidate_log)
    for key in sorted(expected.keys() | actual.keys()):
        a, b = expected.get(key), actual.get(key)
        if a is None or b is None or a[:2] != b[:2]:
            day = str(dt.date(2000, 1, 1) + dt.timedelta(days=key[0] - 2451545)) if key[0] > 2451545 else 'load'
            return {'day': day, 'delivery': key[1], 'expected': a, 'actual': b}
    return None


def compare(row, dll, keys):
    code, out = golden('compare', row, '--dll', str(dll), '--replace', ','.join(keys) or 'simulator')
    path = re.search(r'^comparison: (.+)$', out, re.M)
    if code not in (0, 1) or not path:
        return {'verdict': 'error', 'error': (re.findall(r'golden: (.*)', out) or [out.strip()[-300:]])[-1]}
    return json.loads(Path(path[1]).read_text(encoding='utf-8'))


def describe(name, report, owners, reference):
    if report['verdict'] == 'error':
        return f'{name}: ERROR {report["error"]}'
    if report['verdict'] == 'exact':
        return f'{name}: exact'
    parts = [f'{name}: {report["verdict"]}']
    delivery = first_delivery(reference / 'game.log', Path(report['run']) / 'game.log')
    if delivery:
        target = delivery['expected'][2] if delivery['expected'] else '-'
        parts.append(f'first delivery {delivery["day"]} #{delivery["delivery"]} (reference target vt {target})')
    first = report.get('first_digest_difference')
    if first:
        parts.append(f'first DIGEST {first["date"]} ({", ".join(first["fields"])})')
    records = report['save']['records']
    if records:
        kind = int(records[0]['tgi'].split('/')[0], 16)
        key, cls = owners.get(kind, ('-', '-'))
        parts.append(f'{report["save"]["different_records"]} records differ, first {records[0]["tgi"]} '
                     f'(owner {key}, {cls})')
    return '; '.join(parts)


def run_each(args, rows, dll):
    covers = owner_covers()
    skipped = set(select('all', None, rows)[1])
    jobs = [(key, name) for key in sorted(covers) if key != 'simulator' for name in covers[key] if name not in skipped]
    print(f'{len(jobs)} runs: {len(covers) - 1} owners, each alone on its cover', flush=True)
    owners = record_owners()
    with ThreadPoolExecutor(args.jobs) as pool:
        reports = list(pool.map(lambda job: compare(rows[job[1]], dll, [job[0]]), jobs))
    table, exact = [], {}
    for (key, name), report in zip(jobs, reports):
        line = describe(f'{key} on {name}', report, owners, REPO / 'build-runs/golden' / report.get('key', '-'))
        print(line, flush=True)
        table.append((key, name, report['verdict'], line.split(': ', 1)[1]))
        exact.setdefault(key, []).append(report['verdict'] == 'exact')
    path = REPO / 'build-runs/golden/owner-check.tsv'
    with path.open('w', encoding='utf-8', newline='\n') as stream:
        stream.write(f'# {dll} ({len(jobs)} runs)\nkey\tscenario\tverdict\tdetail\n')
        stream.writelines('\t'.join(row) + '\n' for row in table)
    whole = sorted(key for key, flags in exact.items() if all(flags))
    print(f'owners exact on their whole cover: {len(whole)} of {len(exact)}' + (f': {", ".join(whole)}' if whole else ''))
    print(f'table: {path}')
    errors = sum(report['verdict'] == 'error' for report in reports)
    return 2 if errors else 0 if len(whole) == len(exact) else 1


def run(args):
    rows = scenarios()
    keys = [] if args.replace == 'none' else sorted(set(filter(None, re.split(r'[,\s]+', args.replace))))
    dll = args.dll / 'engine/build-x86/FreeCity4.dll' if args.dll.is_dir() else args.dll
    if args.replace == 'each':
        return run_each(args, rows, dll)
    names, lua = select(args.scenarios, keys, rows)
    print(f'{len(names)} scenarios, owners {", ".join(keys) or "none"}' +
          (f'; not run (Lua or clockless): {", ".join(lua)}' if lua else ''), flush=True)
    owners = record_owners()
    with ThreadPoolExecutor(args.jobs) as pool:
        reports = dict(zip(names, pool.map(lambda n: compare(rows[n], dll, keys), names)))
    for name in names:
        entry = REPO / 'build-runs/golden' / reports[name].get('key', '-')
        print(describe(name, reports[name], owners, entry), flush=True)
    failing = [n for n in names if reports[n]['verdict'] != 'exact']
    errors = [n for n in failing if reports[n]['verdict'] == 'error']
    print(f'exact: {len(names) - len(failing)} of {len(names)}')
    if failing and len(keys) > 1 and not args.no_bisect:
        culprits = [n for n in failing if n not in errors][:2] or failing[:1]
        print(f'each owner alone on {", ".join(culprits)}:', flush=True)
        jobs = [(n, k) for n in culprits for k in keys]
        with ThreadPoolExecutor(args.jobs) as pool:
            alone = list(pool.map(lambda job: compare(rows[job[0]], dll, [job[1]]), jobs))
        for (name, key), report in zip(jobs, alone):
            print(f'  {key} on {name}: {report["verdict"]}' + (f' ({report["error"]})' if 'error' in report else ''))
    return 2 if errors else 1 if failing else 0


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = p.add_subparsers(dest='command', required=True)
    rec = commands.add_parser('record')
    chk = commands.add_parser('run')
    chk.add_argument('--dll', type=Path, required=True)
    chk.add_argument('--replace', required=True)
    chk.add_argument('--no-bisect', action='store_true')
    for sub in (rec, chk):
        sub.add_argument('--scenarios', default='cover')
        sub.add_argument('--jobs', type=int, default=12)
    args = p.parse_args(argv)
    try:
        return record(args) if args.command == 'record' else run(args)
    except (ValueError, OSError) as exc:
        print('check: ' + str(exc), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())

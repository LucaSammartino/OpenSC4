"""Run saved-city scenarios through game_queue; Python standard library only.

Examples: python tools/suite.py run --set smoke --profile shadows --tag smoke1
          python tools/suite.py report smoke1 [--against smoke2]
The queue must include this checkout in its roots. Requests opt into isolated
parallel slots. No file in Documents is written by this tool.
"""
import argparse
import csv
from datetime import date, datetime
import hashlib
import io
import itertools
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
COLUMNS = ('scenario profile verdict end_reached days_per_second actions_ok '
           'actions_failed shadow_lines shadow_exact_lines worst_shadow '
           'compare_exact compare_different reach_functions log').split()
KEY_COLUMNS = ['key', 'scenarios_run', 'scenarios_exact', 'first_mismatch']
SCENARIO_COLUMNS = ['name', 'region', 'city', 'script', 'end', 'sets', 'notes']
SHADOW = re.compile(r'^([a-z0-9_]+(?:[ -][a-z0-9_]+)*?)(?: shadow\b|-shadow:)')
ENVELOPE = re.compile(r'^.*?\[pid \d+ tid \d+\]\s*')
NAME = re.compile(r'^[A-Za-z0-9][A-Za-z0-9._-]{0,39}$')


def game_date(value):
    if not re.fullmatch(r'\d{4}-\d\d-\d\d', value):
        raise ValueError(f'expected YYYY-MM-DD: {value}')
    date.fromisoformat(value)
    return value


def folder_name(value):
    if (not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9 ._()-]*', value)
            or value.endswith(('.', ' '))):
        raise ValueError(f'expected one region/city name: {value!r}')
    return value


def inside(root, path):
    resolved = path.resolve()
    if not resolved.is_relative_to(root.resolve()):
        raise ValueError(f'path outside checkout: {path}')
    return resolved


def read_scenarios(root, selected):
    path = root / 'engine/tests/scenarios/suite.tsv'
    with path.open(encoding='utf-8-sig', newline='') as stream:
        reader = csv.DictReader(stream, delimiter='\t')
        if reader.fieldnames != SCENARIO_COLUMNS:
            raise ValueError(f'{path}: expected columns {SCENARIO_COLUMNS}')
        rows = list(reader)
    seen = set()
    for number, row in enumerate(rows, 2):
        if None in row or any(v is None for v in row.values()):
            raise ValueError(f'{path}:{number}: wrong number of cells')
        if not NAME.fullmatch(row['name']) or row['name'] in seen:
            raise ValueError(f'{path}:{number}: invalid/duplicate scenario name')
        seen.add(row['name'])
        folder_name(row['region']); folder_name(row['city']); game_date(row['end'])
        if row['script'] != '-':
            script_file(root, row['script'])
    result = [r for r in rows if selected in r['sets'].split(',')]
    if not result:
        raise ValueError(f'no scenarios in set {selected}')
    return result


def script_file(root, value):
    path = inside(root, root / value)
    if path.suffix.lower() not in ('.script', '.lua') or not path.is_file():
        raise ValueError(f'no .script or .lua file: {path}')
    return path


def profile_ini(root, profile):
    """Read the house profiles rather than maintaining another replacement list."""
    if profile == 'plain':
        return {}
    source = (root / 'tools/regress.ps1').read_text(encoding='utf-8-sig')
    if profile == 'replace':
        match = re.search(r'\[string\]\$Replace\s*=\s*"([^"]+)"', source)
        if not match:
            raise ValueError('cannot read regress.ps1 default -Replace list')
        return {'enable': match[1]}
    match = re.search(r'^\s*shadows\s*=\s*"([^"\r\n]+)"\s*$', source, re.M)
    if not match or '$' in match[1]:
        raise ValueError('cannot read regress.ps1 shadows profile')
    return dict(line.split(' = ', 1) for line in match[1].split('`n'))


def main_checkout(root):
    result = subprocess.run(['git', '-C', str(root), 'rev-parse', '--git-common-dir'],
                            capture_output=True, text=True, check=True)
    return (root / result.stdout.strip()).resolve().parent


def resolve_dll(root, supplied):
    path = Path(supplied).resolve() if supplied else main_checkout(root)
    if path.is_dir():
        path = path / 'engine/build-x86/FreeCity4.dll'
    if path.suffix.lower() != '.dll' or not path.is_file():
        raise ValueError(f'no DLL: {path}')
    return path


def extras(values):
    result = {}
    for value in values:
        match = re.fullmatch(r'\s*([a-z][a-z0-9_]*)\s*=\s*([^\r\n\x00]+)', value)
        if not match or any(ord(c) < 32 for c in value):
            raise ValueError(f'expected a single key = value: {value!r}')
        key, setting = match[1], match[2].strip()
        if key in ('label', 'dll', 'region', 'city', 'end_date', 'autoload_city', 'script', 'fixture_city'):
            raise ValueError(f'{key} is selected by the scenario/runner')
        result[key] = setting
    return result


def label_for(tag, name, profile):
    label = f'suite-{tag}-{name}-{profile}'
    if len(label) > 64:
        label = label[:55] + '-' + hashlib.sha256(label.encode()).hexdigest()[:8]
    return label


def make_batch(root, scenarios, profile, dll, tag, extra=()):
    if not NAME.fullmatch(tag):
        raise ValueError('tag: use 1..40 letters, digits, dots, underscores or dashes')
    base_ini = profile_ini(root, profile)
    changes = extras(extra)
    source_dll = resolve_dll(root, dll)
    directory = root / 'build-runs/suite' / tag
    if directory.exists():
        raise ValueError(f'tag already exists: {directory}; use report or a new tag')
    queue = root / 'build-runs/game-queue'
    results = root / 'build-runs/game-results'
    prepared = []
    for row in scenarios:
        label = label_for(tag, row['name'], profile)
        for path in (queue / f'{label}.req', results / f'{label}.running', results / f'{label}.verdict'):
            if path.exists():
                raise ValueError(f'request/result already exists: {path}')
        script = script_file(root, row['script']) if row['script'] != '-' else None
        prepared.append((row, label, script))
    directory.mkdir(parents=True)
    staged = directory / 'dll/FreeCity4.dll'
    staged.parent.mkdir()
    shutil.copy2(source_dll, staged)
    if source_dll.with_suffix('.pdb').is_file():
        shutil.copy2(source_dll.with_suffix('.pdb'), staged.with_suffix('.pdb'))
    requests = []
    for row, label, source_script in prepared:
        request = dict(label=label, dll=staged.relative_to(root).as_posix(),
                       region=row['region'], city=row['city'], seconds='750',
                       timeout='900', end_date=row['end'], turbo='1', parallel='1')
        ini = dict(base_ini, autotest_unpause='2', autotest_end='quit', popup_dialogs='0')
        if source_script:
            script = directory / 'scripts' / (row['name'] + source_script.suffix)
            script.parent.mkdir(exist_ok=True)
            shutil.copy2(source_script, script)
            ini['script'] = script.relative_to(root).as_posix()
        for key, value in changes.items():
            (request if key in ('seconds', 'timeout', 'turbo', 'turbo_from', 'minimize', 'priority', 'parallel') else ini)[key] = value
        text = '\n'.join(f'{k} = {v}' for k, v in request.items()) + '\n[ini]\n'
        text += '\n'.join(f'{k} = {v}' for k, v in ini.items()) + '\n'
        (directory / f'{row["name"]}.req').write_text(text, encoding='utf-8')
        requests.append(dict(scenario=row, label=label, request=text))
    manifest = dict(tag=tag, profile=profile, dll=str(source_dll),
                    dll_sha256=hashlib.sha256(staged.read_bytes()).hexdigest(),
                    extra=list(extra), requests=requests,
                    shadow_keys=[k[:-7] for k in dict(base_ini, **changes) if k.endswith('_shadow')])
    (directory / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    return directory, manifest


def mismatch(body):
    totals = re.findall(r'; mismatches=(\d+)', body)
    if totals:
        return int(totals[-1]) != 0
    return bool(re.search(r'mismatch(?:es|ed)?[= ][1-9]\d*|differences [1-9]\d*|\b(?:diff|different|refusal|rejected)=[1-9]\d*|\b[1-9]\d* (?:differ|rejected)\b|\bINCOMPLETE\b', body))


def resynced(body):
    return bool(re.search(r'\bresynced:\s*(?!none\b|0\b)\S|\bresyncs?\s*[=:]\s*[1-9]\d*\b', body))


def shadow_sample(body, key):
    if key == 'formats':
        return (not re.search(r'shadow: (?:store )?total:', body)
                and bool(re.search(r'\b[1-9]\d* compared,', body)))
    measured = re.search(r'mismatch(?:es|ed)?[= ]\d|: clean\b', body)
    dated = re.search(r'\b\d{4}-\d\d-\d\d\b|\bjulian=\d+', body)
    return bool(measured and (dated or '-shadow:' in body))


def aggregate_counts(body):
    match = re.search(r'(\d+) (?:month )?lines, (\d+) with mismatches=0', body)
    if match:
        return int(match[1]), int(match[2])
    match = re.search(r'(?:months|lines)=(\d+) clean=(\d+)', body)
    return (int(match[1]), int(match[2])) if match else None


def parse_log(text, end, verdict_text=''):
    """Count comparison evidence, excluding switches, empty files and duplicate totals.

    Keep *all* shadow summaries verbatim in the JSON evidence. Aggregate-only
    keys supply counts if there were no sample lines, never additional counts.
    """
    result = dict(verdict='missing', end_reached='0', days_per_second='-',
                  actions_ok=0, actions_failed=0, actions_expected=0, actions_pending=0,
                  shadow_lines=0, shadow_exact_lines=0, worst_shadow='-',
                  compare_exact=0, compare_different=0, reach_functions='-',
                  per_key={}, action_lines=[], shadow_records=[], shadow_summaries=[], shadow_payloads=[],
                  inplace_lines=[], reach_lines=[])
    keys, aggregate, actions = {}, {}, {}
    verdict_line = ''
    reached_date = ''
    parse_errors = 0
    for line_no, line in enumerate(io.StringIO(text), 1):
        body = ENVELOPE.sub('', line.rstrip('\r\n'))
        if 'AUTOTEST VERDICT:' in body:
            verdict_line = body
        reached = re.search(r'end date (\d{4}-\d\d-\d\d) reached', body)
        if reached and reached[1] == end:
            result['end_reached'] = '1'
        if body.startswith('script:'):
            loaded = re.search(r': (\d+) action\(s\)', body)
            if loaded:
                result['actions_expected'] = int(loaded[1])
            if 'ERROR line ' in body:
                parse_errors += 1
            if 'REFUSED ' in body:
                parse_errors = max(parse_errors, 1)
            action = re.search(r'^script: line (\d+) .*? -> (.*)', body)
            if action:
                failed = bool(re.search(r'\bok=0\b|\berror=[1-9]\d*|\b(?:skipped|failed|refused|exception)\b|\baccepted=0\b|\bno (?:candidate|city|manager)\b', action[2], re.I))
                actions[action[1]] = (not failed, body)
        if 'inplace summary ' in body:
            result['inplace_lines'].append(dict(line=line_no, text=body))
            fields = dict(re.findall(r'(\w+)=([^ ]+)', body))
            result['compare_exact'] += int(fields.get('exact', 0))
            result['compare_different'] += int(fields.get('different', 0))
        if body.startswith('reach summary '):
            result['reach_lines'].append(dict(line=line_no, text=body))
            fields = dict(re.findall(r'(\w+)=(\d+)', body))
            result['reach_functions'] = fields.get('reached', '-')
        match = SHADOW.match(body)
        if not match:
            continue
        key = match[1].replace(' ', '_')
        evidence = dict(line=line_no, text=body)
        result['shadow_payloads'].append(evidence)
        if 'summary' in body or 'shutdown' in body.lower() or 'totals' in body or 'total:' in body:
            result['shadow_summaries'].append(dict(key=key, **evidence))
        if shadow_sample(body, key):
            count = keys.setdefault(key, dict(lines=0, exact=0, first_mismatch=''))
            exact = not mismatch(body) and not resynced(body)
            count['lines'] += 1
            count['exact'] += int(exact)
            if not exact and not count['first_mismatch']:
                count['first_mismatch'] = f'line {line_no}: {body}'
            result['shadow_records'].append(dict(key=key, **evidence))
        else:
            counts = aggregate_counts(body)
            if counts and counts[0]:
                # Ports has separate airport/seaport totals. Retain each scope.
                scope = re.search(r'shutdown: (airport|seaport) ', body)
                aggregate[(key, scope[1] if scope else '')] = (counts, evidence)
    for (key, _), ((total, exact), evidence) in aggregate.items():
        if any(sample['key'] == key for sample in result['shadow_records']):
            continue
        count = keys.setdefault(key, dict(lines=0, exact=0, first_mismatch=''))
        if resynced(evidence['text']):
            exact = 0
        count['lines'] += total
        count['exact'] += exact
        if total != exact and not count['first_mismatch']:
            count['first_mismatch'] = f'line {evidence["line"]}: {evidence["text"]}'
    if not verdict_line:
        verdict_line = verdict_text
    verdict = re.search(r'AUTOTEST VERDICT:\s*(\w+)|^verdict:\s*([\w-]+)', verdict_line, re.M)
    if verdict:
        result['verdict'] = verdict[1] or verdict[2]
    found_date = re.search(r'\bdate=(\d{4}-\d\d-\d\d)', verdict_line)
    if found_date:
        reached_date = found_date[1]
        if reached_date >= end:
            result['end_reached'] = '1'
    speed = re.search(r'\bdays_per_second=([0-9.]+)', verdict_line)
    if speed:
        result['days_per_second'] = speed[1]
    result['action_lines'] = [a[1] for a in actions.values()]
    result['actions_ok'] = sum(a[0] for a in actions.values())
    result['actions_failed'] = len(actions) - result['actions_ok'] + parse_errors
    result['actions_pending'] = max(0, result['actions_expected'] - len(actions))
    result['per_key'] = keys
    result['shadow_lines'] = sum(v['lines'] for v in keys.values())
    result['shadow_exact_lines'] = sum(v['exact'] for v in keys.values())
    if keys:
        worst = max(keys, key=lambda k: ((keys[k]['lines'] - keys[k]['exact']) / keys[k]['lines'], k))
        count = keys[worst]
        result['worst_shadow'] = (f'{worst}:{count["lines"] - count["exact"]}/{count["lines"]}'
                                  if count['lines'] != count['exact'] else 'exact')
    return result


def write_tsv(path, columns, rows):
    with path.open('w', encoding='utf-8', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=columns, delimiter='\t', lineterminator='\n', extrasaction='ignore')
        writer.writeheader(); writer.writerows(rows)


def write_results(directory, manifest, results):
    rows = []
    per_key = {key: dict(key=key, scenarios_run=0, scenarios_exact=0, first_mismatch='-')
               for key in manifest['shadow_keys']}
    for request in manifest['requests']:
        name = request['scenario']['name']
        row = dict(results.get(name, parse_log('', request['scenario']['end'])),
                   scenario=name, profile=manifest['profile'])
        row.setdefault('log', '-')
        rows.append(row)
        for key, value in row['per_key'].items():
            entry = per_key.setdefault(key, dict(key=key, scenarios_run=0, scenarios_exact=0, first_mismatch='-'))
            entry['scenarios_run'] += 1
            entry['scenarios_exact'] += int(value['lines'] == value['exact'])
            if value['first_mismatch'] and entry['first_mismatch'] == '-':
                entry['first_mismatch'] = f'{name} {row["log"]} {value["first_mismatch"]}'
    write_tsv(directory / 'summary.tsv', COLUMNS, rows)
    write_tsv(directory / 'per_key.tsv', KEY_COLUMNS, [per_key[k] for k in sorted(per_key)])


def wait_for_batch(root, directory, manifest, wait_seconds):
    deadline = time.monotonic() + wait_seconds
    pending = {r['scenario']['name']: r for r in manifest['requests']}
    results = {}
    try:
        while pending:
            for name, request in list(pending.items()):
                source = root / 'build-runs/game-results' / request['label']
                # Labels can contain dots: append the extension, do not replace it.
                verdict = Path(str(source) + '.verdict')
                if not verdict.is_file():
                    continue
                verdict_text = verdict.read_text(encoding='utf-8-sig', errors='replace')
                source_log = Path(str(source) + '.log')
                log = directory / f'{name}.log'
                if source_log.is_file():
                    shutil.copy2(source_log, log)
                text = log.read_text(encoding='utf-8-sig', errors='replace') if log.exists() else ''
                parsed = parse_log(text, request['scenario']['end'], verdict_text)
                if request['scenario']['script'] != '-' and not parsed['actions_expected'] and not parsed['action_lines']:
                    parsed['actions_pending'] = 1  # requested script produced no execution evidence
                parsed['log'] = str(log) if log.exists() else '-'
                (directory / f'{name}.verdict').write_text(verdict_text, encoding='utf-8')
                (directory / f'{name}.json').write_text(json.dumps(parsed, indent=2) + '\n', encoding='utf-8')
                (directory / f'{name}.shadows.txt').write_text(
                    '\n'.join(r['text'] for r in parsed['shadow_payloads']) + '\n', encoding='utf-8')
                results[name] = parsed
                del pending[name]
                print(f'{name}: {parsed["verdict"]}; end={parsed["end_reached"]}; '
                      f'actions={parsed["actions_ok"]} ok/{parsed["actions_failed"]} failed; '
                      f'shadow={parsed["shadow_exact_lines"]}/{parsed["shadow_lines"]}', flush=True)
                write_results(directory, manifest, results)
            if pending and time.monotonic() >= deadline:
                print('queue wait expired; requests remain queued/running: ' + ', '.join(pending), file=sys.stderr)
                for name, request in pending.items():
                    results[name] = parse_log('', request['scenario']['end'], 'verdict: queue-timeout')
                break
            if pending:
                time.sleep(min(2, max(0, deadline - time.monotonic())))
    finally:
        write_results(directory, manifest, results)
    return results


def run_scenarios(root, scenarios, *, profile, dll, tag, extra=(), wait_seconds=7200, dry_run=False):
    directory, manifest = make_batch(root, scenarios, profile, dll, tag, extra)
    if dry_run:
        print(f'prepared {len(scenarios)} requests (not submitted): {directory}')
        return directory, {}
    queue = root / 'build-runs/game-queue'
    queue.mkdir(parents=True, exist_ok=True)
    for request in manifest['requests']:
        path = queue / (request['label'] + '.req')
        temporary = path.with_suffix('.req.tmp')
        temporary.write_text(request['request'], encoding='utf-8')
        temporary.replace(path)
    print(f'submitted {len(scenarios)} requests; results: {directory}', flush=True)
    return directory, wait_for_batch(root, directory, manifest, wait_seconds)


def report(root, tag, against=None):
    if not NAME.fullmatch(tag):
        raise ValueError('invalid tag')
    directory = root / 'build-runs/suite' / tag
    for filename in ('summary.tsv', 'per_key.tsv'):
        print(filename)
        print((directory / filename).read_text(encoding='utf-8').rstrip())
    if against:
        if not NAME.fullmatch(against):
            raise ValueError('invalid comparison tag')
        baseline = directory.parent / against
        manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
        print('scenario\tshadow_determinism\tfirst_difference')
        for request in manifest['requests']:
            name = request['scenario']['name']
            paths = [folder / f'{name}.shadows.txt' for folder in (directory, baseline)]
            if not all(p.exists() for p in paths):
                print(f'{name}\tmissing\t-'); continue
            lines = [p.read_text(encoding='utf-8').splitlines() for p in paths]
            if not any(lines):
                print(f'{name}\tno-evidence\t-'); continue
            difference = next((i for i, (a, b) in enumerate(itertools.zip_longest(*lines), 1) if a != b), None)
            print(f'{name}\t{"different" if difference else "exact"}\t{difference or "-"}')


def new_tag():
    return datetime.now().strftime('%Y%m%d-%H%M%S-') + uuid.uuid4().hex[:6]


def successful(results):
    return bool(results) and all(r['verdict'] == 'clean' and r['end_reached'] == '1'
                                and not r['actions_failed'] and not r['actions_pending']
                                for r in results.values())


def main(argv=None, root=ROOT):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    run = commands.add_parser('run')
    run.add_argument('--set', choices=['smoke', 'full'], required=True)
    run.add_argument('--profile', choices=['shadows', 'replace', 'plain'], default='shadows')
    run.add_argument('--dll', help='worktree or DLL; default main checkout DLL')
    run.add_argument('--tag', default=None)
    run.add_argument('--extra', action='append', default=[])
    run.add_argument('--wait-seconds', type=int, default=7200)
    run.add_argument('--dry-run', action='store_true', help='stage reproducible inputs without submitting')
    show = commands.add_parser('report')
    show.add_argument('tag'); show.add_argument('--against')
    args = parser.parse_args(argv)
    try:
        if args.command == 'report':
            report(root, args.tag, args.against)
            return 0
        directory, results = run_scenarios(root, read_scenarios(root, args.set),
            profile=args.profile, dll=args.dll, tag=args.tag or new_tag(), extra=args.extra,
            wait_seconds=args.wait_seconds, dry_run=args.dry_run)
        if not args.dry_run:
            print(f'summary: {directory / "summary.tsv"}')
        return 0 if args.dry_run or successful(results) else 1
    except (ValueError, OSError, subprocess.SubprocessError) as exc:
        parser.error(str(exc))


if __name__ == '__main__':
    sys.exit(main())

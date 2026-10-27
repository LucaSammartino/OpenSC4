"""Record game scenarios once, then replay their content-addressed traces on CPUs."""
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

import replay_coverage
import suite
import suite_coverage


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def executable(root, supplied=None):
    if supplied:
        path = Path(supplied).resolve()
        if path.is_file():
            return path
    else:
        for base in (root, suite.main_checkout(root)):
            path = base / 'bin/SimCity4.exe'
            if path.is_file():
                return path
    raise ValueError('no game executable; supply --exe (used for recording identity, never launched directly)')


def identity(root, row, exe, profile, extra):
    """Core DLL changes do not invalidate game inputs; source replay has its own cache."""
    scripts = []
    if row['script'] != '-':
        script = suite.script_file(root, row['script'])
        scripts.append(script)
        if script.suffix.lower() == '.lua' and script.with_suffix('.script').is_file():
            scripts.append(script.with_suffix('.script'))
    city = Path(row['_prepared_city']) if row.get('_prepared_city') else suite.city_path(row)
    if not city.is_absolute():
        city = root / city
    harness = ['tools/suite.py', 'tools/suite_replay.py', 'tools/game_queue.ps1', 'tools/ingame_run.ps1',
               'tools/regress.ps1', 'docs/abi/functions.tsv']
    inputs = dict(version=1, scenario=row, exe_sha256=sha(exe), city_sha256=sha(city),
                  scripts={p.relative_to(root).as_posix(): sha(p) for p in scripts},
                  harness={p: sha(root / p) for p in harness if (root / p).is_file()},
                  profile=profile, settings=suite.extras(extra), ini=suite.profile_ini(root, profile))
    digest = hashlib.sha256(json.dumps(inputs, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    return digest, inputs


def trace_directory(root, tag):
    if not suite.NAME.fullmatch(tag) or tag == 'store':
        raise ValueError('invalid trace set tag (store is reserved)')
    return root / 'build-runs/traces' / tag


def completed(root, directory, fingerprint):
    marker = directory / 'recording.json'
    if not marker.is_file():
        return None
    metadata = json.loads(marker.read_text(encoding='utf-8'))
    if metadata['identity'] != fingerprint:
        raise ValueError('recording store identity differs from its directory')
    evidence = replay_coverage.census(root, metadata['suite'], suite_coverage, ())
    scenarios = evidence['scenarios']
    if len(scenarios) != 1 or not scenarios[0]['selection_ready']:
        return None
    rows = replay_coverage.manifest_rows(directory / 'manifest.tsv')
    if not all((directory / r['trace_sha']).is_file() for r in rows):
        return None
    return metadata


def assemble(directory, objects):
    """Hard-link immutable blobs when possible; a set manifest is an inexpensive view."""
    directory.mkdir(parents=True, exist_ok=True)
    lines, recordings = [], {}
    for store, metadata in objects:
        rows = replay_coverage.manifest_rows(store / 'manifest.tsv')
        for row in rows:
            blob = row['trace_sha']
            destination = directory / blob
            if not destination.exists():
                try:
                    os.link(store / blob, destination)
                except OSError:
                    shutil.copy2(store / blob, destination)
            lines.append('\t'.join(row[c] for c in replay_coverage.MANIFEST_COLUMNS))
            recordings[row['scenario']] = dict(suite=metadata['suite'], identity=metadata['identity'])
    (directory / 'manifest.tsv').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    (directory / 'recordings.json').write_text(json.dumps(dict(version=1, recordings=recordings), indent=2) + '\n',
                                               encoding='utf-8')


def record(root, args):
    if not args.reach:
        raise ValueError('record requires --reach')
    settings = suite.extras(args.extra)
    if any(k in settings for k in ('reach', 'reach_output', 'reach_contracts', 'trace_record', 'trace_scenario')):
        raise ValueError('record owns reach and trace INI settings; do not override them with --extra')
    directory = trace_directory(root, args.tag)
    rows = suite.read_scenarios(root, args.set)
    if args.scenario:
        selected = set(args.scenario)
        if selected - {r['name'] for r in rows}:
            raise ValueError('unknown scenario in selected set')
        rows = [r for r in rows if r['name'] in selected]
    exe = executable(root, args.exe)
    extra = ['reach = 1', 'reach_window = month',
             'reach_contracts = ' + (root / 'docs/abi/functions.tsv').as_posix()] + list(args.extra)
    objects, reused, recorded = [], 0, 0
    for row in rows:
        fingerprint, inputs = identity(root, row, exe, args.profile, extra)
        store = root / 'build-runs/traces/store' / fingerprint
        metadata = completed(root, store, fingerprint)
        if metadata:
            print(f'record {row["name"]}: reused {fingerprint}')
            reused += 1
        else:
            tag = 'record-' + fingerprint[:32]
            batch = root / 'build-runs/suite' / tag
            results = None
            if batch.exists():
                print(f'record {row["name"]}: checking staged/resumed batch {tag}')
            else:
                overrides = extra + [f'trace_record = {store.as_posix()}', f'trace_scenario = {row["name"]}']
                batch, results = suite.run_scenarios(root, [row], profile=args.profile, dll=args.dll,
                    tag=tag, extra=overrides, wait_seconds=args.wait_seconds, dry_run=args.dry_run, slots=1)
            if args.dry_run:
                print(f'record {row["name"]}: staged {batch}; no recording cached')
                continue
            if not (batch / 'summary.tsv').is_file():
                raise ValueError(f'unfinished recording; run suite.py resume {tag}, then repeat record')
            evidence = suite_coverage.load(root, tag)
            if ((results is not None and not suite.successful(results))
                    or not all(s['selection_ready'] for s in evidence['scenarios'])):
                print(f'record {row["name"]}: failed/incomplete recording; not cached; {batch}')
                return 1
            trace_rows = replay_coverage.manifest_rows(store / 'manifest.tsv')
            if any(r['scenario'] != row['name'] for r in trace_rows):
                raise ValueError('recording manifest contains another scenario')
            for blob in {r['trace_sha'] for r in trace_rows}:
                if sha(store / blob) != blob:
                    raise ValueError(f'recorded trace digest differs: {blob}')
            metadata = dict(identity=fingerprint, suite=str(batch.resolve()), inputs=inputs,
                            dll_sha256=json.loads((batch / 'manifest.json').read_text(encoding='utf-8'))['dll_sha256'])
            (store / 'recording.json').write_text(json.dumps(metadata, indent=2) + '\n', encoding='utf-8')
            print(f'record {row["name"]}: recorded {fingerprint}')
            recorded += 1
        objects.append((store, metadata))
    if not args.dry_run:
        assemble(directory, objects)
    print(f'record set {args.tag}: recorded={recorded} reused={reused}; manifest={directory / "manifest.tsv"}')
    return 0


def replay(root, args):
    target = Path(args.target)
    path = (root / target).resolve() if not target.is_absolute() else target.resolve()
    if not path.is_file():
        path = trace_directory(root, args.target) / 'manifest.tsv'
    # Replay writes verdicts and caches, so never mutate another checkout's recordings.
    suite.inside(root / 'build-runs/traces', path)
    replay_coverage.manifest_rows(path)
    if args.jobs < 1:
        raise ValueError('--jobs must be positive')
    if args.replayer:
        binary = Path(args.replayer).resolve()
        if not binary.is_file():
            raise ValueError(f'no replayer: {binary}')
        command = [str(binary)]
    else:
        command = ['cargo', 'run', '--release', '--manifest-path', str(root / 'engine/Cargo.toml'),
                   '-p', 'fc4-capi', '--bin', 'fc4-replay', '--']
    command += ['--manifest', str(path), '--jobs', str(args.jobs)]
    if args.no_cache:
        command.append('--no-cache')
    # A failed invocation must not leave older exact verdicts looking current.
    for name in ('replay-verdicts.tsv', 'replay-suite.tsv'):
        (path.parent / name).unlink(missing_ok=True)
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    log = path.parent / f'replay-{stamp}.log'
    with log.open('w', encoding='utf-8') as stream:
        result = subprocess.run(command, cwd=root, stdout=stream, stderr=subprocess.STDOUT)
    if result.returncode not in (0, 1):
        raise ValueError(f'replayer failed (exit {result.returncode}); {log}')
    pairs, _ = replay_coverage.pairs(path, suite_coverage)
    count = sum(pairs.values())
    print(f'replay: {count}/{len(pairs)} exact scenario/key pairs; jobs={args.jobs}; exit={result.returncode}')
    print(f'verdicts: {path.parent / "replay-verdicts.tsv"}; suite: {path.parent / "replay-suite.tsv"}; log: {log}')
    return 0 if result.returncode == 0 and count == len(pairs) else 1


def add_commands(commands):
    record_parser = commands.add_parser('record', help='record a scenario set once through the shared game queue')
    record_parser.add_argument('--set', default='smoke')
    record_parser.add_argument('--tag', required=True)
    record_parser.add_argument('--scenario', action='append')
    record_parser.add_argument('--profile', choices=['shadows', 'replace'], default='shadows')
    record_parser.add_argument('--dll')
    record_parser.add_argument('--exe', help='game exe to fingerprint (default bin/SimCity4.exe)')
    record_parser.add_argument('--extra', action='append', default=[])
    record_parser.add_argument('--reach', action='store_true', required=True, help='required original entry counters')
    record_parser.add_argument('--wait-seconds', type=int, default=7200)
    record_parser.add_argument('--dry-run', action='store_true')
    replay_parser = commands.add_parser('replay', help='replay a recorded set using all available CPUs')
    replay_parser.add_argument('target', help='trace set tag or manifest.tsv path under this checkout build-runs/traces')
    replay_parser.add_argument('--jobs', type=int, default=os.cpu_count() or 1)
    replay_parser.add_argument('--no-cache', action='store_true')
    replay_parser.add_argument('--replayer', help='explicit prebuilt fc4-replay executable; default builds current core')

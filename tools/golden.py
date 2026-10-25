#!/usr/bin/env python3
"""Original-game cache and strict offline/one-game comparisons (stdlib only).

record / compare share the original-input key; the candidate DLL and replacement
keys are deliberately excluded. record --rerun verifies an existing key without
overwriting its artifacts. A differing repeat is retained and marked NONDETERMINISTIC.
No statistical tolerance or automatically learned ignore mask is applied.
"""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import uuid

try:
    from .golden_dbpf import compare_saves, read_records
    from . import golden_envelope
except ImportError:
    from golden_dbpf import compare_saves, read_records
    import golden_envelope

REPO = Path(__file__).resolve().parents[1]
SEED = 0x5EED1234
SCHEMA = 1


def sha256(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def write_json(path, data):
    Path(path).write_text(json.dumps(data, indent=2, sort_keys=True, allow_nan=False) + '\n', encoding='utf-8')


def load_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def date(value):
    parsed = dt.date.fromisoformat(value)
    if str(parsed) != value:
        raise ValueError('dates must be YYYY-MM-DD')
    return parsed


def safe_name(value):
    if not re.fullmatch(r'[A-Za-z0-9 ._()-]+', value) or value in ('.', '..'):
        raise ValueError(f'not a single city/region name: {value!r}')
    return value


def main_checkout(repo):
    text = repo.resolve().as_posix()
    return Path(text.split('/.claude/worktrees/')[0])


def documents():
    if os.name == 'nt':
        import winreg
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER,
                            r'Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders') as key:
            return Path(os.path.expandvars(winreg.QueryValueEx(key, 'Personal')[0]))
    return Path.home() / 'Documents'


def fingerprint(files, settings):
    """files maps stable logical names to paths; no mtime-based hash shortcuts."""
    manifest = {'schema': SCHEMA, 'settings': settings,
                'files': {name: {'sha256': sha256(path), 'size': Path(path).stat().st_size}
                          for name, path in sorted(files.items())}}
    raw = json.dumps(manifest, sort_keys=True, separators=(',', ':')).encode('utf-8')
    return hashlib.sha256(raw).hexdigest(), manifest


def parse_digests(text):
    rows = []
    for number, line in enumerate(text.splitlines(), 1):
        match = re.search(r'DIGEST julian=\d+ \((\d{4}-\d\d-\d\d)\)(.*)', line)
        if match:
            # Prefix wall-clock time, process and thread IDs are log metadata.
            rows.append({'date': match[1], 'text': match[0], 'line': number,
                         'fields': dict(re.findall(r'(\S+)=([^\s]+)', match[0]))})
    return rows


def compare_digests(left, right):
    # Dates and occurrence order matter; repeated script digests are not discarded.
    for index in range(max(len(left), len(right))):
        a = left[index] if index < len(left) else None
        b = right[index] if index < len(right) else None
        if a is None or b is None or a['text'] != b['text']:
            af, bf = (a or {}).get('fields', {}), (b or {}).get('fields', {})
            return {'date': min(row['date'] for row in (a, b) if row),
                    'expected': a, 'actual': b,
                    'fields': {key: {'expected': af.get(key), 'actual': bf.get(key)}
                               for key in sorted(af.keys() | bf.keys()) if af.get(key) != bf.get(key)}}
    return None


def compare_artifacts(expected, actual):
    return {'first_digest_difference': compare_digests(load_json(expected / 'digests.json'),
                                                       load_json(actual / 'digests.json')),
            'save': compare_saves(expected / 'end.sc4', actual / 'end.sc4')}


def artifacts_exact(report):
    return report['first_digest_difference'] is None and report['save']['different_records'] == 0


def shadow_command(region, city, until, dll, script=None):
    """Replay from the SAME start save through the failing month, retaining history."""
    command = ['python', 'tools/play.py', '--region', region, '--city', city,
               '--until', until, '--profile', 'shadows', '--dll', str(dll)]
    if script:
        command += ['--script', str(script)]
    return subprocess.list2cmdline(command)


def _baseline_intact(entry):
    metadata = load_json(entry / 'baseline.json')
    for filename, digest in metadata['artifacts'].items():
        if sha256(entry / filename) != digest:
            raise ValueError(f'corrupt golden artifact: {entry / filename}')
    return metadata


def cache_record(cache, key, manifest, run):
    """Publish immutable baseline artifacts. Atomic directory rename wins races.

    Return (status, comparison). Repeated different results are never overwritten.
    Qualification is a separate audit sidecar, not an alteration of the baseline.
    """
    cache, run = Path(cache), Path(run)
    cache.mkdir(parents=True, exist_ok=True)
    entry = cache / key
    if entry.exists():
        metadata = _baseline_intact(entry)
        if metadata['inputs'] != manifest:
            raise ValueError('cache key collision / input manifest differs')
        report = compare_artifacts(entry, run)
        sample = golden_envelope.snapshot(entry, run)
        audit_path = entry / ('repeat-' + uuid.uuid4().hex + '.json')
        write_json(audit_path, {'exact': artifacts_exact(report), 'run': str(run.resolve()),
                                'sample': sample, 'comparison': report})
        return ('reproduced' if artifacts_exact(report) else 'nondeterministic'), report
    stage = cache / ('.pending-' + uuid.uuid4().hex)
    stage.mkdir()
    try:
        artifacts = ['end.sc4', 'digests.json', 'verdict.json', 'summary.json', 'game.log', 'harness.txt']
        if (run / 'reach.tsv').is_file():
            artifacts.append('reach.tsv')
        for filename in artifacts:
            shutil.copyfile(run / filename, stage / filename)
        write_json(stage / 'baseline.json', {'key': key, 'inputs': manifest, 'source_run': str(run.resolve()),
                                             'artifacts': {name: sha256(stage / name) for name in artifacts}})
        try:
            stage.rename(entry)
        except OSError:
            if not entry.exists():
                raise
            return cache_record(cache, key, manifest, run)
    finally:
        if stage.exists():
            shutil.rmtree(stage)
    return 'recorded', None


def qualification(entry):
    audits = [load_json(p) for p in entry.glob('repeat-*.json')]
    if any(not row['exact'] for row in audits):
        return 'nondeterministic'
    return 'reproduced' if audits else 'unverified'


def compare_cached(entry, run, *, rule='exact'):
    """Suite API: byte-exact comparison unless ranges are explicitly requested.

    Only the candidate runs. Rebuild derived ranges from verified originals;
    a candidate result never changes the cache or widens its envelope.
    """
    if rule not in ('exact', 'envelope'):
        raise ValueError('comparison rule must be exact or envelope')
    _baseline_intact(entry)
    envelope = golden_envelope.build(entry)
    mode = golden_envelope.status(entry, envelope)
    use_exact = rule == 'exact' or mode in ('exact', 'unverified')
    report = (compare_artifacts(entry, run) if use_exact
              else golden_envelope.compare(envelope, run))
    report.update(comparison_rule='exact' if use_exact else 'observed-envelope',
                  envelope_status=mode, envelope_samples=envelope['samples'], envelope_stats=envelope['stats'])
    return report


def subsystem_keys(root):
    text = (root / 'engine/plugin/FC4Subsystems.cpp').read_text(encoding='utf-8')
    registry = text.split('const Subsystem kSubsystems[] = {', 1)[1].split('};', 1)[0]
    keys = re.findall(r'^\s*\{"([a-z][a-z0-9_]*)",', registry, flags=re.M)
    if 'simulator' not in keys or 'budget' not in keys:
        raise ValueError('cannot read subsystem registry for selective replacement')
    return sorted(set(keys))


def input_script(path, until):
    text = Path(path).read_text(encoding='utf-8-sig') if path else ''
    if path and Path(path).suffix.lower() == '.lua':
        raise ValueError('Lua actions need the luacmd harness interface; use a .script file for now')
    for number, line in enumerate(text.splitlines(), 1):
        line = line.split('#', 1)[0].strip()
        if not line:
            continue
        words = line.split()
        if len(words) < 2 or date(words[0]) > date(until):
            raise ValueError(f'script line {number} is invalid or after --until')
        # An early pause or speed 0 can be intentional, but cannot silently pass:
        # the completed terminal save and exact end date are validated in the log.
    return text + f'\n{until} pause\n{until} digest\n{until} save\n{until} mark GOLDEN_FULL_SAVE_COMPLETE\n'


def prepare(args):
    region, city = safe_name(args.region), safe_name(args.city)
    date(args.until)
    root = args.harness_root.resolve()
    save = args.regions_root / region / f'City - {city}.sc4'
    if not save.is_file():
        raise ValueError(f'no start save: {save}')
    settings = {'region': region, 'city': city, 'until': args.until, 'speed': 2,
                'turbo_requested': args.turbo, 'turbo_frame_ms': 250, 'pacing': 'original',
                'clock': args.clock, 'fixed_deliveries_per_frame': 64 if args.clock == 'fixed' else None,
                'session_seed': SEED, 'slot': args.slot if args.backend == 'direct' else 'queue-selected',
                'timeout_seconds': args.seconds, 'cpu_count': 1,
                'affinity': 2 ** (2 * args.slot) if args.backend == 'direct' else '2*slot CPU index', 'popup_dialogs': 0,
                'full_save': True, 'environment': 'explicit switches; FC4_* removed in direct backend',
                'backend': args.backend, 'autotest_end': 'terminate', 'window': 'minimized after load'}
    settings['queue_parallel'] = args.backend == 'queue'
    generated = input_script(args.script, args.until)
    settings['effective_script_sha256'] = hashlib.sha256(generated.encode('utf-8')).hexdigest()
    files = {'exe': args.game_root / 'Apps/SimCity 4.exe', 'start.sc4': save,
             'observer.dll': args.original_dll, 'user.cfg': args.regions_root.parent / 'SimCity 4.cfg'}
    for name in ('ingame_run.ps1', 'city_fixture.ps1', 'sc4mutex.ps1', 'sc4ctl.ps1'):
        files['harness/' + name] = root / 'tools' / name
    # Neighbour cities and region layout are simulation inputs too.
    for path in sorted((args.regions_root / region).rglob('*')):
        if path.is_file():
            files['region/' + path.relative_to(args.regions_root / region).as_posix()] = path
    for path in sorted(args.game_root.rglob('*')):
        if path.is_file() and path.suffix.lower() in ('.dat', '.dll', '.sgr', '.ini', '.cfg'):
            files['game/' + path.relative_to(args.game_root).as_posix()] = path
    if args.script:
        files['script'] = args.script
    key, manifest = fingerprint(files, settings)
    return key, manifest, generated, files


def _validate_log(text, until, selected, run):
    verdicts = re.findall(r'AUTOTEST VERDICT: ([a-z-]+)[^\r\n]*', text)
    enabled = set(re.findall(r"subsystem '([^']+)': enabled", text))
    # netrules is hosted by the replacement network manager/tool, with no class
    # registration of its own (FC4Subsystems startup explicitly reports this).
    if 'netrules: enabled through netmgr/nettool;' in text:
        enabled.add('netrules')
    enabled = sorted(enabled)
    if enabled != sorted(selected):
        raise ValueError(f'wrong replacements: expected {sorted(selected)}, logged {enabled}; see {run}')
    unexpected = [line for line in text.splitlines()
                  if re.search(r'\b(?:shadow|compare)[^\r\n]*:\s*(?:ON|enabled)', line, re.I)]
    if unexpected:
        raise ValueError(f'shadow/compare unexpectedly enabled: {unexpected[0]}')
    if not verdicts or verdicts[-1] != 'clean':
        raise ValueError(f'not a clean plugin verdict: {verdicts}; see {run}')
    reached = re.search(rf'autotest: end date {re.escape(until)} reached at the new day {re.escape(until)}', text)
    saved = re.search(rf'script: line \d+ {re.escape(until)} save\s*-> ok=1 \(SaveCity\(full\)', text)
    if not reached or not saved or 'GOLDEN_FULL_SAVE_COMPLETE' not in text:
        raise ValueError(f'window / terminal full save incomplete; see {run}')
    return verdicts[-1]


def run_game(args, generated, selected, dll, tag, *, allow_window_drift=False):
    # Golden originals use only our scheduler; every simulated subsystem stays
    # original. Candidates must use the same scheduler and window settings.
    selected = effective_replacements(args.clock, selected)
    validate_harness_sources(args)
    run_id = f'{tag}-{time.strftime("%Y%m%d-%H%M%S")}-{uuid.uuid4().hex[:8]}'
    work = args.output / 'golden-runs' / run_id
    work.mkdir(parents=True)
    script = work / 'actions.script'
    script.write_text(generated, encoding='utf-8')
    keys = subsystem_keys(args.harness_root) if selected else []
    unknown = set(selected) - set(keys)
    if unknown:
        raise ValueError(f'unknown replacement keys: {sorted(unknown)}')
    disable = ','.join(k for k in keys if k not in selected) if selected else 'all'
    ini = work / 'FreeCity4.ini'
    ini.write_text(f'disable = {disable}\nenable = {",".join(selected)}\nobserve = 1\n'
                   f'session_seed = {SEED}\npacing = original\nsim_turbo = {int(args.turbo)}\n'
                   f'sim_fixed_step = {int(args.clock == "fixed")}\n'
                   f'sim_turbo_frame_ms = 250\npopup_dialogs = 0\nautotest_unpause = 2\n'
                   f'autotest_end = terminate\nautotest_seconds = {args.seconds}\nscript = {script.resolve()}\n', encoding='utf-8')
    # Explicitly turn every known shadow/compare off, also for a queue worker
    # whose environment may contain developer overrides.
    config = (args.harness_root / 'engine/plugin/FC4Config.cpp').read_text(encoding='utf-8')
    verification = sorted(set(re.findall(r'"([a-z0-9_]*(?:shadow|compare)[a-z0-9_]*)"', config)))
    with ini.open('a', encoding='utf-8') as stream:
        stream.write(''.join(f'{key} = 0\n' for key in verification))
    result = work / 'result'
    command = ['powershell', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File',
               str(Path(__file__).with_name('golden_run.ps1')), '-HarnessRoot', str(args.harness_root.resolve()),
               '-Dll', str(dll.resolve()), '-IniFile', str(ini.resolve()), '-Output', str(result.resolve()),
               '-Region', args.region, '-City', args.city, '-Until', args.until,
               '-Slot', str(args.slot), '-Seconds', str(args.seconds)]
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith('FC4_')}
    start = time.perf_counter()
    if args.backend == 'queue':
        run_queued(args, dll, ini, result, work)
    else:
        process = subprocess.run(command, env=env, capture_output=True, text=True,
                                 timeout=args.seconds + 300)
        (work / 'transport.txt').write_text(process.stdout + process.stderr, encoding='utf-8')
        if process.returncode:
            raise ValueError(f'game harness failed ({process.returncode}); see {work / "transport.txt"}')
    text = (result / 'game.log').read_text(encoding='utf-8-sig', errors='replace')
    verdict = _validate_log(text, args.until, selected, result)
    if args.clock == 'fixed' and 'sim_fixed_step ON: 64 agent deliveries/frame' not in text:
        raise ValueError(f'DLL did not acknowledge sim_fixed_step; not cached: {result}')
    digests = parse_digests(text)
    if not digests:
        raise ValueError(f'no DIGEST observations; see {result}')
    saved_records = read_records(result / 'end.sc4')
    saved_date = save_date(saved_records)
    if saved_date != args.until and not allow_window_drift:
        raise ValueError(f'end save is dated {saved_date}, expected {args.until}; not cached: {result}')
    write_json(result / 'digests.json', digests)
    write_json(result / 'verdict.json', {'verdict': verdict, 'end_reached': True, 'full_save': True,
                                       'window_exact': saved_date == args.until, 'save_date': saved_date})
    figures = re.findall(r'days_per_second=([\d.]+)', text)
    warnings = [line for line in text.splitlines() if 'WARNING' in line or 'session seed:' in line]
    summary = {'wall_seconds': time.perf_counter() - start, 'game': load_json(result / 'run.json'),
               'days_per_second': figures[-1] if figures else None, 'digest_lines': len(digests),
               'replacements': selected, 'dll_sha256': sha256(dll), 'warnings': warnings}
    write_json(result / 'summary.json', summary)
    # A future reach-enabled harness may export alongside the plugin log.
    # Never copy a stale shared slot reach file into this run.
    return result


def validate_harness_sources(args):
    """The existing runner chooses these paths itself; never hash other inputs."""
    expected_regions = documents() / 'SimCity 4/Regions'
    if args.regions_root.resolve() != expected_regions.resolve():
        raise ValueError('--regions-root differs from the harness Documents region source')
    runner = (args.harness_root / 'tools/ingame_run.ps1').read_text(encoding='utf-8')
    installed = re.search(r'^\$GameDir = "([^"]+)"', runner, re.M)
    if not installed or (args.game_root / 'Apps').resolve() != Path(installed[1]).resolve():
        raise ValueError('--game-root differs from the executable the harness launches')
    if args.backend == 'queue' and args.harness_root.resolve() != main_checkout(REPO).resolve():
        raise ValueError('queue backend must fingerprint the main checkout harness actually run by the queue')


def save_date(saved_records):
    """Simulator v4: class-object framing + engine/core/src/save.rs RECORD_LAYOUT_V4."""
    import struct
    values = [value for key, value in saved_records.items() if key[0] == 0x2990C1E5]
    if len(values) != 1 or len(values[0]) != 91 or values[0][12:14] != b'\x04\x00':
        raise ValueError('end save has no unique supported simulator v4 record')
    julian = struct.unpack_from('<I', values[0], 87)[0]
    return str(dt.date(2000, 1, 1) + dt.timedelta(days=julian - 2451545))


def run_queued(args, dll, ini, result, work):
    """Use the already-running coordinator queue (no CIM privileges needed here).

    Only submit from a checkout's build-runs. The snapshot verifies that the
    slot has not been reused; export races fail closed with retained evidence.
    """
    if args.output.resolve() != (REPO / 'build-runs').resolve():
        raise ValueError('queue backend requires --output <this checkout>/build-runs')
    queue_text = (args.harness_root / 'tools/game_queue.ps1').read_text(encoding='utf-8')
    region_ready = bool(re.search(r'\bregion\s*=', queue_text, re.I))
    if args.region != 'FC4 Test' and not region_ready:
        raise ValueError('queue region key is not available yet; refusing an FC4 Test fallback')
    label = 'golden-' + uuid.uuid4().hex
    staged = args.output / 'golden-inputs' / (sha256(dll) + '.dll')
    staged.parent.mkdir(parents=True, exist_ok=True)
    if not staged.exists():
        shutil.copyfile(dll, staged)
    if sha256(staged) != sha256(dll):
        raise ValueError('staged candidate DLL differs')
    queue = args.output / 'game-queue'
    queue.mkdir(parents=True, exist_ok=True)
    request = (f'label = {label}\ndll = {staged.relative_to(REPO).as_posix()}\ncity = {args.city}\n'
               f'parallel = 1\nseconds = {args.seconds}\ntimeout = {args.seconds + 150}\nend_date = {args.until}\n')
    if region_ready:
        request += f'region = {args.region}\n'
    request += '[ini]\n' + ini.read_text(encoding='utf-8')
    temporary = queue / (label + '.tmp')
    temporary.write_text(request, encoding='utf-8')
    result.mkdir()
    temporary.rename(queue / (label + '.req'))
    verdict_file = args.output / 'game-results' / (label + '.verdict')
    verdict = wait_snapshot(args, verdict_file, result, work)
    (result / 'harness.txt').write_text(verdict, encoding='utf-8')
    log = verdict_file.with_suffix('.log')
    if log.exists():
        shutil.copyfile(log, result / 'game.log')
    if not verdict.startswith('verdict: clean'):
        raise ValueError(f'queue did not finish cleanly; see {result / "harness.txt"}')
    slot_match = re.search(r'in slot (\d+) after', verdict)
    if not slot_match:
        raise ValueError('queue result does not identify its slot')
    slot = int(slot_match[1])
    wall = re.search(r'wall (\d+) s', verdict)
    write_json(result / 'run.json', {'slot': slot, 'label': label,
                                    'wall_seconds': int(wall[1]) if wall else None, 'backend': 'queue'})


def wait_snapshot(args, verdict_file, result, work):
    """Queue for UserDir while the game owns it, before another job can reuse it."""
    deadline = time.monotonic() + args.queue_timeout
    process = None
    try:
        while not verdict_file.exists():
            marker = verdict_file.with_suffix('.running')
            if process is None and marker.is_file():
                match = re.search(r'running in slot (\d+)', marker.read_text(encoding='utf-8-sig'))
                if match:
                    command = ['powershell', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
                               '-File', str(Path(__file__).with_name('golden_snapshot.ps1')),
                               '-HarnessRoot', str(args.harness_root.resolve()), '-Output', str(result.resolve()),
                               '-City', args.city, '-Region', args.region, '-Slot', match[1],
                               '-ScriptPath', str((work / 'actions.script').resolve()),
                               '-ExpectedLog', str(verdict_file.with_suffix('.log').resolve()),
                               '-VerdictFile', str(verdict_file.resolve()), '-WaitSeconds', str(args.seconds + 150)]
                    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            if time.monotonic() > deadline:
                raise ValueError(f'queue wait timed out; request may still run: {verdict_file.stem}')
            time.sleep(1)
        verdict = verdict_file.read_text(encoding='utf-8-sig')
        if not verdict.startswith('verdict: clean'):
            (result / 'harness.txt').write_text(verdict, encoding='utf-8')
            raise ValueError(f'queue did not finish cleanly; see {result / "harness.txt"}')
        if process is None:
            raise ValueError('missed active slot marker; no safe save export, retry the run')
        output, _ = process.communicate(timeout=150)
        (work / 'transport.txt').write_text(output, encoding='utf-8')
        if process.returncode:
            raise ValueError(f'completed run could not be exported; see {work / "transport.txt"}')
        return verdict
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.communicate(timeout=30)


def print_comparison(report, args):
    envelope_mode = report.get('comparison_rule') == 'observed-envelope'
    if 'envelope_stats' in report:
        print('comparison rule: ' + report['comparison_rule'] + '; sampling: ' + report['envelope_status'])
        print_envelope({'samples': report['envelope_samples'], 'stats': report['envelope_stats']})
    first = report['first_digest_difference']
    if first:
        print(f'first DIGEST difference: {first["date"]} (month {first["date"][:7]})')
        for key, value in first['fields'].items():
            print(f'  {key}: {value["expected"]} -> {value["actual"]}')
    else:
        print('DIGEST: within envelope' if envelope_mode else 'DIGEST: exact')
    save = report['save']
    label = 'within envelope' if envelope_mode else 'exact'
    print(f'save records: {save["exact_records"]} {label}, {save["different_records"]} different')
    for record in save['records'][:20]:
        print(f'  {record["tgi"]}: {record["status"]}, {record["expected_size"]} -> {record["actual_size"]} bytes; '
              f'first byte {record.get("first_byte", "-")}')
        for field, value in record.get('fields', {}).items():
            print(f'    {field}: {value["expected"]!r} -> {value["actual"]!r}')
    if save['different_records'] > 20:
        print('  remaining records are in comparison.json')
    if not artifacts_exact(report):
        until = first['date'] if first else args.until
        print('localize (replay from original start save through failing month): ' +
              shadow_command(args.region, args.city, until, args.dll or args.original_dll, args.script))


def effective_replacements(clock, selected):
    return sorted(set(selected) | ({'simulator'} if clock == 'fixed' else set()))


def print_envelope(envelope):
    st = envelope['stats']
    print(f'envelope: {envelope["samples"]} originals; {st["varying_records"]}/{st["records"]} records, '
          f'{st["varying_decoded_fields"]} decoded/structure fields, {st["varying_opaque_bytes"]} opaque byte fields, '
          f'{st["varying_digest_fields"]} digest fields vary; median decoded numeric spread '
          f'{st["median_decoded_numeric_spread"]}', flush=True)


def parser():
    p = argparse.ArgumentParser(description=__doc__)
    commands = p.add_subparsers(dest='command', required=True)
    main = main_checkout(REPO)
    for name in ('record', 'compare', 'determinism', 'key'):
        sub = commands.add_parser(name)
        sub.add_argument('--region', default='FC4 Test')
        sub.add_argument('--city', required=True)
        sub.add_argument('--script', type=Path)
        sub.add_argument('--until', required=True)
        sub.add_argument('--harness-root', type=Path, default=main)
        sub.add_argument('--original-dll', type=Path, default=main / 'engine/build-x86/FreeCity4.dll')
        sub.add_argument('--game-root', type=Path, default=Path(r'C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition'))
        sub.add_argument('--regions-root', type=Path, default=documents() / 'SimCity 4/Regions')
        sub.add_argument('--output', type=Path, default=REPO / 'build-runs')
        sub.add_argument('--slot', type=int, choices=range(1, 17), default=6)
        sub.add_argument('--backend', choices=('queue', 'direct'), default='queue')
        sub.add_argument('--queue-timeout', type=int, default=1800)
        sub.add_argument('--seconds', type=int, default=180)
        sub.add_argument('--turbo', action='store_true', help='request turbo (only our simulator supports it)')
        sub.add_argument('--clock', choices=('fixed', 'original'), default='original',
                         help='original scheduler (default); fixed replaces only the scheduler for an explicit experiment')
        sub.add_argument('--comparison', choices=('exact', 'envelope'), default='exact',
                         help='exact (default); envelope is an exploratory range diagnostic, not determinism')
        sub.add_argument('--dll', type=Path, help='candidate worktree or DLL (compare only)')
        sub.add_argument('--replace', default='simulator,budget')
        sub.add_argument('--rerun', action='store_true', help='record: independently repeat an existing key')
        sub.add_argument('--repeats', type=int, default=3, help='determinism: independent original runs')
        sub.add_argument('--max-repeats', type=int, default=9,
                         help='extend originals until the varying-field set stops growing (default 9)')
    return p


def main(argv=None):
    args = parser().parse_args(argv)
    try:
        if args.seconds < 30 or args.seconds > 3600 or not 3 <= args.repeats <= args.max_repeats <= 50:
            raise ValueError('--seconds must be 30..3600 and 3 <= --repeats <= --max-repeats <= 50')
        key, manifest, generated, files = prepare(args)
        entry = args.output / 'golden' / key
        print('key: ' + key, flush=True)
        if args.command == 'key':
            print(json.dumps(manifest, indent=2, sort_keys=True))
            return 0
        if args.command == 'compare':
            if not args.dll:
                raise ValueError('compare requires --dll <worktree-or-file>')
            if not entry.exists():
                raise ValueError('no golden baseline for this key; run record with the same inputs first')
            _baseline_intact(entry)
            selected = sorted(set(filter(None, re.split(r'[,\s]+', args.replace))))
            if not selected:
                raise ValueError('compare needs at least one --replace key')
            dll = args.dll / 'engine/build-x86/FreeCity4.dll' if args.dll.is_dir() else args.dll
            run = run_game(args, generated, selected, dll, 'compare', allow_window_drift=True)
            if fingerprint(files, manifest['settings'])[0] != key:
                raise ValueError(f'input changed during comparison; no verdict: {run}')
            report = compare_cached(entry, run, rule=args.comparison)
            compatible = 'within-envelope' if report['comparison_rule'] == 'observed-envelope' else 'exact'
            report.update(key=key, baseline_repeatability=qualification(entry), run=str(run),
                          verdict=compatible if artifacts_exact(report) else 'different')
            window = load_json(run / 'verdict.json')
            report['window_exact'] = window['window_exact']
            report['save_date'] = window['save_date']
            if not window['window_exact']:
                report['verdict'] = 'invalid-window'
                print(f'window mismatch: save dated {window["save_date"]}, expected {args.until}; diagnostic differences only')
            write_json(run / 'comparison.json', report)
            print('baseline repeatability: ' + report['baseline_repeatability'])
            print_comparison(report, args)
            print('comparison: ' + str(run / 'comparison.json'))
            if not window['window_exact']:
                return 2
            if args.comparison == 'exact' and report['baseline_repeatability'] == 'nondeterministic':
                print('originals are nondeterministic; differences are diagnostic, no regression verdict')
                return 3
            if args.comparison == 'envelope' and report['envelope_status'] in ('unverified', 'growing'):
                print('sampling incomplete; run determinism to qualify the cached comparison')
                return 3
            return 0 if artifacts_exact(report) else 1
        if entry.exists() and args.command == 'record' and not args.rerun:
            _baseline_intact(entry)
            envelope = golden_envelope.build(entry)
            mode = golden_envelope.status(entry, envelope)
            print(f'cached: {entry}; repeatability: {qualification(entry)}; sampling: {mode}; no game launched')
            print_envelope(envelope)
            if args.comparison == 'exact':
                return 3 if qualification(entry) == 'nondeterministic' else 0
            return 0 if mode in ('exact', 'envelope') else 3
        count = args.repeats if args.command == 'determinism' else 1
        limit = args.max_repeats if count > 1 and args.comparison == 'envelope' else count
        mode = 'unverified'
        for attempt in range(limit):
            run = run_game(args, generated, [], args.original_dll, 'original')
            # Reject runs if a source changed underneath the slot mirror or DLL copy.
            if fingerprint(files, manifest['settings'])[0] != key:
                raise ValueError(f'input changed during run; not cached: {run}')
            status, report = cache_record(args.output / 'golden', key, manifest, run)
            print(f'{status}: {run}', flush=True)
            if report:
                write_json(run / 'comparison.json', report)
                print_comparison(report, args)
            envelope = golden_envelope.build(entry)
            mode = golden_envelope.status(entry, envelope)
            write_json(entry / 'envelope.json', envelope)
            print_envelope(envelope)
            print('sampling: ' + mode, flush=True)
            if attempt + 1 >= count and (args.comparison == 'exact' or mode in ('exact', 'envelope') or count == 1):
                break
        print('baseline repeatability: ' + qualification(entry))
        if args.comparison == 'exact':
            return 3 if qualification(entry) == 'nondeterministic' else 0
        return 0 if mode in ('exact', 'envelope') else 3
    except (ValueError, OSError, subprocess.SubprocessError) as exc:
        print('golden: ' + str(exc), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())

"""Declared setup runs and hash-checked saved-state reuse for suite windows."""
import calendar
from datetime import date, timedelta
import hashlib
import json
from pathlib import Path
import re


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def city_source(row):
    return Path.home() / 'Documents/SimCity 4/Regions' / row['region'] / ('City - ' + row['city'] + '.sc4')


def dated_lines(path):
    lines = []
    for line in path.read_text(encoding='utf-8-sig').splitlines():
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        match = re.fullmatch(r'(\d{4}-\d\d-\d\d)\s+(\S+)(.*)', line)
        if not match:
            raise ValueError(f'cannot split setup script line: {line!r}')
        lines.append((date.fromisoformat(match[1]), match[2], line))
    return lines


def state_directory(root, tag):
    import suite
    if not suite.NAME.fullmatch(tag):
        raise ValueError('invalid state tag')
    return root / 'build-runs/suite/states' / tag


def prepare(root, rows, tag, months):
    """Setup is its own plain-profile batch; its actions never enter measured rows."""
    if months < 1:
        raise ValueError('warmup months must be positive')
    directory = state_directory(root, tag)
    if directory.exists():
        raise ValueError('state tag already exists')
    plans, selected = [], []
    for row in rows:
        if 'load-only' in row['notes'].lower() or Path(row['script']).suffix != '.script':
            print(f'{row["name"]}: no prepared state (load-only or unsplittable Lua/input)')
            continue
        saved = re.search(r'\bsaved (\d{4}-\d\d-\d\d)', row['notes'])
        if not saved:
            raise ValueError(f'{row["name"]}: missing saved date for setup')
        start = date.fromisoformat(saved[1])
        year, month0 = divmod(start.year * 12 + start.month - 1 + months, 12)
        cutoff = date(year, month0 + 1, min(start.day, calendar.monthrange(year, month0 + 1)[1]))
        actions = dated_lines(root / row['script'])
        # Emergency pauses and terminal probes belong to the measured window.
        terminal = min([d for d, verb, _ in actions if verb == 'disaster'] + [date.fromisoformat(row['end'])])
        cutoff = min(cutoff, terminal - timedelta(days=1))
        if cutoff <= start:
            raise ValueError(f'{row["name"]}: no warmup window before terminal probes')
        lines = [text for d, verb, text in actions if d <= cutoff and verb != 'save']
        lines += [f'{cutoff} save', f'{cutoff} digest', f'{cutoff} mark setup-complete:{row["name"]}']
        script = directory / (row['name'] + '.setup.script')
        plans.append(dict(scenario=row, setup_until=str(cutoff), setup_actions=len(lines),
                          source_city_sha256=sha(city_source(row)),
                          source_script_sha256=sha(root / row['script']),
                          script=script, text='\n'.join(lines) + '\n'))
    if not plans:
        raise ValueError('no splittable scenarios to prepare')
    directory.mkdir(parents=True)
    for plan in plans:
        script = plan.pop('script')
        script.write_text(plan.pop('text'), encoding='utf-8')
        row = plan['scenario']
        selected.append(dict(row, script=script.relative_to(root).as_posix(), end=plan['setup_until'],
                             _export_city=(directory / (row['name'] + '.sc4')).as_posix(),
                             notes=row['notes'] + '; declared setup, not a measured window'))
    (directory / 'plan.json').write_text(json.dumps(plans, indent=2) + '\n', encoding='utf-8')
    return selected


def finalize(root, tag, manifest, results):
    directory = state_directory(root, tag)
    import suite
    complete = 0
    for plan in json.loads((directory / 'plan.json').read_text(encoding='utf-8')):
        row = plan['scenario']
        name = row['name']
        save = directory / (name + '.sc4')
        if name in results and suite.successful({name: results[name]}) and save.is_file():
            if sha(city_source(row)) != plan['source_city_sha256']:
                raise ValueError(f'{name}: original city changed during setup')
            plan.update(save_sha256=sha(save), dll_sha256=manifest['dll_sha256'], setup_tag=manifest['tag'])
            (directory / (name + '.json')).write_text(json.dumps(plan, indent=2) + '\n', encoding='utf-8')
            complete += 1
    print(f'prepared states: {complete}/{len(manifest["requests"])}; {directory}')
    return complete == len(manifest['requests'])


def reuse(root, rows, tag, dll):
    directory = state_directory(root, tag)
    selected = []
    for row in rows:
        metadata = directory / (row['name'] + '.json')
        if not metadata.is_file():
            raise ValueError(f'{row["name"]}: no completed prepared state in {tag}')
        plan = json.loads(metadata.read_text(encoding='utf-8'))
        if any(plan['scenario'][key] != row[key] for key in ('region', 'city', 'script', 'end')):
            raise ValueError(f'{row["name"]}: stale prepared scenario')
        save = directory / (row['name'] + '.sc4')
        if (sha(save) != plan['save_sha256'] or sha(city_source(row)) != plan['source_city_sha256']
                or sha(root / row['script']) != plan['source_script_sha256'] or sha(dll) != plan['dll_sha256']):
            raise ValueError(f'{row["name"]}: prepared save/source/script/DLL hash changed')
        lines = [text for d, _, text in dated_lines(root / row['script']) if str(d) > plan['setup_until']]
        script = directory / (row['name'] + '.measured.script')
        text = '# Measured window; setup actions are recorded separately in the state metadata.\n'
        text += '\n'.join(lines) + '\n'
        if not script.exists() or script.read_text(encoding='utf-8') != text:
            script.write_text(text, encoding='utf-8')
        selected.append(dict(row, script=script.relative_to(root).as_posix(),
                             _prepared_city=save.as_posix(), _source_scenario=row,
                             _source_script_sha256=plan['source_script_sha256'],
                             _prepared_until=plan['setup_until'], _setup_actions=plan['setup_actions']))
    return selected

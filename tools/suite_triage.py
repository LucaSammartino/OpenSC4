"""Reparse a staged suite without rerunning games; keep failure evidence verbatim."""
import argparse
import collections
import json
from pathlib import Path
import re

import suite


def diagnose(text, verdict_text, end, *, only_load=False):
    result = suite.parse_log(text, end, verdict_text, only_load=only_load)
    lines = text.splitlines()
    raw = next((line for line in verdict_text.splitlines() if line.startswith('line: ')), '')
    if not raw:
        raw = next((line for line in lines if 'AUTOTEST VERDICT:' in line), '-')
    exception = result['exceptions'][0] if result['exceptions'] else None
    stall = next(((i, line) for i, line in enumerate(lines, 1)
                  if 'WATCHDOG: main thread' in line and 'stalled' in line), None)
    point = '-'
    address = exception['address'] if exception else '-'
    location = exception['location'] if exception else '-'
    if exception:
        point = f"line {exception['line']}: {exception['text']}"
    elif stall:
        i, line = stall
        point = f'line {i}: {line}'
        context = next((line for line in lines[i:i + 8] if 'context: eip=' in line), '')
        match = re.search(r'eip=(0x[0-9a-fA-F]+)', context)
        if match:
            address = match[1]
            point += ' / ' + context
            module = re.search(r'\(([^()]+\+0x[0-9a-fA-F]+)\)', context)
            if module:
                location = module[1]
    else:
        # A process exit is not an exception address. Keep the last completed
        # action/window/clock marker, never infer the instruction that faulted.
        progress = [(i, line) for i, line in enumerate(lines, 1)
                    if re.search(r'script: line |reach window |autotest: clock started;', line)]
        if progress:
            i, line = progress[-1]
            point = f'line {i}: {line}'
    verdict = result['verdict']
    if not text and 'lua_script must name an existing repo-relative .lua file' in verdict_text:
        cause = 'launcher-lua-checkout'
        point = 'launcher rejected the staged Lua path before starting the game'
    elif verdict == 'no-load' and 'autotest: clock started;' in text:
        cause = 'launcher-load-detection'
    elif verdict == 'invalid' and 'days_per_second=0.00' in raw and 'date=2000-01-01' in raw:
        cause = 'unestablished-city'
    elif verdict == 'invalid' and result['invalid_reason'] == 'end date not reached':
        cause = 'window-exhausted'
    elif verdict == 'clean-exit-crash':
        cause = 'clean-exit-crash'
    elif exception:
        cause = 'exception'
    elif stall:
        cause = 'watchdog-stall'
    elif verdict == 'exited':
        cause = 'unhandled-exit-no-address'
    else:
        cause = verdict
    watchdog = next((suite.ENVELOPE.sub('', line) for line in lines
                     if 'WATCHDOG:' in line and ('stalled' in line or 'stuck' in line)), '-')
    name = re.search(r'stalled\s*-\s*(.*?)\s+for\s+\d+\s*ms', watchdog)
    watchdog_name = name[1] if name else '-'
    saving = next((line for line in reversed(lines)
                   if 'script: full-save watchdog grace' in line or 'script: line ' in line), '')
    stall_sample = '\n'.join(lines[stall[0]:stall[0] + 80]) if stall else ''
    if stall and ('SaveCity' in point or 'RunSave' in stall_sample
                  or ('full-save watchdog grace' in saving and 'SaveCity(' not in saving)):
        cause = 'save-watchdog'
    owner = ('harness' if cause in ('launcher-lua-checkout', 'launcher-load-detection',
              'unestablished-city', 'window-exhausted', 'save-watchdog', 'rejected',
              'no-load', 'queue-timeout', 'not-submitted', 'missing') else
             'none' if verdict == 'clean' else 'game-or-engine')
    return dict(verdict=verdict, cause=cause, verdict_line=raw, address=address,
                first_exception_or_stall=point, end_reached=result['end_reached'],
                watchdog=watchdog, watchdog_name=watchdog_name, owner=owner, location=location,
                last_lines=' / '.join(lines[-5:]) or '-')


def triage(directory):
    manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
    rows = []
    for request in manifest['requests']:
        scenario = request['scenario']
        name = scenario['name']
        log = directory / (name + '.log')
        verdict = directory / (name + '.verdict')
        verdict_text = verdict.read_text(encoding='utf-8', errors='replace') if verdict.exists() else 'verdict: missing\n'
        row = diagnose(log.read_text(encoding='utf-8', errors='replace') if log.exists() else '',
                       verdict_text, scenario['end'],
                       only_load=suite.load_only(scenario))
        rows.append(dict(scenario=name, **row, reach='present' if
                         (directory / (name + '.reach.tsv')).exists() else 'missing',
                         log=str(log) if log.exists() else '-'))
    if not rows:
        raise ValueError('no returned verdicts in this tag')
    output = directory / 'triage.tsv'
    suite.write_tsv(output, list(rows[0]), rows)
    suite.write_tsv(directory / 'causes.tsv', list(rows[0]),
                    sorted((r for r in rows if r['verdict'] != 'clean'),
                           key=lambda r: (r['owner'], r['cause'], r['location'], r['scenario'])))
    grouped = collections.defaultdict(list)
    for row in rows:
        if row['verdict'] != 'clean':
            grouped[(row['owner'], row['cause'], row['verdict'], row['location'], row['watchdog_name'])].append(row)
    groups = []
    for (owner, cause, verdict, location, watchdog_name), members in sorted(grouped.items()):
        groups.append(dict(owner=owner, cause=cause, verdict=verdict, location=location,
                           watchdog_name=watchdog_name, count=len(members),
                           scenarios=','.join(r['scenario'] for r in members),
                           evidence=members[0]['first_exception_or_stall'], log=members[0]['log']))
    suite.write_tsv(directory / 'cause_groups.tsv', ['owner', 'cause', 'verdict', 'location',
                    'watchdog_name', 'count', 'scenarios', 'evidence', 'log'], groups)
    return rows, output


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tag')
    args = parser.parse_args(argv)
    if not suite.NAME.fullmatch(args.tag):
        parser.error('invalid suite tag')
    try:
        rows, output = triage(suite.ROOT / 'build-runs/suite' / args.tag)
    except (ValueError, OSError) as exc:
        parser.error(str(exc))
    returned = sum(row['verdict'] not in ('missing', 'not-submitted', 'queue-timeout') for row in rows)
    print(f'triage: {returned}/{len(rows)} returned results; {output}; causes.tsv')
    for cause, count in sorted(collections.Counter(row['cause'] for row in rows).items()):
        print(f'{cause}: {count}')


if __name__ == '__main__':
    main()

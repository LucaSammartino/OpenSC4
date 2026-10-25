"""Run one saved city with the same isolated queue/profiles as tools/suite.py."""
import argparse
from pathlib import Path
import sys

import suite


def main(argv=None, root=suite.ROOT):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--region', default='FC4 Test')
    parser.add_argument('--city', required=True)
    parser.add_argument('--script', help='checkout-relative .script or .lua file')
    parser.add_argument('--do', action='append', default=[], help='one dated plugin script action')
    parser.add_argument('--until', required=True)
    parser.add_argument('--profile', choices=['shadows', 'replace', 'plain'], default='plain')
    parser.add_argument('--dll', help='worktree or DLL; default main checkout DLL')
    parser.add_argument('--tag')
    parser.add_argument('--extra', action='append', default=[])
    parser.add_argument('--wait-seconds', type=int, default=1800)
    parser.add_argument('--dry-run', action='store_true')
    args = parser.parse_args(argv)
    try:
        suite.folder_name(args.region); suite.folder_name(args.city); suite.game_date(args.until)
        tag = args.tag or 'play-' + suite.new_tag()
        if not suite.NAME.fullmatch(tag):
            raise ValueError('invalid tag')
        script = args.script or '-'
        if args.script:
            suite.script_file(root, args.script)
        if args.do:
            lines = []
            if args.script:
                source = suite.script_file(root, args.script)
                if source.suffix.lower() == '.lua':
                    raise ValueError('--do cannot be combined with a Lua script')
                lines = source.read_text(encoding='utf-8-sig').splitlines()
            for action in args.do:
                if any(ord(c) < 32 for c in action) or len(action.split()) < 2:
                    raise ValueError('--do expects one dated action per argument')
                suite.game_date(action.split()[0])
                lines.append(action)
            generated = root / 'build-runs/play' / (tag + '.script')
            generated.parent.mkdir(parents=True, exist_ok=True)
            if generated.exists():
                raise ValueError(f'input already exists: {generated}')
            generated.write_text('\n'.join(lines) + '\n', encoding='utf-8')
            script = generated.relative_to(root).as_posix()
        scenario = dict(name='play', region=args.region, city=args.city, script=script,
                        end=args.until, sets='', notes='one-off command')
        directory, results = suite.run_scenarios(root, [scenario], profile=args.profile,
            dll=args.dll, tag=tag, extra=args.extra, wait_seconds=args.wait_seconds, dry_run=args.dry_run)
        if args.dry_run:
            return 0
        result = results['play']
        print(f'verdict: {result["verdict"]}; end_reached={result["end_reached"]}; days/s={result["days_per_second"]}')
        for line in result['action_lines']:
            print(line)
        print(f'actions: {result["actions_ok"]} ok, {result["actions_failed"]} failed, {result["actions_pending"]} pending')
        for key, counts in sorted(result['per_key'].items()):
            print(f'{key}: {counts["exact"]}/{counts["lines"]} shadow lines exact')
            if counts['first_mismatch']:
                print('  ' + counts['first_mismatch'])
        print(f'compare: {result["compare_exact"]} exact, {result["compare_different"]} different; reach={result["reach_functions"]}')
        print(f'evidence: {directory}')
        return 0 if suite.successful(results) else 1
    except (ValueError, OSError) as exc:
        parser.error(str(exc))


if __name__ == '__main__':
    sys.exit(main())

"""Scripted JSONL Codex process for transport tests; never contacts a model."""
import json
import os
from pathlib import Path
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / '.claude/lane-bin'))
import codexguard


def main():
    plan_path = Path(os.environ['FC4_FAKE_CODEX_PLAN'])
    plan = json.loads(plan_path.read_text(encoding='utf-8'))
    calls_path = plan_path.with_suffix('.calls.json')
    calls = json.loads(calls_path.read_text()) if calls_path.exists() else []
    step = plan['steps'][len(calls)]
    args = sys.argv[1:]
    calls.append(dict(args=args, prompt=sys.stdin.read()))
    calls_path.write_text(json.dumps(calls), encoding='utf-8')
    manifest = plan['manifest']
    if step.get('fatal'):
        Path(manifest['manifest']).with_suffix('.fatal.json').write_text('{"error":"test hook failure"}')
    session = step.get('session', 'fake-session')
    if not step.get('skip_hooks'):
        transcript = plan_path.with_suffix(f'.{len(calls)}.jsonl')
        transcript.write_text(json.dumps(dict(type='event_msg', payload=dict(type='token_count',
            info=dict(last_token_usage=dict(input_tokens=1000))))) + '\n', encoding='utf-8')
        data = dict(cwd=manifest['worktree'], session_id=session, transcript_path=str(transcript))
        codexguard.handle(data | dict(hook_event_name='SessionStart'), manifest)
        codexguard.handle(data | dict(hook_event_name='PreToolUse', tool_name='Read', tool_input={},
                                     tool_use_id=str(len(calls))), manifest)
    print(json.dumps(dict(type='thread.started', thread_id=session)), flush=True)
    for event in step.get('events', []):
        print(json.dumps(event), flush=True)
    if step.get('final'):
        Path(args[args.index('-o') + 1]).write_text(json.dumps(step['final']), encoding='utf-8')
    if step.get('sleep'):
        time.sleep(step['sleep'])
    return step.get('code', 0)


if __name__ == '__main__':
    sys.exit(main())

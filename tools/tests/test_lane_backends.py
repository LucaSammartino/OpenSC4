"""Provider parity: shared policy and budgets, correct auth/transport, no unguarded fallback."""
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

from test_laneguard import Fixture, LANE, git

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO/'tools'))
sys.path.insert(0, str(REPO/'.claude/lane-bin'))
import codexguard
import lane_backend
import lane_launch
import lane_refill
import laneguard


class CodexGuard(unittest.TestCase):
    def setUp(self):
        self.f = Fixture(calls=2)
        self.manifest = self.f.main/'build-runs/codex-manifest.json'
        self.transcript = self.f.main/'build-runs/codex.jsonl'
        self.m = dict(main=str(self.f.main), worktree=str(self.f.wt), lane=LANE, kind='tool', provider='codex',
                      guard_id='codex-'+'a'*32, budget=150000, max_calls=2, round=2,
                      status_path=str(self.f.main/'build-runs/status.json'))
        self.manifest.write_text(json.dumps(self.m))
        self.set_context(1000)
        self.start()

    def tearDown(self):
        self.f.close()

    def set_context(self, n):
        self.transcript.write_text(json.dumps(dict(type='event_msg', payload=dict(type='token_count', info=dict(
            last_token_usage=dict(input_tokens=n, cached_input_tokens=n-1),
            total_token_usage=dict(input_tokens=n*100)))))+'\n')

    def data(self, event, **kw):
        return dict(hook_event_name=event, session_id='session1', cwd=str(self.f.wt),
                    transcript_path=str(self.transcript), model='fixture-model', **kw)

    def start(self):
        return codexguard.handle(self.data('SessionStart'), self.m)

    def call(self, tool='Bash', command='Get-Date', identity=None):
        return codexguard.handle(self.data('PreToolUse', tool_name=tool,
                                          tool_input={'command': command}, tool_use_id=identity), self.m)

    def test_context_is_last_input_not_cumulative_or_cache_sum(self):
        self.set_context(100000)
        self.assertEqual(codexguard.context_tokens(self.transcript), 100000)
        self.assertEqual(self.call()[0], 'allow')

    def test_missing_context_denies_instead_of_silently_disabling_budget(self):
        self.transcript.write_text('{}\n')
        p = subprocess.run([sys.executable, str(REPO/'.claude/lane-bin/codexguard.py'), 'hook',
                            '--manifest', str(self.manifest)],
                           input=json.dumps(self.data('PreToolUse', tool_name='Bash', tool_input={'command':'whoami'})),
                           capture_output=True, text=True)
        self.assertEqual(p.returncode, 0)
        self.assertEqual(json.loads(p.stdout)['hookSpecificOutput']['permissionDecision'], 'deny')
        self.assertTrue(self.manifest.with_suffix('.fatal.json').exists())

    def test_stop_allows_only_handoff_patch_and_own_commit(self):
        laneguard.write_stop(str(self.f.main), 'test')
        self.assertEqual(self.call()[0], 'deny')
        good = f'*** Begin Patch\n*** Update File: docs/lanes/{LANE}.md\n@@\n-old\n+new\n*** End Patch'
        self.assertEqual(self.call('apply_patch', good)[0], 'allow')
        self.assertEqual(self.call('apply_patch', good.replace(f'docs/lanes/{LANE}.md', 'engine/source.rs'))[0], 'deny')
        self.assertEqual(self.call(command=self.f.commit_cmd()['command'])[0], 'allow')
        self.assertEqual(self.call(command=f'git -C {self.f.main.as_posix()} commit -m bad')[0], 'deny')

    def test_patch_moves_cannot_escape_handoff(self):
        patch_text = f'*** Begin Patch\n*** Update File: docs/lanes/{LANE}.md\n*** Move to: ../../bad\n*** End Patch'
        self.assertFalse(codexguard.handoff_allowed('apply_patch', {'command':patch_text}, str(self.f.wt), LANE))

    def test_watchdog_persists_after_commit_and_duplicates_do_not_count(self):
        self.assertEqual(self.call(identity='a')[0], 'allow')
        self.assertEqual(self.call(identity='a')[0], 'allow')
        self.assertEqual(self.call(identity='b')[0], 'allow')
        self.assertEqual(self.call(identity='c')[0], 'deny')
        self.f.make_commit()
        self.assertEqual(self.call(identity='d')[0], 'deny')

    def test_claude_calls_carry_across_to_codex(self):
        self.f.agent('claude1')
        self.assertEqual(self.f.call('claude1')[0], 'allow')
        self.assertEqual(self.f.call('claude1')[0], 'allow')
        self.assertEqual(self.call()[0], 'deny')

    def test_budget_requires_continue_and_records_shared_round(self):
        self.set_context(150001)
        self.assertEqual(self.call()[0], 'deny')
        result = dict(lane=LANE, state='ready', tip='abcd123', summary='done')
        codexguard.record_result(self.m, result)
        codexguard.record_result(self.m, result)
        self.assertEqual(result['state'], 'continue')
        rows = laneguard.read_rounds(str(self.f.main), LANE)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['provider'], 'codex')

    def test_parking_combines_provider_results(self):
        laneguard.append_line(laneguard.rounds_path(str(self.f.main), LANE), json.dumps(
            dict(agent_id='claude1', state='blocked', questions='TR33', label='lane:test#1')))
        codexguard.record_result(self.m, dict(state='blocked', questions='TR33', summary='gap'))
        self.assertIn('TR33', laneguard.park_reason(str(self.f.main), LANE))
        self.assertEqual(self.call()[0], 'deny')

    def test_unregistered_session_and_nested_agent_refused(self):
        with self.assertRaisesRegex(ValueError, 'second Codex'):
            codexguard.handle(self.data('SessionStart') | {'session_id':'other'}, self.m)
        self.assertEqual(self.call('spawn_agent')[0], 'deny')


class Launcher(unittest.TestCase):
    def setUp(self):
        self.state = dict(lanes={'sample':dict(rounds=2, role='core', touches=['alpha'])},
                          rounds=225, per_lane_limit=3, concurrency=7, glue_limit=3)
        self.spec = dict(args=dict(name='sample', kind='engine'), touches=['alpha'], provider='codex')

    def test_provider_switch_retains_round_cap(self):
        entry, _ = lane_launch.validate_capacity(self.state, self.spec, [], .75)
        self.assertEqual(entry['rounds'], 2)
        entry['rounds'] = 3
        with self.assertRaisesRegex(ValueError, 'round cap'):
            lane_launch.validate_capacity(self.state, self.spec, [], .75)

    def test_parked_waiting_disk_duplicate_and_glue_caps(self):
        for changes, live, fraction, message in [
            ({'parked':True}, [], .75, 'parked'),
            ({'waiting_for':'AD12'}, [], .75, 'blocker'),
            ({}, [], .86, 'exceeds85'),
            ({}, ['sample'], .75, 'already running')]:
            with self.subTest(message=message):
                state = json.loads(json.dumps(self.state))
                state['lanes']['sample'].update(changes)
                with self.assertRaisesRegex(ValueError, message):
                    lane_launch.validate_capacity(state, self.spec, live, fraction)
        self.state['lanes']['other'] = dict(rounds=1, touches=['alpha'], role='re')
        with self.assertRaisesRegex(ValueError, 'reservation'):
            lane_launch.validate_capacity(self.state, self.spec, ['other'], .75)
        self.spec['lane_role'] = 'glue'
        self.state['lanes'].update({f'g{i}':dict(role='glue', touches=[f'g{i}']) for i in range(3)})
        with self.assertRaisesRegex(ValueError, 'Glue lane cap'):
            lane_launch.validate_capacity(self.state, self.spec, ['g0','g1','g2'], .75)

    def test_legacy_pause_is_claude_and_no_automatic_fallback(self):
        f = Fixture()
        try:
            (f.main/'build-runs/run4-refill-paused.json').write_text('{"reason":"weekly limit"}')
            self.assertIsNotNone(lane_launch.provider_pause(f.main, 'claude'))
            self.assertIsNone(lane_launch.provider_pause(f.main, 'codex'))
        finally:
            f.close()


class PromptAndTransport(unittest.TestCase):
    def test_prompt_reuses_rules_and_stable_prefix(self):
        args = dict(kind='engine', name='test-a', task='only task', maxRounds=1)
        a = lane_backend.render(args, 'codex', 'C:/manifest.json')
        b = lane_backend.render(args | {'name':'test-b'}, 'codex', 'C:/manifest2.json')
        self.assertEqual(a['prompt'].split('## Your lane')[0], b['prompt'].split('## Your lane')[0])
        self.assertIn('Never open, list, grep or read: decomp/', a['prompt'])
        self.assertIn('exec_command uses PowerShell', a['prompt'])
        self.assertNotIn('Co-Authored-By: Claude', a['prompt'])
        self.assertEqual(set(a['schema']['required']), set(a['schema']['properties']))
        c = lane_backend.render(args, 'claude', 'unused')
        self.assertIn('Bash tool is Git Bash', c['prompt'])
        self.assertIn('Co-Authored-By: Claude', c['prompt'])

    def test_subscription_auth_and_hooks_are_required(self):
        m = dict(main='C:/main', worktree='C:/main/.claude/worktrees/test', manifest='C:/manifest.json', kind='tool')
        with patch('lane_backend.shutil.which', return_value='codex.exe'), patch('lane_backend.codex_defaults', return_value={}):
            with patch('lane_backend.subprocess.run', return_value=subprocess.CompletedProcess([],0,'Logged in using ChatGPT','')):
                cmd = lane_backend.codex_command(m, 'schema', 'final', {})
                self.assertIn('forced_login_method="chatgpt"', cmd)
                self.assertIn('--ignore-user-config', cmd)
                self.assertIn('--no-daemon', cmd)
                self.assertTrue(any(c.startswith('hooks.PreToolUse=') for c in cmd))
            with patch('lane_backend.subprocess.run', return_value=subprocess.CompletedProcess([],0,'Logged in using an API key','')):
                with self.assertRaisesRegex(RuntimeError, 'subscription login'):
                    lane_backend.codex_command(m, 'schema', 'final', {})

    def test_environment_excludes_api_keys(self):
        with patch.dict('os.environ', {'OPENAI_API_KEY':'secret','CODEX_API_KEY':'secret','CODEX_HOME':'C:/auth'}):
            env = lane_backend.codex_environment()
            self.assertNotIn('OPENAI_API_KEY', env)
            self.assertNotIn('CODEX_API_KEY', env)
            self.assertEqual(env['CODEX_HOME'], 'C:/auth')

    def test_claude_still_invokes_saved_workflow_once(self):
        f = Fixture()
        try:
            m = dict(kind='tool', main=str(f.main), manifest=str(f.main/'build-runs/manifest.json'),
                     claude_cli='claude.exe', args=dict(name=LANE,kind='tool',maxRounds=1,task='test'))
            def fake_run(cmd, **kw):
                self.assertEqual(cmd[0], 'claude.exe')
                self.assertIn('--allowedTools', cmd)
                text = kw['input'].decode()
                self.assertIn('saved guarded Workflow fc4-lane exactly ONCE', text)
                self.assertIn('Never direct Agent', text)
                kw['stdout'].write(b'{"is_error":false,"result":"ready","modelUsage":{"test-model":{}}}')
                return subprocess.CompletedProcess(cmd, 0)
            with patch('lane_backend.subprocess.run', side_effect=fake_run):
                result = lane_backend.run_claude(m)
            self.assertFalse(result['is_error'])
            self.assertEqual(result['provider'], 'claude')
        finally:
            f.close()

    def test_refiller_recognizes_both_provider_limits_and_errors(self):
        self.assertTrue(lane_refill.provider_error(dict(is_error=True,result='network failed')))
        self.assertTrue(lane_refill.provider_error(dict(result="You've hit your weekly limit")))
        self.assertTrue(lane_refill.provider_error(dict(result='usage_limit reached')))
        self.assertFalse(lane_refill.provider_error(dict(is_error=False,result='ready: tests passed')))


if __name__ == '__main__':
    unittest.main()

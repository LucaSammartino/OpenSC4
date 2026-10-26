import json
from pathlib import Path
import sys
from unittest.mock import patch

import pytest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import suite_health


def recorded_batch(root):
    directory = root / 'build-runs/suite/recorded'
    directory.mkdir(parents=True)
    names = ('complete', 'refused', 'no-reach', 'absent')
    requests = [dict(scenario=dict(name=n, region='Test', city=n, script='-', notes='', end='2000-04-01')) for n in names]
    (directory / 'manifest.json').write_text(json.dumps(dict(requests=requests, profile='shadows',
                                                            dll_sha256='original', shadow_keys=[])))
    for name in names[:-1]:
        (directory / (name + '.verdict')).write_text('verdict: clean\n')
        (directory / (name + '.log')).write_text(
            'script: fixture: 1 action(s)\nscript: line 1 2000-01-01 zone -> ok=' +
            ('0' if name == 'refused' else '1') + '\n'
            'foo shadow 2000-04-01: mismatches=0\nreach summary reached=1 saved=1\n'
            'AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=2.0)\n')
        (directory / (name + '.json')).write_text(json.dumps(dict(
            reach_lines=[dict(line=1, text='reach summary reached=1')], shadow_lines=1,
            shadow_exact_lines=1, actions_ok=1, actions_failed=int(name == 'refused'), actions_pending=0)))
        if name != 'no-reach':
            (directory / (name + '.reach.tsv')).write_text(
                'address\tentries\tfirst_window\tskipped\n00401000\t1\t1\t\n')
    return directory


def test_clean_share_preserves_missing_and_refused_evidence(tmp_path):
    directory = recorded_batch(tmp_path)
    health, addresses = suite_health.audit(tmp_path, 'recorded')
    assert health['requested'] == 4
    assert health['returned'] == 3
    assert health['clean'] == 3
    assert health['complete'] == 2
    assert health['strict'] == 1
    assert health['strict_share'] == .25
    assert not health['census_passed']
    assert addresses == {0x401000}
    gaps = (directory / 'evidence_gaps.tsv').read_text()
    assert 'no-reach\treach' in gaps
    assert 'refused\t-\t1' in gaps
    assert 'absent\tclean-end,reach,shadows,pending-actions' in gaps


def test_known_original_quit_fault_is_complete_and_not_a_new_finding(tmp_path):
    directory = recorded_batch(tmp_path)
    fault = (Path(__file__).parent / 'fixtures/suite-exit-crash.log').read_text()
    log = directory / 'complete.log'
    log.write_text(log.read_text() + fault.replace('2028-01-28', '2000-04-01'))
    (directory / 'complete.verdict').write_text('verdict: corrupt\n')
    health, _ = suite_health.audit(tmp_path, 'recorded')
    assert health['clean'] == 3
    assert health['complete'] == 2
    assert health['known_quit_faults'] == 1
    assert health['causes']['known-original-quit-fault'] == 1
    assert 'known-original-quit-fault' in (directory / 'causes.tsv').read_text()
    assert 'known-original-quit-fault' not in (directory / 'finding_candidates.tsv').read_text()


def test_stale_parsed_json_cannot_supply_missing_raw_evidence(tmp_path):
    directory = recorded_batch(tmp_path)
    (directory / 'complete.log').write_text('AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=2.0)\n')
    health, _ = suite_health.audit(tmp_path, 'recorded')
    assert health['complete'] == 1
    assert health['strict'] == 0


@pytest.mark.parametrize('verdict', ['not-submitted', 'queue-timeout'])
def test_synthetic_wait_results_are_not_returned_games(tmp_path, verdict):
    directory = recorded_batch(tmp_path)
    (directory / 'absent.verdict').write_text('verdict: ' + verdict + '\n')
    health, _ = suite_health.audit(tmp_path, 'recorded')
    assert health['returned'] == 3
    assert health['requested'] == 4
    assert health['complete'] == 2


def correction_batch(root, baseline, name='repair'):
    directory = root / 'build-runs/suite' / name
    directory.mkdir()
    manifest = json.loads((baseline / 'manifest.json').read_text())
    manifest['requests'] = manifest['requests'][:1]
    manifest['dll_sha256'] = 'corrected'
    (directory / 'manifest.json').write_text(json.dumps(manifest))
    return directory


def test_reconcile_keeps_latest_failure_and_original_reach(tmp_path):
    baseline = recorded_batch(tmp_path)
    repair = correction_batch(tmp_path, baseline)
    (repair / 'complete.log').write_text('AUTOTEST VERDICT: corrupt (date=2000-04-01)\n')
    (repair / 'complete.verdict').write_text('verdict: corrupt\n')
    output = suite_health.reconcile(tmp_path, 'current', ['recorded', 'repair'])
    health, addresses = suite_health.audit(tmp_path, 'current')
    assert health['requested'] == 4
    assert health['complete'] == 1
    assert addresses == {0x401000}  # Includes the failed row's earlier trace.
    assert (output / 'recorded/complete.reach.tsv').is_file()
    assert 'complete\tshadows\tcorrupt' in (output / 'summary.tsv').read_text()
    assert '\tcorrected\t' in (output / 'summary.tsv').read_text()
    assert json.loads((output / 'manifest.json').read_text())['timing'] == {}
    assert len((output / 'attempts.tsv').read_text().splitlines()) == 6


def test_absent_correction_cannot_silently_reuse_earlier_success(tmp_path):
    baseline = recorded_batch(tmp_path)
    correction_batch(tmp_path, baseline)
    suite_health.reconcile(tmp_path, 'current', ['recorded', 'repair'])
    health, _ = suite_health.audit(tmp_path, 'current')
    assert health['returned'] == 2
    assert health['complete'] == 1


def test_reconcile_rejects_active_source_and_unknown_correction(tmp_path):
    baseline = recorded_batch(tmp_path)
    repair = correction_batch(tmp_path, baseline)
    manifest = json.loads((repair / 'manifest.json').read_text())
    manifest['requests'][0]['label'] = 'still-running'
    (repair / 'manifest.json').write_text(json.dumps(manifest))
    queue = tmp_path / 'build-runs/game-queue'
    queue.mkdir()
    (queue / 'still-running.req').write_text('request')
    with pytest.raises(ValueError, match='still queued/running'):
        suite_health.reconcile(tmp_path, 'current', ['recorded', 'repair'])
    assert not (tmp_path / 'build-runs/suite/current').exists()
    manifest['requests'][0]['scenario']['name'] = 'unknown'
    (repair / 'manifest.json').write_text(json.dumps(manifest))
    with pytest.raises(ValueError, match='absent from baseline'):
        suite_health.reconcile(tmp_path, 'current', ['recorded', 'repair'])


def test_estimated_or_wrong_slot_wall_never_passes_tier(tmp_path):
    reference = dict(census_passed=True)
    data = dict(game=[(0x401000, 1000, 0, '')], rows=[dict(implemented=True)])
    with patch.object(suite_health, 'audit', return_value=(reference, {0x401000})), \
            patch.object(suite_health.suite_coverage, 'sibling') as sibling:
        sibling.return_value.load_report_data.return_value = data
        for timing in ({'wall_seconds': 600}, {'wall_seconds': 600, 'slots': 8, 'complete': True},
                       {'wall_seconds': 901, 'slots': 6, 'complete': True}):
            health = dict(timing=timing, strict=6, complete=6, requested=6)
            suite_health.compare_tier(tmp_path, health, {0x401000}, 'census')
            assert not health['tier1']['passed']
        health = dict(timing=dict(wall_seconds=900, slots=6, complete=True), strict=6, complete=6, requested=6)
        suite_health.compare_tier(tmp_path, health, {0x401000}, 'census')
        assert health['tier1']['passed']

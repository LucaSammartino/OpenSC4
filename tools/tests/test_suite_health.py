import json
from pathlib import Path
import sys
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import suite_health


def recorded_batch(root):
    directory = root / 'build-runs/suite/recorded'
    directory.mkdir(parents=True)
    names = ('complete', 'refused', 'no-reach', 'absent')
    requests = [dict(scenario=dict(name=n, script='-', notes='', end='2000-04-01')) for n in names]
    (directory / 'manifest.json').write_text(json.dumps(dict(requests=requests)))
    for name in names[:-1]:
        (directory / (name + '.verdict')).write_text('verdict: clean\n')
        (directory / (name + '.log')).write_text('AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=2.0)\n')
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


def test_estimated_or_wrong_slot_wall_never_passes_tier(tmp_path):
    reference = dict(census_passed=True)
    data = dict(game=[(0x401000, 1000, 0, '')], rows=[dict(implemented=True)])
    with patch.object(suite_health, 'audit', return_value=(reference, {0x401000})), \
            patch.object(suite_health.suite_coverage, 'sibling') as sibling:
        sibling.return_value.load_report_data.return_value = data
        for timing in ({'wall_seconds': 600}, {'wall_seconds': 600, 'slots': 8, 'complete': True},
                       {'wall_seconds': 901, 'slots': 6, 'complete': True}):
            health = dict(timing=timing, strict=6, requested=6)
            suite_health.compare_tier(tmp_path, health, {0x401000}, 'census')
            assert not health['tier1']['passed']
        health = dict(timing=dict(wall_seconds=900, slots=6, complete=True), strict=6, requested=6)
        suite_health.compare_tier(tmp_path, health, {0x401000}, 'census')
        assert health['tier1']['passed']

"""Exercise the real catalog plan, map bounds and disjoint lot footprints."""
import csv
from pathlib import Path
import sys

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import lot_sweep


def test_catalog_sweep_covers_every_categorized_lot_without_overlap():
    with (ROOT / 'engine/tests/scenarios/lotconfigs.tsv').open() as stream:
        rows = list(csv.DictReader(stream, delimiter='\t'))
    lots = lot_sweep.plan(rows)
    assert len(lots) == 175
    assert {r['iid'] for r in lots} == {r['iid'] for r in rows if r['category'] != 'other'}
    cells = set()
    for lot in lots:
        footprint = {(x, z) for x in range(lot['x'], lot['x'] + int(lot['width']))
                     for z in range(lot['z'], lot['z'] + int(lot['depth']))}
        assert not footprint & cells
        assert all(0 <= x < 256 and 0 <= z < 256 for x, z in footprint)
        assert not any(220 <= x <= 231 and 220 <= z <= 231 for x, z in footprint)
        cells |= footprint
    generated = (ROOT / 'engine/tests/scenarios/lua/lot-sweep-catalog.lua').read_text()
    assert generated == lot_sweep.render(rows)
    assert generated == lot_sweep.render(reversed(rows))
    with pytest.raises(ValueError, match='fit'):
        lot_sweep.plan(rows, 32)

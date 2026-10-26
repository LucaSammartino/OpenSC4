"""Exercise the real catalog plan, map bounds and disjoint lot footprints."""
import csv
from pathlib import Path
import sys
import re

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import lot_sweep


@pytest.mark.parametrize('size', [128, 256])
def test_catalog_sweep_covers_every_categorized_lot_without_overlap(size):
    with (ROOT / 'engine/tests/scenarios/lotconfigs.tsv').open() as stream:
        rows = list(csv.DictReader(stream, delimiter='\t'))
    lots = lot_sweep.plan(rows, size)
    assert len(lots) == 175
    assert {r['iid'] for r in lots} == {r['iid'] for r in rows if r['category'] != 'other'}
    cells = set()
    for lot in lots:
        footprint = {(x, z) for x in range(lot['x'], lot['x'] + int(lot['width']))
                     for z in range(lot['z'], lot['z'] + int(lot['depth']))}
        assert not footprint & cells
        assert all(0 <= x < size and 0 <= z < size for x, z in footprint)
        assert not any(size - 6 <= x <= size - 2 and size - 6 <= z <= size - 2 for x, z in footprint)
        cells |= footprint
    suffix = '-128' if size == 128 else ''
    generated = (ROOT / f'engine/tests/scenarios/lua/lot-sweep-catalog{suffix}.lua').read_text()
    assert generated == lot_sweep.render(rows, size)
    assert generated == lot_sweep.render(reversed(rows), size)
    with pytest.raises(ValueError, match='fit'):
        lot_sweep.plan(rows, 32)


def test_qualified_civic_pads_fit_the_measured_medium_tile():
    with (ROOT / 'engine/tests/scenarios/lotconfigs.tsv').open() as stream:
        catalog = {r['iid'][2:]: r for r in csv.DictReader(stream, delimiter='\t')}
    text = (ROOT / 'engine/tests/scenarios/lua/lot-sweep-grown-sites.lua').read_text()
    sites = re.findall(r"\{'([0-9A-F]{8})', (\d+), (\d+)\}", text)
    expected = {iid for iid, r in catalog.items() if set(r['category'].split(',')) &
                {'power', 'fire', 'health', 'police', 'education', 'water'}}
    assert len(sites) == len(expected) == 37
    assert {iid for iid, _, _ in sites} == expected
    occupied = set()
    for iid, x, z in sites:
        x, z = int(x), int(z)
        row = catalog[iid]
        pad = {(xx, zz) for xx in range(x-2, x+int(row['width'])+3)
               for zz in range(z-2, z+int(row['depth'])+3)}
        assert all(0 <= xx < 128 and 0 <= zz < 128 for xx, zz in pad)
        assert not pad & occupied
        occupied |= pad

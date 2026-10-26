"""Catalog inherited civic categories, plugin overrides and both exemplar encodings."""
from pathlib import Path
import struct
import sys

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import lotconfigs as lc


def prop(value):
    return ('u32', value)


def records():
    cohort = (lc.TYPE_COHORT, 1, 2)
    building = (lc.TYPE_EXEMPLAR, 1, 3)
    lot = (lc.TYPE_EXEMPLAR, 4, 5)
    return {
        cohort: lc.Record((0, 0, 0), {0x10: prop([2]), lc.OCCUPANT_GROUPS: prop([0x1004, 0x1400])}, 'base'),
        building: lc.Record(cohort, {0x20: ('string', 'Plant')}, 'base'),
        lot: lc.Record((0, 0, 0), {0x10: prop([16]), 0x20: ('string', 'Plant lot'),
                     lc.LOT_SIZE: prop([4, 5]), lc.OBJECT_FIRST: prop([0] * 12 + [3])}, 'base'),
    }


def test_inherits_building_category_and_uses_building_iid_not_object_id():
    row, = lc.catalog(records())
    assert row['iid'] == '0x00000005'
    assert row['building_iids'] == '0x00000003'
    assert row['category'] == 'power'
    assert (row['width'], row['depth']) == (4, 5)


def test_resolves_family_and_multiple_categories():
    data = records()
    data[(lc.TYPE_EXEMPLAR, 1, 3)].props[0x27812870] = prop([99])
    data[(lc.TYPE_COHORT, 1, 2)].props[lc.OCCUPANT_GROUPS] = prop([0x1400, 0x1402])
    data[(lc.TYPE_EXEMPLAR, 4, 5)].props[lc.OBJECT_FIRST] = prop([0] * 12 + [99])
    assert lc.catalog(data)[0]['category'] == 'power,garbage'


def test_reward_park_not_mislabeled_and_unknown_retained():
    assert lc.category({0x1006, 0x150B}) == 'rewards'
    assert lc.category({0x1005, 0x1006}) == 'parks'
    assert lc.category({0x1004}) == 'utilities'
    assert lc.category(set()) == 'other'


def test_cycle_detected():
    data = records()
    data[(lc.TYPE_COHORT, 1, 2)].parent = (lc.TYPE_COHORT, 1, 2)
    with pytest.raises(ValueError, match='cohort cycle'):
        lc.catalog(data)


def text_exemplar(name):
    return ('EQZT1###\nParentCohort=Key:{0,0,0}\nPropCount=0x2\n'
            '0x00000010:{"Exemplar Type"}=Uint32:0:{0x10}\n'
            '0x00000020:{"Exemplar Name"}=String:0:{"' + name + '"}\n').encode('ascii')


def package(path, name):
    raw = text_exemplar(name)
    header = bytearray(96)
    header[:4] = b'DBPF'
    struct.pack_into('<II', header, 4, 1, 0)
    struct.pack_into('<IIII', header, 0x20, 7, 1, 96, 20)
    index = struct.pack('<IIIII', lc.TYPE_EXEMPLAR, 4, 5, 116, len(raw))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(header + index + raw)


def test_read_only_package_plugin_override_and_deterministic_output(tmp_path):
    base = tmp_path / 'install/SimCity_1.dat'
    plugin = tmp_path / 'Plugins/a/override.SC4Lot'
    package(base, 'Original')
    package(plugin, 'Override')
    before = base.read_bytes(), plugin.read_bytes()
    paths = lc.package_paths(base.parent, tmp_path / 'Plugins')
    data, warnings = lc.read_records(paths)
    assert not warnings
    row, = lc.catalog(data)
    assert row['name'] == 'Override'
    assert row['source'] == 'Plugins/a/override.SC4Lot'
    assert before == (base.read_bytes(), plugin.read_bytes())
    assert lc.render([row]) == lc.render(lc.catalog(data))
    assert lc.render([row]).startswith('iid\tname\tcategory\t')


def test_malformed_package_not_silent(tmp_path):
    bad = tmp_path / 'bad.dat'
    bad.write_bytes(b'bad')
    data, warnings = lc.read_records([(bad, 'bad.dat')])
    assert not data
    assert len(warnings) == 1

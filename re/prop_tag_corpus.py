#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""PV2 corpus tool -- READ-ONLY.

Question (PV2): how many saved props carry a lot tag that matches no object of
their lot's configuration?

The rule (prop-developer.md section 4.3 step 2.3.2, proven from the game binary):

    For each lot, for each prop selected by GetLotProps(lot) that HAS a lot tag,
    find the first configuration object whose object ID (the 0x34-byte record's
    +0x24) equals the prop's tag.  If there is NO such object, the prop is
    UNMATCHED (and the game does "k := k - 1", lowering the block's slot count).

This script only reads.  It never writes to the game install, the Documents tree
or any package.

    python -I re/prop_tag_corpus.py
    python -I re/prop_tag_corpus.py --json re/pv2.json
    python -I re/prop_tag_corpus.py --quick          # skip CRC checks

Paths are overridable:

    --dats DIR        the six DBPF .dat files (default the GOG install)
    --regions DIR     Documents\\SimCity 4\\Regions
    --documents DIR   Documents\\SimCity 4
    --dbpf PATH       the tools/dbpf.py to import

Data sources, all read-only:
  * the game catalog: SimCity_1..5.dat, EP1.dat (DBPF packages of exemplars)
  * city saves: Documents\\SimCity 4\\**\\*.sc4 (DBPF packages with a class store)

Layouts come from docs/specs/ (foundations.md 2.3, 3.3-3.4, 9.2, 10.2-10.3;
lot-records.md 2.3, 3.2, 10.1, 10.2; lot-configuration.md 3.5; prop-lotconfig.md
A7.2, B3.3; occupant-manager.md 8.2, 8.4; lot-developer.md 17.1, 17.6).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import struct
import sys
from collections import Counter, defaultdict

# --------------------------------------------------------------------------
# defaults
# --------------------------------------------------------------------------

DEFAULT_DBPF = r"C:\Users\tlori\FreeCity4\tools\dbpf.py"
DEFAULT_DATS = r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition"
DAT_NAMES = ["SimCity_1.dat", "SimCity_2.dat", "SimCity_3.dat",
             "SimCity_4.dat", "SimCity_5.dat", "EP1.dat"]
DEFAULT_REGIONS = r"C:\Users\tlori\Documents\SimCity 4\Regions"
DEFAULT_DOCS = r"C:\Users\tlori\Documents\SimCity 4"

# catalog
TYPE_EXEMPLAR = 0x6534284A
GROUP_LOT_CONFIG = 0xA8FBD372          # lot-configuration.md section 0
PROP_EXEMPLAR_TYPE = 0x10              # exemplar "type" property
EXEMPLAR_TYPE_NETWORK_LOT_CONFIG = 0x21  # network-manager.md 2.4
PROP_LOT_CONFIG_VERSION = 0x88EDC789   # u8; every shipped config is 2
PROP_LOT_OBJECT_BASE = 0x88EDC900      # + k
PROP_LOT_OBJECT_END = 0x88EDCE00       # scan stops here
PROP_NETWORK_CONFIG_ID = 0x21          # network lot configuration ID property

# class-object store (foundations.md 3.3)
TYPE_CLASS_INDEX = 0x499B23FE
GROUP_CLASS_STORE = 0x299B2D1B
CLASS_LOT = 0xC9BD5D4A
CLASS_PROP = 0x2977AA47
CLASS_FX_PROP = 0x2977AA49

# variant disk tags (foundations.md 9.2)
DISK_EMPTY = 0x00
DISK_STRING = 0x800C                  # RZChar array = gzstr
DISK_VOID_ARRAY = 0x8000
DISK_WRITE_FAIL = (0x12, 0x8012, 0x800D)
ELEMENT_SIZE = {
    0x00: 0, 0x01: 1, 0x02: 2, 0x03: 4, 0x04: 8, 0x05: 1, 0x06: 2,
    0x07: 4, 0x08: 8, 0x09: 4, 0x0A: 8, 0x0B: 1, 0x0C: 1, 0x10: 2, 0x11: 1,
}
MARKER_REF = 0xA9BEE9AC

NO_TAG = 0xFFFFFFFF
CELL = 16.0          # metres per standard city cell (city slot 96); 96x96 city
QUERY_EPS = 0.1      # occupant-manager.md 8.4: the box is shrunk by 0.1 m


# --------------------------------------------------------------------------
# CRC-32/MPEG-2 (foundations.md 3.4)
# --------------------------------------------------------------------------

def _crc_table():
    tab = []
    for n in range(256):
        c = n << 24
        for _ in range(8):
            c = ((c << 1) ^ 0x04C11DB7) & 0xFFFFFFFF if c & 0x80000000 else (c << 1) & 0xFFFFFFFF
        tab.append(c)
    return tab


_CRC_TAB = _crc_table()


def crc_mpeg2(data):
    crc = 0xFFFFFFFF
    for b in data:
        crc = ((crc << 8) & 0xFFFFFFFF) ^ _CRC_TAB[((crc >> 24) ^ b) & 0xFF]
    return crc


# --------------------------------------------------------------------------
# stream readers (little endian, no padding)
# --------------------------------------------------------------------------

class ParseError(Exception):
    """A record could not be parsed under the documented layout."""


class Reader:
    __slots__ = ("b", "p", "end")

    def __init__(self, b, start=0, end=None):
        self.b = b
        self.p = start
        self.end = len(b) if end is None else end

    def left(self):
        return self.end - self.p

    def take(self, n):
        if n < 0 or self.p + n > self.end:
            raise ParseError("read past end (want %d, have %d)" % (n, self.left()))
        s = self.b[self.p:self.p + n]
        self.p += n
        return s

    def u8(self):
        return self.take(1)[0]

    def s8(self):
        return struct.unpack("<b", self.take(1))[0]

    def u16(self):
        return struct.unpack("<H", self.take(2))[0]

    def u32(self):
        return struct.unpack("<I", self.take(4))[0]

    def s32(self):
        return struct.unpack("<i", self.take(4))[0]

    def f32(self):
        return struct.unpack("<f", self.take(4))[0]

    def f32s(self, n):
        return struct.unpack("<%df" % n, self.take(4 * n))


def read_gzstr(r):
    """foundations.md 2.3: u32 header (top byte encoding, low 24 count) + bytes."""
    header = r.u32()
    enc = header >> 24
    count = header & 0xFFFFFF
    if count > 200000:
        raise ParseError("gzstr count %d > 200000" % count)
    nbytes = 2 * count if enc == 0x10 else count
    return r.take(nbytes)


def read_variant(r):
    """foundations.md 9.2.  Returns (kind, detail)."""
    marker = r.u32()
    if marker == MARKER_REF:
        oid = r.u32()
        if oid != 0:
            r.u32()
        return ("ref", oid)
    if marker != 0:
        # quirk: any non-0xA9BEE9AC marker is treated as 0
        pass
    tag = r.u32()
    if tag in DISK_WRITE_FAIL:
        raise ParseError("write-failure diskTag 0x%08X" % tag)
    if tag == DISK_EMPTY:
        return ("empty", None)
    if tag == DISK_STRING:
        return ("string", read_gzstr(r))
    if tag == DISK_VOID_ARRAY:
        n = r.u32()
        r.take(n)
        return ("void", n)
    if tag & 0x8000:
        n = r.u32()
        esz = ELEMENT_SIZE.get(tag & 0x7FFF)
        if esz is None:
            raise ParseError("unknown array diskTag 0x%08X" % tag)
        r.take(n * esz)
        return ("array", n)
    esz = ELEMENT_SIZE.get(tag)
    if esz is None:
        raise ParseError("unknown scalar diskTag 0x%08X" % tag)
    r.take(esz)
    return ("scalar", tag)


def read_plain_holder(r):
    """foundations.md 10.2."""
    ver = r.u16()
    if ver != 2:
        raise ParseError("plain holder version %d" % ver)
    count = r.u32()
    for _ in range(count):
        r.u32()          # key (ignored on read)
        r.u32()          # propertyID
        read_variant(r)


def read_exemplar_holder(r):
    """foundations.md 10.3: u16 2, plain holder, then parent key (GROUP, TYPE, INSTANCE)."""
    ver = r.u16()
    if ver != 2:
        raise ParseError("exemplar holder version %d" % ver)
    read_plain_holder(r)
    group = r.u32()
    typ = r.u32()
    inst = r.u32()
    return (group, typ, inst)


def read_occupant_base(r):
    """lot-records.md 2.3: 14 bytes + the exemplar holder."""
    ver = r.u16()
    if ver != 4:
        raise ParseError("occupant base version %d" % ver)
    r.u16()                       # removal counters
    r.u8()                        # highlight
    r.u8()                        # occupant flags
    otype = r.u32()
    mrect = r.take(4)             # manager cell rect x1,z1,x2,z2
    parent = read_exemplar_holder(r)
    return otype, mrect, parent


def parse_prop_payload(buf, start=0, end=None):
    """lot-records.md 10.1.  Returns (fields, consumed)."""
    r = Reader(buf, start, end)
    ver = r.u16()
    otype, mrect, parent = read_occupant_base(r)
    prop_type = r.u32()
    bounds = r.f32s(6)            # min x,y,z, max x,y,z
    orient = r.s8()
    state = r.u8()
    tod_start = r.u8()
    tod_end = r.u8()
    has_dr = r.u8()
    date_range = None
    if has_dr:
        date_range = struct.unpack("<4I", r.take(16))
    chance = r.u8()
    usage = r.u8()
    lot_tag = r.u32()
    if ver > 5:
        timer = r.u8()
    else:
        timer = 0x0F
    fields = {
        "version": ver, "occupant_type": otype, "manager_cell_rect": mrect,
        "parent": parent, "prop_type": prop_type, "bounds": bounds,
        "orientation": orient, "state": state, "tod": (tod_start, tod_end),
        "has_date_range": has_dr, "date_range": date_range, "chance": chance,
        "usage": usage, "lot_tag": lot_tag, "timer": timer,
    }
    return fields, r.p


def parse_fx_prop_payload(buf, start=0, end=None):
    """lot-records.md 10.2: u16 3, then a complete prop record, then 2 bytes, gzstr, u32."""
    r = Reader(buf, start, end)
    ver = r.u16()
    if ver != 3:
        raise ParseError("effect prop version %d" % ver)
    _inner, consumed = parse_prop_payload(buf, r.p, end)
    _inner["fx_version"] = ver          # the outer effect-prop record version
    r.p = consumed                  # absolute index returned by the inner parse
    r.u8()
    r.u8()
    read_gzstr(r)
    r.u32()
    return _inner, r.p


def parse_lot_header(buf, start=0, end=None):
    """lot-records.md 3.2 fields 1-10 (all PV2 needs, plus facing for the position check)."""
    r = Reader(buf, start, end)
    ver = r.u16()
    cfg = r.u32()
    flags = r.u8()
    rect = tuple(r.take(4))       # x1, z1, x2, z2 cells, inclusive
    location = tuple(r.take(2))   # x, z corner
    altitude = r.f32()
    slope_saved = r.f32()
    slope_now = r.f32()
    size = tuple(r.take(2))       # size x, size z in cells
    facing = r.u8()
    return {
        "version": ver, "config_id": cfg, "flags": flags, "rect": rect,
        "location": location, "altitude": altitude, "facing": facing,
        "size": size,
    }, r.p


# --------------------------------------------------------------------------
# Part 1: the catalog
# --------------------------------------------------------------------------

class Config:
    __slots__ = ("kind", "key", "config_id", "version", "objects",
                 "ids_all", "ids_prop", "n_by_type", "n_lod3", "short_props", "source")

    def __init__(self, kind, key, config_id, version, objects, short_props, source):
        self.kind = kind
        self.key = key
        self.config_id = config_id
        self.version = version
        self.objects = objects          # list of dicts, property order
        self.short_props = short_props  # properties with < 10 values (skipped)
        self.source = source
        self.ids_all = set()
        self.ids_prop = set()
        self.n_by_type = Counter()
        self.n_lod3 = 0
        for o in objects:
            self.ids_all.add(o["id"])
            self.n_by_type[o["type"]] += 1
            if o["type"] == 1:
                self.ids_prop.add(o["id"])
            if o["type"] != 0 and ((o["v1"] >> 4) & 3) == 3:
                self.n_lod3 += 1


def build_object_array(props):
    """lot-configuration.md 3.5 / prop-lotconfig.md B3.3.

    Scan 0x88EDC900 + k for k = 0, 1, ...  stopping at the first missing k or at
    0x88EDCE00 (so at most 1280 objects).  A property with fewer than 10 values is
    skipped (it does not stop the scan).  Object ID = v11 (version >= 2).
    """
    objects = []
    short_props = 0
    for k in range(1280):
        pid = PROP_LOT_OBJECT_BASE + k
        if pid >= PROP_LOT_OBJECT_END:
            break
        entry = props.get(pid)
        if entry is None:
            break
        _name, values = entry
        if len(values) < 10:
            short_props += 1
            continue
        v0 = values[0] & 0xFF
        v1 = values[1] & 0xFF
        v2 = values[2]
        # v3 / v5 are signed 16.16 fixed point: float(int32 * 2^-16) metres
        ox = _s32(values[3]) / 65536.0
        oz = _s32(values[5]) / 65536.0
        objects.append({
            "k": k, "type": v0, "v1": v1, "orient": v2 & 0xFF,
            "ox": ox, "oz": oz, "id": values[11] & 0xFFFFFFFF,
            "nvals": len(values),
        })
    return objects, short_props


def _s32(v):
    v &= 0xFFFFFFFF
    return v - 0x100000000 if v >= 0x80000000 else v


def load_catalog(dats_dir):
    dbpf = _import_dbpf()
    lot_cfgs = {}          # instance -> Config
    net_cfgs = {}          # property 0x21 id -> Config (last wins, as the game's map does per piece)
    net_cfgs_by_instance = {}
    report = {
        "dat_files": [], "exemplars_total": 0, "parse_failures": 0,
        "groups_with_object_props": Counter(), "groups_with_version": Counter(),
        "groups_with_type21": Counter(), "lot_config_versions": Counter(),
        "net_config_versions": Counter(), "lot_config_no_version": 0,
        "lot_config_version_zero": 0, "net_config_missing_id": 0,
        "lot_object_counts": [], "net_object_counts": [],
        "lot_short_props": 0, "net_short_props": 0,
        "lot_objects_by_type": Counter(), "net_objects_by_type": Counter(),
        "lot_lod3": 0, "net_lod3": 0, "max_objects": 0, "max_objects_instance": None,
        "net_cfgs_with_object_props": 0, "lot_cfgs_with_object_props": 0,
        "duplicate_lot_instance": 0,
    }
    for name in DAT_NAMES:
        path = os.path.join(dats_dir, name)
        if not os.path.exists(path):
            report["dat_files"].append({"name": name, "exists": False})
            continue
        pkg = dbpf.DBPF(path)
        n_ex = 0
        for e in pkg.find(TYPE_EXEMPLAR):
            n_ex += 1
            report["exemplars_total"] += 1
            try:
                buf = pkg.read(e)
                _parent, props = dbpf.parse_exemplar(buf)
            except Exception:
                report["parse_failures"] += 1
                continue
            t, g, inst = e[0], e[1], e[2]
            if PROP_LOT_OBJECT_BASE in props:
                report["groups_with_object_props"][g] += 1
            if PROP_LOT_CONFIG_VERSION in props:
                report["groups_with_version"][g] += 1
            ex_type = props.get(PROP_EXEMPLAR_TYPE)
            is_net = bool(ex_type and ex_type[1] and (ex_type[1][0] & 0xFFFFFFFF) == EXEMPLAR_TYPE_NETWORK_LOT_CONFIG)
            if is_net:
                report["groups_with_type21"][g] += 1

            # --- lot configuration: exemplar group 0xA8FBD372 (the type is not checked)
            if g == GROUP_LOT_CONFIG:
                ver_entry = props.get(PROP_LOT_CONFIG_VERSION)
                if ver_entry is None:
                    report["lot_config_no_version"] += 1
                    version = None
                else:
                    version = ver_entry[1][0] & 0xFF
                    if version == 0:
                        report["lot_config_version_zero"] += 1
                objects, short = build_object_array(props)
                report["lot_short_props"] += short
                if objects:
                    report["lot_cfgs_with_object_props"] += 1
                report["lot_object_counts"].append(len(objects))
                for o in objects:
                    report["lot_objects_by_type"][o["type"]] += 1
                    if o["type"] != 0 and ((o["v1"] >> 4) & 3) == 3:
                        report["lot_lod3"] += 1
                if len(objects) > report["max_objects"]:
                    report["max_objects"] = len(objects)
                    report["max_objects_instance"] = inst
                if version is not None:
                    report["lot_config_versions"][version] += 1
                if inst in lot_cfgs:
                    report["duplicate_lot_instance"] += 1
                # Load fails on absent/0 version, so only non-zero versions become
                # configurations (lot-configuration.md 3.1 step 5).
                if version:
                    lot_cfgs[inst] = Config("lot", (t, g, inst), inst, version,
                                            objects, short, name)
                continue

            # --- network lot configuration: any group, exemplar type property 0x10 == 0x21
            if is_net:
                ver_entry = props.get(PROP_LOT_CONFIG_VERSION)
                version = (ver_entry[1][0] & 0xFF) if ver_entry else 1
                report["net_config_versions"][version] += 1
                id_entry = props.get(PROP_NETWORK_CONFIG_ID)
                if id_entry is None or not id_entry[1]:
                    report["net_config_missing_id"] += 1
                    cfg_id = 0
                else:
                    cfg_id = id_entry[1][0] & 0xFFFFFFFF
                objects, short = build_object_array(props)
                report["net_short_props"] += short
                if objects:
                    report["net_cfgs_with_object_props"] += 1
                report["net_object_counts"].append(len(objects))
                for o in objects:
                    report["net_objects_by_type"][o["type"]] += 1
                    if o["type"] != 0 and ((o["v1"] >> 4) & 3) == 3:
                        report["net_lod3"] += 1
                cfg = Config("network", (t, g, inst), cfg_id, version,
                             objects, short, name)
                net_cfgs_by_instance[(t, g, inst)] = cfg
                net_cfgs[cfg_id] = cfg
        report["dat_files"].append({"name": name, "exists": True,
                                    "entries": len(pkg.entries), "exemplars": n_ex})
    return lot_cfgs, net_cfgs, net_cfgs_by_instance, report


# --------------------------------------------------------------------------
# Part 2: the corpus
# --------------------------------------------------------------------------

def scan_saves(root):
    """Return [(sha256, path)] for every .sc4 under root, in path order."""
    out = []
    for dirpath, _dirs, files in os.walk(root):
        for fn in sorted(files):
            if fn.lower().endswith(".sc4"):
                p = os.path.join(dirpath, fn)
                with open(p, "rb") as f:
                    data = f.read()
                out.append((hashlib.sha256(data).hexdigest(), p))
    return out


# --------------------------------------------------------------------------
# Part 3: one save
# --------------------------------------------------------------------------

class Prop:
    __slots__ = ("oid", "usage", "tag", "bounds", "cx", "cz", "fx", "version",
                 "occupant_type", "prop_type")

    def __init__(self, oid, fields, fx):
        self.oid = oid
        self.usage = fields["usage"]
        self.tag = fields["lot_tag"]
        self.prop_type = fields["prop_type"]
        b = fields["bounds"]
        self.bounds = b
        self.cx = (b[0] + b[3]) * 0.5     # GetPosition = centre of the bounds box
        self.cz = (b[2] + b[5]) * 0.5
        self.fx = fx
        self.version = fields["version"]
        self.occupant_type = fields["occupant_type"]


class SaveResult:
    def __init__(self, path, sha):
        self.path = path
        self.sha = sha
        self.region = os.path.basename(os.path.dirname(path))
        self.name = os.path.basename(path)
        self.index = {}
        self.entries = {"lot": 0, "prop": 0, "fx": 0}
        self.parse_ok = {"lot": 0, "prop": 0, "fx": 0}
        self.overrun = {"lot": 0, "prop": 0, "fx": 0}
        self.underrun = {"lot": 0, "prop": 0, "fx": 0}
        self.parse_fail = {"lot": 0, "prop": 0, "fx": 0}
        self.trailing_bytes = {"lot": 0, "prop": 0, "fx": 0}
        self.count_mismatch = {}
        self.crc_bad = 0
        self.crc_checked = 0
        self.lot_versions = Counter()
        self.prop_versions = Counter()
        self.fx_versions = Counter()
        self.prop_usage = Counter()
        self.fx_usage = Counter()
        self.no_tag = {"prop": 0, "fx": 0}
        self.lots = []
        self.props = []
        self.fxprops = []
        self.sample_errors = []


def _walk_class_entries(buf):
    """foundations.md 3.3: a sequence of (entrySize, crc, objectID, payload)."""
    out = []
    p = 0
    n = len(buf)
    while p < n:
        if p + 12 > n:
            out.append(("trailing", n - p, None, None))
            break
        entry_size, crc, oid = struct.unpack_from("<III", buf, p)
        if entry_size < 12 or p + entry_size > n:
            out.append(("badsize", entry_size, n - p, None))
            break
        payload = buf[p + 12:p + entry_size]
        out.append(("ok", entry_size, crc, oid, payload, buf[p + 8:p + entry_size]))
        p += entry_size
    return out


def parse_save(path, sha, want_crc=True):
    dbpf = _import_dbpf()
    res = SaveResult(path, sha)
    pkg = dbpf.DBPF(path)
    idx_buf = pkg.get(TYPE_CLASS_INDEX, GROUP_CLASS_STORE, 0)
    if idx_buf is None:
        res.sample_errors.append("no class-store index record")
        return res
    version, class_count, entries_written = struct.unpack_from("<III", idx_buf, 0)
    counts = {}
    for i in range(class_count):
        cid, cnt = struct.unpack_from("<II", idx_buf, 12 + i * 8)
        counts[cid] = cnt
    res.index = {"version": version, "class_count": class_count,
                 "entries_written": entries_written, "sum": sum(counts.values()),
                 "counts": counts}

    for cid, kind, parser in ((CLASS_LOT, "lot", parse_lot_header),
                              (CLASS_PROP, "prop", parse_prop_payload),
                              (CLASS_FX_PROP, "fx", parse_fx_prop_payload)):
        rec = pkg.get(cid, GROUP_CLASS_STORE, 0)
        want = counts.get(cid, 0)
        if rec is None:
            if want:
                res.count_mismatch[kind] = ("missing-record", 0, want)
            continue
        ents = _walk_class_entries(rec)
        n_ok = 0
        for e in ents:
            if e[0] != "ok":
                res.trailing_bytes[kind] += 1
                if len(res.sample_errors) < 12:
                    res.sample_errors.append("%s: %s" % (kind, e[0]))
                continue
            _esize, crc, oid, payload, crc_span = e[1], e[2], e[3], e[4], e[5]
            n_ok += 1
            if want_crc:
                res.crc_checked += 1
                span = crc_span if len(crc_span) <= 250000 else crc_span[:250000]
                if crc_mpeg2(span) != crc:
                    res.crc_bad += 1
            try:
                if kind == "lot":
                    fields, cons = parser(payload)
                    res.lots.append(fields)
                    res.lot_versions[fields["version"]] += 1
                    # only fields 1-10 are needed; the rest of the record is not parsed
                    if cons > len(payload):
                        res.overrun[kind] += 1
                    else:
                        res.parse_ok[kind] += 1
                    continue
                if kind == "prop":
                    fields, cons = parser(payload)
                    res.prop_versions[fields["version"]] += 1
                    res.prop_usage[fields["usage"]] += 1
                    if fields["lot_tag"] == NO_TAG:
                        res.no_tag["prop"] += 1
                    res.props.append(Prop(oid, fields, False))
                else:
                    fields, cons = parser(payload)
                    res.fx_versions[fields["fx_version"]] += 1
                    res.fx_usage[fields["usage"]] += 1
                    if fields["lot_tag"] == NO_TAG:
                        res.no_tag["fx"] += 1
                    res.fxprops.append(Prop(oid, fields, True))
                if cons == len(payload):
                    res.parse_ok[kind] += 1
                elif cons < len(payload):
                    res.underrun[kind] += 1
                    if len(res.sample_errors) < 12:
                        res.sample_errors.append(
                            "%s oid=0x%08X underrun %d < %d" % (kind, oid, cons, len(payload)))
                else:
                    res.overrun[kind] += 1
                    if len(res.sample_errors) < 12:
                        res.sample_errors.append(
                            "%s oid=0x%08X overrun %d > %d" % (kind, oid, cons, len(payload)))
            except ParseError as ex:
                res.parse_fail[kind] += 1
                if len(res.sample_errors) < 12:
                    res.sample_errors.append("%s oid=0x%08X %s" % (kind, oid, ex))
        res.entries[kind] = n_ok
        if n_ok != want:
            res.count_mismatch[kind] = ("count", n_ok, want)
    return res


# --------------------------------------------------------------------------
# Part 4: GetLotProps and the match
# --------------------------------------------------------------------------

def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def object_world_position(lot, ox, oz):
    """lot-developer.md 17.6 step 3 for object types 0/1/3/4/5/6/7 (not the
    base-texture snap of step 2).  Returns (x, z)."""
    x1, z1, x2, z2 = lot["rect"]
    c0x = x1 * CELL
    c0z = z1 * CELL
    c1x = (x2 + 1) * CELL
    c1z = (z2 + 1) * CELL
    f = lot["facing"] & 3
    if f == 0:
        return (f32(c1x - ox), f32(c1z - oz))
    if f == 1:
        return (f32(c0x + oz), f32(c1z - ox))
    if f == 2:
        return (f32(c0x + ox), f32(c0z + oz))
    return (f32(c1x - oz), f32(c0z + ox))


def select_props(lot, props_by_cell):
    """GetLotProps (prop-lotconfig.md A7.2) on a list of candidate props.

    Stage 1: occupant manager slot 40 -> slot 36: strict box overlap of the
    occupant's bounding box with the query box
        x in [x1*16 + 0.1, (x2+1)*16 - 0.1] (same for z), y in [-inf, +inf]
    (occupant-manager.md 8.2 step 6, 8.4 step 4).
    Stage 2: keep only when the prop's position is inside
        wx1 <= px < wx2, wz1 <= pz < wz2
    with (wx1, wz1) = cell(x1, z1) -> world, (wx2, wz2) = cell(x2+1, z2+1) -> world
    (prop-lotconfig.md A7.2).  A NaN coordinate fails.
    """
    x1, z1, x2, z2 = lot["rect"]
    qx1 = f32(x1 * CELL + QUERY_EPS)
    qx2 = f32((x2 + 1) * CELL - QUERY_EPS)
    qz1 = f32(z1 * CELL + QUERY_EPS)
    qz2 = f32((z2 + 1) * CELL - QUERY_EPS)
    wx1 = x1 * CELL
    wz1 = z1 * CELL
    wx2 = (x2 + 1) * CELL
    wz2 = (z2 + 1) * CELL

    picked = []
    picked_world_only = []
    picked_box_only = []
    seen = set()
    for cx in range(x1, x2 + 1):
        for cz in range(z1, z2 + 1):
            for p in props_by_cell.get((cx, cz), ()):
                if id(p) in seen:
                    continue
                seen.add(id(p))
                b = p.bounds
                ox1, oy1, oz1, ox2, oy2, oz2 = b
                # stage 1: strict box overlap on x and z (y always overlaps)
                box_hit = (ox1 < qx2 and qx1 < ox2 and oz1 < qz2 and qz1 < oz2)
                # stage 2: world position filter on the centre
                px, pz = p.cx, p.cz
                world_hit = (wx1 <= px < wx2 and wz1 <= pz < wz2)
                if not (math.isnan(px) or math.isnan(pz) or
                        math.isnan(ox1) or math.isnan(ox2) or
                        math.isnan(oz1) or math.isnan(oz2)):
                    pass
                else:
                    world_hit = False   # "a NaN coordinate fails"
                if box_hit and world_hit:
                    picked.append(p)
                if world_hit:
                    picked_world_only.append(p)
                if box_hit:
                    picked_box_only.append(p)
    return picked, picked_world_only, picked_box_only


# --------------------------------------------------------------------------
# report
# --------------------------------------------------------------------------

def pct(n, d):
    return (100.0 * n / d) if d else 0.0


def fmt(n):
    return "{:,}".format(n)


def main(argv=None):
    ap = argparse.ArgumentParser(description="PV2: unmatched lot-tag props over the save corpus (read-only)")
    ap.add_argument("--dats", default=DEFAULT_DATS)
    ap.add_argument("--regions", default=DEFAULT_REGIONS)
    ap.add_argument("--documents", default=DEFAULT_DOCS)
    ap.add_argument("--dbpf", default=DEFAULT_DBPF)
    ap.add_argument("--json", default=None, help="write the raw tallies to this JSON file")
    ap.add_argument("--quick", action="store_true", help="skip the per-entry CRC check")
    args = ap.parse_args(argv)

    _install_dbpf(args.dbpf)

    out = {}
    print("=" * 78)
    print("PV2 -- saved props whose lot tag matches no object of their lot's configuration")
    print("=" * 78)

    # ---------------- Part 1: catalog ----------------
    print("\n[1] CATALOG  (read-only scan of the game's DBPF packages)")
    lot_cfgs, net_cfgs, net_by_inst, crep = load_catalog(args.dats)
    for df in crep["dat_files"]:
        if df.get("exists"):
            print("    %-16s entries=%-7d exemplars=%d" % (df["name"], df["entries"], df["exemplars"]))
        else:
            print("    %-16s MISSING" % df["name"])
    print("    exemplars scanned: %s   parse failures: %d"
          % (fmt(crep["exemplars_total"]), crep["parse_failures"]))
    print("    groups carrying 0x88EDC900 (object array): %s"
          % {("0x%08X" % k): v for k, v in sorted(crep["groups_with_object_props"].items())})
    print("    groups carrying 0x88EDC789 (version):      %s"
          % {("0x%08X" % k): v for k, v in sorted(crep["groups_with_version"].items())})
    print("    groups carrying exemplar type 0x21:        %s"
          % {("0x%08X" % k): v for k, v in sorted(crep["groups_with_type21"].items())})

    n_lot = len(lot_cfgs)
    n_net = len(net_cfgs)
    print("\n    LOT CONFIGURATIONS (group 0xA8FBD372, ID = the exemplar instance)")
    print("      configurations loaded (version != 0): %s   [published: 1,908]" % fmt(n_lot))
    print("      version distribution: %s"
          % {k: v for k, v in sorted(crep["lot_config_versions"].items())})
    print("      no 0x88EDC789 property: %d ; version 0: %d ; duplicate instance IDs: %d"
          % (crep["lot_config_no_version"], crep["lot_config_version_zero"],
             crep["duplicate_lot_instance"]))
    print("      with at least one object property: %d" % crep["lot_cfgs_with_object_props"])
    oc = crep["lot_object_counts"]
    print("      object properties (<10 values skipped): %s total, %d short, max %d (instance 0x%08X)"
          % (fmt(sum(oc)), crep["lot_short_props"], crep["max_objects"],
             crep["max_objects_instance"] or 0))
    print("      objects by type v0: %s"
          % {("type %d" % k): v for k, v in sorted(crep["lot_objects_by_type"].items())})
    print("      LOD-3 objects (dropped at every detail level): %d" % crep["lot_lod3"])

    print("\n    NETWORK LOT CONFIGURATIONS (exemplar type property 0x10 = 0x21)")
    print("      configurations: %s   [published: 543]" % fmt(n_net))
    print("      version distribution: %s"
          % {k: v for k, v in sorted(crep["net_config_versions"].items())})
    print("      with at least one object property: %d ; missing ID property 0x21: %d"
          % (crep["net_cfgs_with_object_props"], crep["net_config_missing_id"]))
    noc = crep["net_object_counts"]
    print("      object properties (<10 values skipped): %s total, %d short"
          % (fmt(sum(noc)), crep["net_short_props"]))
    print("      objects by type v0: %s"
          % {("type %d" % k): v for k, v in sorted(crep["net_objects_by_type"].items())})
    print("      LOD-3 objects: %d" % crep["net_lod3"])

    # tag universe: every object ID of every lot configuration
    tag_universe = set()
    for cfg in lot_cfgs.values():
        tag_universe |= cfg.ids_all

    # ---------------- Part 2: corpus ----------------
    print("\n[2] CORPUS  (SHA-256 distinctness)")
    reg_files = scan_saves(args.regions)
    doc_files = scan_saves(args.documents)
    reg_hashes = [h for h, _ in reg_files]
    doc_hashes = [h for h, _ in doc_files]
    reg_unique = set(reg_hashes)
    doc_unique = set(doc_hashes)
    print("    Regions:   %s .sc4 files, %s distinct by SHA-256   [published: 502 files / 421 distinct]"
          % (fmt(len(reg_files)), fmt(len(reg_unique))))
    print("    Documents: %s .sc4 files, %s distinct by SHA-256   [published: 587 files / 417 distinct]"
          % (fmt(len(doc_files)), fmt(len(doc_unique))))
    print("    same hash set in both: %s" % (reg_unique == doc_unique))

    # one representative per distinct save, first in path order
    by_hash = {}
    for h, p in reg_files:
        by_hash.setdefault(h, p)
    corpus = sorted(by_hash.items(), key=lambda kv: kv[1].lower())

    # per-region file counts and how each region's files relate
    region_files = defaultdict(list)
    for h, p in reg_files:
        region_files[os.path.basename(os.path.dirname(p))].append((h, p))
    print("    regions (files / distinct hashes):")
    for name in sorted(region_files):
        hs = set(h for h, _ in region_files[name])
        print("      %-16s %4d files  %4d distinct" % (name, len(region_files[name]), len(hs)))
    berlin = set(h for h, _ in region_files.get("Berlin", []))
    fct = set(h for h, _ in region_files.get("FC4 Test", []))
    if berlin and fct:
        print("    Berlin vs FC4 Test: shared %d, only Berlin %d, only FC4 Test %d"
              % (len(berlin & fct), len(berlin - fct), len(fct - berlin)))
        stock = set()
        for name, rows in region_files.items():
            if name != "FC4 Test":
                stock |= set(h for h, _ in rows)
        print("    distinct hashes ignoring the FC4 Test region: %s  (= the GOG stock set)"
              % fmt(len(stock)))

    # ---------------- Part 3: parse validation ----------------
    print("\n[3] PARSE VALIDATION  (class store of every distinct save)")
    totals = Counter()
    totals_stock = Counter()
    per_save = []
    saves_with_lots = 0
    saves_with_props = 0
    overruns = []
    for sha, path in corpus:
        res = parse_save(path, sha, want_crc=not args.quick)
        per_save.append(res)
        for k in ("lot", "prop", "fx"):
            totals[k + "_entries"] += res.entries[k]
            totals[k + "_ok"] += res.parse_ok[k]
            totals[k + "_overrun"] += res.overrun[k]
            totals[k + "_underrun"] += res.underrun[k]
            totals[k + "_fail"] += res.parse_fail[k]
            totals[k + "_trailing"] += res.trailing_bytes[k]
            if sha in stock:
                totals_stock[k] += res.entries[k]
        totals["crc_checked"] += res.crc_checked
        totals["crc_bad"] += res.crc_bad
        if res.count_mismatch:
            totals["count_mismatch_files"] += 1
        if res.lots:
            saves_with_lots += 1
        if res.props or res.fxprops:
            saves_with_props += 1
        if res.overrun["prop"] or res.overrun["fx"] or res.underrun["prop"] or res.underrun["fx"] \
           or res.parse_fail["prop"] or res.parse_fail["fx"]:
            if len(overruns) < 25:
                overruns.append((res.name, res.region, dict(res.overrun), dict(res.underrun),
                                 dict(res.parse_fail), list(res.sample_errors)))

    print("    distinct saves parsed: %s   saves with lots: %d   saves with props: %d"
          % (fmt(len(corpus)), saves_with_lots, saves_with_props))
    print("    LOTS      (0xC9BD5D4A): entries %s   [published: 12,986]"
          % fmt(totals["lot_entries"]))
    print("    PROPS     (0x2977AA47): entries %s   [published: 134,420]"
          % fmt(totals["prop_entries"]))
    print("    FX PROPS  (0x2977AA49): entries %s   [published: 948]"
          % fmt(totals["fx_entries"]))
    print("    the same over the 417-save GOG stock subset (the FC4 Test divergent copies excluded):")
    print("      lots %s [12,986]   props %s [134,420]   fx-props %s [948]"
          % (fmt(totals_stock["lot"]), fmt(totals_stock["prop"]), fmt(totals_stock["fx"])))
    print("    prop records parsed to the exact end of the entry: %s underrun, %s overrun, %s failed"
          % (fmt(totals["prop_underrun"]), fmt(totals["prop_overrun"]), fmt(totals["prop_fail"])))
    print("    fx-prop inner prop records: %s underrun, %s overrun, %s failed"
          % (fmt(totals["fx_underrun"]), fmt(totals["fx_overrun"]), fmt(totals["fx_fail"])))
    print("    lot headers (fields 1-10 only; the rest of the record is not parsed): %s ok"
          % fmt(totals["lot_ok"]))
    print("    class records with trailing / short entries: %s" % fmt(totals["lot_trailing"]
          + totals["prop_trailing"] + totals["fx_trailing"]))
    print("    files whose index objectCount disagrees with the entries walked: %d"
          % totals["count_mismatch_files"])
    if not args.quick:
        print("    entry CRCs checked: %s   mismatches: %s"
              % (fmt(totals["crc_checked"]), fmt(totals["crc_bad"])))
    else:
        print("    entry CRCs: skipped (--quick)")

    # version / usage histograms
    lv = Counter(); pv = Counter(); fv = Counter()
    pu = Counter(); fu = Counter(); nt = Counter()
    for r in per_save:
        lv.update(r.lot_versions); pv.update(r.prop_versions); fv.update(r.fx_versions)
        pu.update(r.prop_usage); fu.update(r.fx_usage)
        nt["prop"] += r.no_tag["prop"]; nt["fx"] += r.no_tag["fx"]
    print("    lot record versions:    %s" % dict(sorted(lv.items())))
    print("    prop record versions:   %s" % dict(sorted(pv.items())))
    print("    fx-prop outer versions: %s" % dict(sorted(fv.items())))
    print("    prop_usage histogram (props): %s" % dict(sorted(pu.items())))
    print("    prop_usage histogram (fx):    %s" % dict(sorted(fu.items())))
    print("    props with lot_tag 0xFFFFFFFF (no tag): %s props, %s fx-props"
          % (fmt(nt["prop"]), fmt(nt["fx"])))

    if overruns:
        print("    first records that did not consume the entry exactly:")
        for name, region, ov, un, pf, errs in overruns[:10]:
            print("      %s / %s  overrun=%s underrun=%s fail=%s" % (region, name, ov, un, pf))
            for e in errs[:4]:
                print("         %s" % e)

    # ---------------- Part 4: the number ----------------
    print("\n[4] PV2 -- the rule, applied")

    # Two scopes: "plain" = class 0x2977AA47 (cSC4PropOccupant), "fx" = the prop
    # record embedded in a 0x2977AA49 visual-effect prop.  The headline number is
    # the plain scope; effect props are reported separately.
    def new_tallies():
        return {
            "tagged": 0, "matched_all": 0, "matched_prop": 0,
            "unmatched_all": 0, "unmatched_prop": 0,
            "reason": Counter(), "reason_prop": Counter(),
            "tag_known_elsewhere": 0, "tag_unknown_globally": 0,
            "unmatched_tags": Counter(), "pairs": 0, "picked": 0,
        }

    T = {"plain": new_tallies(), "fx": new_tallies()}
    position_residuals = []
    position_checked = 0
    exact_zero = 0
    resid_buckets = Counter()
    # per prop type ID: n, sum dx, sum dz, sum dx^2, sum dz^2, n exact-zero
    ptype_stat = defaultdict(lambda: [0, 0.0, 0.0, 0.0, 0.0, 0])
    within_1m = 0
    within_025m = 0
    distinct_props = 0
    multi_lot_props = 0
    selected_world_only = 0
    selected_box_only = 0
    selected_both = 0
    per_region_unmatched = defaultdict(Counter)
    saves_with_unmatched = 0
    saves_with_tagged = 0
    sample_unmatched = []

    for res in per_save:
        if not res.lots:
            continue
        save_unmatched = 0
        save_tagged = 0
        for use_fx in (False, True):
            t = T["fx" if use_fx else "plain"]
            source = res.fxprops if use_fx else res.props
            if not source:
                continue
            by_cell = defaultdict(list)
            for p in source:
                if p.usage != 2:
                    continue
                cx = math.floor(p.cx / CELL)
                cz = math.floor(p.cz / CELL)
                if not (-8 <= cx <= 1024 and -8 <= cz <= 1024):
                    continue
                by_cell[(int(cx), int(cz))].append(p)

            selected_ids = defaultdict(list)   # id(prop) -> [lot index, ...]
            for li, lot in enumerate(res.lots):
                picked, world_only, box_only = select_props(lot, by_cell)
                selected_world_only += len(world_only)
                selected_box_only += len(box_only)
                selected_both += len(picked)
                for p in picked:
                    t["pairs"] += 1
                    t["picked"] += 1
                    selected_ids[id(p)].append(li)
                    if p.tag == NO_TAG:
                        continue
                    t["tagged"] += 1
                    save_tagged += 1
                    cfg_id = lot["config_id"]
                    cfg = lot_cfgs.get(cfg_id) if cfg_id else None
                    if cfg_id == 0:
                        why = "lot has no configuration (lot configuration ID 0)"
                    elif cfg is None:
                        if cfg_id in net_cfgs:
                            why = ("config ID is a network lot configuration ID, "
                                   "not a lot configuration")
                        else:
                            why = "config ID unknown to the catalog"
                    else:
                        why = None
                    ok_all = cfg is not None and p.tag in cfg.ids_all
                    ok_prop = cfg is not None and p.tag in cfg.ids_prop
                    if ok_all:
                        t["matched_all"] += 1
                    else:
                        t["unmatched_all"] += 1
                        if not use_fx:
                            save_unmatched += 1
                        t["reason"][why or "config exists but has no such object ID"] += 1
                        if p.tag in tag_universe:
                            t["tag_known_elsewhere"] += 1
                        else:
                            t["tag_unknown_globally"] += 1
                        t["unmatched_tags"][p.tag] += 1
                        if len(sample_unmatched) < 12:
                            sample_unmatched.append(
                                (res.region, res.name, "fx" if use_fx else "prop",
                                 p.oid, p.tag, cfg_id,
                                 (p.bounds[0], p.bounds[2], p.bounds[3], p.bounds[5])))
                    if ok_prop:
                        t["matched_prop"] += 1
                    else:
                        t["unmatched_prop"] += 1
                        if why:
                            t["reason_prop"][why] += 1
                        elif p.tag in cfg.ids_all:
                            t["reason_prop"][
                                "config has the object ID, but not on a type-1 (prop) object"] += 1
                        else:
                            t["reason_prop"]["config exists but has no such object ID"] += 1
                    # position check: prop centre vs the object's world position
                    if ok_all and cfg is not None:
                        first = None
                        for o in cfg.objects:
                            if o["id"] == p.tag:
                                first = o
                                break
                        if first is not None and first["type"] != 2:
                            ex, ez = object_world_position(lot, first["ox"], first["oz"])
                            d = math.hypot(p.cx - ex, p.cz - ez)
                            position_residuals.append(d)
                            position_checked += 1
                            dx, dz = p.cx - ex, p.cz - ez
                            if d == 0.0:
                                exact_zero += 1
                            if d < 1e-6:
                                resid_buckets["exactly 0"] += 1
                            elif d < 0.25:
                                resid_buckets["< 0.25 m"] += 1
                            elif d < 1.0:
                                resid_buckets["< 1 m"] += 1
                            elif d < 4.0:
                                resid_buckets["< 4 m"] += 1
                            elif d < 16.0:
                                resid_buckets["< 16 m"] += 1
                            else:
                                resid_buckets[">= 16 m"] += 1
                            s = ptype_stat[p.prop_type]
                            s[0] += 1
                            s[1] += dx
                            s[2] += dz
                            s[3] += dx * dx
                            s[4] += dz * dz
                            if d == 0.0:
                                s[5] += 1
                            if d <= 1.0:
                                within_1m += 1
                            if d <= 0.25:
                                within_025m += 1
            for _pid, lots in selected_ids.items():
                distinct_props += 1
                if len(lots) > 1:
                    multi_lot_props += 1
        if save_tagged:
            saves_with_tagged += 1
        if save_unmatched:
            saves_with_unmatched += 1
            per_region_unmatched[res.region]["saves_with_unmatched"] += 1
            per_region_unmatched[res.region]["unmatched"] += save_unmatched
        per_region_unmatched[res.region]["tagged"] += save_tagged

    P, F = T["plain"], T["fx"]
    all_tagged = P["tagged"] + F["tagged"]
    all_unmatched = P["unmatched_all"] + F["unmatched_all"]

    print("\n    --- selection (GetLotProps) ---")
    print("    (lot, prop) pairs examined           : %s  (plain %s + effect-prop %s)"
          % (fmt(P["pairs"] + F["pairs"]), fmt(P["pairs"]), fmt(F["pairs"])))
    print("    distinct props selected              : %s  (selected by more than one lot: %s)"
          % (fmt(distinct_props), fmt(multi_lot_props)))
    print("    props with a lot tag examined        : %s  (plain %s + effect-prop %s)"
          % (fmt(all_tagged), fmt(P["tagged"]), fmt(F["tagged"])))
    print("    selection-rule cross-check over the same props:")
    print("      kept by the world-position filter only : %s" % fmt(selected_world_only))
    print("      kept by the manager box query only     : %s" % fmt(selected_box_only))
    print("      kept by BOTH (the game's GetLotProps)  : %s" % fmt(selected_both))
    print("      -> the manager box query drops %s props that the world filter alone would keep"
          % fmt(selected_world_only - selected_both))

    print("\n    --- THE NUMBER (plain prop records, class 0x2977AA47) ---")
    print("    tagged props examined                : %s" % fmt(P["tagged"]))
    print("    UNMATCHED against ALL object IDs     : %s   (%.4f %% of tagged)"
          % (fmt(P["unmatched_all"]), pct(P["unmatched_all"], P["tagged"])))
    print("    UNMATCHED against type-1 prop IDs    : %s   (%.4f %% of tagged)"
          % (fmt(P["unmatched_prop"]), pct(P["unmatched_prop"], P["tagged"])))
    print("    matched against ALL object IDs       : %s" % fmt(P["matched_all"]))
    print("    matched against type-1 prop IDs      : %s" % fmt(P["matched_prop"]))
    print("    why unmatched (rule = ALL object IDs of the lot's configuration):")
    for k, v in P["reason"].most_common():
        print("      %-72s %s" % (k, fmt(v)))
    print("    why unmatched (rule = type-1 prop objects only):")
    for k, v in P["reason_prop"].most_common():
        print("      %-72s %s" % (k, fmt(v)))
    print("    of those unmatched, the tag is an object ID of SOME shipped configuration: %s"
          % fmt(P["tag_known_elsewhere"]))
    print("    of those unmatched, the tag is no shipped configuration object ID at all: %s"
          % fmt(P["tag_unknown_globally"]))

    print("\n    --- the same, for the prop records inside visual-effect props (0x2977AA49) ---")
    print("    tagged examined %s ; unmatched vs ALL object IDs %s (%.4f %%) ; "
          "vs type-1 prop IDs %s (%.4f %%)"
          % (fmt(F["tagged"]), fmt(F["unmatched_all"]), pct(F["unmatched_all"], F["tagged"]),
             fmt(F["unmatched_prop"]), pct(F["unmatched_prop"], F["tagged"])))
    for k, v in F["reason"].most_common():
        print("      %-72s %s" % (k, fmt(v)))
    print("    combined (plain + effect-prop) tagged %s ; unmatched %s (%.4f %%)"
          % (fmt(all_tagged), fmt(all_unmatched), pct(all_unmatched, all_tagged)))

    print("\n    --- distribution ---")
    print("    distinct saves with at least one tagged prop examined : %s" % fmt(saves_with_tagged))
    print("    distinct saves with at least one UNMATCHED prop       : %s" % fmt(saves_with_unmatched))
    print("    per region (attributed to the region of the representative file of each distinct save):")
    print("      %-16s %10s %10s %10s" % ("region", "tagged", "unmatched", "saves w/ un"))
    for name in sorted(per_region_unmatched):
        c = per_region_unmatched[name]
        if c["tagged"] == 0 and c["unmatched"] == 0:
            continue
        print("      %-16s %10s %10s %10s"
              % (name, fmt(c["tagged"]), fmt(c["unmatched"]), fmt(c["saves_with_unmatched"])))
    if P["unmatched_tags"]:
        print("    most common unmatched tags (tag -> count):")
        for tg, n in P["unmatched_tags"].most_common(15):
            print("      0x%08X  %s" % (tg, fmt(n)))
    if sample_unmatched:
        print("    first unmatched records:")
        for s in sample_unmatched:
            print("      %s / %s %s oid=0x%08X tag=0x%08X cfg=0x%08X bounds=(%g,%g)-(%g,%g)"
                  % (s[0], s[1], s[2], s[3], s[4], s[5], s[6][0], s[6][1], s[6][2], s[6][3]))

    print("\n    --- position-rule check (is GetPosition the centre of the bounds box?) ---")
    print("    matched pairs compared with the object's world position from")
    print("    lot-developer.md 17.6 slot 39 (object offsets + lot facing): %s"
          % fmt(position_checked))
    if position_residuals:
        rs = sorted(position_residuals)
        print("      |prop centre - expected object position| : median %.6f m, mean %.6f m, "
              "p90 %.6f m, max %.6f m"
              % (rs[len(rs) // 2], sum(rs) / len(rs), rs[int(len(rs) * 0.9)], rs[-1]))
        print("      exactly 0 : %s (%.2f %%)   within 0.25 m : %s (%.2f %%)   within 1 m : %s (%.2f %%)"
              % (fmt(exact_zero), pct(exact_zero, position_checked),
                 fmt(within_025m), pct(within_025m, position_checked),
                 fmt(within_1m), pct(within_1m, position_checked)))
        print("      residual buckets:")
        for k in ("exactly 0", "< 0.25 m", "< 1 m", "< 4 m", "< 16 m", ">= 16 m"):
            print("        %-10s %s" % (k, fmt(resid_buckets.get(k, 0))))
        # is the residual a per-prop-type constant (an asymmetric model's AABB
        # centre vs its anchor), or does it vary for the same prop type?
        centred = varies = offset = 0
        for _pt, s in ptype_stat.items():
            n, sx, sz, sx2, sz2, nz = s
            if n < 4:
                continue
            mx, mz = sx / n, sz / n
            vx = max(0.0, sx2 / n - mx * mx)
            vz = max(0.0, sz2 / n - mz * mz)
            if vx < 1e-4 and vz < 1e-4:
                if abs(mx) < 0.01 and abs(mz) < 0.01:
                    centred += 1
                else:
                    offset += 1
            else:
                varies += 1
        print("      prop type IDs checked (>= 4 matched props each): %d" % (centred + varies + offset))
        print("        centred on the object position (offset ~ 0, no scatter) : %d" % centred)
        print("        constant per-type offset, no scatter (model AABB)        : %d" % offset)
        print("        residual varies within the type                        : %d" % varies)
    else:
        print("      no matched pairs available to check")

    out = {
        "catalog": {
            "lot_configurations": n_lot,
            "network_lot_configurations": n_net,
            "lot_versions": dict(crep["lot_config_versions"]),
            "net_versions": dict(crep["net_config_versions"]),
            "lot_objects_by_type": dict(crep["lot_objects_by_type"]),
            "net_objects_by_type": dict(crep["net_objects_by_type"]),
            "lot_object_properties": sum(oc),
            "net_object_properties": sum(noc),
            "groups_with_object_props": {"0x%08X" % k: v for k, v in crep["groups_with_object_props"].items()},
            "groups_with_version": {"0x%08X" % k: v for k, v in crep["groups_with_version"].items()},
            "groups_with_type21": {"0x%08X" % k: v for k, v in crep["groups_with_type21"].items()},
        },
        "corpus": {
            "regions_files": len(reg_files), "regions_distinct": len(reg_unique),
            "documents_files": len(doc_files), "documents_distinct": len(doc_unique),
            "same_hash_set": reg_unique == doc_unique,
        },
        "parse": {
            "lots": totals["lot_entries"], "props": totals["prop_entries"],
            "fx_props": totals["fx_entries"],
            "lots_stock417": totals_stock["lot"],
            "props_stock417": totals_stock["prop"],
            "fx_props_stock417": totals_stock["fx"],
            "prop_exact_end": totals["prop_ok"], "prop_underrun": totals["prop_underrun"],
            "prop_overrun": totals["prop_overrun"], "prop_fail": totals["prop_fail"],
            "fx_underrun": totals["fx_underrun"], "fx_overrun": totals["fx_overrun"],
            "fx_fail": totals["fx_fail"],
            "crc_checked": totals["crc_checked"], "crc_bad": totals["crc_bad"],
            "count_mismatch_files": totals["count_mismatch_files"],
            "saves_with_lots": saves_with_lots, "saves_with_props": saves_with_props,
            "prop_usage": dict(pu), "fx_usage": dict(fu),
            "no_tag": dict(nt),
        },
        "pv2": {
            "plain": {
                "pairs": P["pairs"], "picked": P["picked"], "tagged_examined": P["tagged"],
                "matched_all_object_ids": P["matched_all"],
                "unmatched_all_object_ids": P["unmatched_all"],
                "unmatched_rate_all": pct(P["unmatched_all"], P["tagged"]),
                "matched_prop_objects_only": P["matched_prop"],
                "unmatched_prop_objects_only": P["unmatched_prop"],
                "unmatched_rate_prop": pct(P["unmatched_prop"], P["tagged"]),
                "reason_all": dict(P["reason"]), "reason_prop": dict(P["reason_prop"]),
                "tag_known_elsewhere": P["tag_known_elsewhere"],
                "tag_unknown_globally": P["tag_unknown_globally"],
                "unmatched_tags_top": P["unmatched_tags"].most_common(50),
            },
            "effect_props": {
                "pairs": F["pairs"], "picked": F["picked"], "tagged_examined": F["tagged"],
                "matched_all_object_ids": F["matched_all"],
                "unmatched_all_object_ids": F["unmatched_all"],
                "unmatched_prop_objects_only": F["unmatched_prop"],
                "reason_all": dict(F["reason"]), "reason_prop": dict(F["reason_prop"]),
            },
            "combined": {
                "tagged_examined": all_tagged,
                "unmatched_all_object_ids": all_unmatched,
                "unmatched_rate_all": pct(all_unmatched, all_tagged),
            },
            "distinct_props": distinct_props,
            "multi_lot_props": multi_lot_props,
            "saves_with_tagged": saves_with_tagged,
            "saves_with_unmatched": saves_with_unmatched,
            "per_region": {k: dict(v) for k, v in per_region_unmatched.items()},
            "selection_world_only": selected_world_only,
            "selection_box_only": selected_box_only,
            "selection_both": selected_both,
            "position_checked": position_checked,
            "position_within_1m": within_1m, "position_within_0_25m": within_025m,
            "position_exact_zero": exact_zero,
            "position_buckets": dict(resid_buckets),
            "position_residual_median": (sorted(position_residuals)[len(position_residuals) // 2]
                                         if position_residuals else None),
            "position_residual_max": (max(position_residuals) if position_residuals else None),
        },
    }
    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump(out, f, indent=2, sort_keys=True, default=str)
        print("\n    JSON written to %s" % args.json)

    print("\n" + "=" * 78)
    return 0


# --------------------------------------------------------------------------
# dbpf import
# --------------------------------------------------------------------------

_DBPF = None


def _install_dbpf(path):
    """Import tools/dbpf.py from an explicit path (never from cwd or sys.path)."""
    global _DBPF
    path = os.path.abspath(path)
    if not os.path.isfile(path):
        raise SystemExit("cannot find tools/dbpf.py at %s (pass --dbpf)" % path)
    import importlib.util
    spec = importlib.util.spec_from_file_location("fc4_dbpf", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    _DBPF = mod


def _import_dbpf():
    return _DBPF


if __name__ == "__main__":
    sys.exit(main())

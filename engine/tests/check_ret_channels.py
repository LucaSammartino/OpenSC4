# SPDX-License-Identifier: GPL-3.0-or-later
#
# FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
# Copyright (C) 2026 FreeCity4 contributors
#
# This program is free software: you can redistribute it and/or modify it
# under the terms of the GNU General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option)
# any later version. This program is distributed WITHOUT ANY WARRANTY; see
# the GNU General Public License for more details. You should have received a
# copy of the license along with this program (see LICENSE at the repository
# root); if not, see <https://www.gnu.org/licenses/>.

"""Return-channel check: every place the plugin USES the value a game method
returns, cross-referenced with that method's `ret` column in
docs/abi/interfaces.tsv (and docs/abi/vtables.tsv for the vt: classes) and
with what the specs say the method returns (SPEC_FACTS below).

Prompted by the autoload (merge 4c92365): cISC4Region::GetCityLocations (slot
23, ret `eax-or-void`) is declared `bool` in the SDK, but the game's method
does not set AL; the plugin trusted that leftover register and the queue runs
ended "no city load".

`eax-or-void` is the table's default class (a 32-bit value, a pointer, a bool
or nothing): the table alone cannot tell a real bool from a void method. So a
use of a slot declared `bool` (MSVC reads AL only) is classified like this:

  spec-none  SPEC_FACTS says the original returns nothing meaningful (budget.md
             §6's closing rule, zones.md §11.3 "no bool result", a "none" / "—"
             row): the value is a leftover AL. A defect.
  spec-wide  SPEC_FACTS says the slot returns a count in EAX (the reverse case:
             the SDK's bool reads AL only, false for a multiple of 256). A defect.
  sdk        the specs are silent (framework methods): trusted as the SDK
             declares it, listed with --verbose, never failed; each is a
             numbered question in the lane report (HG2, HG3, LD33).
  unchecked  a bool read of a slot whose row is not `al` and that SPEC_FACTS
             does not cover: add the spec's answer to SPEC_FACTS (with its
             citation), or fix the call. A failure.
  void-ret   the row says `void` or `unknown` and the declaration returns a
             value. A failure.
  al-wide    the row says `al` and the declaration returns 32 bits (int, enum,
             pointer): the upper three bytes are whatever was in EAX. A failure.

A use is "reviewed" when the call's line or one of the three lines above has a
comment with `[abi-ret]` (or the older `[abi]` marker together with "return",
"not used" or "eax-or-void") that says why the value is fine (only logged,
passed through as the original passes it). Reviewed uses are listed, not
failed. A call whose value is discarded (a whole statement, `(void)`) is not a
use.

The receiver's class is resolved from the nearest declaration of the receiver
variable in the same file (or the .cpp's header), a helper's declared result,
a cast, or fc4::AsGame; when it cannot be resolved, every declared interface
with that method name is a candidate and the use counts only if all
candidates agree (--verbose lists the disagreements as "ambiguous").

For interfaces the plugin implements (the game calls our method), --impl lists
the `eax` rows we declare `bool`: information only, never failed.

Usage (from engine/): python tests/check_ret_channels.py [--verbose] [--impl]
Exit status 1 when an unreviewed use is flagged. Run by ctest as
ret_channel_check.
"""

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ENGINE = os.path.dirname(HERE)
ROOT = os.path.dirname(ENGINE)
PLUGIN = os.path.join(ENGINE, "plugin")
TSV = os.path.join(ROOT, "docs", "abi", "interfaces.tsv")
VT_TSV = os.path.join(ROOT, "docs", "abi", "vtables.tsv")

sys.path.insert(0, HERE)
import gen_abi_entries as gen  # noqa: E402  (the declaration parser and the interface list)

UNKNOWN_BASE = "cIGZUnknown"
COM = [("QueryInterface", "bool", "uint32_t riid, void** ppvObj", ""), ("AddRef", "uint32_t", "", ""),
       ("Release", "uint32_t", "", "")]
NARROW = {"bool", "char", "signed char", "unsigned char", "int8_t", "uint8_t", "int16_t", "uint16_t", "short",
          "unsigned short", "wchar_t"}
# Return types that are not in EAX at all (the ABI conformance test checks those channels).
OTHER_CHANNEL = re.compile(r"^(?:const\s+)?(?:float|double|long double|int64_t|uint64_t|long long|"
                           r"unsigned long long|__int64)$")


def load_rows(path, key_col, prefix=""):
    rows = {}
    with open(path, encoding="utf-8") as f:
        header = f.readline().rstrip("\n").split("\t")
        col = {n: i for i, n in enumerate(header)}
        for line in f:
            p = line.rstrip("\n").split("\t")
            if len(p) < len(header) - 1:
                continue
            key = prefix + p[col[key_col]]
            try:
                slot = int(p[col["slot"]])
            except ValueError:
                continue
            rows.setdefault(key, {})[slot] = (p[col["ret"]], p[col["ret_confidence"]], p[col["method"]])
    return rows


def header_path(header):
    return header if os.path.isabs(header) else os.path.join(gen.SDK, header)


def msvc_order(own):
    """MSVC places overloads of one name together, at the first one's position, in reverse
    declaration order (the slot orders gen_abi_entries.py notes for the aura and flora)."""
    groups, order = {}, []
    for m in own:
        if m[0] not in groups:
            groups[m[0]] = []
            order.append(m[0])
        groups[m[0]].append(m)
    out = []
    for name in order:
        out += list(reversed(groups[name]))
    return out


_ordered_cache = {}


def ordered(header, cls):
    """[(name, ret, params, const)] in vtable slot order, cIGZUnknown's three first."""
    k = (header, cls)
    if k in _ordered_cache:
        return _ordered_cache[k]
    base, _ = gen.class_body(header, cls)
    flat = gen.methods(header, cls)
    if base and base != UNKNOWN_BASE:
        bh = gen.BASE_HEADERS.get(base, base + ".h")
        prefix = ordered(bh, base)
        own = flat[len(gen.methods(bh, base)):]
    else:
        prefix = COM
        own = flat
    out = prefix + msvc_order(own)
    _ordered_cache[k] = out
    return out


def norm_ret(ret):
    r = " ".join(ret.replace("virtual", "").split())
    r = re.sub(r"\bconst\b\s*", "", r).strip()
    return r


# What the specs say a slot returns, for the slots whose `ret` column cannot tell (eax-or-void, eax, via-call,
# forward) and that the plugin reads as a bool. Keyed by (interface without its "(variant)" suffix, or the vt: key;
# slot). Kinds:
#   bool  the spec gives a real bool result (AL, or EAX 0/1): reading it is fine;
#   none  the spec says the original returns no meaningful value: a use is a defect;
#   wide  the spec gives a 32-bit value (a count) whose AL is not the answer: a bool read is a defect;
#   sdk   the spec is silent (framework methods no spec covers): trusted as the SDK declares it, listed with
#         --verbose, and a numbered question in the lane report (HG2, HG3, LD33).
B, N, W, S = "bool", "none", "wide", "sdk"
SPEC_FACTS = {
    # framework (GZCOM, streams, variants, windows, the app)
    ("cIGZMessageServer2", 4): (B, "flammability.md / landvalue.md service tables: MessagePost, AL"),
    ("cIGZMessageServer2", 5): (B, "lot-developer.md service table: MessageSend, AL"),
    ("cIGZMessageServer2", 6): (B, "lot-developer.md service table: AL"),
    ("cIGZMessageTarget2", 3): (B, "aura.md 12.3, building-development.md 7.2: DoMessage, AL = 1"),
    ("cIGZSerializable", 3): (B, "building-development.md 7.4, aura.md 12.2: Write's bool"),
    ("cIGZSerializable", 4): (B, "building-development.md 7.4, aura.md 12.2: Read's bool"),
    ("vt:cSC4DemandSimulator::vftable_2", 3): (B, "cIGZSerializable::Write (building-development.md 7.4)"),
    ("vt:cSC4DemandSimulator::vftable_2", 4): (B, "cIGZSerializable::Read (building-development.md 7.4)"),
    ("cIGZIStream", 7): (B, "foundations.md 2.1: every Get* returns the stream's good flag (FQ3: EAX)"),
    ("cIGZIStream", 9): (B, "foundations.md 2.1: every Get* returns the stream's good flag (FQ3: EAX)"),
    ("cIGZOStream", 7): (B, "foundations.md 2.1: every Set* returns the stream's good flag (FQ3: EAX)"),
    ("cIGZOStream", 9): (B, "foundations.md 2.1: every Set* returns the stream's good flag (FQ3: EAX)"),
    ("cIGZVariant", 9): (B, "data-formats.md: GetValBool"),
    ("cIGZVariant", 54): (S, "spec-silent CreateValString (HG3)"),
    ("cIGZVariant", 55): (B, "data-formats.md: GetValString"),
    ("cIGZWinMgr", 24): (B, "modal-dialogs.md: AL"),
    ("cIGZWinMgr", 54): (B, "modal-dialogs.md: AL"),
    ("cIGZWin", 36): (S, "spec-silent GetChildAs (HG3); every site also tests the out pointer"),
    ("cIGZWin", 67): (B, "modal-dialogs.md: GetFlag, EAX 0/1"),
    ("cIGZDate", 4): (S, "spec-silent SetDayNumber (HG3); passed through"),
    ("cIGZDate", 6): (S, "spec-silent SetMonthDayYear (HG3); passed through"),
    ("cIGZPersistResourceManager", 3): (B, "flora.md, residential.md: GetResource, AL"),
    ("cIGZPersistResourceManager", 4): (S, "spec-silent GetPrivateResource (HG3); the site also tests the out pointer"),
    ("cIGZPersistResourceManager", 14): (S, "spec-silent (HG3)"),
    ("vt:cGZCOM::vftable", 3): (S, "spec-silent GetClassObject (HG3); every site also tests the out pointer"),
    ("vt:cSC4App::vftable_3", 10): (S, "spec-silent RequestQuit (HG2)"),
    ("vt:cSC4App::vftable_3", 13): (S, "spec-silent LoadCity (HG2)"),
    ("vt:cSC4App::vftable_3", 15): (S, "spec-silent SaveCity (HG2)"),
    ("vt:cSC4App::vftable_3", 16): (S, "spec-silent SaveCity (HG2)"),
    ("vt:cSC4App::vftable_3", 21): (S, "spec-silent GetDebugFunctionalityEnabled (HG2)"),
    ("vt:cSC4App::vftable_3", 23): (B, "advisors.md gate a: GetPopupDialogsEnabled"),
    ("vt:cSC4City::vftable", 22): (B, "city slot table (spec rows used by the cores)"),
    ("vt:cSC4City::vftable", 98): (B, "city slot table (spec rows used by the cores)"),
    ("vt:cSC4City::vftable", 99): (B, "city slot table (spec rows used by the cores)"),
    ("vt:cSC4Region::vftable", 23): (N, "the GetCityLocations precedent (merge 4c92365): AL not set"),
    ("vt:cSC4DispatchManager::vftable", 10): (B, "dispatch.md 8: GetDispatchInfo, AL"),
    ("vt:cSC4DispatchManager::vftable", 13): (N, "dispatch.md 8: RegisterDispatchSource returns none"),
    ("vt:cSC4LotBaseTextureOccupant::vftable", 9): (S, "spec-silent UpdateVertexAltitudes (LD33)"),
    ("cISCStringDetokenizer", 9): (B, "lua-host.md 10: every token resolved"),
    ("cISCPropertyHolder", 3): (B, "foundations.md 17 table: HasProperty"),
    ("cISCPropertyHolder", 14): (B, "foundations.md 17 table: AddProperty"),
    ("cISCPropertyHolder", 17): (B, "foundations.md 17 table: RemoveProperty"),
    ("cISCExemplarPropertyHolder", 3): (N, "landfill.md slot table (0x00458E60): SetDefaultExemplar sets no result"),
    # simulators and city objects
    ("cISC424HourClock", 25): (B, "clock24.md: EAX 0/1"),
    ("cISC424HourClock", 27): (B, "clock24.md: EAX 0/1"),
    ("cISC424HourClock", 28): (B, "clock24.md: EAX 0/1"),
    ("cISC4BudgetSimulator", 8): (B, "budget.md 6: WithdrawFunds"),
    ("cISC4BudgetSimulator", 29): (N, "budget.md 6, closing rule: GetAllGroups returns no meaningful value"),
    ("cISC4BudgetSimulator", 31): (N, "budget.md 6, closing rule: GetDepartmentBudgetsInGroup, no meaningful value"),
    ("cISC4BudgetSimulator", 50): (B, "budget.md 6"),
    ("cISC4BudgetSimulator", 51): (B, "budget.md 6: GetBudgetItemForPurpose"),
    **{("cISC4BuildingDevelopmentSimulator", s): (B, "building-development.md 7.1: AL")
       for s in (14, 15, 46, 50, 54, 55, 56, 57, 58)},
    ("cISC4CivicBuildingSimulator", 5): (B, "civic slot table: AL"),
    ("cISC4CrimeSimulator", 8): (B, "crime.md slot table"),
    ("cISC4Demand", 30): (B, "building-development.md slot 51: returns EndOfCycle's result"),
    ("cISC4Demolition", 7): (B, "demolition slot table: AL"),
    ("cISC4Demolition", 8): (B, "demolition slot table: AL"),
    ("cISC4DepartmentBudget", 24): (B, "power.md, protection.md: AL"),
    ("cISC4AirportDeveloper", 12): (B, "port-developers.md 8.1: GetAirportUsageValues, AL = found"),
    ("cISC4SeaportDeveloper", 12): (B, "port-developers.md 8.1: GetSeaportUsageValues, AL = found"),
    ("cISC4SeaportDeveloper", 15): (B, "port-developers.md 8.1: CityHasSea, AL"),
    ("cISC4DepartmentBudget", 28): (B, "power.md, protection.md: AL"),
    ("cISC4DisasterLayer", 10): (B, "disaster layer slot table: AL"),
    ("cISC4DisasterManagerFire", 4): (B, "crime.md, flammability.md"),
    ("cISC4DisasterManagerFire", 5): (B, "crime.md, flammability.md"),
    ("cISC4FireProtectionSimulator", 11): (B, "protection.md: IsOnStrike, AL"),
    ("cISC4PoliceSimulator", 10): (B, "protection.md: IsOnStrike, AL"),
    ("cISC4FlammabilitySimulator", 6): (B, "flammability.md: AL"),
    ("cISC4FlammabilitySimulator", 7): (B, "flammability.md: AL"),
    ("cISC4FloraSimulator", 11): (B, "flora.md: EAX 0/1"),
    **{("cISC4GrowthDeveloper", s): (B, "growth-developer.md slot table") for s in (3, 4, 10, 28)},
    **{("cISC4HistoryWarehouse", s): (B, "history.md: AL") for s in (5, 6, 7, 8)},
    ("cISC4LandfillOccupant", 3): (B, "landfill.md: AL"),
    ("cISC4LandfillOccupant", 9): (B, "landfill.md: AL"),
    ("cISC4LotConfigurationManager", 18): (N, "prop-lotconfig.md B7: RegisterLotConfigurationFilter, no defined return"),
    **{("cISC4LotDeveloper", s): (B, "lot-developer.md 11.1") for s in (6, 25, 26, 36, 41, 46, 56, 57)},
    ("cISC4LotDeveloper", 7): (B, "lot-developer.md 11.1: ConstructDefaultLot, AL of slot 6"),
    ("cISC4LotDeveloper", 8): (N, "lot-developer.md 11.1: AL of SetState, which lot-records.md 13.2 leaves undefined"),
    ("cISC4LotDeveloper", 9): (B, "lot-developer.md 11.1: AL of SetPopulation (tract-developer.md table: AL)"),
    ("cISC4LotDeveloper", 19): (W, "lot-developer.md 11.1: EAX = 0 or a count"),
    ("cISC4LotDeveloper", 20): (W, "lot-developer.md 11.1: EAX = 0 or a count"),
    ("cISC4LotDeveloper", 21): (W, "lot-developer.md 11.1: AttemptAreaLevelling, EAX = 0 or ModifyTerrainHeight's count"),
    ("cISC4LotDeveloper", 45): (N, "lot-developer.md 6.6: UpdateVertexAltitudes leaves AL undefined"),
    ("cISC4LotManager", 22): (B, "zones.md 11.3: EAX 0/1"),
    ("cISC4LotManager", 23): (B, "zones.md 11.3: CreateLot, EAX 0/1"),
    ("cISC4LotManager", 24): (N, "zones.md 8.6 / 11.3: DeleteLot, EAX undefined, no bool result"),
    **{("cISC4Lot", s): (B, "lot-developer.md lot table")
       for s in (6, 14, 21, 27, 28, 29, 37, 38, 39, 43, 55, 65, 66)},
    ("cISC4Lot", 34): (N, "lot-records.md 13.2: SetState, none defined"),
    ("cISC4NeighborConnection", 11): (B, "neighbors.md slot table"),
    ("cISC4NeighborConnection", 13): (B, "neighbors.md slot table"),
    ("cISC4NeighborDeal", 35): (B, "neighbors.md: returns true / otherwise false"),
    **{("cISC4NetworkOccupant", s): (B, "traffic-network.md network occupant table") for s in (8, 9, 22, 47)},
    ("cISC4NetworkOccupant", 67): (B, "QUESTIONS.md TR19 (re-answers25): GetPassageCeiling returns true with the float, false otherwise"),
    **{("cISC4Occupant", s): (B, "occupant slot table") for s in (5, 11, 14, 22, 27, 32)},
    ("cISC4OrdinanceSimulator", 8): (B, "ordinance.md"),
    **{("cISC4Ordinance", s): (B, "ordinance.md 3 table")
       for s in (3, 4, 17, 18, 19, 21, 22, 23, 24, 25, 26, 27, 28, 29)},
    ("cISC4PlumbingSimulator", 11): (B, "plumbing.md: AL only"),
    ("cISC4PlumbingSimulator", 24): (B, "plumbing.md: AL only"),
    ("cISC4PollutionSimulator", 49): (B, "pollution.md slot table"),
    ("cISC4PowerLineOccupant", 6): (B, "power.md"),
    ("cISC4PowerPoleOccupant", 11): (B, "power.md"),
    ("cISC4PowerPoleOccupant", 13): (B, "power.md"),
    **{("cISC4PowerSimulator", s): (B, "power.md slot table") for s in (19, 31, 37)},
    ("cISC4RegionalCity", 43): (B, "traffic-network.md: GetEstablished"),
    **{("cISC4Simulator", s): (B, "simulator.md slot table") for s in (9, 12, 15, 16, 17, 19, 28)},
    ("cISTESurfaceWater", 8): (B, "traffic-network.md surface water table (4.10.1): IsNavigable, AL"),
    ("cISC4TractDeveloper", 11): (B, "tract-developer.md slot table"),
    ("cISC4TutorialSystem", 8): (B, "tutorial system slot: EAX 0/1"),
    ("cISC4ZoneManager", 11): (B, "zones.md 11.1"),
    ("cISC4ZoneManager", 14): (B, "zones.md 11.1"),
}


def fact_of(key, slot):
    base = key if key.startswith("vt:") else key.split("(")[0]
    return SPEC_FACTS.get((key, slot)) or SPEC_FACTS.get((base, slot))


def classify(key, slot, decl_ret, row_ret):
    """The flag for a use of this slot's value, or None."""
    r = norm_ret(decl_ret)
    if r == "void" or OTHER_CHANNEL.match(r) or slot <= 2:
        return None  # slots 0-2: the GZCOM QueryInterface / AddRef / Release contract
    narrow = r in NARROW
    fact = fact_of(key, slot)
    if fact is not None:
        kind = fact[0]
        if kind == "none":
            return "spec-none"
        if kind == "wide" and narrow:
            return "spec-wide"
        if kind == "sdk" and r == "bool":
            return "sdk"
        return None
    t = row_ret.lower()
    if t in ("void", "unknown"):
        return "void-ret"
    if t == "al" and not narrow and "&" not in r:
        return "al-wide"
    if r == "bool" and t != "al":
        # eax-or-void (a value, a bool or nothing), eax (0/1, or a count whose AL is not the answer),
        # via-call, forward: the table cannot tell; the spec decides (a SPEC_FACTS entry).
        return "unchecked"
    return None


def build_declarations(verbose):
    """class name -> {method name -> [(key, slot, decl ret, row ret, conf, table method)]}"""
    rows = load_rows(TSV, "interface")
    vt_rows = None
    entries = [(k, c, h) for (k, c, h, _req) in gen.INTERFACES]
    known = {c for (_k, c, _h) in entries}
    # Every other interface of the table that has an SDK header of the same name (the plugin may call it
    # through the SDK declaration without listing it for the ABI test).
    for key in rows:
        base = key.split("(")[0]
        if "<" in base or base in known:
            continue
        if os.path.exists(header_path(base + ".h")):
            entries.append((key, base, base + ".h"))
    decls, checked, mism = {}, 0, []
    for key, cls, header in entries:
        if key.startswith("vt:"):
            if vt_rows is None:
                vt_rows = load_rows(VT_TSV, "class", "vt:")
            table = vt_rows.get(key, {})
        else:
            table = rows.get(key, {})
        if not table:
            continue
        try:
            ms = ordered(header, cls)
        except SystemExit:
            continue
        cname = cls.split("<")[0] if "<" not in key else cls
        for slot, (name, ret, _ps, _c) in enumerate(ms):
            row = table.get(slot)
            if row is None:
                continue
            checked += 1
            tm = row[2].split("::")[-1]
            if not row[2].startswith("icf::") and tm and tm != name and not tm.startswith("~"):
                mism.append("%s %s slot %d: declared %s, table %s" % (key, cls, slot, name, row[2]))
            decls.setdefault(cname, {}).setdefault(name, []).append((key, slot, ret, row[0], row[1], row[2]))
    if verbose:
        print("%d declared slots matched to table rows; %d whose table name differs (ported names, ICF):"
              % (checked, len(mism)))
        for m in mism:
            print("   ", m)
    return decls


# ---- call sites ---------------------------------------------------------------------------------------------------

def strip_keep_lines(text):
    """Comments and string/char literals blanked, newlines kept (so offsets map to lines)."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", text[i:j]))
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            j = min(j + 1, n)
            out.append(c + " " * (j - i - 2) + c if j - i >= 2 else text[i:j])
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


CLOSE = {")": "(", "]": "[", ">": "<"}


def receiver_start(code, pos):
    """Start of the postfix expression that ends just before `pos` (the -> or .)."""
    i = pos - 1
    while i >= 0:
        while i >= 0 and code[i] in " \t\r\n":
            i -= 1
        if i < 0:
            break
        c = code[i]
        if c in CLOSE:
            depth, j = 0, i
            while j >= 0:
                if code[j] == c:
                    depth += 1
                elif code[j] == CLOSE[c]:
                    depth -= 1
                    if depth == 0:
                        break
                j -= 1
            i = j - 1
            # a cast or call name before the bracket
            while i >= 0 and code[i] in " \t":
                i -= 1
            if i >= 0 and (code[i].isalnum() or code[i] == "_" or code[i] in ">"):
                continue
            return i + 1
        elif c.isalnum() or c == "_":
            while i >= 0 and (code[i].isalnum() or code[i] == "_"):
                i -= 1
            k = i
            while k >= 0 and code[k] in " \t":
                k -= 1
            if k >= 1 and code[k - 1:k + 1] in ("->", "::"):
                i = k - 2
                continue
            if k >= 0 and code[k] == ".":
                i = k - 1
                continue
            if k >= 0 and code[k] == "*" and (k == 0 or code[k - 1] in "(*& \t\n=,"):
                # a dereference: (*p)->X is handled by the bracket; a bare *p.X is unusual
                return i + 1
            return i + 1
        else:
            return i + 1
    return 0


def matching_close(code, open_pos):
    depth = 0
    for j in range(open_pos, len(code)):
        if code[j] == "(":
            depth += 1
        elif code[j] == ")":
            depth -= 1
            if depth == 0:
                return j
    return len(code) - 1


STMT_KEYWORDS = ("else", "do", "return")


def value_used(code, start, close):
    """False when the call is a whole statement (its value discarded)."""
    j = close + 1
    while j < len(code) and code[j] in " \t\r\n":
        j += 1
    if j >= len(code) or code[j] != ";":
        return True
    i = start - 1
    while i >= 0 and code[i] in " \t\r\n":
        i -= 1
    if i < 0 or code[i] in ";{}:":
        return False
    if code[i] == ")":
        # (void)x->F(); or if (...) x->F();
        depth, j = 0, i
        while j >= 0:
            if code[j] == ")":
                depth += 1
            elif code[j] == "(":
                depth -= 1
                if depth == 0:
                    break
            j -= 1
        inner = code[j + 1:i].strip()
        if inner == "void":
            return False
        k = j - 1
        while k >= 0 and code[k] in " \t\r\n":
            k -= 1
        m = re.search(r"(\w+)$", code[:k + 1])
        return not (m and m.group(1) in ("if", "for", "while", "switch"))
    m = re.search(r"(\w+)$", code[:i + 1])
    if m and m.group(1) in ("else", "do"):
        return False
    return True


TYPE_DECL = (r"\b((?:cFC4|FC4|cIGZ|cISC|cIST|cS3D|cRZ|cGZ|cSC)\w*)\s*(?:<\s*[\w:]+\s*>)?\s*(?:const\s*)?[*&]?\s*"
             r"(?:const\s*)?%s\b\s*[;=,)\[{(]")
AUTOREF_DECL = r"\bcRZAutoRefCount\s*<\s*(\w+)\s*>\s*&?\s*%s\b"
FUNC_DECL = r"^[ \t]*(?:static\s+|inline\s+)*(?:fc4::)?((?:cFC4|cIGZ|cISC|cIST|cS3D)\w*)\s*\*\s*%s\s*\("
ASGAME = re.compile(r"\bAsGame\s*<\s*(\w+)\s*>")
ASGAME_CALL = re.compile(r"\bAsGame\s*\(\s*(.*)\)\s*$", re.S)
CAST = re.compile(r"_cast\s*<\s*(?:const\s+)?(\w+)\s*(?:const\s*)?[*&]\s*>")


def var_type(code, extra, var, pos):
    best = None
    # before the use, then the .cpp's header, then the whole file (a class member declared below its methods)
    for src, limit in ((code, pos), (extra, None), (code, None)):
        if not src:
            continue
        hay = src if limit is None else src[:limit]
        for pat in (TYPE_DECL, AUTOREF_DECL):
            for mm in re.finditer(pat % re.escape(var), hay):
                best = mm.group(1)
        if best:
            return best
    return None


def func_type(code, extra, fn):
    for src in (code, extra):
        if src:
            m = None
            for m in re.finditer(FUNC_DECL % re.escape(fn), src, re.M):
                pass
            if m:
                return m.group(1)
    return None


def game_view(cls, classes):
    """fc4::AsGame(p): the cFC4I view of p's SDK class (FC4Interfaces.h naming)."""
    if cls and cls.startswith("cI"):
        v = "cFC4" + cls[1:]
        if v in classes:
            return v
    return cls


def resolve_class(code, extra, recv, pos, classes, depth=0):
    """The declared class of the receiver expression: a name, or None when unknown."""
    recv = recv.strip()
    if depth > 3 or not recv:
        return None
    m = CAST.search(recv) or ASGAME.search(recv)
    if m:
        return m.group(1)
    m = ASGAME_CALL.search(recv)
    if m:
        return game_view(resolve_class(code, extra, m.group(1), pos, classes, depth + 1), classes)
    m = re.search(r"(\w+)\s*\((.*)\)\s*$", recv, re.S)
    if m:  # a helper's result
        return func_type(code, extra, m.group(1))
    m = re.search(r"\(\s*\*\s*(\w+)\s*\)\s*$", recv)
    if m:
        return None  # (*it): an iterator's element
    m = re.search(r"(\w+)\s*(?:\[[^\]]*\])?\s*$", recv)
    if not m:
        return None
    return var_type(code, extra, m.group(1), pos)


def scan(decls, verbose):
    names = {}
    for cls, ms in decls.items():
        for name, rs in ms.items():
            names.setdefault(name, []).append((cls, rs))
    call = re.compile(r"(->|\.)\s*(\w+)\s*\(")
    results = []
    for fn in sorted(os.listdir(PLUGIN)):
        if not fn.endswith((".cpp", ".h")):
            continue
        path = os.path.join(PLUGIN, fn)
        raw = open(path, encoding="utf-8", errors="replace").read()
        code = strip_keep_lines(raw)
        lines = raw.split("\n")
        extra = ""
        if fn.endswith(".cpp"):
            hp = os.path.join(PLUGIN, fn[:-4] + ".h")
            if os.path.exists(hp):
                extra = strip_keep_lines(open(hp, encoding="utf-8", errors="replace").read())
        for m in call.finditer(code):
            name = m.group(2)
            if name not in names:
                continue
            start = receiver_start(code, m.start(1))
            recv = code[start:m.start(1)]
            open_pos = m.end() - 1
            close = matching_close(code, open_pos)
            if not value_used(code, start, close):
                continue
            cls = resolve_class(code, extra, recv, start, decls)
            if cls is not None and cls in decls and name in decls[cls]:
                cands = [(cls, decls[cls][name])]
            elif cls is not None:
                continue  # our own class, the SDK's cRZ helpers, or a method the table does not list
            else:
                cands = names[name]
            flags = set()
            per = []
            for c, rs in cands:
                for key, slot, ret, rret, conf, tmethod in rs:
                    f = classify(key, slot, ret, rret)
                    flags.add(f)
                    per.append((f, c, key, slot, ret, rret, conf, tmethod))
            if cls is None and len(flags) != 1:
                # unresolved and the candidates disagree: listed with --verbose only
                if verbose and any(flags):
                    results.append(("ambiguous", fn, raw.count("\n", 0, m.start()) + 1, name, recv.strip(), per))
                continue
            flag = next(iter(flags))
            if flag is None:
                continue
            line = raw.count("\n", 0, m.start()) + 1
            ctx = "\n".join(lines[max(0, line - 4):line])
            reviewed = "[abi-ret]" in ctx or ("[abi]" in ctx and re.search(r"return|not used|eax-or-void", ctx))
            results.append(("reviewed" if reviewed else flag, fn, line, name, recv.strip(),
                            [p for p in per if p[0] == flag]))
    return results


def impl_report(decls):
    """Interfaces the plugin implements: `eax` rows declared bool (the game may read all of EAX)."""
    out = []
    impl = set()
    for fn in os.listdir(PLUGIN):
        if fn.endswith(".h"):
            text = strip_keep_lines(open(os.path.join(PLUGIN, fn), encoding="utf-8", errors="replace").read())
            for m in re.finditer(r"\bclass\s+\w+[^{;]*?:\s*([^{;]+)\{", text):
                impl |= set(re.findall(r"public\s+(\w+)", m.group(1)))
    for cls in sorted(impl & set(decls)):
        for name, rs in decls[cls].items():
            for key, slot, ret, rret, conf, tm in rs:
                if norm_ret(ret) == "bool" and rret == "eax":
                    out.append("%s slot %d %s: declared bool, the game's row returns eax (%s, %s)"
                               % (key, slot, name, conf, tm))
    return out


def main():
    verbose = "--verbose" in sys.argv
    decls = build_declarations(verbose)
    results = scan(decls, verbose)
    bad = 0
    for kind, fn, line, name, recv, per in results:
        if kind == "ambiguous":
            print("ambiguous  plugin/%s:%d %s->%s: candidates disagree" % (fn, line, recv, name))
            for f, c, key, slot, ret, rret, conf, tm in per:
                print("             %-8s %s (%s slot %d): declared %s, table %s %s" % (f, c, key, slot, ret, rret,
                                                                                    conf))
            continue
        if kind == "sdk" and not verbose:
            continue
        if kind not in ("reviewed", "sdk"):
            bad += 1
        f, c, key, slot, ret, rret, conf, tm = per[0]
        print("%-10s plugin/%s:%d %s->%s  [%s slot %d %s: declared %s, ret %s %s]"
              % (kind, fn, line, recv, name, key, slot, tm, norm_ret(ret), rret, conf))
    if "--impl" in sys.argv:
        for line in impl_report(decls):
            print("impl-info ", line)
    print("ret-channel check: %d flagged use(s) not reviewed, %d reviewed"
          % (bad, sum(1 for r in results if r[0] == "reviewed")))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

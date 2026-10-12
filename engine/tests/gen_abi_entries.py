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

"""Writes tests/abi_entries.inc: for every interface in docs/abi/interfaces.tsv
that the plugin declares, the list of its virtual methods as member-function
pointers. The test takes everything else from the compiler: the slot (from the
vcall thunk), the argument bytes and the return channel (from the type).

Only the method NAMES come from this script. A non-overloaded method is
written as &Class::Name, so a changed signature is picked up without
regenerating; an overloaded SDK method needs its type spelled out (a changed
signature then fails to compile, which is the cue to regenerate). A method
added to a declaration but missing here shows up as a gap in the slot
numbers, which the test reports.

Usage (from engine/): python tests/gen_abi_entries.py
"""

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ENGINE = os.path.dirname(HERE)
SDK = os.path.join(ENGINE, "third_party", "gzcom-dll", "gzcom-dll", "include")
FC4 = os.path.join(ENGINE, "plugin", "FC4Interfaces.h")

# (table interface, class, header, required)
#   required = the plugin implements or calls the interface through this
#   declaration: a mismatch fails the test. Otherwise it is reported only.
INTERFACES = [
    ("cISC4Simulator", "cFC4ISC4Simulator", FC4, True),
    ("cISC4BudgetSimulator", "cFC4ISC4BudgetSimulator", FC4, True),
    ("cISC4DepartmentBudget", "cFC4ISC4DepartmentBudget", FC4, True),
    ("cIGZDate", "cFC4IGZDate", FC4, True),
    ("cIGZVariant", "cFC4IGZVariant", FC4, True),
    ("cISCPropertyHolder", "cFC4ISCPropertyHolder", FC4, True),
    ("cISC4LotManager", "cFC4ISC4LotManager", FC4, True),
    ("cISC4Lot", "cFC4ISC4Lot", FC4, True),
    ("cISC4LineItem", "cISC4LineItem", "cISC4LineItem.h", True),
    ("cIGZString", "cIGZString", "cIGZString.h", True),
    ("cIGZPersistResourceManager", "cIGZPersistResourceManager", "cIGZPersistResourceManager.h", True),
    ("cISC4BuildingDevelopmentSimulator", "cISC4BuildingDevelopmentSimulator", "cISC4BuildingDevelopmentSimulator.h", True),
    ("cISC4DisasterManager", "cISC4DisasterManager", "cISC4DisasterManager.h", True),
    ("cISC4HistoryWarehouse", "cISC4HistoryWarehouse", "cISC4HistoryWarehouse.h", True),
    ("cISC4NeighborsSimulator", "cFC4ISC4NeighborsSimulator", FC4, True),
    ("cISC4OrdinanceSimulator", "cISC4OrdinanceSimulator", "cISC4OrdinanceSimulator.h", True),
    ("cISCResExemplar", "cISCResExemplar", "cISCResExemplar.h", True),
    # Not in interfaces.tsv: checked against the implementing class's vtable in
    # docs/abi/vtables.tsv ("vt:<vtable name>"); the class may have more slots.
    ("vt:cSC4City::vftable", "cISC4City", "cISC4City.h", True),
    ("vt:cSC4App::vftable_3", "cISC4App", "cISC4App.h", True),
    ("vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", "cISC4ResidentialSimulator.h", True),
    ("vt:cSC4DemandSimulator::vftable", "cISC4DemandSimulator", "cISC4DemandSimulator.h", True),
    ("cISC4Demand", "cISC4Demand", "cISC4Demand.h", True),
    # Land value (docs/specs/landvalue.md §11, §12): implemented, and the game
    # services and grids it calls. A template instance is named Class<T>.
    ("cISC4LandValueSimulator", "cISC4LandValueSimulator", "cISC4LandValueSimulator.h", True),
    ("cIGZSerializable(LandValue)", "cIGZSerializable", "cIGZSerializable.h", True),
    ("cIGZMessageTarget2(LandValue)", "cIGZMessageTarget2", "cIGZMessageTarget2.h", True),
    ("cISC4SimGrid<uint8>", "cISC4SimGrid<uint8_t>", "cISC4SimGrid.h", True),
    ("cISC4SimGrid<int16>", "cISC4SimGrid<int16_t>", "cISC4SimGrid.h", True),
    ("cISC4SimGrid<float>", "cISC4SimGrid<float>", "cISC4SimGrid.h", True),
    ("cISTETerrain", "cISTETerrain", "cISTETerrain.h", True),
    ("cISC4ZoneManager", "cISC4ZoneManager", "cISC4ZoneManager.h", True),
    ("cISC4LotDeveloper", "cISC4LotDeveloper", "cISC4LotDeveloper.h", True),
    ("cISC4GrowthDeveloper", "cISC4GrowthDeveloper", "cISC4GrowthDeveloper.h", True),
    # The SDK declarations we replaced, and interfaces the plugin does not use yet: reported only.
    ("cISC4Simulator", "cISC4Simulator", "cISC4Simulator.h", False),
    ("cISC4BudgetSimulator", "cISC4BudgetSimulator", "cISC4BudgetSimulator.h", False),
    ("cISC4DepartmentBudget", "cISC4DepartmentBudget", "cISC4DepartmentBudget.h", False),
    ("cIGZDate", "cIGZDate", "cIGZDate.h", False),
    ("cIGZVariant", "cIGZVariant", "cIGZVariant.h", False),
    ("cISCPropertyHolder", "cISCPropertyHolder", "cISCPropertyHolder.h", False),
    ("cISC4LotManager", "cISC4LotManager", "cISC4LotManager.h", False),
    ("cISC4Lot", "cISC4Lot", "cISC4Lot.h", False),
    ("cISC4NeighborsSimulator", "cISC4NeighborsSimulator", "cISC4NeighborsSimulator.h", False),
    ("cISC4Ordinance", "cISC4Ordinance", "cISC4Ordinance.h", False),
    ("cISC4RegionalCity", "cISC4RegionalCity", "cISC4RegionalCity.h", False),
    ("cISC4WeatherSimulator", "cISC4WeatherSimulator", "cISC4WeatherSimulator.h", False),
    ("cISC4GameDataRegistry", "cISC4GameDataRegistry", "cISC4GameDataRegistry.h", False),
]

BASE_HEADERS = {"cIGZSerializable": "cIGZSerializable.h"}


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    return re.sub(r"//[^\n]*", " ", text)


def class_body(path, cls):
    cls = cls.split("<")[0]  # a template instance: the template's declaration
    if not os.path.isabs(path):
        path = os.path.join(SDK, path)
    text = strip_comments(open(path, encoding="utf-8", errors="replace").read())
    m = re.search(r"\bclass\s+%s\s*(?::\s*public\s+(\w+))?\s*\{" % re.escape(cls), text)
    if m is None:
        sys.exit("class %s not found in %s" % (cls, path))
    depth, i = 1, m.end()
    while depth:
        depth += {"{": 1, "}": -1}.get(text[i], 0)
        i += 1
    return m.group(1), text[m.end():i - 1]


def split_params(params):
    out, depth, cur = [], 0, ""
    for ch in params:
        if ch in "(<":
            depth += 1
        elif ch in ")>":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur)
    return [p.split("=")[0].strip() for p in out]


def methods(path, cls):
    base, body = class_body(path, cls)
    found = []
    if base and base != "cIGZUnknown":
        found += methods(BASE_HEADERS.get(base, base + ".h"), base)
    # Nested types used in signatures need the class prefix at namespace scope.
    nested = set(re.findall(r"\b(?:class|struct|enum(?:\s+class)?)\s+(\w+)", body))
    nested |= set(re.findall(r"\busing\s+(\w+)\s*=", body))
    nested |= set(re.findall(r"\btypedef\b[^;(]*?\b(\w+)\s*;", body))
    nested |= set(re.findall(r"\btypedef\b[^;]*?\(\s*\*\s*(\w+)\s*\)", body))
    nested -= {"class", "struct", "enum"}
    decl = re.compile(r"virtual\s+([^;{]*?)\b(operator\s*[^\s(]+|\w+)\s*\(([^;{]*?)\)\s*(const)?\s*(?:override)?\s*=\s*0\s*;",
                      re.S)
    for m in decl.finditer(body):
        ret, name, params, const = (" ".join(m.group(1).split()), m.group(2), " ".join(m.group(3).split()),
                                    m.group(4) or "")
        if name.startswith("~"):
            continue

        def qualify(t, cls=cls):
            return re.sub(r"\b(%s)\b" % "|".join(map(re.escape, nested)), cls + r"::\1", t) if nested else t

        ps = ", ".join(qualify(p) for p in split_params(params) if p != "void")
        found.append((name, qualify(ret), ps, const))
    return found


def main():
    out = ["// SPDX-License-Identifier: GPL-3.0-or-later", "//"]
    out.append("// Generated by tests/gen_abi_entries.py - do not edit. Regenerate after adding")
    out.append("// an interface or a method (a gap in the slots or a compile error says so).")
    table = []
    for i, (iface, cls, header, required) in enumerate(INTERFACES):
        ms = [("QueryInterface", "bool", "uint32_t riid, void** ppvObj", ""), ("AddRef", "uint32_t", "", ""),
              ("Release", "uint32_t", "", "")] + methods(header, cls)
        counts = {}
        for name, _, _, _ in ms:
            counts[name] = counts.get(name, 0) + 1
        fn = "AbiCollect%d_%s" % (i, re.sub(r"\W", "_", cls))
        out.append("")
        out.append("void %s(std::vector<AbiMethod>& m) {" % fn)
        out.append("    using C = %s;" % cls)
        for name, ret, ps, const in ms:
            if counts[name] == 1:
                out.append("    m.push_back(AbiM(&C::%s, \"%s\"));" % (name, name))
            else:
                sig = "%s(%s)%s" % (name, ps, " const" if const else "")
                out.append("    m.push_back(AbiM(static_cast<%s (C::*)(%s)%s>(&C::%s), \"%s\"));"
                           % (ret, ps, " const" if const else "", name, sig.replace('"', '\\"')))
        out.append("}")
        table.append("    {\"%s\", \"%s\", %s, &%s}," % (iface, cls, "true" if required else "false", fn))
    out.append("")
    out.append("const AbiInterface kAbiInterfaces[] = {")
    out += table
    out.append("};")
    path = os.path.join(HERE, "abi_entries.inc")
    open(path, "w", encoding="utf-8", newline="\n").write("\n".join(out) + "\n")
    print("wrote %s (%d interfaces)" % (path, len(INTERFACES)))


if __name__ == "__main__":
    main()

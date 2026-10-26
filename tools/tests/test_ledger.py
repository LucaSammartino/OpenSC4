"""Run-7 credit must come from exact, sufficiently sampled function evidence."""
import csv
import importlib.util
import json
from pathlib import Path
import struct
import sys
from types import SimpleNamespace

import pytest

from tools import compare_metrics as cm, function_boundary, oracle_class as oc

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("ledger_under_test", ROOT / "tools/ledger.py")
ledger = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ledger)


def line(source="natural", count=20, **changes):
    fields = dict(name="Fixture", rva="001000", bytes=40, installed=1, source=source,
                  calls=count, sampled=count, compared=count, exact=count,
                  float_only=0, different=0, outside=0, refused=0, nested=0,
                  mutation=0, detected=0, max_ulp=0)
    fields.update(changes)
    return "2026-10-25 [pid 1] inplace summary " + " ".join(f"{k}={v}" for k, v in fields.items()) + "\n"


def binding(mode="compare"):
    return dict(zip(cm.REGISTRY_FIELDS, ("Fixture", "00401000", "Owner", "thiscall", "ecx",
                                       "0", "0", "eax", "this:4", "fixture_entry", mode)))


def evaluate(tmp_path, text, mode="compare", **thresholds):
    log = tmp_path / "game.log"
    log.write_text(text, encoding="utf-8")
    evidence, warnings = cm.read_comparisons([log])
    return cm.function_state(binding(mode), evidence.get(0x401000, {}), 40, **thresholds), warnings


def test_thresholds_and_replace_in_same_log(tmp_path):
    assert not evaluate(tmp_path, line(count=19))[0]["matched"]
    state, warnings = evaluate(tmp_path, line(), mode="replace")
    assert state["bound"] and state["matched"] and state["replaced-fn"]
    assert not warnings
    assert not evaluate(tmp_path, line(), mode="compare")[0]["replaced-fn"]
    assert not evaluate(tmp_path, line("fixture", 400))[0]["matched"]
    assert evaluate(tmp_path, line(count=5), min_natural=5)[0]["matched"]
    with pytest.raises(ValueError, match="positive"):
        evaluate(tmp_path, line(), min_natural=0)


@pytest.mark.parametrize("source", ["corpus", "dense"])
def test_dense_sources_are_optional_but_each_must_meet_threshold(tmp_path, source):
    assert evaluate(tmp_path, line())[0]["matched"]
    assert not evaluate(tmp_path, line() + line(source, 199))[0]["matched"]
    assert evaluate(tmp_path, line() + line(source, 200))[0]["matched"]
    state, _ = evaluate(tmp_path, line() + line(source, 0))
    assert state["matched"] and state["matched-natural"] and not state["matched-dense"]
    assert not evaluate(tmp_path, line() + line("corpus", 200) + line("dense", 199))[0]["matched"]


@pytest.mark.parametrize("change", [dict(outside=1), dict(mutation=1), dict(detected=1),
    dict(exact=19, float_only=1), dict(exact=19, different=1), dict(installed=0),
    dict(name="Other"), dict(compared=-1), dict(calls=1), dict(exact=0)])
def test_fail_closed_even_if_later_snapshot_looks_exact(tmp_path, change):
    state, _ = evaluate(tmp_path, line(**change) + line(count=100))
    assert not state["matched"]


def test_fixture_mismatch_vetoes_and_skipped_calls_do_not(tmp_path):
    assert not evaluate(tmp_path, line() + line("fixture", 1, exact=0, different=1))[0]["matched"]
    assert evaluate(tmp_path, line(calls=25, refused=2, nested=3))[0]["matched"]


def test_tiers_are_independent_and_require_same_log_for_matched(tmp_path):
    state, _ = evaluate(tmp_path, line() + line("dense", 199))
    assert state["matched-natural"] and not state["matched-dense"] and not state["matched"]
    state, _ = evaluate(tmp_path, line(count=19) + line("dense", 200))
    assert state["matched-dense"] and not state["matched-natural"] and not state["matched"]
    state, _ = evaluate(tmp_path, line(count=1, exact=0, different=1) + line("dense", 200))
    assert state["matched-dense"] and not state["matched-natural"] and not state["matched"]
    state, _ = evaluate(tmp_path, line() + line("dense", 200, exact=199, different=1))
    assert state["matched-natural"] and not state["matched-dense"] and not state["matched"]
    a, b = tmp_path / "a.log", tmp_path / "b.log"
    a.write_text(line() + line("dense", 0), encoding="utf-8")
    b.write_text(line(count=0) + line("dense", 200), encoding="utf-8")
    evidence, _ = cm.read_comparisons([a, b])
    state = cm.function_state(binding("replace"), evidence[0x401000], 40)
    assert state["matched-natural"] and state["matched-dense"]
    assert not state["matched"] and not state["replaced-fn"]


@pytest.mark.parametrize("refused, matched", [(0, True), (2, True), (3, False)])
def test_natural_refusal_ratio_and_boundary(tmp_path, refused, matched):
    state, _ = evaluate(tmp_path, line(calls=30, refused=refused, nested=5))
    assert state["matched"] == matched
    assert state["refusal_ratio"] == refused / 20
    assert (state["natural_refused"], state["natural_sampled"]) == (refused, 20)
    assert state["natural_calls"] == 30


def test_refusal_failure_is_retained_and_zero_sampled_fails_closed(tmp_path):
    state, _ = evaluate(tmp_path, line(calls=30, refused=3) + line(count=100))
    assert not state["matched"] and state["refusal_ratio"] == 0.15
    state, _ = evaluate(tmp_path, line(count=0, calls=3, refused=3) + line())
    assert not state["matched"] and state["refusal_ratio"] is None
    assert "undefined" in cm.evidence_note({1: state}, [], "12345678")
    # Replay refusals do not impose the natural refusal policy.
    assert evaluate(tmp_path, line() + line("dense", 200, refused=199))[0]["matched"]


def test_zero_compare_replay_errors_still_veto(tmp_path):
    state, _ = evaluate(tmp_path, line() + line("dense", 0, outside=1))
    assert state["matched-natural"] and not state["matched"]


def test_summary_byte_field_never_sets_size_or_denominator(tmp_path):
    for size in (0, 1, 41, 1000000):
        state, warnings = evaluate(tmp_path, line(bytes=size))
        assert state["matched"] and not warnings
        assert cm.tally([(0x401000, 40)], {0x401000: state})["matched_b"] == 40


def test_s4_name_join_requires_unique_registry_name(tmp_path):
    log = tmp_path / "s4.log"
    log.write_text(line(bytes=0).replace(" rva=001000", ""), encoding="utf-8")
    evidence, warnings = cm.read_comparisons([log], bindings={0x401000: binding()})
    assert not warnings and cm.function_state(binding(), evidence[0x401000], 40)["matched"]
    for bindings in ({}, {0x401000: binding(), 0x402000: binding()}):
        evidence, warnings = cm.read_comparisons([log], bindings=bindings)
        assert not evidence and warnings
    # A malformed explicit RVA never falls back to a name join.
    log.write_text(line(rva="oops", bytes=0), encoding="utf-8")
    evidence, warnings = cm.read_comparisons([log], bindings={0x401000: binding()})
    assert not evidence and warnings


def test_build_filter_rejects_stale_evidence_without_vetoing_current(tmp_path):
    log = tmp_path / "game.log"
    log.write_text(line(build="deadbeef", exact=19, different=1) + line(build="a3af5de4"), encoding="utf-8")
    evidence, warnings = cm.read_comparisons([log], expected_build="a3af5de4")
    state = cm.function_state(binding(), evidence[0x401000], 40)
    assert state["matched"] and len(warnings) == 1 and "stale compare build=deadbeef" in warnings[0]
    assert "ignored: 1" in cm.evidence_note({1: state}, warnings, "a3af5de4")
    for expected in (None, "ffffffff"):
        evidence, warnings = cm.read_comparisons([log], expected_build=expected)
        assert not evidence and len(warnings) == 2
    assert cm.build_matches("A3AF5DE4" + "0" * 56, "a3af5de4")
    assert not cm.build_matches("a3af", "a3af5de4")
    log.write_text(line(build="deadbeef"), encoding="utf-8")
    evidence, _ = cm.read_comparisons([log], expected_build="a3af5de4")
    assert not evidence  # stale exact evidence cannot grant credit or prove reach


def test_house_build_is_recorded_hash_not_current_dll(tmp_path):
    house = tmp_path / "house.out"
    house.write_text("staged DLL: C:/old.dll (sha256 a3af5de4)\n", encoding="utf-16")
    assert cm.regress_build(house) == "a3af5de4"
    house.write_text("REGRESS house: PASS\n", encoding="utf-8")
    assert cm.regress_build(house) is None
    house.write_text("staged DLL: a (sha256 a3af5de4)\nstaged DLL: b (sha256 deadbeef)\n", encoding="utf-8")
    assert cm.regress_build(house) is None


def test_cumulative_reports_and_multiple_logs_never_pool(tmp_path):
    assert not evaluate(tmp_path, line(count=10) * 3)[0]["matched"]
    a, b = tmp_path / "a.log", tmp_path / "b.log"
    a.write_text(line(count=10), encoding="utf-8")
    b.write_text(line(count=10), encoding="utf-8")
    evidence, _ = cm.read_comparisons([a, b, a])
    assert len(evidence[0x401000]) == 2
    assert not cm.function_state(binding(), evidence[0x401000], 40)["matched"]
    a.write_text(line(), encoding="utf-8")
    b.write_text(line(count=1, exact=0, different=1), encoding="utf-8")
    evidence, _ = cm.read_comparisons([a, b])
    assert not cm.function_state(binding(), evidence[0x401000], 40)["matched"]
    b.write_text(line("dense", 200), encoding="utf-8")
    evidence, _ = cm.read_comparisons([a, b])
    assert not cm.function_state(binding(), evidence[0x401000], 40)["matched"]


def test_utf16_forward_fields_and_malformed_summaries(tmp_path):
    path = tmp_path / "powershell.log"
    path.write_text(line().rstrip() + " future_counter=7\n", encoding="utf-16")
    evidence, warnings = cm.read_comparisons([path])
    assert not warnings
    assert cm.function_state(binding(), evidence[0x401000], 40)["matched"]
    for bad in (line().replace(" exact=20", ""), line().replace("outside=0", "outside=oops"),
                line().rstrip() + " exact=20\n"):
        state, warnings = evaluate(tmp_path, bad + line())
        assert warnings and not state["matched"]


def test_registry_optional_fixture_and_ambiguity(tmp_path):
    assert cm.read_registry(tmp_path / "absent") == ({}, [])
    path = tmp_path / "fixture.tsv"
    row = binding("replace")
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=cm.REGISTRY_FIELDS, delimiter="\t")
        writer.writeheader()
        writer.writerow(row)
    log = tmp_path / "game.log"
    log.write_text(line(), encoding="utf-8")
    game = [(0x401000, 40, None, "shared"), (0x402000, 160, None, "unattributed")]
    states, bindings, _, warnings = cm.collect_states(game, tmp_path, [log])
    assert not warnings
    assert bindings[0x401000]["windows"] == "this:4"
    assert states[0x401000]["replaced-fn"] and not states[0x402000]["bound"]
    assert cm.tally(game, states)["matched_b"] == 40  # 20% bytes, 50% functions
    log.write_text(line(bytes=0, build="a3af5de4").replace(" rva=001000", ""), encoding="utf-8")
    states, _, _, warnings = cm.collect_states(game, tmp_path, [log], expected_build="a3af5de4")
    assert not warnings and states[0x401000]["replaced-fn"]
    assert cm.tally(game, states)["matched-natural_b"] == 40
    assert cm.tally(game, states)["matched-dense_b"] == 0
    path.with_name("duplicate.tsv").write_bytes(path.read_bytes())
    bindings, warnings = cm.read_registry(tmp_path)
    assert not bindings and warnings


def test_only_house_log_paths_are_selected(tmp_path):
    for name in ("regress-house.out", "regress-house-shadows.log", "regress-house-replace.log",
                 "regress-other-shadows.log", "q-main-regress-house-shadows.log"):
        (tmp_path / name).touch()
    assert {p.name for p in cm.regress_logs(tmp_path / "regress-house.out")} == {
        "regress-house.out", "regress-house-shadows.log", "regress-house-replace.log"}


def test_named_shadow_reach_does_not_credit_entire_owner_or_ambiguous_names(tmp_path):
    names = {0x401000: "A::PlaceZone", 0x402000: "A::Other", 0x403000: "B::PlaceZone",
             0x404000: "C::Ambiguous", 0x405000: "C::Ambiguous", 0x406000: "C::Zero", 0x407000: None}
    data = dict(game=[(va, 40, 0 if va < 0x403000 else 1, "simulation") for va in names],
                att=SimpleNamespace(name=names.__getitem__),
                rows=[dict(shadow="zones_shadow"), dict(shadow="other_shadow")])
    log = tmp_path / "shadows.log"
    log.write_text("zones shadow (PlaceZone) load: calls=64 mismatches=0\n"
                   "zones shadow (queries) month: calls=1000\n"
                   "other shadow (Ambiguous) month: calls=20\n"
                   "other shadow (Zero) month: calls=0\n", encoding="utf-8")
    assert ledger.shadow_reach(data, [log]) == {0x401000: 64}


def test_coverage_report_appends_byte_weighted_function_columns(tmp_path, monkeypatch):
    cov = ledger.cov
    registry = tmp_path / "engine/plugin/compare"
    registry.mkdir(parents=True)
    with (registry / "fixture.tsv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=cm.REGISTRY_FIELDS, delimiter="\t")
        writer.writeheader()
        writer.writerow(binding())
    regress = tmp_path / "regress-house.out"
    regress.write_text(line() + line("dense", 200), encoding="utf-8")
    out = tmp_path / "coverage.md"
    game = [(0x401000, 40, 0, "simulation"), (0x402000, 160, 0, "simulation")]
    att = SimpleNamespace(funcs={0x401000: 40, 0x402000: 160}, order=[0x401000, 0x402000],
                          range_overrides={}, edge_counts={}, hubs=set())
    inputs = {key: tmp_path / key for key in ("names", "vtables", "interfaces", "libs", "ranges",
              "noncode refs", "map", "shadows", "regress.ps1", "subsystems")}
    data = dict(export=tmp_path, exe_sha="1" * 64, base=0x400000, text_lo=0x400000, text_hi=0x403000,
                regress=regress, inputs=inputs, att=att, owner={va: "Owner" for va in att.order},
                rule={va: "name" for va in att.order}, rows=[dict(area="simulation", subsystem="fixture",
                    shadow_note="-", **dict.fromkeys(cov.STAT, False))], replace_list=[], shadow_list=[],
                keys={}, reg={"tag": "house", "pass": True, "profile": "fixture"}, rrun=("clean", 0),
                warnings=[], game=game, fun_row={}, lib_b={}, lib_n={})
    monkeypatch.setattr(cov, "REPO", tmp_path)
    monkeypatch.setattr(cov, "load_report_data", lambda *args: data)
    monkeypatch.setattr(cov, "text_sha256", lambda path: "0" * 64)
    monkeypatch.setattr(sys, "argv", ["coverage.py", "--out", str(out)])
    cov.main()
    report = out.read_text(encoding="utf-8")
    area = next(s for s in report.splitlines() if s.startswith("| simulation | 2 |"))
    assert area.endswith("| 20.0% (50.0%) | 20.0% (50.0%) | 20.0% (50.0%) | 20.0% (50.0%) |")
    subsystem = next(s for s in report.splitlines() if s.startswith("| simulation | fixture |"))
    assert subsystem.endswith("| 20.0% (50.0%) | 20.0% (50.0%) | 20.0% (50.0%) | 20.0% (50.0%) |")
    assert "| - | - | - | - | - | 20.0%" in subsystem  # existing owner flags remain false
    assert "| **matched** |" in report


def test_oracle_effects_and_class_precedence():
    iat = {1: "KERNEL32.DLL!HeapAlloc", 2: "kernel32.dll!CreateFileA", 3: "user32.dll!MessageBoxA"}
    assert not oc.instruction_flags("CALL dword ptr [0x1]", iat)
    assert oc.instruction_flags("CALL dword ptr [0x2]", iat) == {"os:kernel32.dll-file"}
    assert oc.instruction_flags("CALL dword ptr [0x3]", iat) == {"os:user32.dll"}
    assert oc.classify(oc.instruction_flags("CALL EAX", iat)) == "vtable"
    assert oc.classify(oc.instruction_flags("CALL dword ptr [ECX + 0x4]", iat)) == "vtable"
    assert oc.classify(oc.instruction_flags("MOV EAX,dword ptr [0x00b07000]", iat)) == "globals"
    assert oc.instruction_flags("MOV dword ptr [0x00b07000],EAX", iat) == {"gread", "gwrite"}
    assert not oc.instruction_flags("MOV EAX,dword ptr [0x00bc0000]", iat)
    assert oc.classify(set()) == "closed"
    assert oc.classify({"gread", "indirect", "os:winmm.dll"}) == "os"


def test_scan_noncontiguous_bodies_and_transitive_library_cycles(tmp_path):
    (tmp_path / "function-details.tsv").write_text(
        "address\tranges\n00401000\t00401000-00401005;00401020-00401025;\n"
        "00402000\t00402000-00402005\n00403000\t00403000-00403005\n", encoding="utf-8")
    (tmp_path / "instructions.asm").write_text(
        "; export\n00401000\t90\tNOP\n00401020\t90\tMOV EAX,dword ptr [0x00b07000]\n"
        "00403000\t90\tCALL dword ptr [0x00a00000]\n"
        "00403010\t90\tCALL EAX\n", encoding="utf-8")  # out of function must be ignored
    (tmp_path / "calls.tsv").write_text(
        "caller\ttarget\n00401000\t00402000\n00402000\t00403000\n00403000\t00402000\n",
        encoding="utf-8")
    flags = oc.scan(tmp_path, {0xA00000: "user32.dll!MessageBoxA"})
    assert flags[0x401000] == {"gread", "os:user32.dll"}
    assert oc.classify(flags[0x402000]) == "os"
    assert "indirect" not in flags[0x403000]


def test_pe32_import_reader_named_ordinal_and_missing_original_thunk(tmp_path):
    data = bytearray(2048)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3c, 0x80)
    data[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<H", data, 0x86, 1)
    struct.pack_into("<H", data, 0x94, 224)
    opt = 0x98
    struct.pack_into("<H", data, opt, 0x10b)
    struct.pack_into("<I", data, opt + 28, 0x400000)
    struct.pack_into("<I", data, opt + 60, 0x200)
    struct.pack_into("<I", data, opt + 104, 0x1000)
    struct.pack_into("<IIII", data, opt + 224 + 8, 0x600, 0x1000, 0x600, 0x200)
    struct.pack_into("<IIIII", data, 0x200, 0, 0, 0, 0x1100, 0x1120)
    data[0x300:0x30d] = b"KERNEL32.dll\0"
    struct.pack_into("<III", data, 0x320, 0x1140, 0x80000005, 0)
    data[0x342:0x34c] = b"HeapAlloc\0"
    exe = tmp_path / "fixture.exe"
    exe.write_bytes(data)
    assert oc.read_iat(exe) == {0x401120: "kernel32.dll!HeapAlloc", 0x401124: "kernel32.dll!ordinal-5"}


def test_contract_and_spec_citations_join_to_ledger(tmp_path, monkeypatch):
    assert ledger.read_contracts(tmp_path / "missing.tsv") == {}
    path = tmp_path / "functions.tsv"
    path.write_text("address\tsize\tname\tconvention\tret\n00401000\t40\tFixture\tthiscall\teax\n", encoding="utf-8")
    contracts = ledger.read_contracts(path)
    specs = tmp_path / "docs/specs"
    specs.mkdir(parents=True)
    (specs / "fixture.md").write_text(
        "# Owner\n## Entry table\n| 00401000 | Fixture |\n"
        "### Returns\nInline `0x00401000` and `0x401000`; unrelated 00402000.\n"
        "```text\n# not a section\n```\nMore FUN_00401000 and sub_00401000.\n", encoding="utf-8")
    excluded = specs / "questions/answered"
    excluded.mkdir(parents=True)
    (excluded / "old.md").write_text("00401000", encoding="utf-8")
    citations = ledger.spec_citations(specs, {0x401000})
    assert len(citations[0x401000]) == 3
    assert citations[0x401000][0]["section"] == "Owner > Entry table"
    assert citations[0x401000][1]["section"] == "Owner > Entry table > Returns"
    registry = tmp_path / "engine/plugin/compare"
    registry.mkdir(parents=True)
    monkeypatch.setattr(ledger, "ROOT", tmp_path)
    data = {"game": [(0x401000, 40, 0, "simulation")], "att": SimpleNamespace(name=lambda va: "Fixture"),
            "owner": {0x401000: "Owner"}, "rule": {0x401000: "name"},
            "rows": [dict.fromkeys(ledger.cov.STAT, True)], "fun_row": {0x401000: "fixture"}}
    state, _ = evaluate(tmp_path, line(), mode="replace")
    row = binding("replace") | {"file": str(registry / "fixture.tsv")}
    rows = list(ledger.ledger_rows(data, {0x401000: state}, {0x401000: row}, {},
                                  {0x401000: {"gread"}}, contracts, citations,
                                  {0x401000: function_boundary.Boundary(direct_game={0x402000})}))
    assert rows[0]["contract_convention"] == "thiscall"
    assert rows[0]["oracle_class"] == "globals"
    assert rows[0]["boundary_class"] == "direct" and rows[0]["direct_game_callees"] == 1
    assert rows[0]["binding_mode"] == "replace" and rows[0]["replaced-fn"] == 1
    assert json.loads(rows[0]["spec_citations"])[0]["file"] == "docs/specs/fixture.md"
    assert all(rows[0][key] == 1 for key in ledger.cov.STAT)

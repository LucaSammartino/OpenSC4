"""Own-body boundary requirements, independent of transitive oracle reach."""
import json
from pathlib import Path
import struct

import pytest

from tools import function_boundary as fb
from tools import oracle_class
from tools.tests.test_ledger import ledger

GAME = {0x401000, 0x402000}
FUNCTIONS = GAME | {0x403000}
IAT = {0xA80000: "kernel32.dll!HeapAlloc", 0xA80004: "user32.dll!MessageBoxA"}
SECTIONS = [(0xA80000, 0xB00000), (0xB07000, 0xBC0000)]


def body(*instructions):
    result = fb.Boundary()
    for ins in instructions:
        result.instruction(ins, GAME, FUNCTIONS, IAT, SECTIONS)
    return result


@pytest.mark.parametrize("ins,reads,writes", [
    ("MOV EAX,dword ptr [0x00b07000]", True, False),
    ("MOV dword ptr [0x00b07000],EAX", False, True),
    ("CMP dword ptr [0x00a80008],0x0", True, False),
    ("LOCK ADD dword ptr [0x00b07000],0x1", True, True),
    ("FSTP qword ptr [0x00b07000]", False, True),
    ("FLD qword ptr [0x00a80008]", True, False),
    ("SETNZ byte ptr [0x00b07000]", False, True),
    ("XCHG EAX,dword ptr [0x00b07000]", True, True),
    ("PUSH dword ptr [0x00b07000]", True, False),
    ("POP dword ptr [0x00b07000]", False, True),
    ("MOV EAX,dword ptr [ECX*0x4 + 0x00b07000]", True, False),
    ("MOV EAX,dword ptr [0x00bfffff]", False, False),
    ("MOV EAX,dword ptr [0x00bc0000]", False, False),
    ("MOV EAX,dword ptr [0x00b06fff]", False, False),
    ("MOV EAX,dword ptr [0x00b00000]", False, False),
    ("LEA EAX,[0x00b07000]", False, False),
    ("MOV EAX,0x00b07000", False, False),
    ("MOV EAX,dword ptr FS:[0x00b07000]", False, False),
])
def test_absolute_global_access_semantics(ins, reads, writes):
    result = body(ins)
    assert (result.global_reads, result.global_writes) == (reads, writes)


@pytest.mark.parametrize("instructions,expected", [
    (("ADD EAX,0x1",), "leaf"),
    (("MOV EAX,[0x00b07000]",), "globals"),
    (("MOV EAX,[0x00b07000]", "CALL 0x00402000"), "direct"),
    (("CALL 0x00402000", "CALL EAX"), "indirect"),
    (("CALL EAX", "CALL dword ptr [0x00a80000]"), "import"),
])
def test_boundary_precedence_in_synthetic_bodies(instructions, expected):
    assert body(*instructions).columns()["boundary_class"] == expected


def test_callees_are_distinct_but_imports_and_indirects_count_sites():
    columns = body("CALL 0x00402000", "CALL 0x00402000", "CALL 0x00403000",
                   "CALL 0x00404000", "CALL dword ptr [0x00a80000]",
                   "CALL dword ptr [0x00a80000]", "CALL dword ptr [0x00a80004]",
                   "CALL EAX", "CALL EAX").columns()
    assert columns["direct_game_callees"] == columns["direct_library_callees"] == 1
    assert columns["direct_other_callees"] == 1
    assert json.loads(columns["direct_game_targets"]) == ["00402000"]
    assert json.loads(columns["direct_library_targets"]) == ["00403000"]
    assert columns["import_calls"] == 3
    assert json.loads(columns["import_names"]) == sorted(IAT.values())
    assert columns["indirect_calls"] == 2 and columns["indirect_slots"] == "[]"
    # Even allocator imports ignored by the reference OS heuristic need answering.
    assert not oracle_class.instruction_flags("CALL dword ptr [0x00a80000]", IAT)
    assert body("CALL dword ptr [0x00a80000]").columns()["boundary_class"] == "import"


@pytest.mark.parametrize("ins,slots", [
    ("CALL dword ptr [EAX + 0x1c]", [28]),
    ("CALL dword ptr [EDX + 28]", [28]),
    ("CALL dword ptr [ECX]", [0]),
    ("CALL EDX", []),
    ("CALL dword ptr [0x00b07000]", []),
    ("CALL dword ptr [ESP + 0x4]", []),
    ("CALL dword ptr [EBP + 0x4]", []),
    ("CALL dword ptr [EAX + 0x3]", []),
    ("CALL dword ptr [EAX - 0x4]", []),
    ("CALL dword ptr [EAX + ECX*0x4 + 0x8]", []),
])
def test_indirect_site_and_candidate_slot_patterns(ins, slots):
    columns = body(ins).columns()
    assert columns["indirect_calls"] == 1
    assert json.loads(columns["indirect_slots"]) == slots
    assert columns["boundary_class"] == "indirect"


def test_own_body_ignores_gap_and_callee_effects_even_for_resolved_indirects(tmp_path):
    (tmp_path / "function-details.tsv").write_text(
        "address\tranges\n00401000\t00401000-00401005;00401020-00401025;\n"
        "00402000\t00402000-00402005;\n00403000\t00403000-00403005;\n", encoding="utf-8")
    (tmp_path / "instructions.asm").write_text(
        "; export\n00401000\t90\tCALL 0x00402000\n"
        "00401010\t90\tCALL dword ptr [0x00a80004]\n"  # gap: no owner
        "00401025\t90\tMOV EAX,[0x00b07000]\n"  # inclusive final byte
        "00402000\t90\tCALL dword ptr [0x00a80000]\n"
        "00403000\t90\tCALL EAX\n", encoding="utf-8")
    (tmp_path / "calls.tsv").write_text(
        "caller\tcallsite\ttarget\n00401000\t00401000\t00402000\n"
        "00403000\t00403000\t00402000\n", encoding="utf-8")
    result = fb.scan(tmp_path, GAME, IAT, SECTIONS)
    assert result[0x401000].direct_game == {0x402000}
    assert result[0x401000].import_calls == 0
    assert result[0x401000].columns()["boundary_class"] == "direct"
    assert result[0x401000].global_reads
    assert result[0x402000].columns()["boundary_class"] == "import"
    assert result[0x403000].direct_game == set()
    assert result[0x403000].indirect_calls == 1


def test_shared_and_repeated_body_ranges_count_each_site_once_per_function(tmp_path):
    (tmp_path / "function-details.tsv").write_text(
        "address\tranges\n00401000\t00401000-00401010;00401000-00401010;\n"
        "00401005\t00401005-00401006;\n00402000\t00402000-00402005;\n", encoding="utf-8")
    (tmp_path / "instructions.asm").write_text(
        "00401006\t90\tCALL EAX\n00401008\t90\tCALL ECX\n", encoding="utf-8")
    result = fb.scan(tmp_path, GAME, IAT, SECTIONS)
    assert result[0x401000].indirect_calls == 2
    assert result[0x401005].indirect_calls == 1
    assert result[0x402000].columns()["boundary_class"] == "leaf"


def test_pe_section_bounds_use_virtual_size_and_exact_section_names(tmp_path):
    data = bytearray(1024)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3c, 0x80)
    data[0x80:0x84] = b"PE\0\0"
    struct.pack_into("<H", data, 0x86, 4)
    struct.pack_into("<H", data, 0x94, 0xe0)
    struct.pack_into("<H", data, 0x98, 0x10b)
    struct.pack_into("<I", data, 0x98 + 28, 0x400000)
    for i, name in enumerate((b".text", b".rdata", b".data", b".data1")):
        pos = 0x178 + i * 40
        data[pos:pos + len(name)] = name
        struct.pack_into("<III", data, pos + 8, 0x123, (i + 1) * 0x1000, 0x200)
    exe = tmp_path / "fixture.exe"
    exe.write_bytes(data)
    assert fb.data_sections(exe) == [(0x402000, 0x402123), (0x403000, 0x403123)]


def summary_row(va, size, owner, instructions=(), **changes):
    return dict(address=f"{va:08x}", bytes=size, owner=owner, oracle_class="closed",
                implemented=1, specified=0, verified=0, replaced=0, bound=0, matched=0,
                **{"replaced-fn": 0}, contract_present=0, spec_citations="[]",
                **body(*instructions).columns()) | changes


def test_answering_cost_unions_over_implemented_code_and_all_direct_callees():
    rows = [summary_row(0x401000, 10, "A", ("CALL 0x00402000", "CALL 0x00402000")),
            summary_row(0x402000, 20, "A"),
            summary_row(0x405000, 30, "A", ("CALL 0x00403000",)),  # library not implemented
            summary_row(0x406000, 40, "A", ("CALL 0x00404000",)),  # unknown not implemented
            summary_row(0x407000, 50, "A", ("CALL dword ptr [0x00a80000]", "CALL dword ptr [EAX + 0x8]")),
            summary_row(0x408000, 60, "A", ("CALL dword ptr [0x00a80000]", "CALL dword ptr [ECX + 0x8]")),
            summary_row(0x409000, 70, "B", ("CALL dword ptr [0x00a80004]", "CALL dword ptr [EAX + 0xc]"), implemented=0)]
    assert ledger.answering_cost(rows) == dict(direct=3, game=1, library=1, other=1,
                                              imports=1, slots=1, ready_bytes=140,
                                              ready_nonempty_bytes=10)
    rows[1]["implemented"] = 0  # an unimplemented game callee also vetoes readiness
    assert ledger.answering_cost(rows)["ready_bytes"] == 110


def test_summary_splits_use_implemented_bytes_but_owner_rank_uses_unmatched_bytes():
    rows = [summary_row(0x401000, 100, "A", oracle_class="globals"),
            summary_row(0x402000, 200, "A", ("CALL dword ptr [0x00a80000]",), matched=1, oracle_class="os"),
            summary_row(0x405000, 150, "B", ("CALL EAX",), oracle_class="vtable"),
            summary_row(0x406000, 1000, "C", implemented=0, oracle_class="os")]
    text = ledger.summary(rows, Path("house.out"), [], 20, 200, [])
    assert "| os | 2 | 1,200 | 1 | 200 | 44.4% |" in text
    assert "| import | 1 | 200 | 1 | 200 | 44.4% |" in text
    owner_lines = [line for line in text.splitlines() if line.startswith(("| A |", "| B |", "| C |"))]
    assert owner_lines == ["| B | 150 | 150 | 0 | 0 | 150 | 0 | 0 | 0 | 0 | 150 | 0 |",
                           "| A | 100 | 300 | 0 | 100 | 0 | 200 | 100 | 0 | 0 | 0 | 200 |"]


def test_pilot_picks_span_classes_limit_owners_and_require_positive_function_reach():
    rows = []
    for cls in fb.CLASSES:
        for rank in range(7):
            rows.append(summary_row(0x401000 + len(rows), 100 + rank, f"{cls}{rank}",
                                    boundary_class=cls, natural_calls=1))
    rows += [summary_row(0x500000, 10000, "Unreached"),
             summary_row(0x500001, 10000, "Matched", natural_calls=1, matched=1),
             summary_row(0x500002, 10000, "Unimplemented", natural_calls=1, implemented=0),
             summary_row(0x500003, 200, "Shadow", shadow_calls=1)]
    picks = ledger.pilot_picks(rows)
    assert len(picks) == 25
    assert all(sum(p["boundary"] == cls for p in picks) == 5 for cls in fb.CLASSES)
    assert {p["owner"] for p in picks}.isdisjoint({"Unreached", "Matched", "Unimplemented"})
    assert picks[0] == dict(boundary="leaf", owner="Shadow", functions=1, bytes=200, example="00500003")
    assert picks[1]["owner"] == "leaf6"
    text = ledger.summary(rows, Path("house.out"), [], 20, 200, [])
    assert "| leaf | Shadow | 1 | 200 | 00500003 |" in text
    assert "Shortfall" not in text
    assert "Shortfall leaf: 0/5" in ledger.summary([], Path("house.out"), [], 20, 200, [])

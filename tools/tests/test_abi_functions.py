"""Small byte fixtures for the contract extractor's failure-prone ABI cases."""
import csv
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import abi_functions as abi
from abi_returns import ret_label


def fixture(lines):
    insns = [abi.Insn.parse(line) for line in lines.strip().splitlines()]
    f = abi.Function(insns[0].address, 'fixture', [(insns[0].address, insns[-1].end - 1)])
    instructions = {i.address: i for i in insns}
    abi.build_graph(f, instructions, {f.address: f})
    return f, instructions


def test_ret_immediate_comes_from_bytes_and_ranges_are_inclusive():
    ins = abi.Insn.parse('00001000\tc20800\tRET 0x4')
    assert ins.pop == 8
    assert abi.Function(0x1000, 'x', [(0x1000, 0x1002), (0x2000, 0x2001)]).size == 5


def test_stack_frame_and_register_alias():
    f, _ = fixture('''00001000\t55\tPUSH EBP
00001001\t8bec\tMOV EBP,ESP
00001003\t83ec10\tSUB ESP,0x10
00001006\t8d4508\tLEA EAX,[EBP + 0x8]
00001009\t8b4808\tMOV ECX,dword ptr [EAX + 0x8]
0000100c\tc9\tLEAVE
0000100d\tc3\tRET''')
    abi.stack_and_registers(f, {f.address: f})
    assert f.body_args == 3
    assert f.live == set()


def test_esp_tracking_and_nonargument_register_save():
    f, _ = fixture('''00001000\t56\tPUSH ESI
00001001\t83ec10\tSUB ESP,0x10
00001004\t8b742420\tMOV ESI,dword ptr [ESP + 0x20]
00001008\t83c410\tADD ESP,0x10
0000100b\t5e\tPOP ESI
0000100c\tc3\tRET''')
    abi.stack_and_registers(f, {f.address: f})
    assert f.body_args == 3
    assert f.live == set()


def test_partial_register_write_does_not_invent_fastcall():
    f, _ = fixture('''00001000\t8a542404\tMOV DL,byte ptr [ESP + 0x4]
00001004\t84d2\tTEST DL,DL
00001006\t8b01\tMOV EAX,dword ptr [ECX]
00001008\tc20400\tRET 0x4''')
    abi.stack_and_registers(f, {f.address: f})
    assert f.live == {'ecx'}
    assert abi.convention(f.live, f.pops, f.body_args) == 'thiscall'


def test_conditional_definition_keeps_entry_live_on_other_path():
    f, _ = fixture('''00001000\t7402\tJZ 0x1004
00001002\t31c9\tXOR ECX,ECX
00001004\t8b01\tMOV EAX,dword ptr [ECX]
00001006\tc3\tRET''')
    abi.stack_and_registers(f, {f.address: f})
    assert f.live == {'ecx'}


def test_tail_pop_and_register_forwarding():
    f, instructions = fixture('00001000\te9fb0f0000\tJMP 0x2000')
    target = abi.Function(0x2000, 'target', [], pops={12}, live={'ecx'})
    funcs = {0x1000: f, 0x2000: target}
    abi.build_graph(f, instructions, funcs)
    abi.propagate_pops(funcs)
    abi.stack_and_registers(f, funcs)
    assert f.pops == {12}
    assert f.live == {'ecx'}


def test_inconsistent_returns_are_not_collapsed():
    f, _ = fixture('''00001000\t7403\tJZ 0x1005
00001002\tc20400\tRET 0x4
00001005\tc20800\tRET 0x8''')
    assert f.pops == {4, 8}


def test_switch_table_displacement_is_not_its_scale():
    class Image:
        def word(self, address):
            return {0x3000: 0x1007}.get(address)
    f, ins = fixture('''00001000\tff248500300000\tJMP dword ptr [EAX*0x4 + 0x3000]
00001007\t8b01\tMOV EAX,dword ptr [ECX]
00001009\tc20800\tRET 0x8''')
    abi.build_graph(f, ins, {f.address: f}, Image())
    abi.stack_and_registers(f, {f.address: f})
    assert f.live == {'eax', 'ecx'}


def test_batched_cleanup_is_attributed_to_each_call():
    f, ins = fixture('''00001000\t6a01\tPUSH 0x1
00001002\te8f90f0000\tCALL 0x2000
00001007\t6a02\tPUSH 0x2
00001009\te8f21f0000\tCALL 0x3000
0000100e\t83c408\tADD ESP,0x8
00001011\tc3\tRET''')
    for site in (0x1002, 0x1009):
        assert abi.argument_setup(ins[site], ins, sorted(ins), f) == 1
        assert abi.caller_cleanup(ins[site], ins, grouped=True)[0] == 2


def test_deferred_nonvolatile_save_is_not_an_outgoing_argument():
    f, ins = fixture('''00001000\t56\tPUSH ESI
00001001\t8bf1\tMOV ESI,ECX
00001003\t57\tPUSH EDI
00001004\t8d7e04\tLEA EDI,[ESI + 0x4]
00001007\t57\tPUSH EDI
00001008\te8f30f0000\tCALL 0x2000
0000100d\t83c404\tADD ESP,0x4
00001010\t5f\tPOP EDI
00001011\t5e\tPOP ESI
00001012\tc3\tRET''')
    assert abi.argument_setup(ins[0x1008], ins, sorted(ins), f) == 1


def test_stack_arguments_copied_by_string_instructions():
    f, _ = fixture('''00001000\t55\tPUSH EBP
00001001\t8bec\tMOV EBP,ESP
00001003\t56\tPUSH ESI
00001004\t57\tPUSH EDI
00001005\t8d7508\tLEA ESI,[EBP + 0x8]
00001008\t8d7df0\tLEA EDI,[EBP + -0x10]
0000100b\ta5\tMOVSD ES:EDI,ESI
0000100c\tb903000000\tMOV ECX,0x3
00001011\tf3a5\tREP MOVSD ES:EDI,ESI
00001013\t5f\tPOP EDI
00001014\t5e\tPOP ESI
00001015\tc9\tLEAVE
00001016\tc3\tRET''')
    abi.stack_and_registers(f, {f.address: f})
    assert f.body_args == 4
    assert f.live == set()


def test_constructor_call_preserves_reserved_by_value_argument_block():
    f, ins = fixture('''00001000\t83ec10\tSUB ESP,0x10
00001003\t90\tNOP
00001004\t83ec0c\tSUB ESP,0xc
00001007\t54\tPUSH ESP
00001008\te8f30f0000\tCALL 0x2000
0000100d\t6a01\tPUSH 0x1
0000100f\te8ec1f0000\tCALL 0x3000
00001014\t83c410\tADD ESP,0x10
00001017\t83c410\tADD ESP,0x10
0000101a\tc3\tRET''')
    ctor = abi.Function(0x2000, 'constructor', [], pops={4})
    assert abi.argument_setup(ins[0x100f], ins, sorted(ins), f, {0x2000: ctor}) == 4


def test_prologue_allocation_is_not_an_argument_of_a_zero_arg_call():
    f, ins = fixture('''00001000\t55\tPUSH EBP
00001001\t8bec\tMOV EBP,ESP
00001003\t51\tPUSH ECX
00001004\t51\tPUSH ECX
00001005\t53\tPUSH EBX
00001006\t8bd9\tMOV EBX,ECX
00001008\te8f30f0000\tCALL 0x2000
0000100d\t6a01\tPUSH 0x1
0000100f\te8ec1f0000\tCALL 0x3000
00001014\t59\tPOP ECX
00001015\t5b\tPOP EBX
00001016\tc9\tLEAVE
00001017\tc3\tRET''')
    assert abi.argument_setup(ins[0x1008], ins, sorted(ins), f) == 0


def test_caller_cleanup_equivalent_pops_and_saved_registers():
    _, ins = fixture('''00001000\te8fb0f0000\tCALL 0x2000
00001005\t59\tPOP ECX
00001006\t5a\tPOP EDX
00001007\t5e\tPOP ESI
00001008\tc3\tRET''')
    assert abi.caller_cleanup(ins[0x1000], ins) == (2, ['00001005', '00001006'])


def test_cleanup_does_not_cross_another_call_and_supports_lea():
    _, ins = fixture('''00001000\te8fb0f0000\tCALL 0x2000
00001005\te8f60f0000\tCALL 0x2000
0000100a\t8d642408\tLEA ESP,[ESP + 0x8]
0000100e\tc3\tRET''')
    assert abi.caller_cleanup(ins[0x1000], ins)[0] == 0
    assert abi.caller_cleanup(ins[0x1005], ins)[0] == 2


def test_x87_store_pop_does_not_return_st0():
    f, _ = fixture('''00001000\td9442404\tFLD dword ptr [ESP + 0x4]
00001004\td95c2404\tFSTP dword ptr [ESP + 0x4]
00001008\tc3\tRET''')
    abi.returns(f, {f.address: f}, {})
    assert 'x87' not in f.kinds
    f, _ = fixture('''00001000\td9442404\tFLD dword ptr [ESP + 0x4]
00001004\tc3\tRET''')
    abi.returns(f, {f.address: f}, {})
    assert ret_label(f.kinds, f.pops) == ('x87', 'high')


def test_end_to_end_cdecl_uses_max_of_both_sources(tmp_path, monkeypatch):
    monkeypatch.setattr(abi, 'ROOT', tmp_path)
    (tmp_path / 'function-details.tsv').write_text(
        'address\tname\tprototype\tcalling_convention\tranges\n'
        '00001000\tcaller\t\tunknown\t00001000-00001008;\n'
        '00002000\tcallee\t\tunknown\t00002000-00002004;\n', encoding='utf-8')
    (tmp_path / 'instructions.asm').write_text(
        '00001000\te8fb0f0000\tCALL 0x2000\n'
        '00001005\t83c408\tADD ESP,0x8\n00001008\tc3\tRET\n'
        '00002000\t8b442404\tMOV EAX,dword ptr [ESP + 0x4]\n00002004\tc3\tRET\n', encoding='utf-8')
    (tmp_path / 'calls.tsv').write_text('caller\tcallsite\ttarget\ttype\n'
        '00001000\t00001000\t00002000\tUNCONDITIONAL_CALL\n', encoding='utf-8')
    vt = tmp_path / 'vt.tsv'
    vt.write_text('target\tcallee_pop\tret\tret_confidence\n', encoding='utf-8')
    out, evidence = tmp_path / 'out.tsv', tmp_path / 'evidence.json'
    abi.extract(tmp_path, None, vt, out, evidence)
    with out.open(encoding='utf-8') as fh:
        rows = list(csv.DictReader(fh, delimiter='\t'))
    assert list(rows[0]) == abi.COLUMNS
    callee = rows[1]
    assert (callee['convention'], callee['stack_arg_dwords'], callee['callers']) == ('cdecl', '2', '1')
    assert 'body_stack_dwords=1;caller_cleanup_dwords=2' in callee['notes']
    assert 'argument-sources-disagree' in callee['notes']
    assert json.loads(evidence.read_text())['rows'] == 2

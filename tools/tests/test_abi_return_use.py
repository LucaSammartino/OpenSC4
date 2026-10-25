"""Return observations distinguish real results from volatile scratch values."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import abi_functions as abi
from abi_return_use import analyse, caller_read, exit_sources
from test_abi_functions import fixture


def run(*bodies):
    fs, instructions = {}, {}
    for body in bodies:
        f, ins = fixture(body)
        fs[f.address] = f
        instructions.update(ins)
    for f in fs.values():
        abi.build_graph(f, instructions, fs)
        abi.returns(f, fs, {})
    sites = {}
    for f in fs.values():
        for i in f.insns.values():
            if i.mnemonic == 'call':
                sites.setdefault(i.target, []).append((f.address, i.address))
    return fs, analyse(fs, sites, {}, {}, abi.access, abi.call_x87_used)


def test_caller_read_through_cleanup_and_conditional_path():
    fs, ev = run('''00001000\te8fb0f0000\tCALL 0x2000
00001005\t83c404\tADD ESP,0x4
00001008\t7402\tJZ 0x100c
0000100a\t85c0\tTEST EAX,EAX
0000100c\tc3\tRET''', '''00002000\t8b01\tMOV EAX,dword ptr [ECX]
00002002\tc3\tRET''')
    assert ev[0x2000]['ret'] == 'eax'
    assert ev[0x2000]['caller_eax_reads'] == 1
    assert ev[0x2000]['callers_checked'] == 1


def test_overwrite_kills_caller_evidence_but_discard_is_not_void():
    _, ev = run('''00001000\te8fb0f0000\tCALL 0x2000
00001005\t31c0\tXOR EAX,EAX
00001007\t50\tPUSH EAX
00001008\tc3\tRET''', '''00002000\t8b01\tMOV EAX,dword ptr [ECX]
00002002\tc3\tRET''')
    assert ev[0x2000]['caller_eax_reads'] == 0
    assert ev[0x2000]['ret'] == 'eax'


def test_push_result_and_partial_write_are_caller_reads():
    fs, _ = run('''00001000\te8fb0f0000\tCALL 0x2000
00001005\tb001\tMOV AL,0x1
00001007\t50\tPUSH EAX
00001008\tc3\tRET''', '00002000\tc3\tRET')
    caller = fs[0x1000]
    assert caller_read(caller.insns[0x1000], caller, 'eax', abi.access, fs)['reads'] == [0x1007]


def test_read_of_overwritten_low_byte_is_not_original_call_result():
    fs, _ = run('''00001000\te8fb0f0000\tCALL 0x2000
00001005\tb001\tMOV AL,0x1
00001007\t84c0\tTEST AL,AL
00001009\tc3\tRET''', '00002000\tc3\tRET')
    f = fs[0x1000]
    assert caller_read(f.insns[0x1000], f, 'eax', abi.access, fs)['reads'] == []


def test_scratch_and_preserved_eax_have_positive_void_evidence():
    _, ev = run('''00002000\t8b01\tMOV EAX,dword ptr [ECX]
00002002\t8902\tMOV dword ptr [EDX],EAX
00002004\tc3\tRET''', '''00003000\tc70101000000\tMOV dword ptr [ECX],0x1
00003006\tc3\tRET''')
    assert ev[0x2000]['ret'] == ev[0x3000]['ret'] == 'void'
    assert 'body-scratch' in ev[0x2000]['kind']
    assert 'body-preserved' in ev[0x3000]['kind']


def test_two_dword_return_requires_caller_edx_use():
    _, ev = run('''00001000\te8fb0f0000\tCALL 0x2000
00001005\t8901\tMOV dword ptr [ECX],EAX
00001007\t895104\tMOV dword ptr [ECX + 0x4],EDX
0000100a\tc3\tRET''', '''00002000\t8b01\tMOV EAX,dword ptr [ECX]
00002002\t8b5104\tMOV EDX,dword ptr [ECX + 0x4]
00002005\tc3\tRET''')
    assert ev[0x2000]['ret'] == 'edx:eax'


def test_forwarding_resolves_transitively_and_mixed_paths_stay_unknown():
    _, ev = run('''00001000\te8fb0f0000\tCALL 0x2000
00001005\tc3\tRET''', '''00002000\te9fb0f0000\tJMP 0x3000''',
                '''00003000\tb801000000\tMOV EAX,0x1
00003005\tc3\tRET''', '''00004000\t7405\tJZ 0x4007
00004002\tb801000000\tMOV EAX,0x1
00004007\tc3\tRET''')
    assert ev[0x1000]['ret'] == ev[0x2000]['ret'] == 'eax'
    assert ev[0x4000]['ret'] == 'unknown'


def test_unknown_indirect_exit_does_not_become_void():
    _, ev = run('00001000\tff20\tJMP dword ptr [EAX]')
    assert ev[0x1000]['ret'] == 'unknown'


def test_empty_leaf_is_void_and_fpu_store_pop_is_not_a_float_return():
    _, ev = run('00001000\tc3\tRET', '''00002000\td901\tFLD dword ptr [ECX]
00002002\td91a\tFSTP dword ptr [EDX]
00002004\tc3\tRET''')
    assert ev[0x1000]['ret'] == ev[0x2000]['ret'] == 'void'

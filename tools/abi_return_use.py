"""Analysis-side return-channel evidence. No prototypes or name heuristics.

Caller reads establish a channel, never the converse. A discarded result can
still be returned. Void requires every observed exit to preserve EAX or leave
a value already consumed as scratch, as well as complete, non-reading callers.
"""
from collections import defaultdict, deque

DECIDED = {'eax', 'al', 'ax', 'edx:eax', 'x87', 'void'}


def operand_mask(ins, reg):
    import re
    aliases = {'eax': {'eax': 15, 'ax': 3, 'al': 1, 'ah': 2},
               'edx': {'edx': 15, 'dx': 3, 'dl': 1, 'dh': 2}}[reg]
    mask = 0
    for op in ins.operands:
        for token in re.findall(r'\b[a-z]+\b', op):
            mask |= aliases.get(token, 0)
    return mask or 15


def write_mask(ins, reg):
    if ins.operands:
        dst = ins.operands[0]
        if dst in ('al', 'dl'):
            return 1
        if dst in ('ah', 'dh'):
            return 2
        if dst in ('ax', 'dx'):
            return 3
    return 15


def caller_read(call, f, reg, access, functions, limit=4000):
    """May-read before a full overwrite, following both sides of every branch."""
    todo = [(va, 15) for va in f.edges.get(call.address, ())]
    seen, reads, complete, read_bits = set(), [], True, 0
    while todo:
        va, alive = todo.pop()
        if (va, alive) in seen:
            continue
        seen.add((va, alive))
        if len(seen) > limit:
            complete = False
            break
        ins = f.insns[va]
        rs, ws = access(ins)
        # Entry analysis suppresses register saves. Here PUSH EAX consumes a result.
        if ins.mnemonic == 'push' and ins.operands == (reg,):
            rs.add(reg)
        if ins.mnemonic == 'call' or ins.mnemonic == 'jmp' and ins.target in functions:
            callee = functions.get(ins.target)
            if callee:
                rs |= callee.live
        mask = operand_mask(ins, reg)
        if reg in rs and mask & alive:
            reads.append(va)
            read_bits |= mask & alive
            continue
        if reg in ws:
            alive &= ~write_mask(ins, reg)
            if not alive:
                continue
        if ins.mnemonic == 'call' or ins.pop is not None:
            continue  # ABI clobber, or result forwarded to our caller (resolved separately).
        if not f.edges.get(va) and ins.mnemonic == 'jmp' and ins.target not in functions:
            complete = False
        todo.extend((dst, alive) for dst in f.edges.get(va, ()))
    return {'reads': sorted(set(reads)), 'complete': complete, 'read_mask': read_bits}


def exit_sources(f, reg, access):
    """Last definition on every reachable return path, including tail exits.

    The used bit distinguishes a terminal value from one consumed as scratch
    after its last definition. No arbitrary backwards instruction cutoff.
    """
    reachable, todo = set(), [f.address]
    predecessors = defaultdict(list)
    for va, edges in f.edges.items():
        for dst in edges:
            predecessors[dst].append(va)
    while todo:
        va = todo.pop()
        if va in reachable or va not in f.insns:
            continue
        reachable.add(va)
        todo.extend(f.edges.get(va, ()))
    exits = [va for va in reachable if f.insns[va].pop is not None]
    sources, seen = set(), set()
    todo = [(va, False) for end in exits for va in predecessors[end]]
    if f.address in exits:
        sources.add(('entry', f.address, False))
    for va in reachable:
        ins = f.insns[va]
        if ins.mnemonic == 'jmp' and ins.target in f.tails:
            sources.add(('tail', ins.target, False))
        elif not f.edges[va] and ins.pop is None:
            sources.add(('unknown', va, False))
    while todo:
        va, used = todo.pop()
        if (va, used) in seen:
            continue
        seen.add((va, used))
        ins = f.insns[va]
        if ins.mnemonic == 'call':
            sources.add(('call', ins.target or 0, used))
            continue
        rs, ws = access(ins)
        if reg in ws:
            kind = 'write' if ins.mnemonic != 'pop' else 'scratch'
            sources.add((kind, va, used))
            continue
        if ins.mnemonic == 'push' and ins.operands == (reg,):
            rs.add(reg)
        # A test of the value does not make it scratch (e.g. a conditional
        # getter). Consuming it in a memory effect / another value does.
        used |= reg in rs and ins.mnemonic not in ('test', 'cmp', 'bt')
        if va == f.address or not predecessors[va]:
            sources.add(('entry', va, used))
        todo.extend((p, used) for p in predecessors[va] if p in reachable)
    return sorted(sources)


def analyse(functions, callsites, vtable, direct, access, call_x87_used):
    evidence = {}
    for addr, f in sorted(functions.items()):
        checked = []
        for owner, site in sorted(set(callsites.get(addr, ()))):
            caller = functions.get(owner)
            if not caller or site not in caller.insns:
                continue
            call = caller.insns[site]
            checked.append({'caller': f'{owner:08x}', 'site': f'{site:08x}',
                            'eax': caller_read(call, caller, 'eax', access, functions),
                            'edx': caller_read(call, caller, 'edx', access, functions),
                            'x87': call_x87_used(call, caller)})
        evidence[addr] = {'callers': checked, 'eax_exits': exit_sources(f, 'eax', access),
                          'edx_exits': exit_sources(f, 'edx', access)}

    result = {addr: ('unknown', 'low', 'unresolved') for addr in functions}
    # Reviewed definite channels seed forwarding; ambiguous source labels do not.
    seeds = {}
    for addr, rows in vtable.items():
        channels = {r['ret'] for r in rows if r['ret'] in DECIDED and
                    r['ret_confidence'] in ('high', 'exact')}
        if len(channels) == 1:
            seeds[addr] = (next(iter(channels)), 'high', 'reviewed-vtable')
    for addr, row in direct.items():
        if row['ret'] in DECIDED:
            seeds[addr] = (row['ret'], 'high', 'reviewed-direct')

    reverse = defaultdict(set)
    for addr, ev in evidence.items():
        for kind, dst, _ in ev['eax_exits']:
            if kind in ('call', 'tail'):
                reverse[dst].add(addr)
    todo, queued = deque(sorted(functions)), set(functions)
    while todo:
        addr = todo.popleft()
        queued.discard(addr)
        f, ev = functions[addr], evidence[addr]
        callers, exits = ev['callers'], ev['eax_exits']
        eax = any(c['eax']['reads'] for c in callers)
        edx = any(c['edx']['reads'] for c in callers)
        fp = any(c['x87'] for c in callers)
        complete = all(c['eax']['complete'] and c['edx']['complete'] for c in callers)
        width = {write_mask(f.insns[site], 'eax') for kind, site, _ in exits if kind == 'write'}
        body_channel = 'al' if width == {1} else 'ax' if width == {3} else 'eax'
        channels, reasons = set(), set()
        for kind, site, used in exits:
            if kind in ('call', 'tail'):
                ret = result.get(site, ('unknown',))[0]
                channels.add('void' if used and ret in DECIDED - {'x87'} else ret)
                reasons.add('callee-channel')
            elif kind == 'write':
                channels.add('void' if used else body_channel)
                reasons.add('body-scratch' if used else 'body-last-write')
            elif kind in ('entry', 'scratch'):
                channels.add('void')
                reasons.add('body-preserved' if kind == 'entry' else 'body-scratch')
            else:
                channels.add('unknown')
        if addr in seeds:
            decided = seeds[addr]
        elif fp or f.kinds == {'x87'}:
            decided = ('x87', 'high', 'caller-st0-read' if fp else 'body-x87-exit')
        elif eax:
            decided = ('edx:eax' if edx else body_channel, 'high',
                       'caller-eax-edx-read' if edx else 'caller-eax-read')
        elif edx:
            decided = ('unknown', 'low', 'caller-edx-only')
        elif channels == {'eax'}:
            decided = ('eax', 'high', '+'.join(sorted(reasons)))
        elif channels == {'void'} and complete and exits and f.insns:
            decided = ('void', 'high', 'no-caller-read+' + '+'.join(sorted(reasons)))
        elif channels <= {'eax', 'al', 'ax', 'edx:eax', 'x87'} and len(channels) == 1:
            decided = (next(iter(channels)), 'high', 'callee-channel')
        elif (complete and exits and not edx and
              all(kind == 'call' or kind in ('entry', 'scratch') or
                  kind == 'write' and used for kind, _, used in exits)):
            # Positive void evidence requested by the contract policy: no
            # observed consumer, and *all* exits leave a callee/scratch value.
            # Tail jumps remain forwarders and are deliberately excluded.
            decided = ('void', 'high', 'no-caller-read+body-callee-or-scratch-residue')
        else:
            decided = ('unknown', 'low', 'mixed-or-unresolved-exits')
        if result[addr] != decided:
            result[addr] = decided
            for parent in sorted(reverse[addr] - queued):
                todo.append(parent)
                queued.add(parent)
    for addr, ev in evidence.items():
        ret, confidence, kind = result[addr]
        ev.update(ret=ret, confidence=confidence, kind=kind,
                  callers_checked=len(ev['callers']),
                  caller_eax_reads=sum(bool(c['eax']['reads']) for c in ev['callers']),
                  caller_edx_reads=sum(bool(c['edx']['reads']) for c in ev['callers']),
                  caller_x87_reads=sum(c['x87'] for c in ev['callers']))
    return evidence

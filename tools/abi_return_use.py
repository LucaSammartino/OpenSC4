"""Analysis-side return-channel evidence. No prototypes or name heuristics.

Caller reads establish a channel, never the converse. A discarded result can
still be returned. Void requires every observed exit to preserve EAX or leave
a value already consumed as scratch, as well as complete, non-reading callers.
"""
from collections import defaultdict, deque
from abi_returns import X87_PUSH, X87_POP1, X87_POP2

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


def exit_sources(f, reg, access, preserved=frozenset()):
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
            if ins.target in preserved:
                todo.extend((p, False) for p in predecessors[va])
                if va == f.address:
                    sources.add(('entry', va, False))
            else:
                sources.add(('tail', ins.target, False))
        elif not f.edges[va] and ins.pop is None:
            sources.add(('unknown', va, False))
    while todo:
        va, used = todo.pop()
        if (va, used) in seen:
            continue
        seen.add((va, used))
        ins = f.insns[va]
        if ins.mnemonic == 'call' and ins.target not in preserved:
            sources.add(('call', ins.target or 0, used))
            continue
        rs, ws = access(ins)
        if reg in ws:
            kind = 'write' if ins.mnemonic != 'pop' else 'scratch'
            if ins.mnemonic in ('inc', 'dec', 'add', 'sub'):
                used |= loop_scratch(f, va, reg, access)
            sources.add((kind, va, used))
            continue
        if ins.mnemonic == 'push' and ins.operands == (reg,):
            rs.add(reg)
        used |= reg in rs
        if va == f.address or not predecessors[va]:
            sources.add(('entry', va, used))
        todo.extend((p, used) for p in predecessors[va] if p in reachable)
    return sorted(sources)


def loop_scratch(f, site, reg, access):
    """An updated cursor consumed on a back edge is scratch even on loop exit."""
    todo, seen = [(dst, False) for dst in f.edges[site]], set()
    while todo:
        va, address_use = todo.pop()
        if (va, address_use) in seen:
            continue
        seen.add((va, address_use))
        if va == site:
            if address_use:
                return True
            continue
        ins = f.insns[va]
        if any('[' in op and reg in op for op in ins.operands):
            return True
        _, writes = access(ins)
        if reg in writes or ins.mnemonic == 'call':
            continue
        address_use |= any('[' in op and reg in op for op in ins.operands)
        todo.extend((dst, address_use) for dst in f.edges.get(va, ()))
    return False


def x87_depths(f, floating, call_x87_used):
    """Preserve live caller temporaries across calls; do not credit them as results."""
    todo, seen, calls, exits = [(f.address, 0)], set(), defaultdict(set), set()
    while todo:
        va, depth = todo.pop()
        if (va, depth) in seen or va not in f.insns:
            continue
        seen.add((va, depth))
        ins = f.insns[va]
        m = ins.mnemonic
        if ins.pop is not None:
            exits.add(depth)
        elif m == 'call':
            calls[va].add(depth)
            if ins.target in floating or depth == 0 and call_x87_used(ins, f):
                depth += 1
        elif m in X87_PUSH:
            depth += 1
        elif m in X87_POP1:
            depth -= 1
        elif m in X87_POP2:
            depth -= 2
        elif m == 'emms':
            depth = 0
        if not 0 <= depth <= 8:
            exits.add(-1)  # Keep uncertainty, never silently drop a broken path.
            continue
        todo.extend((dst, depth) for dst in f.edges.get(va, ()))
    return calls, exits


def analyse(functions, callsites, vtable, direct, access, call_x87_used):
    floating = {addr for addr, rows in vtable.items() if any(
        r['ret'] == 'x87' and r['ret_confidence'] in ('high', 'exact') for r in rows)}
    floating |= {addr for addr, row in direct.items() if row['ret'] == 'x87'}
    depths = {addr: x87_depths(f, floating, call_x87_used) for addr, f in functions.items()}
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
                            'x87': depths[owner][0].get(site) == {0} and call_x87_used(call, caller),
                            'x87_depth_before': sorted(depths[owner][0].get(site, ()))})
        evidence[addr] = {'callers': checked, 'eax_exits': exit_sources(f, 'eax', access),
                          'edx_exits': exit_sources(f, 'edx', access),
                          'x87_exit_depths': sorted(depths[addr][1])}

    # Runtime epilogue helpers preserve a caller's return registers. Treating
    # their unchanged values as new results contaminates every void cleanup
    # wrapper that shares the helper. Follow provenance *through* those calls.
    preserves = {}
    for reg in ('eax', 'edx'):
        known = {a for a, ev in evidence.items() if ev[reg + '_exits'] and
                 all(kind == 'entry' for kind, _, _ in ev[reg + '_exits'])}
        changed = True
        while changed:
            added = {a for a, ev in evidence.items() if a not in known and ev[reg + '_exits'] and
                     all(kind == 'entry' or kind in ('call', 'tail') and dst in known
                         for kind, dst, _ in ev[reg + '_exits']) and
                     all(reg not in access(i)[1] and
                         (i.mnemonic != 'call' or i.target in known)
                         for i in functions[a].insns.values())}
            changed = bool(added)
            known |= added
        preserves[reg] = known
        for addr, f in functions.items():
            evidence[addr][reg + '_exits'] = exit_sources(f, reg, access, known)
            evidence[addr]['preserves_' + reg] = addr in known

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
            seeds[addr] = (row['ret'], 'high', row.get('ret_evidence', 'reviewed-direct'))

    # A consumer of wrapper() also consumes the unchanged value produced by
    # its terminal call/tail. Solve demand backwards before deciding void, so
    # a local RET cannot hide a real consumer one or more wrappers away.
    masks, edx_demand, fp_demand = defaultdict(int), set(), set()
    for addr, ev in evidence.items():
        for c in ev['callers']:
            masks[addr] |= c['eax']['read_mask']
            if c['edx']['reads']:
                edx_demand.add(addr)
            if c['x87']:
                fp_demand.add(addr)
        channel = seeds.get(addr, ('unknown',))[0]
        masks[addr] |= {'eax': 15, 'al': 1, 'ax': 3, 'edx:eax': 15}.get(channel, 0)
        if channel == 'edx:eax':
            edx_demand.add(addr)
        if channel == 'x87':
            fp_demand.add(addr)
    todo, queued = deque(sorted(functions)), set(functions)
    forwarded = defaultdict(set)
    while todo:
        addr = todo.popleft()
        queued.discard(addr)
        for kind, dst, used in evidence[addr]['eax_exits']:
            if kind not in ('call', 'tail') or used or dst not in functions:
                continue
            before = masks[dst], dst in edx_demand, dst in fp_demand
            masks[dst] |= masks[addr]
            if addr in edx_demand:
                edx_demand.add(dst)
            # ST0 demand cannot use the GPR provenance graph: a caller may
            # already have produced its floating result before this cleanup.
            if before != (masks[dst], dst in edx_demand, dst in fp_demand):
                forwarded[dst].add(addr)
                if dst not in queued:
                    todo.append(dst)
                    queued.add(dst)

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
        eax, edx, fp = bool(masks[addr]), addr in edx_demand, addr in fp_demand
        complete = all(c['eax']['complete'] and c['edx']['complete'] for c in callers)
        width = {write_mask(f.insns[site], 'eax') for kind, site, _ in exits if kind == 'write'}
        # Full-width zero on one path and SETcc AL on another defines only AL
        # on *every* path. Comparing all of EAX would credit undefined bits.
        body_channel = ('al' if 1 in width and width <= {1, 3, 15} else
                        'ax' if 3 in width and width <= {3, 15} else 'eax')
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
        elif fp or ev['x87_exit_depths'] == [1]:
            decided = ('x87', 'high', 'caller-st0-read' if fp else 'body-x87-exit')
        elif ev['preserves_eax']:
            decided = ('void', 'high', 'body-preserved+caller-value-preexists')
        elif eax:
            read_bits = masks[addr]
            observed = ('al' if read_bits == 1 else 'ax' if read_bits <= 3 else body_channel)
            if 'al' in channels:
                observed = 'al'
            elif 'ax' in channels and observed == 'eax':
                observed = 'ax'
            decided = ('edx:eax' if edx else observed, 'high',
                       'caller-eax-edx-read' if edx else 'caller-eax-read')
        elif edx:
            decided = ('unknown', 'low', 'caller-edx-only')
        elif channels and channels <= {'eax', 'al', 'ax'}:
            decided = ('al' if 'al' in channels else 'ax' if 'ax' in channels else 'eax',
                       'high', '+'.join(sorted(reasons)))
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
                  forwarding_consumers=[f'{a:08x}' for a in sorted(forwarded[addr])],
                  callers_checked=len(ev['callers']),
                  caller_eax_reads=sum(bool(c['eax']['reads']) for c in ev['callers']),
                  caller_edx_reads=sum(bool(c['edx']['reads']) for c in ev['callers']),
                  caller_x87_reads=sum(c['x87'] for c in ev['callers']))
    return evidence

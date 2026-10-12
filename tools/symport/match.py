"""Match Windows SC4 functions to named Mac Rev A Intel functions.

Stage A (anchors): IDF-weighted overlap of rare constants + string references; keep mutual-best
pairs whose score clears a threshold and beats the runner-up by a margin.
Stage B (propagation): repeatedly pair unmatched callees/callers of matched pairs. A candidate's
support is the number of distinct matched pairs (w, m) such that w->w' and m->m' (or callers).
Accept mutual-best candidates with enough support and a plausible size ratio.

Output: re/symport/matches.tsv (win, mac, mac_name, stage, score, support)
"""
import collections
import math
import os
import pickle
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "re", "symport")


def load(w):
    with open(os.path.join(OUT, f"{w}_features.pkl"), "rb") as fh:
        return pickle.load(fh)


def tokens(f):
    return {("c", c) for c in f["consts"]} | {("s", s) for s in f["strings"]}


def size_ok(a, b, lo=0.33, hi=3.0):
    if a["size"] == 0 or b["size"] == 0:
        return True
    r = a["size"] / b["size"]
    return lo <= r <= hi


def stage_a(win, mac):
    wt = {a: tokens(f) for a, f in win.items() if not f["thunk"]}
    mt = {a: tokens(f) for a, f in mac.items() if not f["thunk"]}
    wdf, mdf = collections.Counter(), collections.Counter()
    widx, midx = collections.defaultdict(list), collections.defaultdict(list)
    for a, ts in wt.items():
        for t in ts:
            wdf[t] += 1
            widx[t].append(a)
    for a, ts in mt.items():
        for t in ts:
            mdf[t] += 1
            midx[t].append(a)
    nw, nm = len(wt), len(mt)

    def idf(t):
        return math.log(nw / wdf[t]) + math.log(nm / mdf[t])

    shared = [t for t in wdf if t in mdf and wdf[t] <= 12 and mdf[t] <= 12]
    scores = collections.defaultdict(float)
    for t in shared:
        w_ = idf(t)
        for a in widx[t]:
            for b in midx[t]:
                scores[(a, b)] += w_
    # penalize tokens present on one side only (normalize by union weight)
    best_w, best_m = {}, {}
    for (a, b), s in scores.items():
        ua = sum(idf(t) for t in wt[a] if t in mdf) or 1
        ub = sum(idf(t) for t in mt[b] if t in wdf) or 1
        norm = s / max(ua, ub)
        if not size_ok(win[a], mac[b], 0.2, 5.0):
            norm *= 0.5
        for best, k, v in ((best_w, a, b), (best_m, b, a)):
            cur = best.setdefault(k, [])
            cur.append((norm, s, v))
    out = {}
    for a, lst in best_w.items():
        lst.sort(reverse=True)
        n1, s1, b = lst[0]
        n2 = lst[1][0] if len(lst) > 1 else 0
        mlst = sorted(best_m[b], reverse=True)
        if mlst[0][2] != a:
            continue
        m2 = mlst[1][0] if len(mlst) > 1 else 0
        if s1 >= 12 and n1 >= 0.5 and n1 - max(n2, m2) >= 0.15:
            out[a] = (b, "A", round(n1, 3), 0)
    return out


def stage_b(win, mac, matched, rounds=50):
    rev = {v[0]: k for k, v in matched.items()}
    for r in range(rounds):
        cand = collections.defaultdict(set)
        for a, (b, *_) in list(matched.items()):
            wf, mf = win[a], mac[b]
            wc = [x for x in dict.fromkeys(wf["callees"]) if x not in matched and x in win]
            mc = [x for x in dict.fromkeys(mf["callees"]) if x not in rev and x in mac]
            wr = [x for x in wf["callers"] if x not in matched and x in win]
            mr = [x for x in mf["callers"] if x not in rev and x in mac]
            for ws, ms in ((wc, mc), (wr, mr)):
                if len(ws) * len(ms) > 400:
                    continue
                for x in ws:
                    for y in ms:
                        if size_ok(win[x], mac[y]) and not win[x]["thunk"]:
                            cand[(x, y)].add(a)
        bw, bm = collections.defaultdict(list), collections.defaultdict(list)
        for (x, y), sup in cand.items():
            s = len(sup)
            # tie-break by size similarity and token overlap
            sz = 1 - abs(math.log((win[x]["size"] + 1) / (mac[y]["size"] + 1))) / 5
            tok = len(tokens(win[x]) & tokens(mac[y]))
            key = (s + 0.5 * tok, sz)
            bw[x].append((key, y))
            bm[y].append((key, x))
        added = 0
        for x, lst in bw.items():
            lst.sort(reverse=True)
            (k1, y) = lst[0]
            if len(lst) > 1 and lst[1][0][0] >= k1[0]:
                continue
            ml = sorted(bm[y], reverse=True)
            if ml[0][1] != x or (len(ml) > 1 and ml[1][0][0] >= ml[0][0][0]):
                continue
            sup = k1[0]
            if sup >= 2 or (sup >= 1 and k1[1] > 0.8 and len(lst) == 1 and len(ml) == 1):
                matched[x] = (y, f"B{r}", round(k1[1], 3), sup)
                rev[y] = x
                added += 1
        print(f"  stage B round {r}: +{added} (total {len(matched)})")
        if not added:
            break
    return matched


def win_thunk_targets(win_addrs):
    """ICC/MSVC adjustor thunks: `sub ecx, N` / `add ecx, N` followed by `jmp target`."""
    import capstone
    import pefile
    pe = pefile.PE(os.path.join(ROOT, "bin", "SimCity4.exe"))
    base = pe.OPTIONAL_HEADER.ImageBase
    img = pe.get_memory_mapped_image()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    out = {}
    for a in win_addrs:
        ins = list(md.disasm(img[a - base:a - base + 16], a, count=2))
        if len(ins) == 2 and ins[0].mnemonic in ("sub", "add") and ins[0].op_str.startswith("ecx,") \
                and ins[1].mnemonic == "jmp":
            try:
                out[a] = int(ins[1].op_str, 16)
            except ValueError:
                pass
    return out


PURECALL = 0x005D4A10
VT_PAIRS = {}  # win vtable start -> (mac ZTV name#sub, votes, win len, mac len)
FOLDED = {}  # win addr -> distinct Mac names proposed by different vtables (identical-code folding)


def stage_c(win, mac, matched, vt, thunks):
    rev = {v[0]: k for k, v in matched.items()}
    wvt = {s: [thunks.get(x, x) for x in slots] for s, slots in vt["win"].items()}
    mvt = vt["mac"]
    pos = collections.defaultdict(list)  # (mac_addr, index) -> mac table ids
    for k, (_, slots) in enumerate(mvt):
        for i, (_, a) in enumerate(slots):
            pos[(a, i)].append(k)
    proposals = collections.defaultdict(collections.Counter)
    paired = 0
    for s, slots in wvt.items():
        votes = collections.Counter()
        for i, w in enumerate(slots):
            if w in matched:
                for k in pos.get((matched[w][0], i), ()):
                    votes[k] += 1
        if not votes:
            continue
        (k1, v1), *rest = votes.most_common(2)
        v2 = rest[0][1] if rest else 0
        msl = mvt[k1][1]
        known = sum(1 for i, w in enumerate(slots[:len(msl)]) if w in matched)
        if v1 < 2 and not (v1 == 1 and len(slots) == len(msl) and len(slots) >= 3):
            continue
        if v1 <= v2 or v1 < 0.6 * known:
            continue
        paired += 1
        VT_PAIRS[s] = (mvt[k1][0], v1, len(slots), len(msl))
        for i, w in enumerate(slots[:len(msl)]):
            name, b = msl[i]
            if b and name != "?" and w != PURECALL and w in win:
                proposals[w][b] += 1
    added = 0
    for w, cnt in proposals.items():
        if len(cnt) > 1:
            FOLDED[w] = sorted({mac[b]["name"] for b in cnt if b in mac})
        if w in matched or len(cnt) != 1:
            continue
        b = next(iter(cnt))
        if b in rev or b not in mac:
            continue
        matched[w] = (b, "C", 1.0, cnt[b])
        rev[b] = w
        added += 1
    print(f"  stage C: {paired} vtables paired, +{added} (total {len(matched)})")
    return added


def demangle_ztv(z):
    """__ZTV13cSC4Simulator -> cSC4Simulator; __ZTVN4nSC46cFooE -> nSC4::cFoo (simple cases only)."""
    s = z[5:] if z.startswith("__ZTV") else z
    nested = s.startswith("N")
    if nested:
        s = s[1:]
    parts, i = [], 0
    while i < len(s) and s[i].isdigit():
        j = i
        while j < len(s) and s[j].isdigit():
            j += 1
        n = int(s[i:j])
        parts.append(s[j:j + n])
        i = j + n
        if not nested:
            break
    return "::".join(parts) if parts and i <= len(s) and (not nested or s[i:i + 1] == "E") else z


def stage_d(win, mac, matched, max_gap=3):
    """Ordered callee alignment: matched callees are fixed points; equal-length gaps pair up."""
    rev = {v[0]: k for k, v in matched.items()}
    cand = collections.defaultdict(collections.Counter)
    for a, (b, *_) in list(matched.items()):
        ws = list(dict.fromkeys(x for x in win[a]["callees"] if x in win))
        ms = list(dict.fromkeys(y for y in mac[b]["callees"] if y in mac))
        # LCS over matched callees
        wi = [(i, matched[x][0]) for i, x in enumerate(ws) if x in matched]
        mpos = {y: j for j, y in enumerate(ms)}
        anchors, last = [(-1, -1)], -1
        for i, y in wi:
            j = mpos.get(y)
            if j is not None and j > last:
                anchors.append((i, j))
                last = j
        anchors.append((len(ws), len(ms)))
        for (i0, j0), (i1, j1) in zip(anchors, anchors[1:]):
            gw = [x for x in ws[i0 + 1:i1] if x not in matched]
            gm = [y for y in ms[j0 + 1:j1] if y not in rev]
            if 0 < len(gw) == len(gm) <= max_gap:
                for x, y in zip(gw, gm):
                    if size_ok(win[x], mac[y], 0.4, 2.5) and not win[x]["thunk"]:
                        cand[x][y] += 1
    added = 0
    inv = collections.defaultdict(set)
    for x, c in cand.items():
        for y in c:
            inv[y].add(x)
    for x, c in cand.items():
        if len(c) != 1:
            continue
        y, n = next(iter(c.items()))
        if len(inv[y]) != 1 or y in rev or x in matched:
            continue
        matched[x] = (y, "D", 0.5, n)
        rev[y] = x
        added += 1
    print(f"  stage D: +{added} (total {len(matched)})")
    return added


def main():
    win, mac = load("win"), load("mac")
    with open(os.path.join(OUT, "vtables.pkl"), "rb") as fh:
        vt = pickle.load(fh)
    thunks = win_thunk_targets({x for slots in vt["win"].values() for x in slots})
    matched = stage_a(win, mac)
    print(f"stage A anchors: {len(matched)}")
    while True:
        before = len(matched)
        matched = stage_b(win, mac, matched)
        stage_c(win, mac, matched, vt, thunks)
        stage_d(win, mac, matched)
        if len(matched) == before:
            break
    with open(os.path.join(OUT, "matches.tsv"), "w", encoding="utf-8") as fh:
        fh.write("win\tmac\tmac_name\tstage\tscore\tsupport\n")
        for a in sorted(matched):
            b, st, sc, sup = matched[a]
            fh.write(f"{a:08x}\t{b:08x}\t{mac[b]['name']}\t{st}\t{sc}\t{sup}\n")
    with open(os.path.join(OUT, "folded.tsv"), "w", encoding="utf-8") as fh:
        fh.write("win\tcount\tmac_names\n")
        for a in sorted(FOLDED):
            fh.write(f"{a:08x}\t{len(FOLDED[a])}\t{' | '.join(FOLDED[a][:40])}\n")
    print(f"folded (ICF) functions: {len(FOLDED)}")
    with open(os.path.join(OUT, "vtable_pairs.tsv"), "w", encoding="utf-8") as fh:
        fh.write("win_vtable\tmac_vtable\tclass\tsub\tvotes\twin_slots\tmac_slots\n")
        for s in sorted(VT_PAIRS):
            z, v, lw, lm = VT_PAIRS[s]
            name, sub = z.split("#")
            fh.write(f"{s:08x}\t{name}\t{demangle_ztv(name)}\t{sub}\t{v}\t{lw}\t{lm}\n")
    print(f"vtable pairs: {len(VT_PAIRS)}")
    # validation against hand-verified mappings
    truth = {}
    for path in [os.path.join(ROOT, "re", "symbols_windows.tsv")] + [
            os.path.join(ROOT, "re", "symbols", f) for f in os.listdir(os.path.join(ROOT, "re", "symbols"))
            if f.endswith(".tsv")] if os.path.isdir(os.path.join(ROOT, "re", "symbols")) else [
            os.path.join(ROOT, "re", "symbols_windows.tsv")]:
        for line in open(path, encoding="utf-8").read().splitlines()[1:]:
            p = line.split("\t")
            if len(p) >= 4 and p[3].strip():
                try:
                    truth[int(p[0], 16)] = int(p[3], 16)
                except ValueError:
                    pass
    hit = sum(1 for a, b in truth.items() if a in matched and matched[a][0] == b)
    wrong = [(a, b, matched[a][0]) for a, b in truth.items() if a in matched and matched[a][0] != b
             and not (a in FOLDED and mac[b]["name"] in FOLDED[a])]
    folded_ok = sum(1 for a, b in truth.items() if a in FOLDED and mac[b]["name"] in FOLDED[a])
    print(f"validation: {len(truth)} known, {hit} correct, {len(wrong)} wrong, "
          f"{folded_ok} folded (expected name among ICF names), "
          f"{len(truth) - hit - len(wrong) - folded_ok} unmatched")
    for a, b, c in wrong[:20]:
        print(f"  WRONG {a:08x}: expected {mac[b]['name']} got {mac[c]['name']}")


if __name__ == "__main__":
    main()

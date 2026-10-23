"""Render the coverage treemap (docs/coverage-map.svg) from tools/coverage.py's attribution.

Every game function of SimCity 4.exe, grouped area > subsystem > owning class, sized by bytes and coloured by the
furthest status its class has reached (docs/COVERAGE.md). RE-side tool, like coverage.py: run it from the main checkout
after `python tools/coverage.py`.

    python tools/coverage_treemap.py [--out docs/coverage-map.svg]
"""
import argparse
import fnmatch
import html
import importlib.util
import subprocess
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("coverage", REPO / "tools" / "coverage.py")
cov = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cov)

STAGES = [  # (label, fill)
    ("No spec yet", "#d5d9d4"),
    ("Specified", "#9cc3e4"),
    ("Implemented", "#e8b04a"),
    ("Replaced, shadow not exact", "#c97b2a"),
    ("Verified (shadow exact)", "#4fa36b"),
    ("Verified and replaced", "#1f6b43"),
]
STAT = ("specified", "implemented", "verified", "replaced")


def stage(st):
    s, i, v, r = st
    return 5 if v and r else 4 if v else 3 if r else 2 if i else 1 if s else 0


def collect(regress_path=None):
    export = cov.find_input("ghidra/export/windows", None)
    exe = cov.find_input("bin/SimCity4.exe", None)
    _, lo, hi = cov.pe_text(exe)
    exe_sha = cov.sha256(exe)
    if exe_sha != cov.EXE_SHA256:
        raise ValueError("coverage_treemap: foreign executable")
    if f"executable_sha256={cov.EXE_SHA256}" not in (export / "export-summary.txt").read_text(encoding="utf-8"):
        raise ValueError("coverage_treemap: foreign export")
    regress = Path(regress_path) if regress_path else cov.newest_regress()
    att = cov.Attribution(export, REPO / "re" / "names_windows.tsv", REPO / "docs" / "abi" / "vtables.tsv",
                          REPO / "docs" / "abi" / "interfaces.tsv", REPO / "tools" / "coverage-libs.tsv",
                          REPO / "tools" / "coverage-ranges.tsv", lo, hi,
                          REPO / "tools" / "coverage-noncode-refs.tsv")
    owner, rule = att.run()
    rows = cov.read_map(REPO / "tools" / "coverage-map.tsv")
    replace_list, shadow_list = cov.regress_lists(REPO / "tools" / "regress.ps1")
    keys = cov.subsystem_keys(REPO / "engine" / "plugin" / "FC4Subsystems.cpp")
    reg = cov.read_regress(regress)
    shadow_ev, _ = cov.read_shadow_evidence(REPO / "tools" / "coverage-shadows.tsv", REPO)
    cov.map_statuses(rows, replace_list, shadow_list, keys, reg, shadow_ev, regress.name)
    statuses, _, hashes = cov.function_statuses(att, owner, rule, rows, export, exe_sha)
    tree, stat_b = collect_entries(att, owner, rows, statuses)
    return tree, regress.name, stat_b, hashes


def collect_entries(att, owner, rows, statuses):
    """Group actual entries by resolved flags; mixed owners get separate boxes."""
    row_of = {}

    def match(o):
        for i, r in enumerate(rows):
            if fnmatch.fnmatchcase(o, r["pattern"]):
                return i
        return None

    agg = defaultdict(int)
    stat_b = {s: 0 for s in STAT}  # the four headline totals, summed on the row's own flags (not on the colour)
    for va in att.order:
        size, o = att.funcs[va], owner[va]
        if o is not None and o.startswith("lib:"):
            continue
        if o is None:
            k = ("unattributed", "unattributed", "unattributed", 0)
        elif o == cov.SHARED:
            k = ("shared", "shared helpers", "shared", 0)
        else:
            if o not in row_of:
                row_of[o] = match(o)
            r = rows[row_of[o]]
            for s in STAT:
                if statuses[va][s]:
                    stat_b[s] += size
            k = (r["area"], r["subsystem"], o, stage([statuses[va][s] for s in STAT]))
        agg[k] += size
    tree = defaultdict(lambda: defaultdict(list))
    for (a, s, o, st), b in agg.items():
        tree[a][s].append((b, st, o))
    return tree, stat_b


def worst(row, w):
    s = sum(row)
    return max(max(row) * w * w / (s * s), (s * s) / (w * w * min(row))) if s and min(row) > 0 else float("inf")


def squarify(values, x, y, w, h):
    """Bruls-Huizing-van Wijk squarified layout; values sorted descending, sum scaled to w*h."""
    total = sum(values)
    if total <= 0 or w <= 0 or h <= 0:
        return [(x, y, 0, 0)] * len(values)
    areas = [v * w * h / total for v in values]
    out, i = [], 0
    while i < len(areas):
        side = min(w, h)
        row = [areas[i]]
        j = i + 1
        while j < len(areas) and worst(row + [areas[j]], side) <= worst(row, side):
            row.append(areas[j])
            j += 1
        s = sum(row)
        if w >= h:  # lay the row as a column on the left
            cw = s / h
            cy = y
            for a in row:
                out.append((x, cy, cw, a / cw))
                cy += a / cw
            x, w = x + cw, w - cw
        else:  # lay the row along the top
            rh = s / w
            cx = x
            for a in row:
                out.append((cx, y, a / rh, rh))
                cx += a / rh
            y, h = y + rh, h - rh
        i = j
    return out


def render(tree, regress, head, out, stat_b, hashes=None):
    W, H = 1600, 1000
    top, legend_h = 120, 46
    mx, my, mw, mh = 16, top, W - 32, H - top - legend_h - 16
    total = sum(b for subs in tree.values() for os in subs.values() for b, _, _ in os)
    stage_b = [0] * 6
    for subs in tree.values():
        for os in subs.values():
            for b, st, _ in os:
                stage_b[st] += b
    svg = []
    e = html.escape
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
               'font-family="Segoe UI, Helvetica, Arial, sans-serif">')
    evidence_metadata = "; ".join(f"{p}={h}" for p, h in sorted((hashes or {}).items()))
    svg.append(f'<metadata>Exact-function evidence SHA-256 (CRLF normalized): {e(evidence_metadata)}</metadata>')
    svg.append(f'<rect width="{W}" height="{H}" fill="#f6f7f4"/>')
    svg.append('<text x="16" y="44" font-size="30" font-weight="700" fill="#1d2320">OpenSC4 coverage of SimCity 4.exe</text>')
    # the four headline shares, on each status's own flag (a row can be implemented without a spec, or replaced
    # without a module, so the colour's stage bucket is not the same set)
    svg.append(f'<text x="16" y="74" font-size="17" fill="#4a544f">Game code: {total / 1048576:.2f} MB of machine code, '
               f'libraries excluded. Specified {stat_b["specified"] / total:.1%} · '
               f'implemented {stat_b["implemented"] / total:.1%} · verified {stat_b["verified"] / total:.1%} · '
               f'replaced in game {stat_b["replaced"] / total:.1%}.</text>')
    svg.append(f'<text x="16" y="100" font-size="14" fill="#6b746f">Box area = bytes. Colour = furthest status of the '
               f'owning class (tools/coverage.py, docs/COVERAGE.md). {e(head)}, {e(regress)}.</text>')
    areas = sorted(tree.items(), key=lambda kv: -sum(b for os in kv[1].values() for b, _, _ in os))
    avals = [sum(b for os in subs.values() for b, _, _ in os) for _, subs in areas]
    for (aname, subs), (ax, ay, aw, ah), av in zip(areas, squarify(avals, mx, my, mw, mh), avals):
        pad_t = 26 if aw > 90 and ah > 60 else 2
        sx, sy, sw, sh = ax + 2, ay + pad_t, aw - 4, ah - pad_t - 2
        sitems = sorted(subs.items(), key=lambda kv: -sum(b for b, _, _ in kv[1]))
        svals = [sum(b for b, _, _ in os) for _, os in sitems]
        for (sname, os), (bx, by, bw, bh) in zip(sitems, squarify(svals, sx, sy, sw, sh)):
            os = sorted(os, key=lambda t: -t[0])
            for (b, st, o), (x, y, w, h) in zip(os, squarify([t[0] for t in os], bx, by, bw, bh)):
                if w > 0.2 and h > 0.2:
                    svg.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" '
                               f'fill="{STAGES[st][1]}" stroke="#f6f7f4" stroke-width="0.4"/>')
            svg.append(f'<rect x="{bx:.1f}" y="{by:.1f}" width="{bw:.1f}" height="{bh:.1f}" fill="none" '
                       'stroke="#f6f7f4" stroke-width="1.6"/>')
            if bw > 80 and bh > 22:
                n = int((bw - 8) / 6.4)
                label = sname if len(sname) <= n else sname[: max(n - 1, 0)] + "…"
                if label:
                    svg.append(f'<text x="{bx + 4:.1f}" y="{by + 14:.1f}" font-size="11.5" font-weight="600" '
                               f'fill="#1d2320" stroke="#f6f7f4" stroke-width="3" paint-order="stroke">{e(label)}</text>')
        svg.append(f'<rect x="{ax:.1f}" y="{ay:.1f}" width="{aw:.1f}" height="{ah:.1f}" fill="none" '
                   'stroke="#1d2320" stroke-width="2"/>')
        if pad_t > 2:
            t = f"{aname.upper()}  {av / total:.1%}"
            n = int((aw - 10) / 10)
            if len(t) > n:
                t = aname.upper()[: max(n, 0)]
            svg.append(f'<text x="{ax + 6:.1f}" y="{ay + 19:.1f}" font-size="16" font-weight="700" '
                       f'fill="#1d2320">{e(t)}</text>')
    lx, ly = 16, H - legend_h + 4
    for label, fill in STAGES:
        svg.append(f'<rect x="{lx}" y="{ly}" width="18" height="18" fill="{fill}" stroke="#1d2320" stroke-width="0.5"/>')
        svg.append(f'<text x="{lx + 26}" y="{ly + 14}" font-size="14" fill="#1d2320">{e(label)}</text>')
        lx += 40 + int(len(label) * 7.6)
    svg.append("</svg>")
    out.write_text("\n".join(svg) + "\n", encoding="utf-8")
    return total, [b / total for b in stage_b]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", default=str(REPO / "docs" / "coverage-map.svg"))
    ap.add_argument("--regress", help="same pinned regress input as coverage.py")
    args = ap.parse_args()
    def git(*a):
        return subprocess.run(["git", "-C", str(REPO), *a], capture_output=True, text=True).stdout.strip()
    short = git("rev-parse", "--short", "HEAD")
    branch = git("rev-parse", "--abbrev-ref", "HEAD")  # a worktree is not on `main`; name the branch it is on
    head = f"{branch} {short}" if branch and branch != "HEAD" else short
    tree, regress, stat_b, hashes = collect(args.regress)
    total, shares = render(tree, regress, head, Path(args.out), stat_b, hashes)
    print(f"coverage_treemap: {total} bytes; stage shares " + ", ".join(f"{s:.1%}" for s in shares)
          + f"; wrote {args.out}")


if __name__ == "__main__":
    main()

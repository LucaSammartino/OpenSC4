"""Render the coverage treemap (docs/coverage-map.svg) from tools/coverage.py's attribution.

Every game function of SimCity 4.exe, grouped area > subsystem > owning class, sized by bytes and coloured by the
furthest status its class has reached (docs/COVERAGE.md). RE-side tool, like coverage.py: run it from the main checkout
after `python tools/coverage.py`.

    python tools/coverage_treemap.py [--out docs/coverage-map.svg] [--suite <tag>]

With --suite (a tag under build-runs/suite, as for coverage.py), each owner's box is hatched from the bottom in proportion
to its suite-verified bytes (reached by the suite and exact across it, not only in the house city), and the headline adds strict suite-verified.
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


def collect(suite=None):
    data = cov.load_report_data()
    att, owner, rows = data["att"], data["owner"], data["rows"]
    regress, methods = data["regress"], data["methods"]

    strict = {}
    if suite:
        sc = cov._sibling("suite_coverage")
        evidence = sc.load(REPO, suite, [k for k, on in data["keys"].items() if on])
        strict = {va: st["suite-verified"] for va, st in
                  sc.states(evidence, data["game"], rows, cov.SHADOW_LABEL_ALIAS).items()}
    agg = defaultdict(int)
    agg_strict = defaultdict(int)
    stat_b = {s: 0 for s in STAT + ("suite-verified",)}
    for va, size, index, _ in data["game"]:
        o = owner[va]
        flags = [cov.coverage_methods.function_status(rows, index, va, methods, s) for s in STAT]
        for s, flag in zip(STAT, flags):
            if flag:
                stat_b[s] += size
        if o is None:
            k = ("unattributed", "unattributed", "unattributed", stage(flags))
        elif o == cov.SHARED:
            k = ("shared", "shared helpers", "shared", stage(flags))
        else:
            r = rows[index]
            k = (r["area"], r["subsystem"], o, stage(flags))
        agg[k] += size
        if strict.get(va):
            agg_strict[k] += size
            stat_b["suite-verified"] += size
    tree = defaultdict(lambda: defaultdict(list))
    for (a, s, o, st), b in agg.items():
        tree[a][s].append((b, st, o, agg_strict[(a, s, o, st)] / b))
    return tree, regress.name, stat_b


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


def render(tree, regress, head, out, stat_b, suite=None):
    W, H = 1600, 1000
    top, legend_h = 120, 46
    mx, my, mw, mh = 16, top, W - 32, H - top - legend_h - 16
    total = sum(b for subs in tree.values() for os in subs.values() for b, *_ in os)
    stage_b = [0] * 6
    for subs in tree.values():
        for os in subs.values():
            for b, st, *_ in os:
                stage_b[st] += b
    svg = []
    e = html.escape
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
               'font-family="Segoe UI, Helvetica, Arial, sans-serif">')
    svg.append('<defs><pattern id="strict" width="7" height="7" patternUnits="userSpaceOnUse" '
               'patternTransform="rotate(45)"><line x1="0" y1="0" x2="0" y2="7" stroke="#f6f7f4" stroke-width="2.2"/>'
               '</pattern></defs>')
    svg.append(f'<rect width="{W}" height="{H}" fill="#f6f7f4"/>')
    svg.append('<text x="16" y="44" font-size="30" font-weight="700" fill="#1d2320">OpenSC4 coverage of SimCity 4.exe</text>')
    # the four headline shares, on each status's own flag (a row can be implemented without a spec, or replaced
    # without a module, so the colour's stage bucket is not the same set)
    svg.append(f'<text x="16" y="74" font-size="17" fill="#4a544f">Game code: {total / 1048576:.2f} MB of machine code, '
               f'libraries excluded. Specified {stat_b["specified"] / total:.1%} · '
               f'implemented {stat_b["implemented"] / total:.1%} · verified {stat_b["verified"] / total:.1%} · '
               f'replaced in game {stat_b["replaced"] / total:.1%}'
               + (f' · strict suite-verified {stat_b["suite-verified"] / total:.1%}' if suite else '') + '.</text>')
    svg.append(f'<text x="16" y="100" font-size="14" fill="#6b746f">Box area = bytes. Colour = furthest status of the '
               f'owning class (tools/coverage.py, docs/COVERAGE.md). {e(head)}, {e(regress)}'
               + (f', suite {e(suite)}' if suite else '') + '.</text>')
    areas = sorted(tree.items(), key=lambda kv: -sum(b for os in kv[1].values() for b, *_ in os))
    avals = [sum(b for os in subs.values() for b, *_ in os) for _, subs in areas]
    for (aname, subs), (ax, ay, aw, ah), av in zip(areas, squarify(avals, mx, my, mw, mh), avals):
        pad_t = 26 if aw > 90 and ah > 60 else 2
        sx, sy, sw, sh = ax + 2, ay + pad_t, aw - 4, ah - pad_t - 2
        sitems = sorted(subs.items(), key=lambda kv: -sum(b for b, *_ in kv[1]))
        svals = [sum(b for b, *_ in os) for _, os in sitems]
        for (sname, os), (bx, by, bw, bh) in zip(sitems, squarify(svals, sx, sy, sw, sh)):
            os = sorted(os, key=lambda t: -t[0])
            for (b, st, o, frac), (x, y, w, h) in zip(os, squarify([t[0] for t in os], bx, by, bw, bh)):
                if w > 0.2 and h > 0.2:
                    svg.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" '
                               f'fill="{STAGES[st][1]}" stroke="#f6f7f4" stroke-width="0.4"/>')
                    if frac > 0:  # bottom strip, its height the owner's strict share of bytes
                        sh_ = h * frac
                        svg.append(f'<rect x="{x:.1f}" y="{y + h - sh_:.1f}" width="{w:.1f}" height="{sh_:.1f}" '
                                   'fill="url(#strict)"/>')
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
    if suite:
        label = "Hatched share: exact across the whole suite (strict)"
        svg.append(f'<rect x="{lx}" y="{ly}" width="18" height="18" fill="{STAGES[5][1]}" stroke="#1d2320" '
                   'stroke-width="0.5"/>')
        svg.append(f'<rect x="{lx}" y="{ly}" width="18" height="18" fill="url(#strict)"/>')
        svg.append(f'<text x="{lx + 26}" y="{ly + 14}" font-size="14" fill="#1d2320">{e(label)}</text>')
    svg.append("</svg>")
    out.write_text("\n".join(svg) + "\n", encoding="utf-8")
    return total, [b / total for b in stage_b]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", default=str(REPO / "docs" / "coverage-map.svg"))
    ap.add_argument("--suite", help="suite tag under build-runs/suite: hatch strict suite-verified owners")
    args = ap.parse_args()
    def git(*a):
        return subprocess.run(["git", "-C", str(REPO), *a], capture_output=True, text=True).stdout.strip()
    short = git("rev-parse", "--short", "HEAD")
    branch = git("rev-parse", "--abbrev-ref", "HEAD")  # a worktree is not on `main`; name the branch it is on
    head = f"{branch} {short}" if branch and branch != "HEAD" else short
    tree, regress, stat_b = collect(args.suite)
    total, shares = render(tree, regress, head, Path(args.out), stat_b, args.suite)
    print(f"coverage_treemap: {total} bytes; stage shares " + ", ".join(f"{s:.1%}" for s in shares)
          + (f"; strict suite-verified {stat_b['suite-verified'] / total:.1%}" if args.suite else "")
          + f"; wrote {args.out}")


if __name__ == "__main__":
    main()

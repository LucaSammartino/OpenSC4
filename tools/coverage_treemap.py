"""Render the coverage treemap (docs/coverage-map.svg) from tools/coverage.py's attribution.

The top of the picture is the funnel: specified -> implemented -> exact-replaced, each the share of the game's
machine code bytes, and Reach, the ceiling (docs/NEXT_STEPS.md section 3). Below it every game function of
SimCity 4.exe, grouped area > subsystem > owning class, sized by bytes and coloured by the furthest step it has reached
(one colour ramp, light to dark); the hatch marks code the corpus reaches in the original. RE-side tool, like
coverage.py: run it from the main checkout.

    python tools/coverage_treemap.py [--out docs/coverage-map.svg] [--suite <tag>]

--suite defaults to the tag in build-runs/suite/CURRENT (the census whose entry counters give Reach).
"""
import argparse
import html
import importlib.util
import subprocess
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("coverage", REPO / "tools" / "coverage.py")
cov = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cov)

STAT = cov.STAT
STEPS = [  # the funnel steps in order (the map's original palette); exact-replaced gets "#1f6b43" once measured
    ("Not specified", "#d5d9d4"),
    ("Specified", "#9cc3e4"),
    ("Implemented", "#e8b04a"),
]
BG, INK, MUTED, TRACK = "#f6f7f4", "#1d2320", "#5b6560", "#e3e6e0"


def step(flags):
    s, i = flags
    return 2 if i else 1 if s else 0


def current_suite():
    f = REPO / "build-runs" / "suite" / "CURRENT"
    return f.read_text(encoding="utf-8").strip() if f.is_file() else None


def collect(suite=None):
    data = cov.load_report_data()
    owner, rows = data["owner"], data["rows"]
    reached = set()
    if suite:
        sc = cov._sibling("suite_coverage")
        for item in sc.load(REPO, suite)["scenarios"]:
            if sc.clean(item["row"]):
                reached |= item["reached"]
    agg = defaultdict(int)
    stat_b = {s: 0 for s in STAT + ("reach",)}
    for va, size, index, _ in data["game"]:
        o = owner[va]
        flags = [index is not None and rows[index][s] for s in STAT]
        for s, flag in zip(STAT, flags):
            if flag:
                stat_b[s] += size
        hatch = va in reached
        if hatch:
            stat_b["reach"] += size
        if o is None:
            k = ("unattributed", "unattributed", "unattributed")
        elif o == cov.SHARED:
            k = ("shared", "shared helpers", "shared")
        else:
            r = rows[index]
            k = (r["area"], r["subsystem"], o)
        agg[k + (step(flags), hatch)] += size
    tree = defaultdict(lambda: defaultdict(list))
    for (a, s, o, st, hatch), b in agg.items():
        tree[a][s].append((b, st, o, hatch))
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


def fill(st):
    return STEPS[st][1]


def funnel(svg, stat_b, total, suite, y0):
    """One bar per step on a common 0-100% scale; Reach, the ceiling, is a hatched bar."""
    e = html.escape
    bx, bw, bh, gap = 560, 860, 26, 8
    bars = [("Specified", "a clean-room spec describes the code", "specified", 1, False),
            ("Implemented", "our code exists for it", "implemented", 2, False),
            ("Exact-replaced", "our code runs instead and matches the references", None, None, False)]
    if suite:
        bars.append(("Reach", "the corpus executes it in the original (ceiling)", "reach", 0, True))
    for n, (label, meaning, key, st, hatch) in enumerate(bars):
        y = y0 + n * (bh + gap)
        svg.append(f'<text x="16" y="{y + 18}" font-size="16" font-weight="700" fill="{INK}">{e(label)}</text>')
        svg.append(f'<text x="150" y="{y + 18}" font-size="14" fill="{MUTED}">{e(meaning)}</text>')
        svg.append(f'<rect x="{bx}" y="{y}" width="{bw}" height="{bh}" rx="3" fill="{TRACK}"/>')
        if key is None:
            svg.append(f'<text x="{bx + bw + 12}" y="{y + 19}" font-size="15" fill="{MUTED}">not measured</text>')
            continue
        share = stat_b[key] / total
        svg.append(f'<rect x="{bx}" y="{y}" width="{bw * share:.1f}" height="{bh}" rx="3" '
                   f'fill="{MUTED if hatch else STEPS[st][1]}"/>')
        if hatch:
            svg.append(f'<rect x="{bx}" y="{y}" width="{bw * share:.1f}" height="{bh}" rx="3" fill="url(#reach)"/>')
        svg.append(f'<text x="{bx + bw + 12}" y="{y + 19}" font-size="18" font-weight="700" fill="{INK}">'
                   f'{share:.1%}</text>')
    end = y0 + len(bars) * (bh + gap)
    for t in (25, 50, 75):
        x = bx + bw * t / 100
        svg.append(f'<line x1="{x:.1f}" y1="{y0}" x2="{x:.1f}" y2="{end - gap}" stroke="{BG}" stroke-width="1.5"/>')
    for t in (0, 50, 100):
        x = bx + bw * t / 100
        svg.append(f'<text x="{x:.1f}" y="{end + 10}" font-size="11" fill="{MUTED}" text-anchor="middle">{t}%</text>')
    return end + 14


def legend(svg, y, suite):
    e = html.escape
    x = 16
    svg.append(f'<text x="{x}" y="{y + 14}" font-size="14" font-weight="700" fill="{INK}">Colour = furthest step '
               'reached:</text>')
    x += 236
    for n, (label, colour) in enumerate(STEPS):
        svg.append(f'<rect x="{x}" y="{y}" width="18" height="18" fill="{colour}" stroke="{INK}" stroke-width="0.5"/>')
        svg.append(f'<text x="{x + 25}" y="{y + 14}" font-size="14" fill="{INK}">{e(label)}</text>')
        x += 25 + int(len(label) * 7.4) + 10
        if n < len(STEPS) - 1:
            svg.append(f'<text x="{x}" y="{y + 14}" font-size="15" fill="{MUTED}">→</text>')
            x += 26
    y += 28
    x = 16
    if suite:
        svg.append(f'<rect x="{x}" y="{y}" width="18" height="18" fill="{STEPS[2][1]}" stroke="{INK}" '
                   'stroke-width="0.5"/>')
        svg.append(f'<rect x="{x}" y="{y}" width="18" height="18" fill="url(#reach)"/>')
        svg.append(f'<text x="{x + 25}" y="{y + 14}" font-size="14" fill="{INK}">Hatched = reached: the corpus '
                   'executes it in the original, so a reference can check it</text>')


def render(tree, head, out, stat_b, suite=None):
    W, H = 1600, 1200
    e = html.escape
    total = sum(b for subs in tree.values() for os in subs.values() for b, *_ in os)
    svg = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
           'font-family="Segoe UI, Helvetica, Arial, sans-serif">',
           '<defs><pattern id="reach" width="8" height="8" patternUnits="userSpaceOnUse" '
           'patternTransform="rotate(45)"><line x1="0" y1="0" x2="0" y2="8" stroke="#ffffff" stroke-width="2.4"/>'
           '</pattern></defs>',
           f'<rect width="{W}" height="{H}" fill="{BG}"/>',
           f'<text x="16" y="40" font-size="30" font-weight="700" fill="{INK}">OpenSC4 coverage of SimCity 4.exe</text>',
           f'<text x="16" y="66" font-size="15" fill="{MUTED}">Share of the game\'s {total / 1048576:.2f} MB of '
           'machine code (libraries excluded) that has reached each step. The goal is 100% '
           'exact-replaced; Reach is its ceiling.</text>']
    y = funnel(svg, stat_b, total, suite, 84)
    svg.append(f'<text x="16" y="{y + 14}" font-size="13" fill="{MUTED}">The map below: one box per class of the '
               'original, area = its bytes, grouped by area and subsystem. '
               f'{e(head)}' + (f', suite {e(suite)}' if suite else '') + '.</text>')

    legend_h = 62
    mx, my = 16, y + 28
    mw, mh = W - 32, H - my - legend_h - 8
    areas = sorted(tree.items(), key=lambda kv: -sum(b for os in kv[1].values() for b, *_ in os))
    avals = [sum(b for os in subs.values() for b, *_ in os) for _, subs in areas]
    for (aname, subs), (ax, ay, aw, ah), av in zip(areas, squarify(avals, mx, my, mw, mh), avals):
        pad_t = 26 if aw > 90 and ah > 60 else 2
        sx, sy, sw, sh = ax + 2, ay + pad_t, aw - 4, ah - pad_t - 2
        sitems = sorted(subs.items(), key=lambda kv: -sum(b for b, *_ in kv[1]))
        svals = [sum(b for b, *_ in os) for _, os in sitems]
        for (sname, os), (rx, ry, rw, rh) in zip(sitems, squarify(svals, sx, sy, sw, sh)):
            owners = defaultdict(list)  # keep one class's pieces (one per step) together in its own box
            for t in os:
                owners[t[2]].append(t)
            oitems = sorted(owners.values(), key=lambda ts: -sum(t[0] for t in ts))
            for ts, (ox, oy, ow, oh) in zip(oitems, squarify([sum(t[0] for t in g) for g in oitems], rx, ry, rw, rh)):
                ts = sorted(ts, key=lambda t: -t[0])
                for (b, st, o, hatch), (x, y, w, h) in zip(ts, squarify([t[0] for t in ts], ox, oy, ow, oh)):
                    if w > 0.2 and h > 0.2:
                        svg.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" fill="{fill(st)}"/>')
                        if hatch:
                            svg.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" '
                                       'fill="url(#reach)"/>')
                if ow > 0.4 and oh > 0.4:
                    svg.append(f'<rect x="{ox:.1f}" y="{oy:.1f}" width="{ow:.1f}" height="{oh:.1f}" fill="none" '
                               f'stroke="{BG}" stroke-width="0.6"/>')
            svg.append(f'<rect x="{rx:.1f}" y="{ry:.1f}" width="{rw:.1f}" height="{rh:.1f}" fill="none" '
                       f'stroke="{BG}" stroke-width="1.8"/>')
            if rw > 80 and rh > 22:
                n = int((rw - 8) / 6.4)
                label = sname if len(sname) <= n else sname[: max(n - 1, 0)] + "…"
                if label:
                    svg.append(f'<text x="{rx + 4:.1f}" y="{ry + 14:.1f}" font-size="11.5" font-weight="600" '
                               f'fill="{INK}" stroke="{BG}" stroke-width="3" paint-order="stroke">{e(label)}</text>')
        svg.append(f'<rect x="{ax:.1f}" y="{ay:.1f}" width="{aw:.1f}" height="{ah:.1f}" fill="none" '
                   f'stroke="{INK}" stroke-width="2"/>')
        if pad_t > 2:
            t = f"{aname.upper()}  {av / total:.1%}"
            n = int((aw - 10) / 10)
            if len(t) > n:
                t = aname.upper()[: max(n, 0)]
            svg.append(f'<text x="{ax + 6:.1f}" y="{ay + 19:.1f}" font-size="16" font-weight="700" '
                       f'fill="{INK}">{e(t)}</text>')
    legend(svg, H - legend_h + 4, suite)
    svg.append("</svg>")
    out.write_text("\n".join(svg) + "\n", encoding="utf-8")
    return total


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", default=str(REPO / "docs" / "coverage-map.svg"))
    ap.add_argument("--suite", default=current_suite(),
                    help="census tag whose entry counters give Reach (default: suite/CURRENT)")
    args = ap.parse_args()

    def git(*a):
        return subprocess.run(["git", "-C", str(REPO), *a], capture_output=True, text=True).stdout.strip()
    short = git("rev-parse", "--short", "HEAD")
    branch = git("rev-parse", "--abbrev-ref", "HEAD")  # a worktree is not on `main`; name the branch it is on
    head = f"{branch} {short}" if branch and branch != "HEAD" else short
    tree, stat_b = collect(args.suite)
    total = render(tree, head, Path(args.out), stat_b, args.suite)
    print(f"coverage_treemap: {total} bytes; " + ", ".join(f"{s} {b / total:.1%}" for s, b in stat_b.items())
          + f"; wrote {args.out}")


if __name__ == "__main__":
    main()

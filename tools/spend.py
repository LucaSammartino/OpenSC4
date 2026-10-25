"""Daily spend metrics from an OpenRouter activity export (Activity page -> Export CSV).

Usage: python tools/spend.py <openrouter_activity.csv> [--coverage-delta PTS] [--since T] [--until T]
                             [--model SUBSTR] [--label NAME]

Prints one markdown table row for docs/history/efficiency-log.md plus a short breakdown. Every number is read
from the CSV; nothing is fitted. --coverage-delta is the day's change in implemented % from
python tools/coverage.py (start-of-day vs end-of-day), used for the $/point column.

--since/--until cut one run out of a multi-day export ("2026-10-20 20:31", UTC as in the CSV; until is exclusive),
so two sessions or two models can be compared from the same file; --label replaces the date in the row.
"""
import argparse
import collections
import csv
import statistics


def f(r, k):
    return float(r.get(k) or 0)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("csv")
    ap.add_argument("--coverage-delta", type=float, help="implemented %% points gained that day")
    ap.add_argument("--since", default="", help="first created_at to keep (UTC, as in the CSV)")
    ap.add_argument("--until", default="~", help="first created_at to drop")
    ap.add_argument("--model", default="", help="keep rows whose model_permaslug contains this")
    ap.add_argument("--label", help="row label instead of the first date")
    a = ap.parse_args()
    rows = [r for r in csv.DictReader(open(a.csv, encoding="utf8"))
            if a.since <= r["created_at"] < a.until and a.model in r["model_permaslug"]]
    if not rows:
        raise SystemExit("no rows")
    cost = sum(f(r, "cost_total") for r in rows)
    prompt = [f(r, "tokens_prompt") for r in rows]
    cached = sum(f(r, "tokens_cached") for r in rows)
    out = [f(r, "tokens_completion") for r in rows]
    miss = [r for r in rows if f(r, "tokens_prompt") - f(r, "tokens_cached") > 100_000]
    big = [r for r in rows if f(r, "tokens_prompt") > 400_000]
    prov = collections.Counter(r["provider_name"] for r in rows)
    day = a.label or min(r["created_at"] for r in rows)[:10]
    gen_h = sum(f(r, "generation_time_ms") for r in rows) / 3.6e6
    med_p = statistics.median(prompt)
    p90 = sorted(prompt)[int(0.9 * (len(prompt) - 1))]
    per_pt = f"${cost / a.coverage_delta:.0f}" if a.coverage_delta else "-"
    print("| Date | $ | Calls | Median ctx | p90 ctx | Cache hit | $ in misses | $ in >400k ctx "
          "| Median out | Providers | Model hours | Impl. +pts | $/pt |")
    print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    print(f"| {day} | {cost:.2f} | {len(rows):,} | {med_p / 1e3:.0f}k | {p90 / 1e3:.0f}k "
          f"| {100 * cached / sum(prompt):.1f}% | {sum(f(r, 'cost_total') for r in miss):.2f} ({len(miss)}) "
          f"| {sum(f(r, 'cost_total') for r in big):.2f} | {statistics.median(out):.0f} | {len(prov)} "
          f"| {gen_h:.1f} | {a.coverage_delta if a.coverage_delta is not None else '-'} | {per_pt} |")
    print()
    print(f"tokens: prompt {sum(prompt) / 1e9:.2f}B (cached {cached / 1e9:.2f}B), completion {sum(out) / 1e6:.1f}M")
    print("models:", dict(collections.Counter(r["model_permaslug"] for r in rows)))
    print("providers (calls, $, $ per M prompt tokens):")
    for p, n in prov.most_common():
        sel = [r for r in rows if r["provider_name"] == p]
        c, t = sum(f(r, "cost_total") for r in sel), sum(f(r, "tokens_prompt") for r in sel)
        print(f"  {p:16s} {n:6d} {c:8.2f} {1e6 * c / t if t else 0:8.4f}")


if __name__ == "__main__":
    main()

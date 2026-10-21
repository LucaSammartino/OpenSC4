#!/usr/bin/env python3
"""What fills a lane's re-sent context, and what output caps would have saved, from recorded agent transcripts.

Every tool result and tool-call argument stays in context, so it is re-sent on each later API call. Its cost is
size x remaining calls. Chars are converted to tokens with a ratio calibrated against the transcript's own usage
records, after subtracting the fixed preamble (system prompt and tool schemas) from every call.

Usage: python tools/ctx_weight.py <dir-or-jsonl>... [--cap-lines 60] [--cap-bytes 4096] [--script-lines 20]
  Reads every agent-*.jsonl under the given directories (workflow subagent transcripts).
  --cap-lines/--cap-bytes   the cap simulated on re_query, grep and ad-hoc Bash output (head kept, one-line footer);
                            Bash file reads (sed -n ranges, cat, head) are deliberate reads and stay uncapped
  --script-lines            inline heredoc scripts longer than this are simulated as a file written once and
                            then changed with Edit: a near-repeat of an earlier script costs only its diff
  --preamble N              fixed per-call preamble in tokens (default 7800, run 2's measured value)
Prints the share of re-sent context by category, and the same table with the caps applied.
"""
import argparse
import difflib
import glob
import json
import os
import re
import sys
from collections import defaultdict

QUIET = re.compile(r"\b(qbuild|qwait|ctx|fc4verify|lanes)\.py\b")
REQ = re.compile(r"\bre_query\.py\b")
FILE_READ = re.compile(r"^(sed|cat|head|tail|awk|nl)\s")
GREP = re.compile(r"(^|[;&|(]\s*|\s)(qgrep\.py|grep|rg|findstr|Select-String)\b")
HEREDOC = re.compile(r"<<-?\s*'?\"?(\w+)'?\"?[^\n]*\n(.*?)\n\1\b", re.S)
FOOTER = 70  # "... N more lines; rerun with --full or narrow the query"


def text_of(content):
    if isinstance(content, str):
        return content
    out = []
    for b in content or []:
        if b.get("type") == "text":
            out.append(b.get("text", ""))
        elif b.get("type") == "tool_result":
            out.append(text_of(b.get("content")))
    return "\n".join(out)


def bash_kind(cmd):
    first = cmd.split("|")[0]
    if REQ.search(cmd):
        return "re_query"
    if QUIET.search(cmd):
        return "quiet wrappers (qbuild/qwait/...)"
    if GREP.search(first) or GREP.search(cmd.split(";")[0]):
        return "grep"
    lead = re.sub(r"^(cd \S+ && |[A-Za-z0-9_]+=\S+;? ?)+", "", cmd.strip())
    if FILE_READ.match(lead):
        return "bash file reads (sed/cat/head/awk)"
    return "ad-hoc bash"


def capped(text, lines, nbytes):
    ls = text.splitlines()
    if len(ls) <= lines and len(text.encode("utf-8", "replace")) <= nbytes:
        return len(text)
    keep, size = 0, 0
    for l in ls[:lines]:
        if size + len(l) + 1 > nbytes:
            break
        size += len(l) + 1
        keep += 1
    return size + FOOTER


def diff_size(old, new):
    """Chars an Edit would carry to turn old into new (changed lines on both sides plus a small overhead)."""
    sm = difflib.SequenceMatcher(None, old.splitlines(), new.splitlines(), autojunk=False)
    a, b = old.splitlines(), new.splitlines()
    n = 0
    for op, i1, i2, j1, j2 in sm.get_opcodes():
        if op != "equal":
            n += sum(len(x) + 1 for x in a[i1:i2]) + sum(len(x) + 1 for x in b[j1:j2]) + 120
    return n


def agent_rows(path):
    with open(path, encoding="utf-8") as f:
        for line in f:
            try:
                yield json.loads(line)
            except ValueError:
                continue


def analyse(path, a, acc):
    calls, usage = [], {}
    events = []  # (call_index, category, chars_now, chars_capped)
    tool_of = {}
    scripts = []  # earlier inline scripts in this agent
    written = {}  # path -> last content written
    for r in agent_rows(path):
        t = r.get("type")
        m = r.get("message") or {}
        if t == "assistant":
            mid = m.get("id") or r.get("uuid")
            if mid not in usage:
                calls.append(mid)
                u = m.get("usage") or {}
                usage[mid] = (u.get("input_tokens", 0) + u.get("cache_read_input_tokens", 0)
                              + u.get("cache_creation_input_tokens", 0))
            k = len(calls)
            for b in m.get("content") or []:
                if b.get("type") == "text":
                    events.append((k, "assistant text", len(b.get("text", "")), None))
                if b.get("type") != "tool_use":
                    continue
                name, inp = b.get("name"), b.get("input") or {}
                raw = len(json.dumps(inp, ensure_ascii=False))
                if name == "Bash":
                    cmd = inp.get("command", "")
                    kind = bash_kind(cmd)
                    tool_of[b.get("id")] = ("Bash", kind)
                    sim = raw
                    for _, body in HEREDOC.findall(cmd):
                        if body.count("\n") + 1 > a.script_lines:
                            best = min((diff_size(s, body) for s in scripts), default=None)
                            if best is not None and best < len(body):
                                sim -= len(body) - best
                            scripts.append(body)
                    events.append((k, "args: inline scripts" if "<<" in cmd else "args: bash commands", raw, sim))
                elif name in ("Write", "Edit"):
                    tool_of[b.get("id")] = (name, name)
                    sim = raw
                    if name == "Write":
                        p, c = inp.get("file_path", ""), inp.get("content", "")
                        if p in written:
                            sim = min(raw, diff_size(written[p], c) + 200)
                        written[p] = c
                    events.append((k, "args: Write/Edit", raw, sim))
                else:
                    tool_of[b.get("id")] = (name, name)
                    events.append((k, "args: other", raw, None))
        elif t == "user":
            k = len(calls)
            content = m.get("content")
            if isinstance(content, str):
                events.append((k, "prompt and user text", len(content), None))
                continue
            for b in content or []:
                if b.get("type") != "tool_result":
                    if b.get("type") == "text":
                        events.append((k, "prompt and user text", len(b.get("text", "")), None))
                    continue
                txt = text_of(b.get("content"))
                name, kind = tool_of.get(b.get("tool_use_id"), ("?", "?"))
                if name == "Bash":
                    cat = "out: " + kind
                    sim = capped(txt, a.cap_lines, a.cap_bytes) if kind in ("re_query", "grep", "ad-hoc bash") else None
                elif name == "Grep":
                    cat, sim = "out: grep", capped(txt, a.cap_lines, a.cap_bytes)
                elif name == "Read":
                    cat, sim = "out: Read", None
                else:
                    cat, sim = "out: " + ("other tools" if name not in ("Write", "Edit") else "Write/Edit"), None
                events.append((k, cat, len(txt), sim))
        elif t == "attachment":
            k = len(calls)
            events.append((k, "prompt and user text", len(json.dumps(r.get("attachment"), ensure_ascii=False)),
                           None))
    n = len(calls)
    if not n:
        return
    total_tokens = sum(usage.values())
    acc["calls"] += n
    acc["tokens"] += total_tokens
    for k, cat, now, sim in events:
        w = n - k  # calls after the one that produced it
        if w <= 0:
            continue
        acc["now"][cat] += now * w
        acc["sim"][cat] += (now if sim is None else sim) * w


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("paths", nargs="+")
    ap.add_argument("--cap-lines", type=int, default=60)
    ap.add_argument("--cap-bytes", type=int, default=4096)
    ap.add_argument("--script-lines", type=int, default=20)
    ap.add_argument("--preamble", type=int, default=7800)
    a = ap.parse_args()
    files = []
    for p in a.paths:
        files += [p] if os.path.isfile(p) else glob.glob(os.path.join(p, "**", "agent-*.jsonl"), recursive=True)
    acc = {"calls": 0, "tokens": 0, "now": defaultdict(int), "sim": defaultdict(int)}
    for f in sorted(files):
        analyse(f, a, acc)
    pre = a.preamble * acc["calls"]
    body = acc["tokens"] - pre
    chars = sum(acc["now"].values())
    if not chars or body <= 0:
        sys.exit("ctx_weight: no usable transcripts")
    tpc = body / chars  # tokens per weighted char, calibrated
    print(f"{len(files)} transcripts, {acc['calls']} calls, {acc['tokens'] / 1e6:.1f}M context tokens re-sent "
          f"({pre / 1e6:.1f}M preamble); {1 / tpc:.2f} chars/token")
    print(f"caps: {a.cap_lines} lines / {a.cap_bytes} B on re_query, grep and ad-hoc bash output; "
          f"inline scripts over {a.script_lines} lines as file + Edit; Write of a known file as Edit\n")
    print(f"{'category':40s} {'now Mtok':>9s} {'share':>6s} {'capped':>9s} {'saved':>7s}")
    cats = sorted(acc["now"], key=lambda c: -acc["now"][c])
    tot_now = acc["tokens"]
    tot_sim = pre
    for c in cats:
        now, sim = acc["now"][c] * tpc, acc["sim"][c] * tpc
        tot_sim += sim
        print(f"{c:40s} {now / 1e6:9.1f} {now / tot_now:6.1%} {sim / 1e6:9.1f} {(now - sim) / 1e6:7.1f}")
    print(f"{'fixed preamble':40s} {pre / 1e6:9.1f} {pre / tot_now:6.1%} {pre / 1e6:9.1f} {0:7.1f}")
    print(f"\ntotal {tot_now / 1e6:.1f}M -> {tot_sim / 1e6:.1f}M re-sent tokens: "
          f"{1 - tot_sim / tot_now:.1%} fewer (same calls; fewer calls from shorter context are not counted)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

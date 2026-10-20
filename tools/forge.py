#!/usr/bin/env python3
"""forge: implement one FreeCity4 plugin class from its spec with direct model calls, no agent loop.

A bounded, scripted pipeline: one deterministic context pack per class, ONE generation call that writes the class's
files, then at most N build/fix rounds that send back only the trimmed compiler errors and the files concerned.
Every call's token usage and cost is appended to build-runs/forge/<class>.jsonl.

  python tools/forge.py pack    <class>                  # build the pack, print its token estimate (no API call)
  python tools/forge.py setup                            # make the scratch engine copy and configure it
  python tools/forge.py run     <class> [--provider openrouter|anthropic] [--model M] [--rounds 3] [--reasoning low]
  python tools/forge.py run     <class> --replay <response.txt>   # parse + build a saved response, no API call
  python tools/forge.py compare <class> [--run <dir>]    # generated files against the implementation in engine/
  python tools/forge.py game    <class> --which forged|baseline [--run <dir>]  # queue a game run of that DLL
  python tools/forge.py report                           # one table over every build-runs/forge/*.jsonl

Clean room (the engine-lane rules of .claude/workflows/fc4-lane.js): the pack is built only from docs/specs/,
docs/abi/ (vtable and target address columns dropped) and engine/ (the template class and headers); a path under
decomp/, re/, ghidra/, tools/, external/, docs/subsystems/ or docs/sim-engine.md refuses the pack.

Generated files never land in engine/: they go to build-runs/forge/<class>/<run>/ and are copied into a scratch copy
of engine/ (build-runs/forge/engine, its own build-x86) for the build, then the scratch's originals are restored.

Keys: OPENROUTER_API_KEY or ANTHROPIC_API_KEY (or --key-env NAME). They are read from the environment, sent only in
the request header and never printed or logged. Standard-library Python 3 only.
"""

import argparse
import datetime
import difflib
import json
import os
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FORGE = ROOT / "build-runs" / "forge"
SCRATCH = FORGE / "engine"
MAIN = Path(r"C:\Users\tlori\FreeCity4")  # the main checkout: the SDK copy (untracked there) and the game queue
VS = Path(r"C:\Program Files\Microsoft Visual Studio\2022\Community")

FORBIDDEN = ("decomp", "re/", "ghidra/", "tools/", "external/", "docs/subsystems/", "docs/sim-engine.md")
SDK_INC = "engine/third_party/gzcom-dll/gzcom-dll/include"
BIG_HEADER = 12_000  # bytes; larger headers are sliced to the declarations the class needs
CHARS_PER_TOKEN = 3.6

# Pilot classes: what the pack needs that is not derivable from the class name alone. `files` are the outputs,
# `interfaces` the docs/abi/interfaces.tsv rows, `seeds` the identifiers that select declarations from the big
# headers (fc4.h, FC4Interfaces.h, FC4Config.h), `ini` the game-run switches that exercise the class (the replace
# run: the <x>_shadow switches compare the ORIGINAL class with the core and are inactive once ours is enabled, so
# they cannot check a plugin class), `log` the plugin-log lines that `gamediff` compares between two runs.
PRESETS = {
    "cFC4History": {
        "files": ["engine/plugin/cFC4History.h", "engine/plugin/cFC4History.cpp"],
        "specs": ["history.md"],
        "interfaces": ["cISC4HistoryWarehouse"],
        "seeds": [r"\bfc4_history\w*", r"HistoryWarehouse", r"\bCoreLock\b"],
        "ini": ["enable = history"], "log": r"\bhistory\b",
    },
    "cFC4Dispatch": {
        "files": ["engine/plugin/cFC4Dispatch.h", "engine/plugin/cFC4Dispatch.cpp"],
        "specs": ["dispatch.md"],
        "interfaces": ["cISC4DispatchManager"],
        "seeds": [r"\bfc4_dispatch\w*", r"DispatchManager", r"DispatchSource", r"\bCoreLock\b"],
        "ini": ["enable = dispatch"], "log": r"\bdispatch\b",
    },
    "cFC4Parks": {
        "files": ["engine/plugin/cFC4Parks.h", "engine/plugin/cFC4Parks.cpp"],
        "specs": ["parks.md"],
        "interfaces": ["cISC4ParkManager"],
        "seeds": [r"\bfc4_parks?\w*", r"ParkManager", r"\b[Pp]arks?\b", r"\bCoreLock\b"],
        "ini": ["enable = parks"], "log": r"\bparks?\b",
    },
}
TEMPLATE = ["engine/plugin/cFC4Aura.h", "engine/plugin/cFC4Aura.cpp"]

# $/MTok for providers that do not report the cost (Anthropic): input, output, cache read, cache write (5 min).
PRICES = {
    "claude-opus-5-5": (4.00, 20.00, 0.20, 5.00),
    "claude-sonnet-5-5": (2.00, 10.00, 0.20, 2.50),
    "claude-haiku-5-5": (0.10, 0.50, 0.01, 0.125),
}

# The static block: byte-identical for every class (no class name, date or path), so a provider can cache it.
RULES = """You are a clean-room ENGINE IMPLEMENTER for FreeCity4 (OpenSC4), a GPL-3.0 reimplementation of SimCity 4 \
Deluxe: a Rust core behind a C ABI (fc4.h), and a C++ GZCOM plugin loaded by the real 32-bit game. You write ONE \
plugin class: the GZCOM object that adapts the game's interface to its Rust core through the fc4_* C API.

Sources: only what this conversation gives you - the behaviour spec (docs/specs), the interface's ABI rows \
(docs/abi), the project and SDK headers the class includes (big ones sliced to the declarations it needs), one \
existing class of this codebase as a template for conventions, and the contract (how the rest of the code uses the \
class's header). Nothing else exists for you. Never guess from other sources.

Rules:
- Behaviour comes from the spec. Where it is silent or ambiguous, take the most literal reading and mark it with a \
`// spec silent:` comment.
- Follow the template's conventions: reference counting, QueryInterface, the core lock, logging, the message target, \
serialization through the fc4 stream adapters, the factory function at the end of the header.
- Never Release a pointer you did not AddRef or create (FC4_RELEASE_CREATED for temporaries the game creates for \
you, FC4_ACQUIRED/FC4_RELEASE for references you take).
- The game calls getters from a second thread during OnTick: core access goes under the class's lock. Read runs \
before Init: never size saved state from the city in Read. Other subsystems only through the game's interfaces.
- Keep every name the contract uses (class, factory, constants, members used by other files) exactly.
- MSVC 2022, 32-bit, C++17, /permissive-, /W3, /utf-8. Only call functions and members declared in the given \
headers.

Output format, nothing else (no prose, no markdown fences). Each file complete, as:
<<<FILE path/relative/to/repo>>>
...file content...
<<<END>>>"""


def now():
    return datetime.datetime.now().isoformat(timespec="seconds")


def read(path):
    return Path(path).read_text(encoding="utf-8", errors="replace")


def rel(p):
    return Path(p).resolve().relative_to(ROOT.resolve()).as_posix()


def guard(path):
    r = path if isinstance(path, str) else rel(path)
    r = r.replace("\\", "/")
    if any(r.startswith(f) or f"/{f}" in f"/{r}" for f in FORBIDDEN) or not (
            r.startswith("docs/specs/") or r.startswith("docs/abi/") or r.startswith("engine/")):
        raise SystemExit(f"forge: clean-room refusal: {r} may not go into an engine prompt")
    return r


def tokens(text):
    return int(len(text) / CHARS_PER_TOKEN)


# ---- header slicing -------------------------------------------------------------------------------------------

def units(text):
    """Split a C/C++ header into top-level declaration units (comments attach to the declaration after them)."""
    out, cur, depth, in_c = [], [], 0, False
    for line in text.split("\n"):
        cur.append(line)
        s = line.strip()
        if not in_c and re.match(r'^(namespace\b[^{;]*|extern\s+"C"\s*)\{\s*(//.*)?$', s):
            out.append("\n".join(cur)); cur = []
            continue
        i, n = 0, len(line)
        while i < n:
            two = line[i:i + 2]
            if in_c:
                if two == "*/":
                    in_c = False; i += 2; continue
                i += 1; continue
            if two == "/*":
                in_c = True; i += 2; continue
            if two == "//":
                break
            c = line[i]
            if c in "\"'":
                j = i + 1
                while j < n and line[j] != c:
                    j += 2 if line[j] == "\\" else 1
                i = j + 1; continue
            if c == "{":
                depth += 1
            elif c == "}":
                depth = max(0, depth - 1)
            i += 1
        if depth == 0 and not in_c and (not s or s.startswith("#") or s.endswith((";", "}", "};"))
                                        or re.match(r"^}\s*(//.*)?$", s)):
            out.append("\n".join(cur)); cur = []
    if cur:
        out.append("\n".join(cur))
    return [u for u in out if u.strip()]


DEF_RX = [r"\btypedef\s+(?:struct|enum|union)\s+\w+\s*\{[^}]*?\}\s*(\w+)\s*;", r"\btypedef\s+[^;{]*?\b(\w+)\s*;",
          r"\b(?:class|struct|enum(?:\s+class)?|union)\s+(\w+)\b[^;{]*\{", r"\}\s*(\w+)\s*;\s*$", r"#define\s+(\w+)",
          r"\b(\w+)\s*\([^;{]*\)\s*(?:const\s*)?(?:;|\{)", r"\bstatic\s+const\w*\s+\w+\s+(\w+)\s*=",
          r"\bconstexpr\s+\w+\s+(\w+)\s*="]
REF_RX = re.compile(r"\b(fc4_\w+|cFC4\w+|kFC4\w+|FC4_\w+|FC4\w+|fc4::\w+|SC4\w+|cS3D\w+)\b")


def defines(u):
    names = set()
    for rx in DEF_RX:
        names.update(m for m in re.findall(rx, u, re.S) if m)
    return names


def code_of(u):
    """A unit without its comments: seeds and references are matched on code only."""
    u = re.sub(r"/\*.*?\*/", " ", u, flags=re.S)
    return re.sub(r"//[^\n]*", " ", u)


def slice_header(text, seeds, budget, closure_depth=2):
    us = units(text)
    code = [code_of(u) for u in us]
    defs = [defines(c) for c in code]
    rx = re.compile("|".join(seeds))
    take = {i for i, c in enumerate(code) if rx.search(c) and not c.lstrip().startswith("#")}
    frontier = set(take)
    for _ in range(closure_depth):
        refs = set()
        for i in frontier:
            refs.update(r.split("::")[-1] for r in REF_RX.findall(code[i]))
        new = {i for i, d in enumerate(defs) if i not in take and d & refs}
        take |= new
        frontier = new
    picked, size = [], 0
    for i in sorted(take):
        if size + len(us[i]) > budget:
            continue
        picked.append(us[i]); size += len(us[i]) + 1
    return "\n".join(picked), len(picked), len(us)


# ---- the pack -------------------------------------------------------------------------------------------------

def includes_of(paths):
    inc = []
    for p in paths:
        if (ROOT / p).exists():
            for m in re.findall(r'^\s*#include\s+"([^"]+)"', read(ROOT / p), re.M):
                if m not in inc:
                    inc.append(m)
    return inc


def resolve_header(name):
    for d in ("engine/plugin", SDK_INC, "engine/capi/include"):
        p = ROOT / d / name
        if p.exists():
            return p
    p = MAIN / SDK_INC / name  # a worktree has no submodule checkout: the SDK headers (public, GPL) from main
    if p.exists():
        return p
    return None


def abi_rows(interfaces):
    t = read(ROOT / "docs/abi/interfaces.tsv").splitlines()
    head = t[0].split("\t")
    keep = [i for i, h in enumerate(head) if h not in ("vtable", "target", "notes")]
    rows = ["\t".join(head[i] for i in keep)]
    for line in t[1:]:
        c = line.split("\t")
        if c and c[0] in interfaces:
            rows.append("\t".join(c[i] for i in keep if i < len(c)))
    return "\n".join(rows)


def contract(cls, files):
    """How the rest of engine/ uses the class's header: every line outside the class's files that names an
    identifier the header defines (pilot: taken from the existing header; forward use passes --contract)."""
    hdr = [f for f in files if f.endswith(".h")]
    names = set()
    for h in hdr:
        if (ROOT / h).exists():
            for u in units(read(ROOT / h)):
                c = code_of(u)
                m = re.search(r"\b(?:class|struct)\s+(\w+)\b[^;{]*\{", c)
                # a class or struct: only its name (its members are the class's own business); else every name
                names |= {m.group(1)} if m else {n for n in defines(c) if len(n) > 4}
    names |= {Path(h).name for h in hdr}
    if not names:
        return ""
    rx = re.compile("|".join(sorted((re.escape(n) for n in names), key=len, reverse=True)))
    own = {Path(f).name for f in files}
    lines = []
    for p in sorted(list((ROOT / "engine/plugin").glob("*")) + list((ROOT / "engine/tests").glob("*"))
                    + [ROOT / "engine/CMakeLists.txt"]):
        if p.suffix not in (".cpp", ".h", ".txt") or p.name in own:
            continue
        try:
            text = read(p)
        except OSError:
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if rx.search(code_of(line)):
                lines.append(f"{rel(p)}:{n}: {line.strip()[:200]}")
    return "\n".join(lines[:120])


def build_pack(cls, preset, template=TEMPLATE, header_budget=60_000):
    files = preset["files"]
    sections = []  # (title, text) in prompt order; the first block (headers) is shared with the fix rounds

    inc = [h for h in includes_of(files) if Path(h).name not in {Path(f).name for f in files}]
    hdr_parts, hdr_stats = [], []
    for h in inc:
        p = resolve_header(h)
        if p is None:
            hdr_stats.append(f"{h}: not found"); continue
        r = guard(p) if str(p).startswith(str(ROOT)) else guard(SDK_INC + "/" + h)
        text = read(p)
        if len(text) > BIG_HEADER:
            text, k, n = slice_header(text, preset["seeds"], budget=header_budget // 3)
            hdr_stats.append(f"{h}: sliced {k}/{n} units, {len(text)} B")
            text = f"// (sliced: the {k} of {n} declarations this class needs)\n{text}"
        else:
            hdr_stats.append(f"{h}: {len(text)} B")
        hdr_parts.append(f"=== header {h} ===\n{text}")
    sections.append(("headers", "\n\n".join(hdr_parts)))
    sections.append(("abi", "=== docs/abi/interfaces.tsv rows (game vtable order; addresses dropped) ===\n"
                     + abi_rows(preset["interfaces"])))
    sections.append(("contract", "=== contract: lines elsewhere in engine/ that use this class's header ===\n"
                     + (contract(cls, files) or "(none)")))
    for s in preset["specs"]:
        guard(f"docs/specs/{s}")
        sections.append((f"spec {s}", f"=== spec docs/specs/{s} ===\n" + read(ROOT / "docs/specs" / s)))
    for t in template:
        guard(t)
        sections.append((f"template {Path(t).name}", f"=== template {t} (another class; conventions only) ===\n"
                         + read(ROOT / t)))
    task = (f"Write the class {cls} as the files {', '.join(files)}, implementing the interface(s) "
            f"{', '.join(preset['interfaces'])} per the spec(s) {', '.join(preset['specs'])} over the core's fc4_* API "
            f"in the headers above. Output both files complete in the format of the rules.")
    return sections, task, hdr_stats


def print_pack(sections, task, hdr_stats):
    total = 0
    for title, text in sections:
        t = tokens(text); total += t
        print(f"  {title:<34} {len(text):>8} B  ~{t:>6} tok")
    total += tokens(RULES) + tokens(task)
    print(f"  {'rules + task':<34} {len(RULES) + len(task):>8} B  ~{tokens(RULES) + tokens(task):>6} tok")
    print(f"  {'TOTAL':<34} {'':>8}    ~{total:>6} tok (estimate: chars / {CHARS_PER_TOKEN})")
    for s in hdr_stats:
        print(f"    - {s}")
    return total


# ---- providers ------------------------------------------------------------------------------------------------

def claude_bin(arg):
    """The claude CLI: --claude-bin, FORGE_CLAUDE_BIN, `claude` on PATH, else the newest copy the desktop app
    bundles. A .py path runs under this Python (the test stub)."""
    c = arg or os.environ.get("FORGE_CLAUDE_BIN") or shutil.which("claude")
    if not c:
        base = Path(os.environ.get("LOCALAPPDATA", "")) / "Claude-3p" / "claude-code"
        found = sorted(base.glob("*/*/claude.exe"), key=lambda p: [int(x) if x.isdigit() else 0
                                                                  for x in p.parent.parent.name.split(".")])
        if not found:
            raise SystemExit("forge: no claude CLI (npm i -g @anthropic-ai/claude-code, or pass --claude-bin)")
        c = str(found[-1])
    c = str(Path(c).resolve()) if Path(c).exists() else c  # the call runs in another cwd
    return [sys.executable, c] if c.endswith(".py") else [c]


class Provider:
    def __init__(self, a):
        self.kind = a.provider
        self.reasoning = a.reasoning
        self.max_tokens = a.max_tokens
        if self.kind == "claude-code":
            # headless Claude Code: whatever login the CLI has (a subscription, or a gateway); no key here
            self.model = a.model or "default"
            self.bin = claude_bin(a.claude_bin)
            self.settings = a.cc_settings
            self.key, self.env = "-", "claude-code login"
            return
        if self.kind == "openrouter":
            self.model = a.model or "z-ai/glm-5.3-flash"
            self.url = (a.base_url or "https://openrouter.ai/api/v1").rstrip("/") + "/chat/completions"
            env = a.key_env or "OPENROUTER_API_KEY"
        else:
            self.model = a.model or "claude-opus-5-5"
            self.url = (a.base_url or "https://api.anthropic.com").rstrip("/") + "/v1/messages"
            env = a.key_env or "ANTHROPIC_API_KEY"
        self.key = os.environ.get(env, "")
        self.env = env
        # or a file outside the repo (a running session cannot see an env var set after it started)
        kf = Path(a.key_file or os.environ.get("FORGE_KEY_FILE") or Path.home() / ".config" / "fc4-forge" /
                  f"{self.kind}.key")
        if not self.key and kf.is_file():
            if str(kf.resolve()).lower().startswith(str(ROOT.resolve()).lower()):
                raise SystemExit("forge: refusing a key file inside the repository")
            self.key = kf.read_text(encoding="utf-8").strip()
            self.env = f"{env} (from {kf})"
        self.reasoning = a.reasoning
        self.max_tokens = a.max_tokens
        self.provider_order = [x for x in (a.provider_order or "DigitalOcean").split(",") if x]

    def check(self):
        if not self.key:
            raise SystemExit(f"forge: no key: set {self.env} or write it to ~/.config/fc4-forge/{self.kind}.key")

    def call(self, blocks, task):
        """blocks: [(text, cache)] - the stable prefix, in order; task: the varying tail. Returns (text, usage)."""
        if self.kind == "claude-code":
            return self._claude_code("\n\n".join(b for b, _ in blocks) + "\n\n" + task)
        if self.kind == "openrouter":
            body = {"model": self.model, "max_tokens": self.max_tokens, "usage": {"include": True},
                    "provider": {"order": self.provider_order, "allow_fallbacks": False},
                    "messages": [{"role": "system", "content": RULES},
                                 {"role": "user", "content": "\n\n".join(b for b, _ in blocks) + "\n\n" + task}]}
            if self.reasoning:
                body["reasoning"] = {"effort": self.reasoning}
            hdr = {"Authorization": "Bearer " + self.key, "Content-Type": "application/json",
                   "X-Title": "FreeCity4 forge"}
            r = self._post(body, hdr)
            choice = (r.get("choices") or [{}])[0]
            msg = choice.get("message") or {}
            u = r.get("usage") or {}
            ptd = u.get("prompt_tokens_details") or {}
            ctd = u.get("completion_tokens_details") or {}
            usage = {"input_tokens": u.get("prompt_tokens"), "prompt_total": u.get("prompt_tokens"),
                     "output_tokens": u.get("completion_tokens"),
                     "cached_tokens": ptd.get("cached_tokens"), "cache_write_tokens": ptd.get("cache_write_tokens"),
                     "reasoning_tokens": ctd.get("reasoning_tokens"), "cost_usd": u.get("cost"),
                     "cost_source": "api" if u.get("cost") is not None else "none",
                     "finish": choice.get("finish_reason"), "served_by": r.get("provider"), "id": r.get("id")}
            return msg.get("content") or "", usage
        content = [{"type": "text", "text": b, **({"cache_control": {"type": "ephemeral"}} if c else {})}
                   for b, c in blocks] + [{"type": "text", "text": task}]
        body = {"model": self.model, "max_tokens": self.max_tokens, "stream": True,
                "system": [{"type": "text", "text": RULES}], "messages": [{"role": "user", "content": content}]}
        if self.reasoning:
            body["output_config"] = {"effort": self.reasoning}
        hdr = {"x-api-key": self.key, "anthropic-version": "2023-06-01", "Content-Type": "application/json"}
        text, u, stop = self._stream_anthropic(body, hdr)
        p = PRICES.get(self.model)
        cost = None
        if p:
            cost = ((u.get("input_tokens") or 0) * p[0] + (u.get("output_tokens") or 0) * p[1]
                    + (u.get("cache_read_input_tokens") or 0) * p[2]
                    + (u.get("cache_creation_input_tokens") or 0) * p[3]) / 1e6
        usage = {"input_tokens": u.get("input_tokens"), "output_tokens": u.get("output_tokens"),
                 "cached_tokens": u.get("cache_read_input_tokens"),
                 "cache_write_tokens": u.get("cache_creation_input_tokens"), "reasoning_tokens": None,
                 "prompt_total": sum(u.get(k) or 0 for k in ("input_tokens", "cache_read_input_tokens",
                                                             "cache_creation_input_tokens")),
                 "cost_usd": cost, "cost_source": "computed (PRICES)" if p else "none", "finish": stop}
        return text, usage

    def cc_command(self):
        """One headless turn, stripped down: our system prompt instead of Claude Code's, no tools, no MCP servers,
        no skills/slash commands, no session file; project/local settings off (the cwd is an empty folder anyway),
        user settings kept because a gateway (e.g. OpenRouter) is configured there. Not --bare: it refuses OAuth,
        i.e. a subscription login."""
        cmd = self.bin + ["-p", "--output-format", "json", "--system-prompt", RULES, "--tools", "",
                          "--strict-mcp-config", "--mcp-config", '{"mcpServers":{}}', "--disable-slash-commands",
                          "--no-session-persistence", "--setting-sources", self.settings]
        if self.model != "default":
            cmd += ["--model", self.model]
        if self.reasoning:
            cmd += ["--effort", self.reasoning]
        return cmd

    def _claude_code(self, prompt):
        cwd = Path(os.environ.get("TEMP") or "/tmp") / "fc4-forge-cc"  # outside the repo: no CLAUDE.md, no memory
        cwd.mkdir(parents=True, exist_ok=True)
        # a nested run from inside a Claude Code session must not inherit that session's markers
        env = {k: v for k, v in os.environ.items() if not (k.startswith("CLAUDE_CODE_") or k == "CLAUDECODE")}
        p = subprocess.run(self.cc_command(), input=prompt.encode("utf-8"), capture_output=True, cwd=cwd, env=env,
                           timeout=1800)
        out = p.stdout.decode("utf-8", "replace").strip()
        try:
            r = json.loads(out.splitlines()[-1] if out else "{}")
        except ValueError:
            r = {}
        if p.returncode != 0 or not r or r.get("is_error"):
            err = (p.stderr.decode("utf-8", "replace") + "\n" + out)[-800:]
            raise SystemExit(f"forge: claude-code call failed (exit {p.returncode}): {err.strip()}")
        u = r.get("usage") or {}
        models = r.get("modelUsage") or {}
        usage = {"input_tokens": u.get("input_tokens"), "output_tokens": u.get("output_tokens"),
                 "cached_tokens": u.get("cache_read_input_tokens"),
                 "cache_write_tokens": u.get("cache_creation_input_tokens"), "reasoning_tokens": None,
                 "prompt_total": sum(u.get(k) or 0 for k in ("input_tokens", "cache_read_input_tokens",
                                                             "cache_creation_input_tokens")),
                 "cost_usd": r.get("total_cost_usd"),
                 "cost_source": "claude-code total_cost_usd (API-price equivalent; a subscription is not billed "
                                "per call)",
                 "finish": r.get("subtype"), "turns": r.get("num_turns"), "served_by": ",".join(models) or None,
                 "id": r.get("session_id")}
        return r.get("result") or "", usage

    def _request(self, body, hdr):
        data = json.dumps(body).encode("utf-8")
        for attempt in range(4):
            req = urllib.request.Request(self.url, data=data, headers=hdr, method="POST")
            try:
                return urllib.request.urlopen(req, timeout=1200)
            except urllib.error.HTTPError as e:
                msg = e.read().decode("utf-8", "replace")[:600]
                if e.code in (429, 500, 502, 503, 504, 529) and attempt < 3:
                    time.sleep(10 * (attempt + 1)); continue
                raise SystemExit(f"forge: HTTP {e.code} from {self.kind}: {msg}")
            except (urllib.error.URLError, TimeoutError) as e:
                if attempt < 3:
                    time.sleep(10 * (attempt + 1)); continue
                raise SystemExit(f"forge: {self.kind} unreachable: {e}")

    def _post(self, body, hdr):
        with self._request(body, hdr) as resp:
            r = json.loads(resp.read().decode("utf-8"))
        if "error" in r:
            raise SystemExit(f"forge: {self.kind} error: {json.dumps(r['error'])[:600]}")
        return r

    def _stream_anthropic(self, body, hdr):
        parts, usage, stop = [], {}, None
        with self._request(body, hdr) as resp:
            for raw in resp:
                line = raw.decode("utf-8", "replace").strip()
                if not line.startswith("data:"):
                    continue
                ev = json.loads(line[5:].strip() or "{}")
                t = ev.get("type")
                if t == "message_start":
                    usage.update(ev["message"].get("usage") or {})
                elif t == "content_block_delta" and ev["delta"].get("type") == "text_delta":
                    parts.append(ev["delta"]["text"])
                elif t == "message_delta":
                    usage.update({k: v for k, v in (ev.get("usage") or {}).items() if v is not None})
                    stop = (ev.get("delta") or {}).get("stop_reason", stop)
                elif t == "error":
                    raise SystemExit(f"forge: anthropic stream error: {json.dumps(ev)[:600]}")
        return "".join(parts), usage, stop


# ---- parse, build, errors -------------------------------------------------------------------------------------

FILE_RX = re.compile(r"<<<FILE\s+([^>\s]+)\s*>>>\s*\n(.*?)\n?<<<END>>>", re.S)


def parse_files(text, allowed):
    out = {}
    for path, body in FILE_RX.findall(text):
        path = path.strip().replace("\\", "/")
        body = re.sub(r"^```[a-zA-Z+]*\n", "", body)
        body = re.sub(r"\n```\s*$", "", body)
        if path in allowed:
            out[path] = body.rstrip() + "\n"
    return out


def vcvars_cmd(inner):
    vc = VS / "VC/Auxiliary/Build/vcvars32.bat"
    return f'"{vc}" >nul 2>&1 && {inner}'


def run_cmd(inner, timeout=3600):
    env = dict(os.environ)
    env["PATH"] = str(VS / "Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin") + ";" + env.get("PATH", "")
    env["_MSPDBSRV_ENDPOINT_"] = "fc4forge"
    env["CARGO_INCREMENTAL"] = "0"
    # one string, cmd's own quoting (cmd /c ""C:\...\vcvars32.bat" >nul && ..."): a list would be \"-escaped
    p = subprocess.run(f'cmd /c "{vcvars_cmd(inner)}"', capture_output=True, env=env, timeout=timeout)
    out = (p.stdout + p.stderr).decode("utf-8", "replace").replace("\0", "")
    return p.returncode, out


def cmd_setup(a):
    """Create the scratch engine, or bring an existing one in line with engine/ (every tracked file that differs)."""
    fresh = not SCRATCH.exists()
    files = subprocess.run(["git", "-C", str(ROOT), "ls-files", "engine"], capture_output=True, text=True,
                           check=True).stdout.split()
    synced = 0
    for f in files:
        if f.startswith("engine/third_party/gzcom-dll"):
            continue
        dst = FORGE / f
        if dst.exists() and dst.read_bytes() == (ROOT / f).read_bytes():
            continue
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / f, dst)
        synced += 1
    if fresh:
        shutil.copytree(MAIN / "engine/third_party/gzcom-dll", SCRATCH / "third_party/gzcom-dll", dirs_exist_ok=True)
    print(f"forge: scratch engine {rel(SCRATCH)}: {synced} file(s) {'copied' if fresh else 'synced from engine/'}")
    code, out = run_cmd(f'cd /d "{SCRATCH}" && cmake -S . -B build-x86 -G Ninja')
    print("\n".join(out.splitlines()[-3:]))
    print(f"configure exit={code}")
    return code


def build(timeout=3600):
    t0 = time.time()
    code, out = run_cmd(f'cd /d "{SCRATCH}" && cmake --build build-x86 --target FreeCity4 -- -k 0', timeout)
    (FORGE / "last-build.txt").write_text(out, encoding="utf-8")
    return code == 0 and "FAILED:" not in out, out, round(time.time() - t0, 1)


ERR_RX = re.compile(r"^(?P<file>[A-Za-z]:\\[^(]+|[^\s(]+)\((?P<line>\d+)(?:,\d+)?\)\s*:\s*(?P<kind>fatal error|error)\s+"
                    r"(?P<code>[A-Z]+\d+)\s*:\s*(?P<msg>.*)$")
LNK_RX = re.compile(r"(error LNK\d+:.*|fatal error LNK\d+:.*)$")


def trim_errors(out, limit=40):
    """The first `limit` distinct compiler/linker errors, paths made repo-relative; returns (text, [files])."""
    seen, lines, files = set(), [], []
    scr = str(SCRATCH.resolve()).lower()
    for line in out.splitlines():
        m = ERR_RX.match(line.strip())
        if m:
            f = m.group("file")
            fl = str(Path(f).resolve()).lower() if re.match(r"^[A-Za-z]:\\", f) else f.lower()
            if fl.startswith(scr):
                f = "engine/" + Path(f).resolve().relative_to(SCRATCH.resolve()).as_posix()
            key = (f, m.group("line"), m.group("code"))
            if key in seen:
                continue
            seen.add(key)
            lines.append(f"{f}({m.group('line')}): {m.group('kind')} {m.group('code')}: {m.group('msg')[:300]}")
            if f not in files:
                files.append(f)
        else:
            m = LNK_RX.search(line)
            if m and m.group(1) not in seen:
                seen.add(m.group(1)); lines.append(m.group(1)[:400])
        if len(lines) >= limit:
            lines.append("(more errors cut)"); break
    return "\n".join(lines), files


def snippet(path, errs, ctx=4):
    """Lines around each error in a file the model did not write (e.g. another file using the class)."""
    p = SCRATCH / Path(path).relative_to("engine") if path.startswith("engine/") else None
    if not p or not p.exists():
        return ""
    src = read(p).splitlines()
    nums = sorted({int(n) for n in re.findall(re.escape(path) + r"\((\d+)\)", errs)})
    keep = set()
    for n in nums:
        keep.update(range(max(1, n - ctx), min(len(src), n + ctx) + 1))
    return "\n".join(f"{i:5}: {src[i - 1]}" for i in sorted(keep))


def install(files):
    for path, body in files.items():
        dst = SCRATCH / Path(path).relative_to("engine")
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_text(body, encoding="utf-8", newline="\n")


def restore(paths):
    for path in paths:
        src, dst = ROOT / path, SCRATCH / Path(path).relative_to("engine")
        if src.exists():
            shutil.copy2(src, dst)
        elif dst.exists():
            dst.unlink()


def log(cls, rec):
    FORGE.mkdir(parents=True, exist_ok=True)
    with open(FORGE / f"{cls}.jsonl", "a", encoding="utf-8") as f:
        f.write(json.dumps(rec) + "\n")


def spent_total():
    tot = 0.0
    for j in FORGE.glob("*.jsonl"):
        for line in read(j).splitlines():
            try:
                r = json.loads(line)
            except ValueError:
                continue
            if r.get("kind") == "call" and r.get("cost_usd"):
                tot += r["cost_usd"]
    return tot


def preset_of(a):
    p = dict(PRESETS.get(a.cls) or {})
    for k in ("files", "specs", "interfaces", "seeds"):
        v = getattr(a, k, None)
        if v:
            p[k] = v.split(",")
    missing = [k for k in ("files", "specs", "interfaces", "seeds") if not p.get(k)]
    if missing:
        raise SystemExit(f"forge: no preset for {a.cls}; pass --{', --'.join(missing)}")
    p.setdefault("ini", [])
    return p


PROBE = "Reply with the single word OK."


def cmd_probe(a):
    """The fixed cost of one call: what the backend sends besides our system prompt and the user text."""
    prov = Provider(a)
    prov.check()
    t0 = time.time()
    text, usage = prov.call([], PROBE)
    total = usage.get("prompt_total") or 0
    ours = tokens(RULES) + tokens(PROBE)
    rec = {"ts": now(), "kind": "probe", "provider": prov.kind, "model": prov.model, **usage, "prompt_total": total,
           "ours_estimate": ours, "latency_s": round(time.time() - t0, 1), "reply": text[:40]}
    log("probe", rec)
    print(f"forge probe {prov.kind} ({prov.model}): prompt {total} tok (ours ~{ours}) -> overhead ~{total - ours} "
          f"tok/call; out {usage.get('output_tokens')}; ${(usage.get('cost_usd') or 0):.5f} [{usage.get('cost_source')}]"
          f"; {rec['latency_s']}s; reply {text[:20]!r}")
    if prov.kind == "claude-code":
        shown = ["<RULES>" if x == RULES else x for x in prov.cc_command()[len(prov.bin):]]
        print("  command: claude " + " ".join(json.dumps(x) if (" " in x or not x) else x for x in shown)
              + " < prompt")
    return 0


def cmd_pack(a):
    pre = preset_of(a)
    sections, task, stats = build_pack(a.cls, pre)
    print(f"forge pack {a.cls}:")
    total = print_pack(sections, task, stats)
    out = FORGE / a.cls / "pack.md"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(RULES + "\n\n" + "\n\n".join(t for _, t in sections) + "\n\n" + task, encoding="utf-8")
    print(f"  written: {rel(out)}")
    if total > a.max_pack:
        print(f"forge: pack ~{total} tok is over --max-pack {a.max_pack}")
        return 1
    return 0


def cmd_run(a):
    pre = preset_of(a)
    files = pre["files"]
    sections, task, stats = build_pack(a.cls, pre)
    print(f"forge run {a.cls}:")
    est = print_pack(sections, task, stats)
    if est > a.max_pack:
        raise SystemExit(f"forge: pack ~{est} tok is over --max-pack {a.max_pack}")
    prov = Provider(a)
    if not a.replay:
        prov.check()
        spent = spent_total()
        if spent >= a.budget:
            raise SystemExit(f"forge: ${spent:.3f} already logged in build-runs/forge/*.jsonl >= --budget {a.budget}")
    run_id = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    while (FORGE / a.cls / run_id).exists():
        time.sleep(1)
        run_id = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    rdir = FORGE / a.cls / run_id
    rdir.mkdir(parents=True, exist_ok=True)
    (rdir / "pack.md").write_text("\n\n".join(t for _, t in sections) + "\n\n" + task, encoding="utf-8")
    # cacheable prefix: the headers/ABI/contract block first (shared with the fix rounds), then spec + template
    shared = "\n\n".join(t for title, t in sections if title in ("headers", "abi", "contract"))
    rest = "\n\n".join(t for title, t in sections if title not in ("headers", "abi", "contract"))
    totals = {"calls": 0, "input_tokens": 0, "output_tokens": 0, "cached_tokens": 0, "cost_usd": 0.0}
    base = {"class": a.cls, "run": run_id, "provider": prov.kind, "model": prov.model, "pack_estimate": est, "replay": bool(a.replay)}

    def call(round_no, blocks, tail):
        t0 = time.time()
        text, usage = prov.call(blocks, tail)
        (rdir / f"response-r{round_no}.txt").write_text(text, encoding="utf-8")
        rec = {"ts": now(), "kind": "call", "round": round_no, **base, **usage,
               "latency_s": round(time.time() - t0, 1)}
        log(a.cls, rec)
        totals["calls"] += 1
        totals["input_tokens"] += usage.get("prompt_total") or 0  # the whole prompt, cached parts included
        for k in ("output_tokens", "cached_tokens"):
            totals[k] += usage.get(k) or 0
        totals["cost_usd"] += usage.get("cost_usd") or 0.0
        print(f"  call r{round_no}: in {usage.get('prompt_total')} (cached {usage.get('cached_tokens')}) out "
              f"{usage.get('output_tokens')} (reasoning {usage.get('reasoning_tokens')}) "
              f"${(usage.get('cost_usd') or 0):.4f} [{usage.get('cost_source')}] {usage.get('finish')} "
              f"{rec['latency_s']}s")
        if totals["cost_usd"] > a.max_cost:
            raise SystemExit(f"forge: run cost ${totals['cost_usd']:.3f} over --max-cost {a.max_cost}")
        return text

    if a.replay:
        text = read(a.replay)
    else:
        text = call(0, [(shared, True), (rest, True)], task)
    got = parse_files(text, files)
    if not got:
        log(a.cls, {"ts": now(), "kind": "summary", **base, "ok": False, "why": "no files parsed", **totals})
        raise SystemExit(f"forge: no <<<FILE>>> blocks for {files} in the response ({rdir / 'response-r0.txt'})")
    current = {f: got.get(f, "") for f in files}
    ok, rounds, errs = False, 0, ""
    try:
        for rnd in range(a.rounds + 1):
            d = rdir / f"r{rnd}"
            for f, body in current.items():
                (d / f).parent.mkdir(parents=True, exist_ok=True)
                (d / f).write_text(body, encoding="utf-8", newline="\n")
            install({f: b for f, b in current.items() if b})
            ok, out, secs = build()
            errs, efiles = trim_errors(out)
            (d / "build-errors.txt").write_text(errs or ("ok" if ok else out[-4000:]), encoding="utf-8")
            n_err = len([l for l in errs.splitlines() if " error " in l or "error LNK" in l])
            log(a.cls, {"ts": now(), "kind": "build", "round": rnd, **base, "ok": ok, "errors": n_err,
                        "seconds": secs})
            print(f"  build r{rnd}: {'ok' if ok else f'{n_err} errors'} ({secs}s)")
            rounds = rnd
            if ok or rnd == a.rounds or a.replay:
                break
            if not errs:
                errs = "\n".join(out.splitlines()[-30:])
            others = "\n\n".join(f"=== {f} (not yours; the lines around the errors) ===\n{snippet(f, errs)}"
                                 for f in efiles if f not in current and snippet(f, errs))
            mine = "\n\n".join(f"<<<FILE {f}>>>\n{b}<<<END>>>" for f, b in current.items())
            fix = (f"Your files for {a.cls} do not build. Fix exactly these errors; keep the behaviour. Output every "
                   f"file you change, complete, in the format of the rules.\n\n=== errors (trimmed) ===\n{errs}\n\n"
                   f"{others}\n\n=== your current files ===\n{mine}")
            text = call(rnd + 1, [(shared, True)], fix)
            for f, b in parse_files(text, files).items():
                current[f] = b
    finally:
        restore(files)
    if ok:
        dll = SCRATCH / "build-x86" / "FreeCity4.dll"
        dst = rdir / "dll"
        dst.mkdir(exist_ok=True)
        # the DLL was built with the forged files installed: keep it for the game run
        for ext in (".dll", ".pdb"):
            if dll.with_suffix(ext).exists():
                shutil.copy2(dll.with_suffix(ext), dst / ("FreeCity4" + ext))
    cmp = compare(files, rdir / f"r{rounds}")
    summ = {"ts": now(), "kind": "summary", **base, "ok": ok, "fix_rounds": rounds, **totals, "compare": cmp}
    log(a.cls, summ)
    print(f"forge {a.cls}: build {'OK' if ok else 'FAILED'} after {rounds} fix round(s); {totals['calls']} calls, "
          f"in {totals['input_tokens']} / out {totals['output_tokens']} tok, ${totals['cost_usd']:.4f}")
    for f, c in cmp.items():
        print(f"  {f}: similarity {c['similarity']}, methods {c['methods']}, fc4 calls {c['fc4_calls']}")
    print(f"  files: {rel(rdir)}")
    # the scratch DLL now holds the original files' objects again only after a rebuild
    return 0 if ok else 1


# ---- compare --------------------------------------------------------------------------------------------------

def norm_lines(text):
    out = []
    in_c = False
    for line in text.splitlines():
        s = line.strip()
        if in_c:
            if "*/" in s:
                in_c = False
            continue
        if s.startswith("/*") and "*/" not in s:
            in_c = True; continue
        if not s or s.startswith("//") or (s.startswith("/*") and s.endswith("*/")):
            continue
        out.append(re.sub(r"\s+", " ", re.sub(r"//.*$", "", s)).strip())
    return [l for l in out if l]


def jacc(a, b):
    return f"{len(a & b)}/{len(a)} of the original's, {len(b - a)} extra"


def compare(files, gdir):
    res = {}
    for f in files:
        orig = read(ROOT / f) if (ROOT / f).exists() else ""
        gen = read(gdir / f) if (gdir / f).exists() else ""
        a, b = norm_lines(orig), norm_lines(gen)
        sim = difflib.SequenceMatcher(None, a, b, autojunk=False).ratio() if a and b else 0.0
        meth = lambda t: set(re.findall(r"\b(\w+)::(\w+)\s*\(", t))
        fc4 = lambda t: set(re.findall(r"\bfc4_\w+", t))
        res[f] = {"similarity": round(sim, 3), "lines": f"{len(b)} vs {len(a)} code lines",
                  "methods": jacc(meth(orig), meth(gen)), "fc4_calls": jacc(fc4(orig), fc4(gen))}
    return res


def latest_run(cls):
    d = FORGE / cls
    runs = sorted(p for p in d.iterdir() if p.is_dir() and re.match(r"\d{8}-\d{6}$", p.name)) if d.exists() else []
    if not runs:
        raise SystemExit(f"forge: no runs for {cls}")
    return runs[-1]


def cmd_compare(a):
    pre = preset_of(a)
    rdir = Path(a.run) if a.run else latest_run(a.cls)
    rs = sorted(p for p in rdir.iterdir() if p.is_dir() and re.match(r"r\d+$", p.name))
    for f, c in compare(pre["files"], rs[-1]).items():
        print(f"{f}: {json.dumps(c)}")
    return 0


# ---- game run -------------------------------------------------------------------------------------------------

def cmd_game(a):
    pre = preset_of(a)
    if a.which == "forged":
        rdir = Path(a.run) if a.run else latest_run(a.cls)
        src = rdir / "dll"
        if not (src / "FreeCity4.dll").exists():
            raise SystemExit(f"forge: {rel(rdir)} has no built DLL (the run did not build)")
    else:
        restore(pre["files"])
        ok, out, secs = build()
        if not ok:
            raise SystemExit("forge: the scratch baseline does not build:\n" + trim_errors(out)[0])
        src = FORGE / "baseline-dll"
        src.mkdir(exist_ok=True)
        for ext in (".dll", ".pdb"):
            p = SCRATCH / "build-x86" / ("FreeCity4" + ext)
            if p.exists():
                shutil.copy2(p, src / p.name)
    label = a.label or f"forge-{a.cls}-{a.which}-{datetime.datetime.now().strftime('%H%M%S')}"
    req = [f"label = {label}", f"dll = {rel(src / 'FreeCity4.dll').replace('/', chr(92))}", "turbo = 1",
           f"end_date = {a.end_date}", f"seconds = {a.seconds}", "[ini]"]
    ini = list(pre["ini"])
    for other in (a.with_ or "").split(","):  # the same switches for every run of a comparison
        ini += [x for x in PRESETS.get(other.strip(), {}).get("ini", []) if x not in ini]
    enables = [x.split("=", 1)[1].strip() for x in ini if x.startswith("enable")]
    req += ([f"enable = {', '.join(enables)}"] if enables else []) + [x for x in ini if not x.startswith("enable")]
    q = ROOT / "build-runs" / "game-queue"
    q.mkdir(parents=True, exist_ok=True)
    tmp = q / f"{label}.tmp"
    tmp.write_text("\n".join(req) + "\n", encoding="ascii")
    tmp.replace(q / f"{label}.req")
    print(f"forge: queued {label} (verdict: build-runs/game-results/{label}.verdict)")
    return 0


def log_lines(label, rx):
    """The run's plugin-log lines matching rx, without the timestamp/pid/tid prefix (a multiset, in order)."""
    p = ROOT / "build-runs" / "game-results" / f"{label}.log"
    if not p.exists():
        raise SystemExit(f"forge: no {rel(p)} (run not finished?)")
    r = re.compile(rx, re.I)
    out = []
    for line in read(p).replace("\0", "").splitlines():
        body = re.sub(r"^\S+ \S+ \[pid \d+ tid \d+\] ", "", line)
        if r.search(body):
            # object addresses differ per run (city=030E3014, 0x00E23390)
            out.append(re.sub(r"\b(?:0x)?[0-9A-Fa-f]{8}\b", "<ptr>", body))
    return out


def cmd_gamediff(a):
    """Compare the class's log lines of two runs (e.g. the baseline DLL and the forged one); exact = identical."""
    pre = preset_of(a)
    va = read(ROOT / "build-runs" / "game-results" / f"{a.a}.verdict").splitlines()[0]
    vb = read(ROOT / "build-runs" / "game-results" / f"{b}.verdict").splitlines()[0] if (b := a.b) else ""
    la, lb = log_lines(a.a, pre["log"]), log_lines(a.b, pre["log"])
    if la == lb:
        print(f"{a.cls}: {a.a} | {a.b}: EXACT ({len(la)} lines); {va} | {vb}")
        return 0
    # Two runs of the SAME DLL differ in the city's own state (occupants, records, the turbo end date), so compare
    # what the class decides: mask the city-driven counts and cut the month lines to the months both runs reached.
    def mask(lines):
        out = []
        for x in lines:
            x = re.sub(r"\boccupants=\d+", "occupants=#", x)
            x = re.sub(r"\b\d+ (points|records sent|queries)", r"# \1", x)
            out.append(x)
        return out
    ma, mb = mask(la), mask(lb)
    months = lambda L: [x for x in L if " month " in x]
    na, nb = len(months(ma)), len(months(mb))
    keep = min(na, nb)
    def cut(L):
        seen, out = 0, []
        for x in L:
            if " month " in x:
                seen += 1
                if seen > keep:
                    continue
            out.append(x)
        return out
    ca, cb = cut(ma), cut(mb)
    diff = list(difflib.unified_diff(ca, cb, a.a, a.b, n=0, lineterm=""))
    nd = sum(1 for d in diff[2:] if d[:1] in "+-")
    word = "MATCH" if not diff else f"DIFFERS ({nd} lines)"
    print(f"{a.cls}: {a.a} | {a.b}: {word} on the class's fields ({len(ca)} lines compared, {keep} common months; "
          f"city counters masked); {va} | {vb}")
    for d in diff[2:2 + a.show]:
        print("  " + d[:220])
    return 0 if not diff else 1


# ---- report ---------------------------------------------------------------------------------------------------

def cmd_report(a):
    rows = []
    for j in sorted(FORGE.glob("*.jsonl")):
        for line in read(j).splitlines():
            try:
                r = json.loads(line)
            except ValueError:
                continue
            if r.get("kind") == "summary" and not r.get("replay"):
                rows.append(r)
    print("| class | run | model | build | fix rounds | calls | input tok | cached | output tok | $ |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for r in rows:
        print(f"| {r['class']} | {r['run']} | {r['model']} | {'ok' if r['ok'] else 'FAILED'} | {r.get('fix_rounds')} "
              f"| {r.get('calls')} | {r.get('input_tokens')} | {r.get('cached_tokens')} | {r.get('output_tokens')} "
              f"| {r.get('cost_usd', 0):.4f} |")
    print(f"\nlogged spend: ${spent_total():.4f}")
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(prog="forge.py", description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    def backend_args(p):
        p.add_argument("--backend", "--provider", dest="provider", default="openrouter",
                       choices=("openrouter", "anthropic", "claude-code"),
                       help="openrouter (default), anthropic (Messages API, prompt caching), claude-code (headless "
                            "`claude -p`, the CLI's own login: subscription or gateway, no key)")
        p.add_argument("--model"); p.add_argument("--base-url"); p.add_argument("--key-env")
        p.add_argument("--key-file", help="a file holding the key (outside the repo)")
        p.add_argument("--provider-order", help="OpenRouter provider order (default DigitalOcean), no fallbacks")
        p.add_argument("--reasoning", help="low|medium|high (OpenRouter reasoning.effort / Anthropic and Claude "
                                           "Code effort)")
        p.add_argument("--max-tokens", type=int, default=32_000)
        p.add_argument("--claude-bin", help="the claude CLI (default: PATH, then the desktop app's copy)")
        p.add_argument("--cc-settings", default="user", help="claude-code --setting-sources (default user)")

    p = sub.add_parser("probe", help="one tiny call: the backend's fixed per-call overhead in input tokens")
    backend_args(p)
    for name in ("pack", "run", "compare", "game"):
        p = sub.add_parser(name)
        p.add_argument("cls")
        p.add_argument("--files"); p.add_argument("--specs"); p.add_argument("--interfaces"); p.add_argument("--seeds")
        p.add_argument("--max-pack", type=int, default=80_000)
        if name == "run":
            backend_args(p)
            p.add_argument("--rounds", type=int, default=3)
            p.add_argument("--max-cost", type=float, default=1.50, help="stop a run above this many dollars")
            p.add_argument("--budget", type=float, default=5.00, help="refuse when the logged total reaches this")
            p.add_argument("--replay", help="a saved response to parse and build instead of calling the API")
        if name in ("compare", "game"):
            p.add_argument("--run")
        if name == "game":
            p.add_argument("--which", choices=("forged", "baseline"), required=True)
            p.add_argument("--label"); p.add_argument("--end-date", default="2028-01-01")
            p.add_argument("--seconds", type=int, default=900)
            p.add_argument("--with", dest="with_", help="also the ini switches of these classes (comma list)")
    p = sub.add_parser("gamediff", help="the class's log lines in two game runs (labels a, b)")
    p.add_argument("cls"); p.add_argument("a"); p.add_argument("b"); p.add_argument("--show", type=int, default=20)
    p.add_argument("--files"); p.add_argument("--specs"); p.add_argument("--interfaces"); p.add_argument("--seeds")
    sub.add_parser("setup"); sub.add_parser("report")
    a = ap.parse_args(argv)
    return {"probe": cmd_probe, "pack": cmd_pack, "run": cmd_run, "compare": cmd_compare, "game": cmd_game, "setup": cmd_setup, "gamediff": cmd_gamediff,
            "report": cmd_report}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())

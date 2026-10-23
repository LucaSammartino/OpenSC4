"""Strict reviewed exact-entry evidence. No attribution or native algorithm lives here.

The caller supplies unchanged Attribution and row statuses. Invalid input fails the
measurement, never just the diagnostic. Hashes bind review to files, not semantics.
"""
import fnmatch
import json
import re
import subprocess
from pathlib import Path

STAT = ("specified", "implemented", "verified", "replaced")


def fail(message):
    raise ValueError("function evidence: " + message)


def exact(obj, keys, label):
    if not isinstance(obj, dict) or set(obj) != set(keys.split()):
        fail(label + ": wrong fields")


def words(value, label):
    if not isinstance(value, str) or not value.strip() or value.strip() == "-":
        fail(label + ": missing citation/text")


def load_json(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                fail("duplicate JSON key " + key)
            result[key] = value
        return result
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique)
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        fail(str(exc))


def resolve(att, owner, rule, rows, ledger, repo, text_hash, exe_hash,
            artifact, tracked=None):
    """Return (entry flags, owner-only flags, input hashes), or raise ValueError.

    artifact is the current export identity, computed by coverage.py, never by
    the ledger. Each flag is independent: implemented does not imply specified.
    """
    repo = Path(repo).resolve()
    if tracked is None:
        proc = subprocess.run(["git", "-C", str(repo), "ls-files", "-z"],
                              capture_output=True, check=True)
        tracked = set(proc.stdout.decode("utf-8").split("\0"))
    hashes = {}

    def file(path, prefixes):
        if (not isinstance(path, str) or "\\" in path or ":" in path
                or Path(path).is_absolute() or any(p in ("", ".", "..") for p in path.split("/"))
                or not any(path.startswith(p) for p in prefixes) or path not in tracked):
            fail(f"untracked/forbidden path {path!r}")
        target = (repo / path).resolve()
        if not target.is_relative_to(repo) or not target.is_file():
            fail(f"missing/escaped evidence {path}")
        hashes[path] = text_hash(target)
        return target

    ledger_path = Path(ledger).resolve()
    if not ledger_path.is_relative_to(repo):
        fail("ledger outside repository")
    file(ledger_path.relative_to(repo).as_posix(), ("tools/",))
    data = load_json(ledger_path)
    exact(data, "version claims", "ledger")
    if type(data["version"]) is not int or data["version"] != 1 or not isinstance(data["claims"], list):
        fail("unsupported ledger version/claims")
    baseline = {}
    for va in att.order:
        o = owner[va]
        row = next((r for r in rows if o and fnmatch.fnmatchcase(o, r["pattern"])), None)
        baseline[va] = {s: bool(row and row[s]) if o and o != "<shared>" and not o.startswith("lib:")
                        else False for s in STAT}
    flags = {va: dict(st) for va, st in baseline.items()}
    seen = set()
    for record in data["claims"]:
        exact(record, "address owner stages artifact evidence", "claim")
        address = record["address"]
        if not isinstance(address, str) or not re.fullmatch("[0-9A-F]{8}", address):
            fail("address must be an exact uppercase entry")
        va = int(address, 16)
        if va in seen:
            fail("duplicate/conflicting entry " + address)
        seen.add(va)
        stages = record["stages"]
        if (not isinstance(stages, list) or not stages or any(s not in STAT[:2] for s in stages)
                or len(set(stages)) != len(stages)):
            fail(address + ": forbidden/duplicate stages")
        o = owner.get(va)
        if (va not in att.funcs or not o or o == "<shared>" or o.startswith("lib:")
                or record["owner"] != o or "shared" in rule[va] or "folded" in rule[va]
                or len(att.vt.get(va, set())) > 1):
            fail(address + ": nonexistent, foreign, ambiguous or wrong owner")
        own_ranges = [(a, b) for a, b, v in att.rng if v == va]
        if any(a <= d and c <= b for a, b in own_ranges for c, d, v in att.rng if v != va):
            fail(address + ": overlapping/shared native body")
        if record["artifact"] != artifact or artifact.get("exe_sha256") != exe_hash:
            fail(address + ": stale/foreign original artifact identity")
        evidence = record["evidence"]
        roles = {"contract", "qualification"}
        if "implemented" in stages:
            roles |= {"production", "assertions", "validation"}
        if not isinstance(evidence, dict) or set(evidence) != roles:
            fail(address + ": missing/extra evidence roles")
        prefixes = {"contract": ("docs/specs/",), "production": ("engine/",),
                    "assertions": ("engine/", "tools/tests/"),
                    "qualification": ("docs/coverage-functions/",),
                    "validation": ("docs/coverage-functions/",)}
        paths = {}
        for role, ref in sorted(evidence.items()):
            exact(ref, "path sha256 citation", role)
            p = file(ref["path"], prefixes[role])
            if ref["sha256"] != hashes[ref["path"]]:
                fail(address + ": stale " + role)
            if (not isinstance(ref["citation"], str)
                    or not ref["citation"].startswith(ref["path"] + "#")
                    or not ref["citation"].split("#", 1)[1].strip()):
                fail(address + ": missing " + role + " citation")
            paths[role] = p
        if len(set(paths.values())) != len(paths):
            fail(address + ": evidence roles must use distinct files")
        q = load_json(paths["qualification"])
        exact(q, "address owner stages review normal_domain_complete decisions limitations validation", "qualification")
        if q["address"] != address or q["owner"] != o or q["stages"] != stages:
            fail(address + ": qualification identity/stage mismatch")
        exact(q["review"], "by citation approved", "review")
        for key in ("by", "citation"):
            words(q["review"][key], "review " + key)
        if q["review"]["approved"] is not True or q["normal_domain_complete"] is not True:
            fail(address + ": incomplete/unapproved whole function")
        words(q["limitations"], "limitations (use 'none' if applicable)")
        if not isinstance(q["decisions"], list) or not q["decisions"]:
            fail(address + ": missing decision correspondence")
        for decision in q["decisions"]:
            exact(decision, "contract production assertion", "decision")
            for key, value in decision.items():
                if "implemented" in stages or key == "contract":
                    words(value, "decision " + key)
                    role = {"contract": "contract", "production": "production", "assertion": "assertions"}[key]
                    if value not in paths[role].read_text(encoding="utf-8"):
                        fail(address + ": missing cited " + key + " correspondence")
                elif value is not None:
                    fail("specified-only production/assertion must be null")
        if "implemented" in stages:
            v = q["validation"]
            exact(v, "command result production_sha256 assertions_sha256 log_sha256 provenance", "validation")
            words(v["command"], "validation command")
            words(v["provenance"], "validation provenance")
            if v["result"] != "PASS" or any(v[k + "_sha256"] != evidence[r]["sha256"] for k, r in
                    (("production", "production"), ("assertions", "assertions"), ("log", "validation"))):
                fail(address + ": validation not PASS/source matched")
            log = paths["validation"].read_text(encoding="utf-8")
            required = [v["command"], "result=PASS", "production_sha256=" + v["production_sha256"],
                        "assertions_sha256=" + v["assertions_sha256"]]
            if any(text not in log for text in required):
                fail(address + ": retained validation lacks command/PASS/source identity")
        elif q["validation"] is not None:
            fail(address + ": specified-only validation must be null")
        for s in stages:
            flags[va][s] = True
    return flags, baseline, dict(sorted(hashes.items()))

#!/usr/bin/env python3
"""Check a lane's coverage claim against its merge-base with main.

Usage: python tools/creditcheck.py --wt WORKTREE [--log GAME.log ...] [--json OUT]
Uses coverage.py's attribution, row assignment and statuses. It never writes a map or report.
"""
import argparse
from contextlib import contextmanager
import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile


_spec = importlib.util.spec_from_file_location("fc4_credit_coverage", Path(__file__).with_name("coverage.py"))
coverage = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(coverage)
STAT = coverage.STAT
MAP = "tools/coverage-map.tsv"
SCRATCH = Path(__file__).resolve().parents[1] / "build-runs"
# Only the scoring inputs: no checkout, index changes, or nested worktree is needed.
TREE_PATHS = ("docs/specs", "docs/abi/vtables.tsv", "docs/abi/interfaces.tsv", "engine",
              "re/names_windows.tsv", "tools/coverage-libs.tsv", "tools/coverage-ranges.tsv",
              "tools/coverage-noncode-refs.tsv", MAP, "tools/coverage-shadows.tsv", "tools/regress.ps1")


def git(wt, *args):
    return subprocess.run(["git", "-C", str(wt), *args], capture_output=True, check=True)


def tree_coverage(tree):
    """Read run evidence with the parser belonging to that tree, including new labels."""
    spec = importlib.util.spec_from_file_location("fc4_tree_coverage", tree / "tools/coverage.py")
    module = importlib.util.module_from_spec(spec)
    # A check must not create __pycache__ in another lane or the main checkout.
    exec(compile(spec.loader.get_source(spec.name), spec.origin, "exec"), module.__dict__)
    return module


@contextmanager
def coverage_root(root):
    """coverage's original API uses a module root; always restore it, including on refusal."""
    old = coverage.REPO
    coverage.REPO = root
    try:
        yield
    finally:
        coverage.REPO = old


@contextmanager
def snapshot(wt, revision):
    SCRATCH.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="credit-tree-", dir=SCRATCH) as tmp:
        root = Path(tmp)
        paths = git(wt, "ls-tree", "--name-only", revision, "--", *TREE_PATHS).stdout.decode().splitlines()
        if not paths:
            raise ValueError(f"{revision}: no coverage inputs")
        raw = git(wt, "archive", revision, "--", *paths).stdout
        with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
            for member in archive:
                target = (root / member.name).resolve()
                if not target.is_relative_to(root.resolve()):
                    raise ValueError(f"unsafe archive path: {member.name}")
                # Symlinks and submodules cannot supply a file's coverage evidence.
                if member.isfile():
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(archive.extractfile(member).read())
        yield root


def load_attribution(tree, data_repo):
    """Use the same checked export and byte attribution as the coverage report."""
    with coverage_root(data_repo):
        export = coverage.find_input("ghidra/export/windows")
        exe = coverage.find_input("bin/SimCity4.exe")
    if coverage.sha256(exe) != coverage.EXE_SHA256:
        raise ValueError("coverage executable is not GOG 1.1.641")
    summary = (export / "export-summary.txt").read_text(encoding="utf-8", errors="replace")
    if f"executable_sha256={coverage.EXE_SHA256}" not in summary:
        raise ValueError("coverage export is not GOG 1.1.641")
    _, lo, hi = coverage.pe_text(exe)
    att = coverage.attribution_for_tree(tree, export, lo, hi)
    owners, _ = att.run()
    return att, owners


def measure_tree(tree, data_repo, reg, *, lane=False, inherited=None, regress_name="lane logs"):
    rows = coverage.read_map(tree / MAP)
    replace, shadows = coverage.regress_lists(tree / "tools/regress.ps1")
    keys = coverage.subsystem_keys(tree / "engine/plugin/FC4Subsystems.cpp")
    evidence = {}
    evidence_path = data_repo / "tools/coverage-shadows.tsv"
    if not lane and evidence_path.is_file():
        evidence, _ = coverage.read_shadow_evidence(evidence_path, data_repo)
    with coverage_root(tree):
        warnings, _ = coverage.map_statuses(rows, replace, shadows, keys, reg, evidence, regress_name)
    for row in rows:
        old = (inherited or {}).get(row["pattern"], {})
        if old.get("verified") and old["shadow"] == row["shadow"]:
            row["verified"] = True
            row["shadow_note"] = old["shadow_note"]
    att, owners = load_attribution(tree, data_repo)
    game, _, _, _, _ = coverage.map_functions(rows, att, owners)
    weights = [0] * len(rows)
    for _, size, index, _ in game:
        if index is not None:
            weights[index] += size
    by_pattern = {r["pattern"]: {**r, "bytes": weights[i]} for i, r in enumerate(rows)}
    totals = {s: sum(r["bytes"] for r in by_pattern.values() if r[s]) for s in STAT}
    return {"rows": by_pattern, "totals": totals, "game_bytes": sum(g[1] for g in game),
            "warnings": warnings}


def measure_current(wt):
    """Measured main coverage for the train's before/after landing report."""
    wt = Path(wt).resolve()
    with coverage_root(wt):
        source = coverage.newest_regress()
        reg = coverage.read_regress(source)
    return measure_tree(wt, wt, reg, regress_name=source.name)


def main_statuses(parser, tree, reg, source):
    """Main's verified floor uses all of main's current scoring inputs, not the lane's profile."""
    rows = parser.read_map(tree / MAP)
    replace, shadows = parser.regress_lists(tree / "tools/regress.ps1")
    keys = parser.subsystem_keys(tree / "engine/plugin/FC4Subsystems.cpp")
    path = tree / "tools/coverage-shadows.tsv"
    evidence = parser.read_shadow_evidence(path, tree)[0] if path.is_file() else {}
    parser.map_statuses(rows, replace, shadows, keys, reg, evidence, source.name)
    return {row["pattern"]: row for row in rows}


def summarize_log(wt, log):
    shell = shutil.which("powershell") or shutil.which("pwsh")
    if not shell:
        raise ValueError("PowerShell is required to run run_summary.ps1")
    if not log.is_file():
        raise ValueError(f"game log does not exist: {log}")
    result = subprocess.run([shell, "-NoProfile", "-NonInteractive", "-File",
                             str(wt / "tools/run_summary.ps1"), "-Log", str(log)],
                            capture_output=True, text=True, encoding="utf-8", errors="replace",
                            check=True, timeout=180)
    return result.stdout


def log_evidence(wt, logs, baseline):
    # Supplied logs add verified credit; replacement cleanliness keeps main's regress verdict.
    reg = {**baseline, "shadows": {}, "resynced": {}}
    parser = tree_coverage(wt)
    SCRATCH.mkdir(exist_ok=True)
    for log in dict.fromkeys(Path(p).resolve() for p in logs):
        summary = summarize_log(wt, log)
        with tempfile.TemporaryDirectory(prefix="credit-summary-", dir=SCRATCH) as tmp:
            path = Path(tmp) / "summary.out"
            path.write_text("=== regress-creditcheck-shadows (lane log)\n" + summary, encoding="utf-8")
            parsed = parser.read_regress(path, audit_legacy=False)
        for label, (clean, lines) in parsed["shadows"].items():
            old_clean, old_lines = reg["shadows"].get(label, (0, 0))
            reg["shadows"][label] = (old_clean + clean, old_lines + lines)
            # Missing reset evidence must remain missing, even if another file proves zero resets.
            if label not in parsed["resynced"] or (old_lines and reg["resynced"].get(label) is None):
                reg["resynced"][label] = None
            else:
                reg["resynced"][label] = reg["resynced"].get(label, 0) + parsed["resynced"][label]
    return reg


def shadow_label(value):
    key, _, own = value.partition(":")
    label = key[:-len("_shadow")] if key.endswith("_shadow") else key
    return key, own or coverage.SHADOW_LABEL_ALIAS.get(label, label)


def inside(tree, prefix, value):
    root = (tree / prefix).resolve()
    path = (root / value).resolve()
    if not path.is_relative_to(root):
        return None
    return path


def validate_changes(before, after, tree, reg):
    """Refuse only newly claimed cells; old debt does not prevent an unrelated lane from landing."""
    replace, shadows = coverage.regress_lists(tree / "tools/regress.ps1")
    refusals = []
    for pattern, row in after.items():
        old = before.get(pattern, {})
        for cell in ("spec", "core", "shadow", "replace"):
            value = row[cell]
            if value == old.get(cell, "-") or value == "-":
                continue
            reasons = []
            if cell == "spec":
                for item in coverage.split_list(value):
                    path = inside(tree, "docs/specs", item)
                    if path is None or not path.exists():
                        reasons.append(f"spec path `{item}` does not exist in the tree")
            elif cell == "core":
                for item in coverage.split_list(value):
                    path = inside(tree, "engine", item)
                    if path is None or not (path.is_file() and path.suffix == ".rs"
                                            or path.is_dir() and any(p.is_file() for p in path.rglob("*.rs"))):
                        reasons.append(f"core path `{item}` has no Rust source")
            elif cell == "shadow":
                key, label = shadow_label(value)
                if key not in shadows:
                    reasons.append(f"shadow key `{key}` is not in the shadows profile")
                clean, lines = reg["shadows"].get(label, (0, 0))
                if not (lines > 0 and clean == lines and reg["resynced"].get(label) == 0):
                    reasons.append(f"shadow `{label}` is not exact in the supplied logs "
                                   f"({clean}/{lines}, resynced={reg['resynced'].get(label)})")
            elif value not in replace:
                reasons.append(f"replace key `{value}` is not in the regress replace list")
            refusals.extend({"row": pattern, "cell": cell, "value": value, "reason": reason}
                            for reason in reasons)
    return refusals


def compare(before, after):
    flipped = []
    for pattern in sorted(before["rows"].keys() | after["rows"].keys()):
        old, new = before["rows"].get(pattern, {}), after["rows"].get(pattern, {})
        changes = {s: {"before": bool(old.get(s)), "after": bool(new.get(s))}
                   for s in STAT if bool(old.get(s)) != bool(new.get(s))}
        if changes:
            delta = {s: new.get("bytes", 0) * bool(new.get(s)) - old.get("bytes", 0) * bool(old.get(s))
                     for s in STAT}
            flipped.append({"row": pattern, "statuses": changes, "deltas": delta,
                            "bytes_before": old.get("bytes", 0), "bytes_after": new.get("bytes", 0)})
    return {"rows_flipped": flipped, "deltas": {s: after["totals"][s] - before["totals"][s] for s in STAT},
            "before": {"bytes": before["totals"], "game_bytes": before["game_bytes"]},
            "after": {"bytes": after["totals"], "game_bytes": after["game_bytes"]}}


def check(wt, logs=()):
    wt = Path(wt).resolve()
    tip = git(wt, "rev-parse", "HEAD").stdout.decode().strip()
    base = git(wt, "merge-base", "HEAD", "main").stdout.decode().strip()
    data_repo = coverage.main_checkout(wt)
    main_coverage = tree_coverage(data_repo)
    source = main_coverage.newest_regress()
    baseline = main_coverage.read_regress(source)
    main_rows = main_statuses(main_coverage, data_repo, baseline, source)
    reg = log_evidence(wt, logs, baseline)
    with snapshot(wt, base) as tree:
        before = measure_tree(tree, data_repo, baseline, inherited=main_rows, regress_name=source.name)
    after = measure_tree(wt, wt, reg, lane=True, inherited=before["rows"])
    result = compare(before, after)
    result.update({"tip": tip, "base": base, "logs": [str(Path(p).resolve()) for p in logs],
                   "refusals": validate_changes(before["rows"], after["rows"], wt, reg)})
    return result


def checked_credit(wt, logs=()):
    """A failed/missing input is a refusal, never an empty successful claim."""
    try:
        return check(wt, logs)
    except (OSError, ValueError, KeyError, subprocess.SubprocessError, SystemExit) as exc:
        return {"rows_flipped": [], "deltas": {},
                "refusals": [{"row": "<inputs>", "reason": str(exc)}]}


def delta_text(credit):
    return ", ".join(f"{s} {credit.get('deltas', {}).get(s, 0):+d} B" for s in STAT)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wt", type=Path, required=True)
    parser.add_argument("--log", type=Path, action="append", default=[])
    parser.add_argument("--json", type=Path)
    args = parser.parse_args(argv)
    result = checked_credit(args.wt, args.log)
    for row in result["rows_flipped"]:
        changes = ", ".join(f"{s} {int(v['before'])}->{int(v['after'])}" for s, v in row["statuses"].items())
        print(f"{row['row']}: {changes}; {delta_text(row)}")
    print("coverage delta: " + delta_text(result))
    for refusal in result["refusals"]:
        print(f"REFUSED {refusal['row']}: {refusal['reason']}")
    if args.json:
        args.json.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return 2 if result["refusals"] else 0


if __name__ == "__main__":
    raise SystemExit(main())

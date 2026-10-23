#!/usr/bin/env python3
"""Read-only, scoped psutil evidence. See docs/ops/process-snapshot.md."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import sys
from datetime import datetime, timezone


BUILD_NAMES = {"python", "python3", "pythonw", "powershell", "pwsh", "bash", "sh",
               "cmd", "cmake", "ninja", "msbuild", "cl", "link", "cargo", "rustc",
               "ctest", "sccache", "ccache", "fc4verify"}
MAX_PROCESSES = 16384
MAX_DIAGNOSTICS = 100


def normalized(value):
    return os.path.normcase(os.path.realpath(os.path.abspath(value)))


def within(value, target):
    if not isinstance(value, str) or not os.path.isabs(value):
        return False
    try:
        return os.path.commonpath([normalized(value), target]) == target
    except (ValueError, OSError):
        return False


def argv_digest(argv):
    return hashlib.sha256(json.dumps(argv, ensure_ascii=True, separators=(",", ":")).encode()).hexdigest()


def load_identities(path):
    if path is None:
        return []
    if Path(path).stat().st_size > 65536:
        raise ValueError("identity file exceeds 64 KiB")
    rows = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(rows, list) or len(rows) > 128:
        raise ValueError("identities must be a list of at most 128 objects")
    seen = set()
    for row in rows:
        if not isinstance(row, dict) or set(row) - {"pid", "create_time", "argv_sha256"}:
            raise ValueError("identity keys: pid, create_time, optional argv_sha256")
        pid, stamp = row.get("pid"), row.get("create_time")
        digest = row.get("argv_sha256")
        if type(pid) is not int or pid <= 0 or pid in seen:
            raise ValueError("identity pid must be positive and unique")
        if type(stamp) not in (int, float) or not math.isfinite(stamp) or stamp <= 0:
            raise ValueError("identity requires a finite positive create_time")
        if digest is not None and (not isinstance(digest, str) or len(digest) != 64
                                   or any(c not in "0123456789abcdef" for c in digest)):
            raise ValueError("argv_sha256 must be 64 lowercase hex characters")
        seen.add(pid)
    return rows


def snapshot(target, identities, ps):
    now = datetime.now(timezone.utc)
    result = {"schema": 1, "capture_utc": now.isoformat(),
              "capture_local": now.astimezone().isoformat(), "target": target,
              "provider": "psutil", "provider_version": getattr(ps, "__version__", "unknown"),
              "scope": "same-user target or build-name candidates and recorded PIDs",
              "status": "UNKNOWN", "authoritative": False,
              "warning": "Observational evidence only; never permission to launch or control processes.",
              "diagnostics": [], "diagnostics_omitted": 0,
              "target_related": [], "foreign": [], "unknown": [], "recorded": []}
    incomplete = False

    def diagnostic(pid, field, kind):
        nonlocal incomplete
        incomplete = True
        if len(result["diagnostics"]) < MAX_DIAGNOSTICS:
            result["diagnostics"].append({"pid": pid, "field": field, "kind": kind,
                                          "source": "psutil"})
        else:
            result["diagnostics_omitted"] += 1

    errors = (ps.AccessDenied, ps.NoSuchProcess, ps.ZombieProcess, OSError)

    def read(proc, field):
        try:
            return getattr(proc, field)()
        except errors as exc:
            diagnostic(proc.pid, field, type(exc).__name__)
            return None

    try:
        owner = ps.Process().username()
    except errors as exc:
        diagnostic(None, "caller.username", type(exc).__name__)
        owner = None
    result["caller_owner_readable"] = owner is not None
    requested = {r["pid"]: r for r in identities}
    records = {}
    seen = set()
    try:
        # No attrs/as_dict prefetch: unrelated command lines are never requested.
        for index, proc in enumerate(ps.process_iter()):
            if index >= MAX_PROCESSES:
                diagnostic(None, "inventory", "process_limit")
                break
            pid = proc.pid
            seen.add(pid)
            try:
                name = proc.name()
            except errors as exc:
                diagnostic(pid, "name", type(exc).__name__)
                name = None
            build = name is None or name.lower().removesuffix(".exe") in BUILD_NAMES
            try:
                username = proc.username()
            except errors as exc:
                username = None
                if build or pid in requested:
                    diagnostic(pid, "username", type(exc).__name__)
            # Do not inspect other users' paths, argv or handles.
            if owner is not None and username is not None and username != owner:
                if pid in requested:
                    result["recorded"].append({"pid": pid, "state": "UNKNOWN", "reason": "different_owner"})
                    diagnostic(pid, "recorded", "different_owner")
                continue
            if owner is None or username is None:
                diagnostic(pid, "scope", "owner_unreadable")
                result["unknown"].append({"pid": pid, "reason": "owner_unreadable",
                                           "source": "psutil.name/username"})
                continue
            cwd = read(proc, "cwd")
            exe = read(proc, "exe")
            path_target = within(cwd, target) or within(exe, target)
            if not (build or path_target or pid in requested):
                # Missing paths could conceal a target process even with a custom name.
                if cwd is None or exe is None:
                    result["unknown"].append({"pid": pid, "reason": "scope_paths_unreadable"})
                continue
            start = read(proc, "create_time")
            ppid = read(proc, "ppid")
            argv = read(proc, "cmdline")
            files = read(proc, "open_files")
            end = read(proc, "create_time")
            running = read(proc, "is_running")
            stable = start is not None and start == end and running is True
            if not stable:
                diagnostic(pid, "identity", "changed_or_unverifiable")
            argv = argv if isinstance(argv, list) and argv else None
            digest = argv_digest(argv) if argv is not None else None
            # Absolute path operands provide attribution; option values are never evidence.
            paths = [a for a in (argv or []) if os.path.isabs(a)]
            matched = [a for a in paths if within(a, target)]
            # Do not assert foreignness for relative operands, flag values or shell
            # programs we cannot safely interpret. Even an apparent switch may embed
            # a target path, so only all-absolute argv permits negative classification.
            opaque_argv = argv is None or any(not os.path.isabs(a) for a in argv)
            handles = [{"path": f.path, "fd": getattr(f, "fd", None),
                        "mode": getattr(f, "mode", "UNKNOWN"), "source": "psutil.open_files",
                        "output_intent": "UNKNOWN (handle presence does not prove writer)"}
                       for f in (files or []) if within(f.path, target)]
            evidence = []
            if within(cwd, target):
                evidence.append("psutil.cwd within target")
            if within(exe, target):
                evidence.append("psutil.exe within target")
            if matched:
                evidence.append("psutil.cmdline absolute operand within target (reference, not ownership)")
            if handles:
                evidence.append("psutil.open_files target handle (not exclusive ownership)")
            complete = (stable and not opaque_argv and bool(cwd) and bool(exe)
                        and os.path.isabs(cwd) and os.path.isabs(exe)
                        and all(v is not None for v in (argv, files, ppid)))
            if argv is None:
                diagnostic(pid, "cmdline", "empty_or_unreadable")
            category = "target_related" if stable and evidence else "foreign" if complete else "unknown"
            row = {"pid": pid, "create_time": start, "ppid": ppid, "name": name,
                   "owner": "same_as_caller", "stable_identity": stable,
                   "source": "psutil (sequential reads; not atomic)", "evidence": evidence,
                   "opaque_argv": opaque_argv,
                   "children": [], "argv_sha256": digest,
                   "fields_unknown": [k for k, v in {"cwd": cwd, "exe": exe, "cmdline": argv,
                                         "open_files": files, "ppid": ppid, "create_time": start}.items()
                                      if v is None]}
            if category != "foreign":
                # Actual path and safe argv projection, never env or option values.
                row.update(cwd=cwd, exe=exe, command={"source": "psutil.cmdline",
                           "argc": len(argv) if argv else None,
                           "target_path_operands": matched,
                           "display": [a if within(a, target) else "<redacted>" for a in (argv or [])],
                           "redacted": True}, open_output_handles=handles)
            else:
                row["reason"] = "same-user build candidate; readable paths/absolute argv/handles have no target reference"
            if pid in requested:
                old = requested[pid]
                state = ("UNKNOWN" if not stable else "stale_pid" if start != old["create_time"]
                         else "UNKNOWN" if old.get("argv_sha256") and digest is None
                         else "argv_mismatch" if old.get("argv_sha256", digest) != digest else "match")
                result["recorded"].append({"pid": pid, "state": state,
                                           "source": "recorded creation time and optional argv digest vs psutil"})
                if state == "UNKNOWN":
                    diagnostic(pid, "recorded", "identity_unverifiable")
            result[category].append(row)
            records[pid] = row
    except errors as exc:
        diagnostic(None, "inventory", type(exc).__name__)
    for old in identities:
        if old["pid"] not in seen:
            result["recorded"].append({"pid": old["pid"], "state": "not_observed",
                                       "reason": "not proof of absence or permission"})
    for row in records.values():
        parent = records.get(row["ppid"])
        if parent and row["stable_identity"] and parent["stable_identity"]:
            if parent["create_time"] <= row["create_time"]:
                parent["children"].append({"pid": row["pid"], "create_time": row["create_time"],
                                           "source": "observed child.ppid; scoped inventory only"})
    result["capture_end_utc"] = datetime.now(timezone.utc).isoformat()
    result["counts"] = {k: len(result[k]) for k in ("target_related", "foreign", "unknown")}
    result["inventory_pids_seen"] = len(seen)
    if not seen:
        diagnostic(None, "inventory", "empty_inventory")
    if not incomplete and not result["unknown"]:
        result["status"] = "OBSERVED" if result["target_related"] else "NO_TARGET_OBSERVED"
    return result


def main(argv=None, provider=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, help="explicit worktree directory")
    parser.add_argument("--identities", help="JSON list: pid, create_time, optional argv_sha256")
    args = parser.parse_args(argv)
    own = Path(__file__).resolve().parents[2]
    output = own / "build-runs" / "process-snapshots"
    try:
        target = normalized(args.target)
        if not os.path.isdir(target):
            raise ValueError("target must be an existing directory")
        identities = load_identities(args.identities)
    except (ValueError, OSError) as exc:
        print("ERROR input: " + type(exc).__name__ + "; check target/identity format")
        return 2
    if sys.platform != "win32" and not sys.platform.startswith(("linux", "darwin")):
        result = {"status": "UNKNOWN", "authoritative": False, "target": target,
                  "error": "unsupported platform: Windows, Linux and macOS only"}
    else:
        try:
            if provider is None:
                import psutil as provider
            result = snapshot(target, identities, provider)
        except ImportError:
            result = {"status": "UNKNOWN", "authoritative": False, "target": target,
                      "error": "psutil unavailable for this Python interpreter; no fallback or installation attempted"}
        except (OSError, RuntimeError, AttributeError) as exc:
            result = {"status": "UNKNOWN", "authoritative": False, "target": target,
                      "error": "provider failed: " + type(exc).__name__}
    result.setdefault("capture_utc", datetime.now(timezone.utc).isoformat())
    result.setdefault("capture_local", datetime.now().astimezone().isoformat())
    output.mkdir(parents=True, exist_ok=True)
    path = output / (datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ") + f"-{os.getpid()}.json")
    with path.open("x", encoding="utf-8") as stream:
        json.dump(result, stream, indent=2, ensure_ascii=True)
        stream.write("\n")
    print(f"process snapshot: {result['status']} counts={result.get('counts', {})} authoritative=false")
    print(f"capture UTC={result['capture_utc']} local={result['capture_local']}")
    print(f"target={target}\nsnapshot={path}")
    print("Evidence at capture time only; no permission to launch or control processes.")
    # psutil's cross-process reads (especially Windows open_files) cannot establish
    # authoritative absence. Non-success is intentional even for readable evidence.
    return 3


if __name__ == "__main__":
    sys.exit(main())

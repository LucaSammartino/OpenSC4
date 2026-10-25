"""Coverage claims use real tree snapshots and the report's rules, with synthetic byte attribution."""
import argparse
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
from types import SimpleNamespace

import pytest


ROOT = Path(__file__).resolve().parents[2]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


cc = load("credit_tests", ROOT / "tools/creditcheck.py")
train = load("credit_train_tests", ROOT / "tools/merge-train.py")
HEADER = "pattern\tsubsystem\tarea\tspec\tcore\tshadow\treplace\n"
EXACT = "  probe 2 lines, 2 with every mismatch count 0\n  probe resynced lines: 0\n"


def put(root, name, text):
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def git(root, *args):
    return subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True, check=True).stdout.strip()


def commit(root):
    git(root, "add", ".")
    git(root, "commit", "-qm", "fixture")


def map_row(root, **cells):
    row = dict(pattern="Probe", subsystem="Probe", area="simulation", spec="-", core="-", shadow="-", replace="-")
    row.update(cells)
    put(root, cc.MAP, HEADER + "\t".join(row.values()) + "\n")


def fake_attribution(tree, data_repo):
    att = SimpleNamespace(order=[1, 2, 3, 4, 5], funcs={1: 128, 2: 32, 3: 11, 4: 7, 5: 9},
                          name=lambda va: {1: "Probe::First (void)", 2: "Probe::Rest (void)"}.get(va))
    return att, {1: "Probe", 2: "Probe", 3: "lib:crt", 4: cc.coverage.SHARED, 5: None}


@pytest.fixture
def repo(tmp_path, monkeypatch):
    git(tmp_path, "init", "-b", "main")
    git(tmp_path, "config", "user.name", "Fixture")
    git(tmp_path, "config", "user.email", "fixture@example.invalid")
    map_row(tmp_path)
    put(tmp_path, "tools/regress.ps1", '[string]$Replace = "probe"\n    shadows = "probe_shadow = 1"\n')
    put(tmp_path, "engine/plugin/FC4Subsystems.cpp", '{"probe", nullptr, kProbe, false}\n')
    put(tmp_path, "engine/core/src/probe.rs", "pub fn probe() {}\n")
    put(tmp_path, "docs/specs/Probe.md", "# Probe\n")
    commit(tmp_path)
    git(tmp_path, "checkout", "-qb", "lane")
    put(tmp_path, "build-runs/regress-fixture.out",
        "REGRESS fixture: PASS\nreplace clean 0 0\n=== regress-fixture-shadows (fixture)\n" + EXACT)
    monkeypatch.setattr(cc, "load_attribution", fake_attribution)
    monkeypatch.setattr(cc, "summarize_log", lambda wt, log: EXACT)
    return tmp_path


def test_justified_flip_uses_base_tree_and_byte_weights(repo):
    map_row(repo, spec="Probe.md", core="core/src/probe.rs", shadow="probe_shadow", replace="probe")
    commit(repo)
    result = cc.check(repo, [repo / "clean.log"])
    assert not result["refusals"]
    assert result["base"] == git(repo, "rev-parse", "main")
    assert result["tip"] == git(repo, "rev-parse", "HEAD")
    assert result["deltas"] == dict.fromkeys(cc.STAT, 160)
    assert result["after"]["game_bytes"] == 176  # shared and unattributed stay in the denominator
    assert [r["row"] for r in result["rows_flipped"]] == ["Probe"]
    assert all(v == {"before": False, "after": True}
               for v in result["rows_flipped"][0]["statuses"].values())


@pytest.mark.parametrize("cell,value,reason", [
    ("spec", "Missing.md", "does not exist"),
    ("spec", "Probe.md,Missing.md", "Missing.md"),
    ("spec", "../../elsewhere", "does not exist"),
    ("core", "core/missing", "no Rust source"),
    ("core", "plugin/FC4Subsystems.cpp", "no Rust source"),
    ("shadow", "other_shadow", "not in the shadows profile"),
    ("replace", "other", "not in the regress replace list"),
])
def test_refuses_each_unjustified_cell(repo, cell, value, reason):
    map_row(repo, **{cell: value})
    result = cc.check(repo, [repo / "clean.log"])
    assert any(r["row"] == "Probe" and r["cell"] == cell and reason in r["reason"]
               for r in result["refusals"])


def test_empty_core_directory_is_not_rust_source(repo):
    (repo / "engine/core/empty").mkdir()
    map_row(repo, core="core/empty")
    assert "no Rust source" in cc.check(repo)["refusals"][0]["reason"]


@pytest.mark.parametrize("summary", [
    "",  # at least one comparison is required
    "  probe 0 lines, 0 with every mismatch count 0\n  probe resynced lines: 0\n",
    "  probe 2 lines, 1 with every mismatch count 0\n  probe resynced lines: 0\n",
    "  probe 2 lines, 2 with every mismatch count 0\n",  # unknown resets fail closed
    "  probe 2 lines, 2 with every mismatch count 0\n  probe resynced lines: 1\n",
])
def test_nonexact_shadow_is_refused(repo, monkeypatch, summary):
    map_row(repo, shadow="probe_shadow")
    monkeypatch.setattr(cc, "summarize_log", lambda wt, log: summary)
    result = cc.check(repo, [repo / "game.log"])
    assert not result["after"]["bytes"]["verified"]
    assert "not exact" in result["refusals"][0]["reason"]


def test_no_logs_cannot_borrow_main_verification(repo):
    map_row(repo, shadow="probe_shadow")
    result = cc.check(repo)
    assert "0/0" in result["refusals"][0]["reason"]


def test_exact_shadow_outside_profile_is_never_credited(repo):
    map_row(repo, shadow="probe_shadow")
    put(repo, "tools/regress.ps1", '[string]$Replace = "probe"\nshadows = "other_shadow = 1"\n')
    result = cc.check(repo, [repo / "game.log"])
    assert not result["after"]["bytes"]["verified"]
    assert "not in the shadows profile" in result["refusals"][0]["reason"]


@pytest.mark.parametrize("shadow,label", [("clock24_shadow", "clock"),
                                         ("flora_shadow:flora-occupant", "flora-occupant")])
def test_aliases_and_owner_families_share_coverage_rules(repo, monkeypatch, shadow, label):
    key = shadow.split(":")[0]
    map_row(repo, shadow=shadow)
    put(repo, "tools/regress.ps1", f'[string]$Replace = "probe"\nshadows = "{key} = 1"\n')
    monkeypatch.setattr(cc, "summarize_log", lambda wt, log:
                        f"  {label} 3 lines, 3 with every mismatch count 0\n  {label} resynced lines: 0\n")
    result = cc.check(repo, [repo / "game.log"])
    assert not result["refusals"]
    assert result["deltas"]["verified"] == 160


def test_bad_log_cannot_be_hidden_by_later_clean_log(repo, monkeypatch):
    map_row(repo, shadow="probe_shadow")
    monkeypatch.setattr(cc, "summarize_log", lambda wt, log: EXACT if log.stem == "good" else
                        "  probe 1 lines, 0 with every mismatch count 0\n  probe resynced lines: 0\n")
    result = cc.check(repo, [repo / "bad.log", repo / "good.log"])
    assert "2/3" in result["refusals"][0]["reason"]


@pytest.mark.parametrize("reverse", [False, True])
def test_missing_resets_in_one_log_remain_missing(repo, monkeypatch, reverse):
    map_row(repo, shadow="probe_shadow")
    monkeypatch.setattr(cc, "summarize_log", lambda wt, log: EXACT if log.stem == "good" else
                        "  probe 1 lines, 1 with every mismatch count 0\n")
    logs = [repo / "unknown.log", repo / "good.log"]
    result = cc.check(repo, list(reversed(logs)) if reverse else logs)
    assert "resynced=None" in result["refusals"][0]["reason"]


def test_default_enable_cannot_justify_changed_replace_key(repo):
    put(repo, "engine/plugin/FC4Subsystems.cpp", '{"defaulted", nullptr, kProbe, true}\n')
    map_row(repo, replace="defaulted")
    result = cc.check(repo)
    assert result["after"]["bytes"]["replaced"] == 160  # preserve coverage.py's scoring
    assert "not in the regress replace list" in result["refusals"][0]["reason"]


def test_old_unjustified_cells_are_not_new_claims(repo):
    map_row(repo, spec="Missing.md", core="core/missing", shadow="other_shadow", replace="other")
    commit(repo)
    git(repo, "branch", "-f", "main", "HEAD")
    result = cc.check(repo)
    assert not result["refusals"]


def test_removing_cells_and_deleting_rows_reports_losses(repo):
    map_row(repo, spec="Probe.md", core="core/src/probe.rs", shadow="probe_shadow", replace="probe")
    commit(repo)
    git(repo, "branch", "-f", "main", "HEAD")
    put(repo, cc.MAP, HEADER)
    # A deleted map row leaves functions unmapped. The report expects an exhaustive owner map,
    # so use a replacement catch-all row for these fixture functions.
    with (repo / cc.MAP).open("a", encoding="utf-8") as out:
        out.write("*\tUncredited\tsimulation\t-\t-\t-\t-\n")
    result = cc.check(repo)
    assert result["deltas"] == dict.fromkeys(cc.STAT, -160)
    assert result["rows_flipped"][0]["row"] == "Probe"
    assert not result["refusals"]


def test_method_rows_take_bytes_from_owner_without_double_counting(repo):
    map_row(repo, spec="Probe.md")
    with (repo / cc.MAP).open("a", encoding="utf-8") as out:
        out.write("m:Probe::First\tProbe first\tsimulation\t-\tcore/src/probe.rs\t-\t-\n")
    result = cc.check(repo)
    assert result["deltas"]["specified"] == 32
    assert result["deltas"]["implemented"] == 128
    assert sum(r["bytes_after"] for r in result["rows_flipped"]) == 160


def test_status_flip_without_map_edit_is_detected(repo):
    map_row(repo, spec="Future.md")
    commit(repo)
    git(repo, "branch", "-f", "main", "HEAD")
    put(repo, "docs/specs/Future.md", "# Implemented later\n")
    result = cc.check(repo)
    assert result["deltas"]["specified"] == 160
    assert not result["refusals"]


def test_missing_input_is_failure_not_zero_credit(repo, monkeypatch):
    monkeypatch.setattr(cc, "load_attribution", lambda *args: (_ for _ in ()).throw(ValueError("missing fixture")))
    result = cc.checked_credit(repo)
    assert result["refusals"] == [{"row": "<inputs>", "reason": "missing fixture"}]


def test_cli_exit_two_names_refused_row_and_writes_json(repo, monkeypatch, capsys):
    map_row(repo, spec="Missing.md")
    output = repo / "credit.json"
    assert cc.main(["--wt", str(repo), "--json", str(output)]) == 2
    assert "REFUSED Probe: spec path `Missing.md`" in capsys.readouterr().out
    assert json.loads(output.read_text())["refusals"][0]["cell"] == "spec"


@pytest.mark.skipif(not (shutil.which("powershell") or shutil.which("pwsh")), reason="requires PowerShell")
def test_actual_run_summary_output_is_consumed(tmp_path):
    log = put(tmp_path, "game.log",
              "2026-10-25 [pid 1 tid 2] probe shadow (month) 2026-04-01: mismatches=0; resyncs=0\n"
              "2026-10-25 [pid 1 tid 2] probe shadow (month) 2026-05-01: mismatches=1; resynced: lot\n")
    (tmp_path / "build-runs").mkdir()
    # ROOT has the actual run_summary.ps1; scratch output belongs to the fixture.
    summary = cc.summarize_log(ROOT, log)
    path = put(tmp_path, "summary.out", "=== regress-probe-shadows (fixture)\n" + summary)
    parsed = cc.coverage.read_regress(path, audit_legacy=False)
    assert parsed["shadows"]["probe"] == (1, 2)
    assert parsed["resynced"]["probe"] == 1


def credit(refusals=()):
    return {"tip": "a" * 40, "rows_flipped": [], "deltas": dict.fromkeys(cc.STAT, 128),
            "refusals": list(refusals)}


def record():
    return dict(lane="probe", branch="probe", tip="a" * 40, kind="engine", abi=False, state="ready", credit=credit())


@pytest.mark.parametrize("mode", ["valid", "refused", "missing", "stale"])
def test_readiness_requires_credit(monkeypatch, mode):
    monkeypatch.setattr(train, "lane_branch_exists", lambda branch: True)
    monkeypatch.setattr(train, "lane_tip", lambda branch: "a" * 40)
    monkeypatch.setattr(train, "lane_worktree", lambda lane: None)
    monkeypatch.setattr(train, "git", lambda *args, **kwargs: SimpleNamespace(stdout="1", returncode=0))
    rec = record()
    if mode == "refused":
        rec["credit"]["refusals"] = [{"row": "Probe", "reason": "not exact"}]
    elif mode == "missing":
        del rec["credit"]
    elif mode == "stale":
        rec["credit"]["tip"] = "b" * 40
    assert train.describe_lane(rec)["_ready"] == (mode == "valid")


@pytest.mark.parametrize("refused", [False, True])
def test_enqueue_carries_claim_and_refusals_block_ready(tmp_path, monkeypatch, capsys, refused):
    claim = credit([{"row": "Probe", "reason": "not exact"}] if refused else [])
    monkeypatch.setattr(train, "QUEUE_DIRS", [tmp_path])
    monkeypatch.setattr(train, "lane_worktree", lambda lane: tmp_path)
    calls = []
    monkeypatch.setattr(train, "lane_credit", lambda *args: calls.append(args) or claim)
    args = argparse.Namespace(lane="probe", branch="probe", tip="a" * 40, kind="engine", abi=False,
                              game_runs="probe-one,probe-two", summary="fixture", log=[tmp_path / "run.log"])
    assert train.cmd_enqueue(args) == (2 if refused else 0)
    queued = json.loads((tmp_path / "probe.json").read_text())
    assert queued["credit"] == claim
    assert queued["state"] == ("wip" if refused else "ready")
    assert calls == [(tmp_path, ["probe-one", "probe-two"], [tmp_path / "run.log"])]


def test_plan_prints_claim_and_refusal(monkeypatch, capsys):
    ready = {**record(), "_ready": True, "_problems": []}
    refused = {**record(), "lane": "bad", "_ready": False, "_problems": ["credit Probe: not exact"]}
    monkeypatch.setattr(train, "pick_batch", lambda *args: ([ready], [ready, refused]))
    assert train.cmd_plan(argparse.Namespace(size=8)) == 0
    output = capsys.readouterr().out
    assert "claimed coverage: specified +128 B, implemented +128 B, verified +128 B, replaced +128 B" in output
    assert "bad: not ready: credit Probe: not exact" in output


def test_lane_credit_resolves_game_run_labels_and_explicit_logs(tmp_path, monkeypatch):
    log = put(tmp_path, "build-runs/game-results/run-one.log", "fixture")
    monkeypatch.setattr(train.creditcheck, "checked_credit", lambda wt, logs: list(logs))
    assert train.lane_credit(tmp_path, ["run-one"]) == [log]
    explicit = tmp_path / "explicit.log"
    assert train.lane_credit(tmp_path, ["run-one"], [explicit]) == [explicit]


def test_land_prints_measured_before_after_and_delta(tmp_path, monkeypatch, capsys):
    head = "a" * 40
    cur = {"id": "train-fixture", "worktree": str(tmp_path), "lanes": ["probe"],
           "regress": {"ok": True, "head": head}}
    monkeypatch.setattr(train, "MAIN", tmp_path)
    monkeypatch.setattr(train, "current_branch", lambda: "main")
    monkeypatch.setattr(train, "main_is_clean", lambda: True)
    monkeypatch.setattr(train, "load_state", lambda: {"current": cur})
    monkeypatch.setattr(train, "integration_head", lambda wt: head)
    monkeypatch.setattr(train, "batch_touches_engine", lambda wt: False)
    monkeypatch.setattr(train, "main_moved_under", lambda sha: False)
    monkeypatch.setattr(train, "land_blockers", lambda *args: [])
    monkeypatch.setattr(train, "write_notices", lambda *args: None)
    monkeypatch.setattr(train, "save_state", lambda st: None)
    monkeypatch.setattr(train, "queue_files", lambda: [])
    calls = []

    def fake_git(args, **kwargs):
        calls.append(args)
        return SimpleNamespace(returncode=0, stdout='shadows = "fixture"\n' if args[0] == "show" else head)

    monkeypatch.setattr(train, "git", fake_git)

    def measure(wt):
        landed = any(args[0] == "merge" for args in calls)
        return {"rows": {}, "totals": dict.fromkeys(cc.STAT, 160 if landed else 32), "game_bytes": 176}

    monkeypatch.setattr(train.creditcheck, "measure_current", measure)
    assert train.cmd_land(argparse.Namespace(force=False)) == 0
    output = capsys.readouterr().out
    assert "measured coverage before: specified 32 B" in output
    assert "measured coverage after:  specified 160 B" in output
    assert "measured coverage delta:  specified +128 B" in output

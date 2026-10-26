"""Range/scheduling properties and differential parsing with the real plugin."""
from datetime import date
import argparse
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("scenario_gen", ROOT / "tools/scenario_gen.py")
gen = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gen)


def corpus():
    for seed in range(25):
        for size in gen.SIZES:
            for mix in gen.MIXES:
                start = (date(2000, 2, 29), date(2026, 1, 31), date(2026, 4, 1))[seed % 3]
                months = (1, 2, 4, 24, 48)[seed % 5]
                yield (seed, size, start, months, mix)


def compiler():
    env = os.environ.copy()
    for name in ("c++", "g++", "clang++", "cl"):
        if shutil.which(name):
            return shutil.which(name), env
    # PowerShell sessions need not have called vcvars. Discover the installed
    # MSVC environment without requiring any shell setup from test callers.
    base = Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Microsoft Visual Studio"
    candidates = sorted(base.glob("*/*/Common7/Tools/VsDevCmd.bat"))
    if candidates:
        result = subprocess.run(
            'cmd /d /s /c ""{}" -no_logo -arch=x64 >nul && set"'.format(candidates[-1]),
            capture_output=True, text=True, check=True)
        for line in result.stdout.splitlines():
            key, sep, value = line.partition("=")
            if sep and key:
                env[key.upper()] = value
        found = shutil.which("cl", path=env.get("PATH"))
        if found:
            return found, env
    raise RuntimeError("a C++ compiler is required to check the plugin's parser")


class ScenarioGeneratorTests(unittest.TestCase):
    def test_repeatable_cli_and_seed_variation(self):
        args = [sys.executable, str(ROOT / "tools/scenario_gen.py"), "--seed", "-17",
                "--size", "medium", "--start", "2026-01-31", "--months", "24"]
        first = subprocess.check_output(args)
        self.assertEqual(first, subprocess.check_output(args))
        self.assertEqual(first.decode(), gen.generate(-17, "medium", date(2026, 1, 31), 24))
        # Ignore seed markers: placement geometry must vary too.
        bodies = {"\n".join(line for line in gen.generate(s, "small", date(2000, 1, 1), 4).splitlines()
                            if not line.startswith("#") and " mark " not in line) for s in range(10)}
        self.assertEqual(len(bodies), 10)

    def test_ranges_dates_and_clock_progress_for_300_scripts(self):
        for seed, size, start, months, mix in corpus():
            with self.subTest(seed=seed, size=size, mix=mix):
                script = gen.generate(seed, size, start, months, mix)
                side = gen.SIZES[size]
                end = gen.add_months(start, months)
                last_day = start
                paused, stopped, turbo_off = False, False, False
                for line in script.splitlines():
                    if not line or line.startswith("#"):
                        continue
                    raw_day, op, *args = line.split()
                    day = date.fromisoformat(raw_day)
                    self.assertTrue(start <= last_day <= day <= end, line)
                    if day != last_day:
                        self.assertFalse(paused or stopped or turbo_off, line)
                    last_day = day
                    coords = []
                    if op in ("zone", "pricezone", "dezone", "network", "terrain"):
                        coords = list(map(int, args[:4]))
                        if op != "network":
                            self.assertLessEqual(coords[0], coords[2])
                            self.assertLessEqual(coords[1], coords[3])
                        if op in ("zone", "pricezone", "network"):
                            self.assertTrue(0 <= int(args[4]) <= (12 if op == "network" else 15))
                        if op == "terrain":
                            self.assertTrue(0 <= float(args[4]) <= 5000)
                            self.assertTrue(0 <= int(args[5]) <= 6)
                    elif op == "port":
                        self.assertIn(args[0], ("airport", "seaport"))
                        coords = list(map(int, args[1:3]))
                        self.assertTrue(0 <= int(args[3]) <= 3)
                        self.assertTrue(0 <= int(args[4]) <= 15)
                    elif op == "disaster":
                        self.assertIn(args[0], ("fire", "riot", "survey"))
                        if args[0] == "survey":
                            self.assertEqual(len(args), 1)
                        elif args[1] != "auto":
                            coords = list(map(int, args[1:]))
                            self.assertEqual(len(coords), 2)
                    elif op in ("speed", "clockmode", "lightinghour"):
                        self.assertTrue(0 <= int(args[0]) <= (23 if op == "lightinghour" else 2))
                        if op == "speed":
                            stopped = args[0] == "0"
                    elif op == "pause":
                        paused = True
                    elif op == "resume":
                        paused = False
                    elif op == "turbo":
                        self.assertIn(args[0], ("on", "off"))
                        turbo_off = args[0] == "off"
                    elif op == "save":
                        self.assertIn(args, ([], ["fast"]))
                    elif op == "funds":
                        self.assertEqual(args, [str(gen.TREASURY)])
                    elif op == "establish":
                        self.assertEqual(args, ["FC4", "generated", str(seed)])
                    else:
                        self.assertIn(op, ("mark", "digest"))
                    self.assertTrue(all(0 <= c < side and c <= 4095 for c in coords), line)
                self.assertFalse(paused or stopped or turbo_off)

    def test_funding_reserve_and_empty_bootstrap(self):
        for mix in gen.MIXES:
            lines = [line for line in gen.generate(2, "large", date(2000, 1, 1), 48, mix).splitlines()
                     if line and not line.startswith("#")]
            self.assertEqual(lines[0], "2000-01-01 establish FC4 generated 2")
            self.assertEqual(lines[1], "2000-01-01 funds " + str(gen.TREASURY))
            balance, months, refills = 0, 0, 0
            for line in lines:
                _, action = line.split(" ", 1)
                if action.startswith("funds "):
                    balance = int(action.split()[1])
                    refills += 1
                elif " month=" in action:
                    balance -= gen.MONTHLY_RESERVE
                    months += 1
                else:
                    cost = gen.estimated_cost(action)
                    if cost:
                        self.assertGreaterEqual(balance, cost)
                        balance -= cost
            self.assertEqual(months, 48)
            self.assertGreater(refills, 1)

    def test_growth_extends_instead_of_repeating_completed_utility_runs(self):
        for size in gen.SIZES:
            seen = set()
            for line in gen.generate(2, size, date(2000, 1, 1), 24, 'grow').splitlines():
                if ' network ' not in line:
                    continue
                _, _, x0, z0, x1, z1, kind = line.split()
                key = (kind, tuple(sorted(((int(x0), int(z0)), (int(x1), int(z1))))))
                self.assertNotIn(key, seen, line)
                seen.add(key)

    def test_required_mix_coverage(self):
        for mix in gen.MIXES:
            lines = [line.split() for line in gen.generate(7, "small", date(2026, 1, 1), 24, mix).splitlines()
                     if line and not line.startswith("#")]
            if mix in ("grow", "default"):
                self.assertEqual({int(p[6]) for p in lines if p[1] == "zone"}, set(range(16)))
                self.assertTrue({0, 1, 3, 4, 5, 6} <= {int(p[6]) for p in lines if p[1] == "network"})
                self.assertEqual({p[2] for p in lines if p[1] == "port"}, {"airport", "seaport"})
            if mix in ("churn", "default"):
                self.assertTrue({"dezone", "disaster", "save", "speed", "pause", "resume", "clockmode", "lightinghour"}
                                <= {p[1] for p in lines})
                self.assertEqual({int(p[6]) for p in lines if p[1] == "network"}, set(range(13)))
                self.assertEqual({tuple(p[2:]) for p in lines if p[1] == "disaster" and p[3:] == ["auto"]},
                                 {("fire", "auto"), ("riot", "auto")})
                # Native disaster mode can stop calendar time. No later day
                # should be needed to reach the requested game end date.
                self.assertEqual({p[0] for p in lines if p[1] == "disaster" and p[2] != "survey"},
                                 {"2028-01-01"})
            if mix in ("edge", "default"):
                self.assertEqual({int(p[6]) for p in lines if p[1] == "pricezone" and p[2:6] == ["0", "0", "63", "63"]},
                                 set(range(16)))
                self.assertTrue({"0", "5000"} <= {p[6] for p in lines if p[1] == "terrain"})

    def test_plugin_parser_accepts_300_generated_scripts(self):
        cc, env = compiler()
        scratch = ROOT / "build-runs"
        scratch.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="scenario-parser-", dir=scratch) as tmp:
            exe = Path(tmp) / ("parser.exe" if os.name == "nt" else "parser")
            source = ROOT / "tools/tests/scenario_parser_driver.cpp"
            include = ROOT / "engine/plugin"
            if Path(cc).stem.lower() == "cl":
                args = [cc, "/nologo", "/EHsc", "/std:c++17", "/I" + str(include), str(source), "/Fe:" + str(exe)]
            else:
                args = [cc, "-std=c++17", "-I", str(include), str(source), "-o", str(exe)]
            built = subprocess.run(args, cwd=tmp, env=env, capture_output=True, text=True)
            self.assertEqual(built.returncode, 0, built.stdout + built.stderr)
            scripts = [gen.generate(*params) for params in corpus()]
            result = subprocess.run([str(exe)], input="\0".join(scripts) + "\0", capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertRegex(result.stdout, r"scripts=300 actions=\d+ parse_errors=0")
            print(result.stdout.strip())

    def test_invalid_arguments_and_month_boundaries(self):
        self.assertEqual(gen.add_months(date(2000, 1, 31), 1), date(2000, 2, 29))
        self.assertEqual(gen.add_months(date(1900, 1, 31), 1), date(1900, 2, 28))
        for months in (0, -1, 1201):
            with self.assertRaises(ValueError):
                gen.generate(1, "small", date(2026, 1, 1), months)
        with self.assertRaises(ValueError):
            gen.generate(1, "small", date(9999, 12, 1), 1)
        for raw in ("2026-2-01", "20260201", "2026-02-29"):
            with self.assertRaises(argparse.ArgumentTypeError):
                gen.iso_date(raw)


if __name__ == "__main__":
    unittest.main()

"""qbuild's cargo slots: at most FC4_QBUILD_MAX cargo test/clippy runs at once, released when the holder dies."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

LANE_BIN = Path(__file__).resolve().parents[2] / '.claude/lane-bin'
sys.path.insert(0, str(LANE_BIN))
import qbuild

HOLDER = (
    "import sys, time; sys.path.insert(0, sys.argv[1]); import qbuild\n"
    "with qbuild.cargo_slot('holder', poll=0.05):\n"
    "    print('held', flush=True)\n"
    "    time.sleep(float(sys.argv[2]))\n"
)
WAITER = (
    "import sys; sys.path.insert(0, sys.argv[1]); import qbuild\n"
    "with qbuild.cargo_slot('waiter', poll=0.05, report=0.1):\n"
    "    print('ran', flush=True)\n"
)


class CargoSlotTests(unittest.TestCase):
    def env(self, tmp, limit):
        env = dict(os.environ, FC4_QBUILD_MAX=str(limit), FC4_QBUILD_SEM_DIR=str(Path(tmp) / 'sem'))
        return env

    def start(self, code, env, *args):
        return subprocess.Popen([sys.executable, '-c', code, str(LANE_BIN), *map(str, args)], env=env,
                                stdout=subprocess.PIPE, text=True)

    def test_limit_blocks_the_next_run_until_a_holder_ends(self):
        with tempfile.TemporaryDirectory() as tmp:
            env = self.env(tmp, 2)
            holders = [self.start(HOLDER, env, 60) for _ in range(2)]
            try:
                for holder in holders:
                    self.assertEqual(holder.stdout.readline().strip(), 'held')
                waiter = self.start(WAITER, env)
                time.sleep(1.0)
                self.assertIsNone(waiter.poll(), 'a third run must wait while two hold the slots')
                holders[0].kill()  # a dead holder frees its slot without any cleanup
                out, _ = waiter.communicate(timeout=30)
                self.assertIn('waiting for a cargo slot', out)
                self.assertIn('ran', out)
            finally:
                for holder in holders:
                    holder.kill()
                    holder.wait()

    def test_slots_are_reusable_after_release(self):
        with tempfile.TemporaryDirectory() as tmp:
            os.environ.update(self.env(tmp, 1))
            try:
                for _ in range(3):
                    with qbuild.cargo_slot('again', poll=0.05):
                        pass
            finally:
                for key in ('FC4_QBUILD_MAX', 'FC4_QBUILD_SEM_DIR'):
                    os.environ.pop(key, None)

    def test_zero_disables_the_limit_and_creates_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            env = self.env(tmp, 0)
            out = subprocess.run([sys.executable, '-c', WAITER, str(LANE_BIN)], env=env,
                                 capture_output=True, text=True, timeout=30)
            self.assertIn('ran', out.stdout)
            self.assertFalse((Path(tmp) / 'sem').exists())

    def test_unusable_lock_directory_runs_unthrottled(self):
        with tempfile.TemporaryDirectory() as tmp:
            blocker = Path(tmp) / 'file'
            blocker.write_text('not a directory')
            env = dict(os.environ, FC4_QBUILD_MAX='1', FC4_QBUILD_SEM_DIR=str(blocker / 'sem'))
            out = subprocess.run([sys.executable, '-c', WAITER, str(LANE_BIN)], env=env,
                                 capture_output=True, text=True, timeout=30)
            self.assertIn('ran', out.stdout)


if __name__ == '__main__':
    unittest.main()

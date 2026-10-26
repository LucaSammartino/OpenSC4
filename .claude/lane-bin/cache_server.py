"""Explicit sccache lifecycle for tests/benchmarks in restricted Windows jobs.

Foreground startup avoids the daemon startup notification named pipe. Compiler
requests still use sccache's normal loopback protocol and on-disk cache.
"""
from contextlib import contextmanager
import os
import subprocess
import threading


@contextmanager
def foreground_server(executable, env):
    server_env = dict(env, SCCACHE_START_SERVER="1", SCCACHE_LOG=env.get("SCCACHE_LOG", "info"))
    server_env.pop("SCCACHE_STARTUP_NOTIFY", None)
    server_env.pop("SCCACHE_ERROR_LOG", None)
    ready = threading.Event()
    lines = []
    server = subprocess.Popen([executable], env=server_env, stdout=subprocess.DEVNULL,
                              stderr=subprocess.PIPE, text=True, errors="replace")

    def read():
        for line in server.stderr:
            lines.append(line)
            if "server started, listening" in line:
                ready.set()

    reader = threading.Thread(target=read, daemon=True)
    reader.start()
    try:
        if not ready.wait(15):
            raise RuntimeError("sccache did not start: " + "".join(lines)[-3000:])
        yield lines
    finally:
        if server.poll() is None:
            subprocess.run([executable, "--stop-server"], env=env, capture_output=True, timeout=15)
        server.wait(timeout=15)
        reader.join(timeout=2)

"""Local heartbeat journal and dispatcher identity. Never edits coordinator or queue state."""
from contextlib import contextmanager
import ctypes
import json
import math
import os
from pathlib import Path
import subprocess
import time
import uuid


class SupervisionError(RuntimeError):
    pass


def atomic_json(path, value):
    atomic_text(path, json.dumps(value, indent=2) + '\n')


def atomic_text(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + '.' + uuid.uuid4().hex + '.tmp')
    try:
        with temporary.open('x', encoding='utf-8') as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        # Windows readers may hold a handle without FILE_SHARE_DELETE. Retry a
        # sharing/replace failure briefly, preserving the old artifact on exhaustion.
        end = time.monotonic() + 0.5
        while True:
            try:
                temporary.replace(path)
                break
            except OSError:
                if time.monotonic() >= end:
                    raise
                time.sleep(0.02)
    finally:
        temporary.unlink(missing_ok=True)


@contextmanager
def ownership(path, *, timeout=0, existing=False):
    """Separate byte/file lock, bounded contention; never unlink an ownership lock."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('r+b' if existing else 'a+b') as stream:
        if stream.seek(0, 2) == 0:
            if existing:
                raise OSError(f'empty ownership lock: {path}')
            stream.write(b'0')
            stream.flush()
        end = time.monotonic() + timeout
        while True:
            try:
                if os.name == 'nt':
                    import msvcrt
                    stream.seek(0)
                    msvcrt.locking(stream.fileno(), msvcrt.LK_NBLCK, 1)
                else:
                    import fcntl
                    fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except OSError as exc:
                if time.monotonic() >= end:
                    raise SupervisionError(f'ownership lock unavailable: {path}: {exc}') from exc
                time.sleep(min(0.05, max(0, end - time.monotonic())))
        try:
            yield
        finally:
            if os.name == 'nt':
                stream.seek(0)
                msvcrt.locking(stream.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(stream, fcntl.LOCK_UN)


def _number(value):
    return type(value) in (int, float) and math.isfinite(value) and value >= 0


def read_artifact(path):
    """Bounded Windows rename/read sharing recovery; never substitute healthy defaults."""
    for attempt in range(5):
        try:
            return json.loads(path.read_text(encoding='utf-8'))
        except FileNotFoundError:
            raise
        except (OSError, ValueError):
            if attempt == 4:
                raise
            time.sleep(0.02)


def _validate_incident(old):
    if not isinstance(old, dict) or old.get('version') != 1:
        raise ValueError('unsupported capacity journal schema')
    for key in ('last_observed', 'last_verified_age_seconds'):
        if not _number(old.get(key)):
            raise ValueError(f'invalid {key}')
    for key in ('active', 'clock_uncertain', 'stop', 'provider_paused', 'suppressed', 'launchable_shortfall'):
        if type(old.get(key)) is not bool:
            raise ValueError(f'invalid {key}')
    for key in ('target', 'live', 'open_slots', 'launchable', 'waiting', 'launchable_deficit'):
        if type(old.get(key)) is not int or old[key] < 0:
            raise ValueError(f'invalid {key}')
    age = old.get('age_seconds')
    if age is not None and not _number(age):
        raise ValueError('invalid age_seconds')
    first = old.get('first_observed')
    if 'first_observed' not in old or (first is not None and not _number(first)):
        raise ValueError('invalid first_observed')
    if first is None and (old['active'] or age != 0 or old['clock_uncertain'] or old.get('resolved_at') is not None):
        raise ValueError('inconsistent never-underfilled journal')
    if not old['clock_uncertain'] and (age is None or (first is not None and old['last_observed'] < first)):
        raise ValueError('inconsistent journal time')
    if old['active'] != (old['live'] < old['target']):
        raise ValueError('inconsistent capacity journal')
    if not old['active'] and first is not None and not _number(old.get('resolved_at')):
        raise ValueError('missing recovery observation')
    if old['open_slots'] != max(old['target'] - old['live'], 0):
        raise ValueError('inconsistent open slots')
    if old['launchable_deficit'] != max(old['open_slots'] - old['launchable'], 0):
        raise ValueError('inconsistent launchable deficit')
    if old['launchable_shortfall'] != (old['launchable_deficit'] > 0):
        raise ValueError('inconsistent launchable shortfall')
    if old['suppressed'] != (old['stop'] or old['provider_paused']):
        raise ValueError('inconsistent suppression')


def capacity(root, observe, *, now=time.time):
    """Serialize observation + update, so an older overlapping snapshot cannot resolve a newer incident.

    Age is wall-clock span since first observation, including suppression and unobserved gaps;
    it is NOT proof of continuous shortage. Clock rollback makes age unknown until recovery.
    Invalid prior bytes remain untouched and require operator repair, never silent reset.
    """
    directory = root / 'build-runs' / 'capacity-supervision'
    path = directory / 'incident-v1.json'
    try:
        with ownership(directory / 'writer.lock', timeout=0.5):
            old = None
            try:
                old = read_artifact(path)
            except FileNotFoundError:
                pass
            if old is not None:
                _validate_incident(old)
            elif path.exists():
                raise ValueError('null capacity journal')
            value = observe()
            at = now()
            if not _number(at):
                raise ValueError('invalid wall clock')
            active = value['live'] < value['target']
            continuing = old is not None and old['active']
            closed = old is not None and not old['active'] and not active
            first = old['first_observed'] if continuing or closed else (at if active else None)
            uncertain = bool(first is not None and (continuing or closed) and
                             (old['clock_uncertain'] or at < old['last_observed']))
            age = None if uncertain else (old['age_seconds'] if closed else (at - first if first is not None else 0))
            incident = dict(version=1, active=active, first_observed=first, last_observed=at,
                            age_seconds=age, clock_uncertain=uncertain,
                            last_verified_age_seconds=(old['last_verified_age_seconds'] if uncertain else age),
                            duration_basis='observation span; gaps and suppression included; continuity unverified',
                            target=value['target'], live=value['live'], open_slots=value['open_slots'],
                            launchable=len(value['launchable']), waiting=len(value['waiting']),
                            launchable_deficit=max(value['open_slots'] - len(value['launchable']), 0),
                            launchable_shortfall=len(value['launchable']) < value['open_slots'],
                            stop=value['stop'], provider_paused=value['provider_paused'],
                            suppressed=bool(value['stop'] or value['provider_paused']),
                            resolved_at=(None if active or first is None else
                                         (old['resolved_at'] if old and not old['active'] else at)))
            atomic_json(path, incident)
            value['capacity_incident'] = incident
            return value
    except (OSError, ValueError, SupervisionError) as exc:
        raise SupervisionError(f'capacity journal {path}: {exc}; evidence retained; inspect/repair before retry') from exc


def process_identity(pid):
    """Supported OS creation identity + actual argv. Failure is UNKNOWN, never name-only success."""
    if type(pid) is not int or pid <= 0:
        raise ValueError('invalid PID')
    if os.name == 'nt':
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.OpenProcess.argtypes = [ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
        kernel.OpenProcess.restype = ctypes.c_void_p
        kernel.GetProcessTimes.argtypes = [ctypes.c_void_p, *([ctypes.c_void_p] * 4)]
        kernel.GetExitCodeProcess.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
        kernel.QueryFullProcessImageNameW.argtypes = [ctypes.c_void_p, ctypes.c_ulong,
                                                     ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_ulong)]
        kernel.CloseHandle.argtypes = [ctypes.c_void_p]
        handle = kernel.OpenProcess(0x1000, False, pid)
        if not handle:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            created, exited, cpu_kernel, cpu_user = (ctypes.c_ulonglong() for _ in range(4))
            if not kernel.GetProcessTimes(handle, ctypes.byref(created), ctypes.byref(exited),
                                          ctypes.byref(cpu_kernel), ctypes.byref(cpu_user)):
                raise ctypes.WinError(ctypes.get_last_error())
            status = ctypes.c_ulong()
            if not kernel.GetExitCodeProcess(handle, ctypes.byref(status)) or status.value != 259:
                raise OSError('process is not running')
            size = ctypes.c_ulong(32768)
            executable = ctypes.create_unicode_buffer(size.value)
            if not kernel.QueryFullProcessImageNameW(handle, 0, executable, ctypes.byref(size)):
                raise ctypes.WinError(ctypes.get_last_error())
            if pid == os.getpid():
                kernel.GetCommandLineW.restype = ctypes.c_wchar_p
                data = dict(command=kernel.GetCommandLineW())
            else:
                data = _windows_command(pid)
            argv = _windows_argv(data['command'])
            if not kernel.GetExitCodeProcess(handle, ctypes.byref(status)) or status.value != 259:
                raise OSError('process exited during identity query')
            return dict(created='windows:' + str(created.value), argv=argv, executable=executable.value)
        finally:
            kernel.CloseHandle(handle)
    if os.name == 'posix' and Path('/proc').is_dir():
        proc = Path('/proc') / str(pid)
        stat = (proc / 'stat').read_text().rsplit(')', 1)[1].split()
        if stat[0] == 'Z':
            raise OSError('process is zombie')
        boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
        argv = (proc / 'cmdline').read_bytes().rstrip(b'\0').decode().split('\0')
        return dict(created=f'linux:{boot}:{stat[19]}', argv=argv,
                    executable=str((proc / 'exe').resolve(strict=True)))
    raise OSError('unsupported OS process identity')


def _windows_command(pid):
    command = (f'$p=Get-CimInstance Win32_Process -Filter "ProcessId={pid}" -ErrorAction Stop; '
                   'if (!$p) { throw "process missing" }; '
                   '[pscustomobject]@{command=$p.CommandLine} | ConvertTo-Json -Compress')
    result = subprocess.run(['powershell.exe', '-NoProfile', '-NonInteractive', '-Command', command],
                                capture_output=True, text=True, timeout=4,
                                creationflags=subprocess.CREATE_NO_WINDOW)
    if result.returncode:
        raise OSError(f'CIM command identity unavailable (exit {result.returncode}): '
                      f'stdout={result.stdout!r}; stderr={result.stderr!r}')
    data = json.loads(result.stdout)
    if not data.get('command'):
        raise OSError('CIM command unavailable (access denied or exited)')
    return data


def _windows_argv(command):
    shell = ctypes.WinDLL('shell32', use_last_error=True)
    shell.CommandLineToArgvW.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_int)]
    shell.CommandLineToArgvW.restype = ctypes.POINTER(ctypes.c_wchar_p)
    count = ctypes.c_int()
    pointer = shell.CommandLineToArgvW(command, ctypes.byref(count))
    if not pointer:
        raise OSError('CommandLineToArgvW failed')
    try:
        return [pointer[i] for i in range(count.value)]
    finally:
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.LocalFree.argtypes = [ctypes.c_void_p]
        kernel.LocalFree(pointer)


def _role(root, argv, cwd, provider):
    # Python flags/modules or wrappers are deliberately UNKNOWN, rather than guessed.
    if len(argv) < 2 or Path(argv[1]).name != 'lane_refill.py' or '--queue' in argv:
        raise ValueError('wrong dispatcher role/argv')
    script = Path(argv[1])
    if not Path(cwd).is_absolute():
        raise ValueError('missing canonical dispatcher cwd')
    if not script.is_absolute():
        script = Path(cwd) / script
    if script.resolve() != (root / 'tools/lane_refill.py').resolve():
        raise ValueError('wrong dispatcher script root')
    options = {}
    args = iter(argv[2:])
    for arg in args:
        if arg == '--once':
            continue
        key, separator, value = arg.partition('=')
        if key not in ('--root', '--provider') or key in options:
            raise ValueError('unverifiable dispatcher arguments')
        options[key] = value if separator else next(args, '')
        if not options[key]:
            raise ValueError('missing dispatcher argument')
    if '--provider' in options and options['--provider'] != provider:
        raise ValueError('wrong dispatcher provider argv')
    actual_root = Path(options['--root']) if '--root' in options else script.parent.parent
    if not actual_root.is_absolute():
        actual_root = Path(cwd) / actual_root
    if actual_root.resolve() != root.resolve():
        raise ValueError('wrong dispatcher root')


def publish_dispatcher(root, provider, *, identity=None):
    identity = identity or process_identity
    pid = os.getpid()
    observed = identity(pid)
    cwd = str(Path.cwd().resolve())
    _role(root, observed['argv'], cwd, provider)
    if not observed.get('created') or not observed.get('executable'):
        raise SupervisionError('dispatcher creation identity unavailable')
    marker = dict(version=1, pid=pid, root=str(root.resolve()), role='reviewed-refill-dispatcher',
                  provider=provider, cwd=cwd, **observed)
    runs = root / 'build-runs'
    # The owner lock must be held by the caller. Companion first: a partial pair is unhealthy.
    atomic_json(runs / 'run4-refill-owner-v1.json', marker)
    atomic_text(runs / 'run4-refill.pid', str(pid) + '\n')
    return marker


def dispatcher(root, provider, *, identity=None):
    identity = identity or process_identity
    try:
        runs = root / 'build-runs'
        marker = read_artifact(runs / 'run4-refill-owner-v1.json')
        if not isinstance(marker, dict) or marker.get('version') != 1 or marker.get('role') != 'reviewed-refill-dispatcher':
            raise ValueError('unsupported marker schema or wrong role')
        if Path(marker['root']).resolve() != root.resolve() or marker['provider'] != provider:
            raise ValueError('wrong root/provider')
        pid = int((runs / 'run4-refill.pid').read_text().strip())
        if pid != marker['pid']:
            raise ValueError('legacy PID disagrees with companion')
        actual = identity(pid)
        if not actual.get('created') or not actual.get('executable'):
            raise ValueError('missing creation identity')
        if any(actual.get(key) != marker.get(key) for key in ('created', 'argv', 'executable')):
            raise ValueError('stale/reused PID or changed process identity')
        _role(root, actual['argv'], marker['cwd'], provider)
        # An exited dispatcher (including --once) must not pass just because its PID still exists.
        try:
            with ownership(runs / 'run4-refill-owner.lock', existing=True):
                pass
        except SupervisionError:
            return dict(verified=True, status='VERIFIED', pid=pid, reason='OS identity and held owner lock match')
        raise ValueError('dispatcher owner lock is not held')
    except (OSError, ValueError, KeyError, TypeError, IndexError, subprocess.SubprocessError) as exc:
        return dict(verified=False, status='UNKNOWN', reason=f'{type(exc).__name__}: {exc}')

# Read-only worker process snapshots

Run with the worker's existing Python and installed `psutil`; no WMI, installation,
elevation or process management is used:

```
python .claude/lane-bin/process_snapshot.py --target C:/path/to/worktree
python .claude/lane-bin/process_snapshot.py --target C:/path/to/worktree --identities build-runs/identities.json
```

The helper writes a timestamped JSON snapshot under **the helper's own worktree**
`build-runs/process-snapshots/` and prints its path, capture UTC/local time and counts.
It never reads another worktree's source. Target must be an explicit existing directory.
Windows, Linux and macOS are supported. A missing psutil import produces an explicit
UNKNOWN dependency diagnostic for that interpreter; there is no fallback.

Optional identity input is a JSON list (up to 128 entries, 64 KiB):

```
[{"pid":1234,"create_time":1790000000.125,"argv_sha256":"<64 lowercase hex digits>"}]
```

Copy the exact creation time and optional argv digest from an earlier snapshot;
do not round times. Digest is SHA-256 of UTF-8 `json.dumps(argv, ensure_ascii=True,
separators=(",", ":"))`. An identity match says only that these fields match the
earlier observation. It does not attribute a process to the target by itself.
Changed creation time means stale PID; changed argv means argv mismatch. Races or
unreadable fields mean UNKNOWN. Missing PIDs are `not_observed`, never safe absence.

Scope is same-user processes with cwd/exe in the target, common build/interpreter
process names, or explicitly recorded PIDs. Same-user cwd/exe are inspected to
discover custom target processes; command lines and handles are requested only for
these candidates. Other users' paths/commands/handles are not inspected. Unreadable
ownership or scope paths remain conservative UNKNOWN candidates. No environment
is collected. Foreign candidates retain identity fields but omit command/path details.
Commands for other candidates retain exact absolute target path operands, argument
count and full-argv digest; all other argv elements are redacted, including option
values. This is a safe projection of the actual command, not a reconstructed command
or an executable invocation. Target paths themselves may be sensitive: keep snapshots local.

`target_related` means readable cwd/exe, an absolute argv reference, or a handle
under target/build-runs, with stable observed creation time. Evidence has explicit
psutil provenance. A reference does not prove exclusive ownership or writer intent.
`foreign` means a fully readable same-user build candidate without those references;
`unknown` means attribution could not be established. Parent/child links use observed
PPID and creation order only, within the scoped inventory, and never transfer ownership.
Relative argv paths, embedded shell commands and other opaque arguments are not parsed;
classification is limited to the documented absolute-reference evidence.

AccessDenied, NoSuchProcess, zombie processes and read races remain typed diagnostics
(no exception messages); diagnostics cap at 100 with an omitted count. Inventory caps
at 16,384 processes and reports truncation. Empty/failed inventory is UNKNOWN.
`OBSERVED` and `NO_TARGET_OBSERVED` describe readable scoped observations; errors or
unknown candidates produce `UNKNOWN`. All snapshots have `authoritative=false`:
psutil reads are sequential, Windows open-file enumeration can omit handles, and no
snapshot establishes global absence. **Exit 3 is intentional for all non-authoritative
snapshots**, including readable observations; input errors exit 2. Read the JSON and
summary rather than inferring safety from an exit code.

This is evidence at the recorded timestamp only. It grants no continuing permission
to launch, kill, restart, suspend, wait for, authenticate or reuse any process/service.

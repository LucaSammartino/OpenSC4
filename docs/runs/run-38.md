tag: run-38
run: 38
date: unknown
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: fire-fix build `69D9CA30…`, same config as 37, 420 s
result: **The fix holds.** A real ignition (`ignited=1/1`) with **0 over-release and 0 last-reference warnings** in the whole log. `nested=1/0/0/0` shows that `reentrant=1` is the fire manager reading our flammability grid once during ignition, which is benign. The game was closed by hand before the verdict, so this run also went through a full `Shutdown` after the fire (`flammability Shutdown (100 cycles)`), still with no warnings.

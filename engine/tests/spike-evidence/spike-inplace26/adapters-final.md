# Adapter cost

adapters-final.csv gives each function's one-line entry binding and the shared
input-packet mapping it uses. Shared numbers repeat for reachability; count them
once, not once per row. Physical line spans include comments and blank lines.

New native entry bindings: 20 lines. Shared C++ Dispatch input/window mapping:
29 lines, plus a 3-line C callback. Cached Rust ground-line input repair: 11 lines
used by view-box. Total new bindings/mapping: 63 lines. The shared machine-state
bridge is a separate 139-line header. Capture/comparator and trampoline decoding
are generic facility code. Tests are excluded from these adapter counts.

Existing private Rust word transports and production algorithms are reused;
functions.csv's binding_lines column records the INITIAL typed wrappers, not
the corrected machine-state entries. No new production Rust algorithm was needed.
Elapsed time includes all adapter/facility work, machine-state debugging, the
input repair, both rounds, waits and verification; no time is subtracted for
reuse. The reused adapter's missing lines were a real cost, not a native numeric
disagreement. Cohort selection favours one existing word transport, so the line
count is not an estimate for the nine unadapted brief candidates.

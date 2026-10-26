# Host-boundary replay (pilot)

Set `trace_record = C:/.../build-runs/traces/<batch>` and an explicit
`trace_scenario = <census scenario name>` in the plugin ini, with the budget,
power and pollution shadows enabled. Recording is off by default. Use the
ordinary shared game queue; the recorder does not launch the game.

The recorder delegates each host call once, preserving callback enumeration
order and returned object ownership. It records callback arguments and
answers, before/after original save records, step dates, forwarded power
messages, wants decisions, comparison return values and the exact shadow
log payloads. Budget stores the original assessed sums, taxes, total and the
income/expense observability used by the C++ report. Expense composition is
an observation of the existing shadow, not an independently simulated result.

Completed sessions become SHA-256-named files. Budget produces one session per
month; power/pollution produce one per shadow lifetime. Incomplete `.partial`
files are not put in the manifest. Identical completed files share storage.
`manifest.tsv` appends six tab-separated columns without a header:
`scenario key sha256 compared_steps mismatches incomplete_steps`.
Scenario names must not contain control characters. A trace directory should
have one recording process writing its manifest at a time.

Run without a game:

```
cargo run --release --manifest-path engine/Cargo.toml -p fc4-capi --bin fc4-replay -- --manifest <dir>/manifest.tsv
```

Every unique trace is checked on the available CPUs (`--jobs N` overrides).
The replayer calls the same C ABI shadow entry points and their existing
comparison code. It verifies every callback name, argument and sequence;
an extra read, missing frame, changed argument, malformed data or unexpected
output fails replay. It never calls a live host on a miss. Callback fallbacks
return zero only after recording the failure, to unwind the shadow safely;
such a run cannot receive an exact verdict. Capture references are inert
numeric identities, not pointers into the replay process.

`replay-verdicts.tsv` reports each scenario/key, trace and source hashes,
exact/inexact/incomplete, shadow agreement, comparison/mismatch/incomplete
counts, cache use and error details. Agreement with a recorded inexact shadow
is distinct from an exact verdict. Zero observations never count as exact.
Exit codes: 0 all exact, 1 inexact/incomplete, 2 invalid invocation/manifest.

Cache files under `verdict-cache/` are keyed by trace SHA-256 and the compiled
source fingerprint. The fingerprint covers all Rust sources and Cargo
manifests/lockfile of `fc4-capi` and its local dependencies (`core`, `formats`),
including the thunk generator. These owners share crates, so invalidation is
conservative: any source change in those crates invalidates all pilot owners.
The fingerprint is embedded at build time, not read from a possibly edited
checkout at runtime. Trace content hashes are checked even on cache hits.
`--no-cache` recomputes verdicts.

Format v1 starts with `FC4TRACE` and a little-endian version word (1). Each
frame has a u32 byte length, u8 kind, u64 UTF-8 name length, name bytes and
typed payload. Scalars are little endian; floats retain their bits; usize
lengths use u64; nullable pointers are presence flags plus typed contents;
there is no native-struct padding or function address. Event size is bounded
at 128 MiB. Only a successfully closed trace has an end frame. This is a
local testing format, not an untrusted network protocol.

Coverage tool ingestion and attribution of the recording census's reach
counters belong to the coordinator. This implementation does not create reach
credit or infer reach from replay. Further callback-host keys and live-object
or draw-stream adapters remain outside the three-owner pilot.

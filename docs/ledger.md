# Run-7 function ledger

Coordinator/RE tooling; engine lanes consume contracts and value-only failures.

```text
python tools/ledger.py
python tools/ledger.py --log build-runs/compare.log --min-natural 20 --min-dense 200
python tools/coverage.py
```

The default output is `build-runs/ledger/functions.tsv`, one row per game
function in coverage's denominator, plus `summary.md` and `inputs.json` with
input hashes, selected logs, thresholds and warnings. `--out` changes the TSV
path; the two companion files go beside it. `--export`, `--exe` and `--regress`
have the same meanings as in coverage. Repeating `--log` overrides the house
compare evidence for the ledger only; coverage always uses the house run.

Attribution, subsystem assignment and specified/implemented/verified/replaced
come from `coverage.load_report_data`; the ledger does not redefine them.
Entry contracts (`docs/abi/functions.tsv`) and bindings
(`engine/plugin/compare/*.tsv`) are optional until their lanes land. Missing
inputs produce empty fields and zero credit. `contract_*` and `binding_*`
preserve their source columns. `spec_citations` is a JSON list of direct address
mentions with file, section hierarchy and line, including tables and prose.
Question records are excluded. Owner-level specified coverage does not imply a
direct address citation.

`bound` means a unique registry row. `matched` requires a binding and at least
20 exact natural comparisons in one log. If supplied evidence contains corpus
or dense summaries, each such source needs at least 200 exact comparisons in
that same log. The thresholds are configurable and must be positive. Fixture
comparisons must be exact but do not satisfy either threshold. Float-only,
different, outside, mutation or detected-error signals anywhere in the supplied
logs veto matching. Malformed or incompatible summaries also veto it. Refused
and nested calls do not count as comparisons. An installed adapter, matching
name and matching function byte size are required. Extra trailing summary
fields are tolerated. Repeated cumulative summaries use counter maxima,
retaining failures, and counts from different logs are never pooled.
`replaced-fn` additionally requires registry mode `replace` and the same passing
log evidence. It is separate from coverage's existing owner-level `replaced`.

By default both tools read the selected house regress `.out` plus adjacent
`-shadows.log` and `-replace.log` files when present. They never search other
lane runs for positive evidence. The coverage report appends byte-weighted
Bound and Matched columns (function percentages in brackets) to its area and
subsystem tables. The original columns retain their existing rules.

The oracle classifier ports the coordinator's reference scan with precedence
`os` > `vtable` > `globals` > `closed`. It follows direct calls, including library
callees and recursive call graphs, and recognises indirect calls, imports and
absolute accesses in writable data at `0xb07000..0xbbffff`. The reference's
KERNEL32 allowlist is preserved. Library bodies are scanned as well as game
bodies so a transitive effect is not lost. These are static planning classes,
not proofs of runtime equivalence; unknown indirect calls are `vtable`.

The summary lists the ten largest owners by implemented-but-unmatched bytes,
with the oracle-class byte split, for selecting binding lanes.

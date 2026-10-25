# Run-7 compare harness evidence

The stable API is in `docs/harness.md`; class-binding instructions are in
`docs/ops/compare.md`. All compare switches remain off by default.

`measure.py <label> --raw <worktree>/build-runs` recomputes each JSON summary;
`--rows` recomputes the CSV. It uses the last cumulative report per function and
source, not the sum of repeated reports. Requests beside the data reproduce runs.
DLL fingerprints distinguish earlier source versions from final checks.

The 28 requested prior bindings are retained, plus two historic declined rows
(system-country and lighting-night). The other declined rows are rotation and
visible-edges: entry relocation still refuses all four. There are 26 installed
functions, with no function dropped to make the pass fraction larger.

Prior combined fixture count is reproduced: 512 view-utility comparisons plus
128 round-3 comparisons = 640, all exact. Debug mutations reproduce the combined
26 fixture + 17 natural detections. Natural call volumes vary with rendering
cadence; the absolute old counts were not reproduced. All compared natural
outputs remain bit exact. This is no new executable-byte coverage/rate claim.

Earlier failures remain separate:

- capture-1, DLL 60202b37: startup watchdog stall, caused by an unsampled outer
  hook retaining the lock across a sampled nested call. Fixed and regression
  tested with authored native code. Partial corpus quarantined in build-runs.
- capture-2, DLL 2f7c7163: clean; 2150 natural and 640 fixture comparisons exact,
  43/43 mutations detected and located. One clip3 output-alias refusal.
- dense-1, same DLL: clean, but 7 corpus and 56 dense view-box return differences.
  The persisted scalar schema omitted the published camera ground lines at
  +0x208..+0x224; native replay therefore received inconsistent camera state.
  `ground-lines-omission.jsonl` preserves the first non-mutation failing case.
  No Rust algorithm change was made. Old corpus/cases remain quarantined.
  A natural lighting sample also had one unattributed neighbourhood write;
  it was excluded rather than credited as a match.
- capture-3 / dense-2, DLL 31d74285: clean; 2233 natural + 640 fixture exact,
  43/43 mutations detected; fresh corpus 1460/1460 and dense 11664/11664 exact.
  Dense has 16 explicit domain refusals for zero/current-language fallback.
- record-1 / record-2: clean; class selector works, no Rust comparisons, 449 / 497
  appended inputs. Record-2 used a 100-record write cap; final replay uses the
  default 50 cap and must still read every previously stored input.
- replace-1, DLL a5e4911c: clean; clip2 and box-rectangle replaced 2061 calls.

`cross-mutations.jsonl` is a small valid-input value example. Conversion with
`engine/tests/compare_case_to_rust.py` produced `engine/tests/vectors/compare_cross.rs`;
the generated module compiles with rustc and passes the owned-file fmt check.
Its native outputs are test expectations; mutation=true labels the deliberately
incorrect ours outputs. Image pointer fields use image_offsets metadata and RVAs.

The native dispatch tests prove compare runs both sides, record runs no adapter,
replace runs no original, replay leaves inputs isolated, and lowering the write
cap does not truncate replay. Schema tests prove pointer/padding stripping and
preservation of scalar strided fields without persisting stride gaps.

Limitations: net changes in containing private allocations only; undone writes,
unrelated allocations and image globals are not watched. Concurrent changes are
unattributed. Aliases and unsupported adapter domains refuse matching evidence.
The final run JSON/CSV files carry the final measurements, including all refusals.

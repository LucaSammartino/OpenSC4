# Binding and comparing one function

Everything is opt-in; ordinary game runs install no compare hooks.

1. Read the function's spec section, `docs/abi/functions.tsv` when available,
   and its published layout. Never derive a contract from original code.
2. Add a row to `engine/plugin/compare/<Class>.tsv`, keeping the exact header
   documented in `docs/harness.md`. Use the published address, convention,
   argument count, cleanup and return channel. Unknown contracts cannot bind.
3. Describe each receiver/argument window and its scalar fields. Leave pointers,
   padding and unknown fields unspecified. `image` fields persist RVAs only.
   Mark safe scalar fields `!` for dense perturbation; preserve object invariants.
4. Add an entry to `engine/capi/src/compare.rs` using the existing Rust owner.
   It receives owned native-layout buffers, sizes and scalar argument words.
   Never dereference raw argument pointers. Return false outside its domain.
   Optional hand fixtures use the adjacent fixture adapter. No plugin C++ edit
   is needed; CMake generates entry shells and dispatch rows from the registry.
5. Build with the lane build gate, then submit a game request with
   `compare = <name>` (or `class:<Class>` / `all`). Use turbo plus an end date.
   `inplace` is the compatible selection alias. Inspect install/refusal lines.
6. For reusable inputs add `compare_record = 1`, or use
   `compare_mode = record` to record without running Rust. Files are bounded,
   value-only `build-runs/compare-corpus/<name>.bin` in the building worktree.
7. On a later run set `compare_corpus = 1` and `compare_dense = 8` to replay
   each stored input and nearby variants after city load. Keep hand fixtures
   with `inplace_fixture = 1`; they have their own source counters.
8. Read the last `inplace summary` for each function/source. Exact means equal
   declared return and all captured bytes; float_only is still a mismatch.
   Outside/refused samples provide no matching evidence. Confinement only
   watches net changes in the captured allocations, not the entire process.
9. First differing cases, including debug mutations, are appended to
   `build-runs/compare-cases/<name>.jsonl` (bounded per run). Convert one with
   `python engine/tests/compare_case_to_rust.py <case-file> --output
   engine/tests/vectors/<name>.rs` (put the command on one line).
   Add the generated module to the owning Rust tests and reproduce the failure
   using values. `mutation:true` cases are deliberate comparator tests.
10. Fix from the spec; mark spec gaps in code. Rerun the failing vector and game
    compare, retaining real mismatches in evidence. Use `inplace_mutate = <name>`
    to prove one flipped output bit is detected and located independently.
11. Only after matching, set that row's mode to `replace`, or explicitly use
    `compare_mode = replace` for a previously matched selection. Supported calls
    then run Rust only; refused domains fall back to the original and count as
    refused. Compare replay always runs both implementations in isolated copies.
12. Commit code and small evidence, report all source counts and limitations,
    run the gate and queue the lane normally. Do not claim coverage credit here.

The stable row grammar, binary corpus framing, summary and ini keys live in
`docs/harness.md`. Internal `CompareAttach(name, callbacks)` lets outgoing-call
interposition attach before startup without changing registry dispatch.

# Registry pilot review: withheld

This is a tooling admission review, not a native or engine validation. No records are activated in
`tools/coverage-functions.json`, including specified-only records. No coverage-map row is changed.

The public [Registry contract §3.2](../specs/game-data-registry.md#32-lookup-and-access-channels) publishes
null false/read-null behavior, full 32-bit context store/read and `(flags & mask)==mask`, with zero mask true
for every non-null record. Its per-body table supplies the only candidate addresses used here.

Running unchanged `Attribution.run()` on the sanctioned normal inputs on 2026-10-23 independently returned:

| Entry | Published method | Current owner | Current rule | Actual body bytes | Admission |
|---|---|---|---|---:|---|
| 00473B50 | CheckEntryFlags | cSC4GameDataRegistry | vtable | 32 | withheld |
| 00473B70 | SetEntryContext | cSC4GameDataRegistry | vtable | 25 | withheld |
| 00473B90 | GetEntryContext | cSC4GameDataRegistry | vtable | 19 | withheld |

These are measured metadata, not credited bytes. The public owner map currently has no spec/core baseline for
this owner. Production `check_flags`, `set_context`, `context` in
`engine/core/src/game_data_registry/mod.rs` have corresponding operations. Independent literal assertions in
`g6_g7_full_width_flags_context_and_raw_publisher_dispatch` and
`g8_null_missing_leaf_visits_and_borrowed_outputs` in `tests.rs` exercise full-width flags, zero node mask,
null flags/context/setter, full-width context read and successful zero context store/read. Source presence and
these assertions alone do not establish a reviewed whole-function validation record.

The copied `core-game-data-registry4.md` report describes a successful final-source gate, retained source hashes,
raw-log digests and `registry-r2-final/qualification.json`. Only the report was supplied in this lane's copied
evidence; the referenced qualification/raw logs were not supplied as tracked public source-matched proof. This
lane neither reaches into another worktree nor reruns engine gates to fill that gap. The report's successful
counts are not being disputed, and no new gate/proof is minted from them. No independent exact-entry semantic
qualification and numeric-credit review has been published for these three records. All claims therefore remain
withheld. Publishing and reviewing the actual source-matched provenance and correspondence would be necessary
before a future ledger change; no amount of future credit is promised.

The completed Directory/HTML owner audit remains unchanged and withheld. No other owners are qualified here.

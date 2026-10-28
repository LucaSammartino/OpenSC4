| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-sr36 save layouts (RE/spec) | Codex; re-sr36 worktree; detached HEAD, reserved branch re-sr36 | wip: specs complete; commits c225f49b7, 9f1e343d4, 206e025a4; corpus 60/60; diff passed; global question check failed (18 pre-existing errors); ABI unchanged, no seam reservations; game/shadow n/a; SR33-2 and SR33-3 answered | 1. Clear the existing question-check failures before enqueueing. 2. Reconcile the two archived SR33 answers with save-roundtrip33's question file when it lands. |

- Layouts: traffic-network §6.2 (map v3+ gates, tiles, intersections/sides, references, occupant-to-four-lists table); foundations §3.6 (city v8–12); save-records §2.1 (auxiliary v1 and inner cheat data).
- Map ID correction: IID 0xCA0F81B9, class/record type 0x6A0F82B2; documented creation-order walks and variable-width references.
- Verification: build-runs/verify_sr36.py, run directly in PowerShell, exit 0; first 20 lexicographically keyed main/build-runs/golden/*/end.sc4 files, read in place.
- Map: 20 v7 objects, 37,968,364 decompressed record bytes; 87,983 tiles, 3,027,869 path pairs, 2,785 tile references, 112,980 extra triples, 6,859 intersections, 18 final occupant entries / 36 list values.
- City: 20 v12 objects, 11,649 decompressed record bytes, 1,060 component-reference slots.
- Auxiliary: 20 v1 records, 520 decompressed bytes; all inner 20-byte empty cheat blocks independently parsed, including both total sizes and section lengths.
- Total: 60/60 target records, 37,980,533 bytes consumed, zero bytes left over; class byte counts include entry headers. No partial/failed corpus parses.
- Legacy gates, nonempty city property holders, marked-train lists and keyed-reference lists were verified statically, not exercised by these 20 saves.
- REA batch returned 4/4 target_unavailable (no app open); used re_query.py, main decomp/ and byte-bearing assembly pages as prescribed. Float constant 0x00A8FDF4 also checked in the executable.
- The q.py attempt failed before launching the parser (Bash CreateFileMapping error 5); both subsequent direct PowerShell corpus runs passed. Final run additionally checked the inner auxiliary layout.
- SR33-2 / SR33-3: docs/specs/questions/answered/save-records.md; questions.py index passed; find resolves both as answered. The open save-records.md question file was absent in this base and main, so archive entries use the assigned brief.
- Gate: git diff --check passed. questions.py check failed with 18 existing diagnostics in untouched files (application, aura, data-formats, demand, growth-developer, landvalue, network-manager, network-occupants, ordinance, pollution, terrain); no diagnostic concerns this lane's files.
- Cargo/clippy/ctest and game/shadow runs n/a for this spec-only lane. No engine/, tools/, docs/abi/ or executable edits; no proposed ABI rows.
- Commits succeeded on detached HEAD despite the shared packed-refs.lock permission diagnostic; shared refs were not changed.

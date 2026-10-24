| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-persist-shadow8: persistence resource shadow | Codex, glue-persist-shadow8 | ready (measured partial scope) | Land cache type projection; answer PR9/SCR11 before whole-owner verification. |

## Result

Default-off `persist_shadow = 1` adds a read-only, record-only **cache type projection** at city load and monthly. This is measured partial work, not full resource-manager or buffer-resource verification.

The unfiltered native cached key sequence is input to production core `persistence_manager::Filter::Type` and `KeyList::insert`. Every observed type, plus independent type `0xF4C4E008`, is queried separately through native `GetResourceListForType`. Counts, ordering, duplicate occurrences, and all three 32-bit key fields are compared exactly. Native filtered outputs are never supplied as expected answers. A one-million-key capture limit fails the sample rather than silently truncating it. Factory count is diagnostic only. No native registration, load, cache mutation, pixel lock or factory replacement occurs. Only newly created result lists and a scoped SDK service reference are acquired; all references end inside the sample. Globals contain only flags and counters. No resynchronization exists.

Public C ABI exports and ABI version are unchanged; the Rust transport is a private static-link symbol. Shared CMake/config additions are at the ends of their lists. `persist_shadow` is registered as a known ini key and remains off by default.

## Validation

- Focused duplicate/order/all-key-field and bounds test: 1/1 passed. Explicit owned Rust file formatting: clean.
- Final stable source `11cb1d7fa`: `qbuild.py ... all` PASS; Rust **6939 passed, 0 failed, 0 ignored**, clippy **0 diagnostics**, queued native build **48/48 edges**, full **CTest 78/78**. Native queue ID `qbuild-20261024-062651-build-ctest`; logs `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Earlier full gates passed too: initial 250/250 native edges and 78/78 CTest; after known-key correction 205/205 edges and 78/78 CTest. Repeats followed actual diagnostics: missing known-key registration, then the load-date issue found in game. Initial logs remain in `build-runs/persist-initial-*.log`; the first game evidence is retained separately.
- Final [run 435](../runs/persist-shadow8-r2-2032-20261024a-435.md), label `persist-shadow8-r2-2032-20261024a`, DLL SHA256 prefix `1f6e0fdc`: **clean**, turbo through **2032-01-02**, **34.781 s**, **foreign_exceptions=0**, **ref_warnings=0**, all other harness problem counts zero.
- Exact counted result: **`persist 73/73`**, **0 resynced lines**. Valid load date **2026-01-27**, 72 monthly lines ending **2032-01-01**. **149605** ordered keys compared across samples (minimum **1750**, maximum **9962**); all mismatch counts zero. Shutdown reports 73 lines, 73 with mismatches=0, resyncs=0.
- [Run 434](../runs/persist-shadow8-r1-2032-20261024a-434.md) is diagnostic, not final dated-line evidence: clean and all comparisons exact, but load line used zero PublishedJulian and logged -4713-11-24. Fixed by the same borrowed simulator date getter used by house shadows. No core algorithm changed for that fix. No turbo-only comparison differences occurred, so no normal-speed rerun was required.

## Coverage rows and gaps

Rows are identified by the owner names in the lane brief; the coordinator-owned tools map was not read or edited. **No whole-owner row is verified by this lane.**

| Owner / coverage row | Wholly compared? | Measured or open |
|---|---|---|
| cGZPersistResourceManager | No | Cached type-filtered key projections only. Factory-ID/type registry, cache storage/history, refcounts, loads/creates/writes, lifecycle and concurrency replay remain open (PR9; existing PR6/PR7 also apply). Factory count observed at load: 22, not compared to core. |
| cGZPersistResourceKeyList | No | Core manager key-list append/size/order/duplicates participate; erase, presence, QI and reference lifecycle are not compared. |
| cGZPersistBufferResource | No | No content replay or pixel comparison; SCR11 requests independent source/mutation inputs and safe observation conditions. |
| cGZPersistDBRecord | No | Untouched. |
| cGZPersistDBSerialRecord | No | Untouched. |
| cGZPersistResource | No | No native resource metadata/refcount replay. |
| cGZPersistResourceFactory | No | No factory registration or creation replay. |
| cGZPersistASCIIWriter | No | Untouched. |
| Other persistence/resources rows | No | No credit requested. |

Questions appended: **PR9** in `docs/specs/questions/persistence-manager.md`; **SCR11** in `docs/specs/questions/sc-resources.md`. Native answers are not sufficient independent replay inputs for registry/cache history or mutable buffer pixels. Those scopes remain disabled rather than seeded from native expected answers.

## Next steps

Land the measured projection under its explicit scope. Coordinator: regenerate the questions index; resolve PR9/SCR11 before implementing broader replay or granting whole-owner coverage. Tools coverage maps and the regress shadows profile remain untouched under the clean-room/no-tools-edit rule; any profile append must be coordinator-owned after review of this partial scope. No outstanding build or game request remains.

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rzdirectory4 (complete cRZDirectory core owner) | `core-rzdirectory4` / detached reserved worktree (engine lane) | ready; chapter 6 owner complete with RZ9-RZ11 literal edge choices; 25 directory tests; cargo 4732/0, clippy 0 diagnostics, ctest 68/68; ABI unchanged; game/shadow n/a | (1) Integrate; (2) regenerate question index; (3) resolve RZ9-RZ11 when specified. |

## Report

Implemented the complete cRZDirectory owner published in
[framework-rz-files.md chapter 6](../specs/framework-rz-files.md#6-crzdirectory-round-3), in
`engine/core/src/rz_files/directory.rs`. The owner stores its ordered entries, count, byte path,
filter and dot policy. Its host exposes primitive OS find-first/find-next/find-finish and single
directory create/remove operations. All list construction, dot handling, recursive traversal,
filter switching, sorting and rollback are performed by the owner.

Windows path separators and framework-encoded bytes remain explicit. Invalid UTF-8 is preserved;
encoding conversion and Win32 wildcard matching belong at the OS boundary. Enumeration retains OS
order, using only attribute bit 0x10 to distinguish directory type 4 from file type 8. The directory
entry struct is passive data; this does not implement the separate cRZDirectoryEntry/path owners.

No CAPI, plugin, CMake, ABI, coverage-map, runtime or default configuration changes. No game requests,
shadow comparison, replacement or default promotion; the native result below is the build/ctest gate.

## Published-owner checklist

Every body/helper published in chapter 6 is implemented; the three edge choices below are explicitly
commented and questioned rather than claimed as measured native equivalence.

| Spec | Published bodies / owner responsibility | Implementation and evidence |
|---|---|---|
| 6.1 | Owned entry list, count, path, filter, dot state | Private state; read-only entry/count/path/filter access; passive entry values |
| 6.2 | Separator predicate; MakeSurePathEndsWithSeparator | Slash/backslash byte checks; empty becomes backslash; mixed separators, spaces and non-UTF-8 retained |
| 6.3 | Constructor; ChangeDirectory; SetNewDirectoryEntryFilter; destructor | Optional reads, star default/empty filter, stale snapshots without read, unconditional ChangeDirectory success; explicit filter/path/list destruction order |
| 6.4 | ReadDirectoryEntries; GetNthEntry; existing-name synthesis check | First failure clears; raw attribute 0x10; OS order; first-dot policies and ordinary duplicates; front synthesis with duplicate check; front-walking nth and bounds |
| 6.5 | ReadDirectorySpecificEntriesIntoStringList; file wrapper; directory wrapper | Caller append, independent flags 8/4/1/2, wrapper masks, front synthesis without duplicate check, first-failure output preservation; owner snapshot untouched |
| 6.5 | AlphabetizeEntryList | Explicit stable merge sort, unsigned byte/prefix ordering, equal-allocation order preserved; owner entries never sorted |
| 6.6 | ReadDirectoryFileEntriesIntoStringListRecursive | Files-first preorder/OS order, process-wide depth, original filter restored for descent, last-directory owner snapshot, always true; missing child separator retained; no invented limit or cycle repair |
| 6.7 | DoesAnyEntryExistThatMatchesPattern; DoesDirectoryExist | Exact primitive find patterns; successful first find immediately closed without find-next; empty/root and byte paths |
| 6.7 | CreateDirectory | Prefix-by-prefix existence/create, UNC server-prefix skip, existing-level skip, creation-order rollback including failed parent removal |
| 6.7 | SplitDirectoryPath | Last-separator split, mixed separators, drive-root empty parent, transactional empty/length failure, 254-byte boundary |

The process-wide depth counter uses an atomic solely to avoid Rust data races; traversals are not
serialized or isolated. Cross-thread interference remains observable. GetNthEntry has a narrow
`clippy::iter_nth` allowance because section 6.4 explicitly requires front traversal.

## Independent tests and measured verification

`engine/core/src/rz_files/directory_tests.rs` contains 25 Rust tests with scripted primitive OS
results and independent expected outputs. It covers all 16 flag masks, all four owner dot policies,
OS order, hidden/system/read-only attributes, repeated and synthesized dots, exact byte names,
stale snapshots, first-find failures and handle closure, stable duplicate ordering, wrapper masks,
recursive filter restoration/preorder and missing separators, cross-owner and cross-thread global
depth, 81 traversal levels with a 405-byte final directory path, prefix creation/UNC/rollback,
published split examples and both output-length boundaries.

- Final `qbuild all` on committed HEAD `8254ca05`: **PASS**, tree unchanged during the run.
- Workspace cargo: **4732 passed / 0 failed / 0 ignored**; all **25** directory tests passed.
- Clippy with `-D warnings`: **exit 0, diagnostics 0**.
- Native build: **2/2 edges PASS**; full ctest: **68/68 PASS** (87.9 s; fc4verify 89.7 s).
- Post-correction stable `qbuild quick`: **2746/0** core, dependents compiled, **4/4** native edges,
  PASS (182.2 s).
- `qbuild fmt` on all three touched Rust files: **PASS**. `git diff --check`: **PASS**.

Final logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`,
`build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`; focused logs are
`build-runs/fc4verify-quick-{test,deps}.log`. The initial full run caught only the traversal lint;
its logs are retained as `build-runs/first-all-{test,clippy,ctest}.log`. After its documented lint
correction, both the focused check and full gate passed on stable committed source.

Implementation commits: `e8bfee92` (owner and initial vectors), `5d755808` (depth/byte boundaries),
`6fac505f` (documented traversal lint exception). Other commits record verification milestones.

## Questions and next steps

Appended to [questions/framework-rz-files.md](../specs/questions/framework-rz-files.md):

- **RZ9:** dot-seen bookkeeping when flag 4 is absent; literal skip-before-seen implemented.
- **RZ10:** split limit before/after adding a separator; resulting-part limit implemented.
- **RZ11:** UNC server prefix without a third backslash; entire-prefix skip implemented.

Coordinator can integrate this core-only lane and regenerate the question index. Apply later spec
answers to these explicitly marked edge cases. No additional lane work or game request is pending.

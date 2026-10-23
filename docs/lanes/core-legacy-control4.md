| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-legacy-control4 (portable LegacyControl owner) | Codex / core-legacy-control4 (engine lane, detached HEAD) | ready: bounded 45 entries, 42 complete safe-domain / 3 partial; cargo 5162/0, owner 22/22, clippy clean, native complete, ctest 68/68; post-fix fmt passed; ABI unchanged; LC4/LC5/LC8/LC9/LC10 retained | (1) Coordinator land bounded portable scope only; no fourth round or owner restart |

# Portable LegacyControl owner report

Source milestones: independent literal fixtures `02bd46f2` preceded production
`bf469091`; source/tests finalized in `d8d3da6f`. The original round stopped at
`ce60e13a`, preserved as an ancestor. Round 2 reconciled validation in `8949db4a`
and recorded the audit/format result in `34dc4033`. Round 2 removed only two
redundant mutable local rebindings from the vocabulary test after Clippy reported
them. Production, fixtures, all assertions/counts/tolerances and all 22 test
functions remain intact. No ignored tests or lint suppressions were added.

## Exact 45-entry inventory

Complete means the published portable effects in the declared safe domain,
including explicitly allowed deterministic choices. It does not mean native
memory layout, unsafe CRT inputs, whole native UI behavior, or default readiness.
Conservatively: 42 complete entries, 3 partial entries, 0 wholly unsupported
entries within these 45; 12 further owned interpreters remain Unsupported (LC4).

1. QueryInterface — complete: four IIDs, interface identity, AddRef, untouched output on failure.
2. Release — complete: decrement above one; count one or zero destroys without first decrementing.
3. Destructor/scalar deleting destructor — complete portable detach/shutdown/ref order; Rust tombstone/storage replaces native deletion.
4. GetControlVocabulary — complete borrowed getter.
5. SetOwnControl — complete flag setter.
6. SetOwnControl legacy thunk — complete same setter.
7. SetParent — complete release/null old, legacy QI, acquire control, release temporary, unconditional true.
8. UseWinProc — complete equality no-op, new AddRef before old Release.
9. ForceControl — complete same ref ordering, leaves own flag unchanged.
10. GetID — complete null/default/read channel.
11. SetID — complete null/no-op/write channel.
12. DoExceptions — complete true-only QI, one-shot procedure handoff, procedure then interface release.
13. Constructor — complete defaults, lazy creation, exact registration order, vocabulary reuse AddRef, usable on failure.
14. ImplementInterface — complete ignored-argument true stub.
15. SetParameters — partial: complete published known/no-interpreter builder; successful unknown per-type QI stops explicitly Unsupported (LC4).
16. GetScriptFragment — partial overall: complete allocation/reference/output wrapper and known frame/common; unknown successful writer QI marks PartialLc8.
17. ControlVocabularyCommon — complete 64 words, no map inserts.
18. ControlVocabularyFlatRect — complete registration and map inserts.
19. ControlVocabularyText — complete registration; interpreter remains LC4.
20. ControlVocabularyTextEdit — complete registration; interpreter remains LC4.
21. ControlVocabularyListBox — complete registration; interpreter remains LC4.
22. ControlVocabularyCombo — complete registration; interpreter remains LC4.
23. ControlVocabularyBtn — complete registration; interpreter remains LC4.
24. ControlVocabularyBMP — complete registration; interpreter remains LC4.
25. ControlVocabularySlider — complete registration.
26. ControlVocabularyScrollbar — complete registration; interpreter remains LC4.
27. ControlVocabularyLineInput — complete registration; interpreter remains LC4.
28. ControlVocabularyOptGrp — complete registration; interpreter remains LC4.
29. ControlVocabularySpinner — complete registration; interpreter remains LC4.
30. ControlVocabularyOutline — complete registration.
31. ControlVocabularyTextTicker — complete registration; interpreter remains LC4.
32. ControlVocabularyFileBrowser — complete registration.
33. ControlVocabularyTreeView — complete registration.
34. ControlVocabularyFolders — complete registration, no own property words.
35. ControlVocabularyGrid — complete registration.
36. ControlVocabularyCustom — complete registration, no own property words.
37. ControlVocabularySC3WinGen — complete registration, SC3WinGen/GZWinGen aliases share their class.
38. InterpretPropertiesCommon — complete published safe effects, explicit area-zero choice.
39. InterpretPropertiesSlider — complete: max before min, direction, resource references/keys, signed second-pass value.
40. InterpretPropertiesTreeView — complete published unique-key effects, retained positions, maps/root/fonts/depth-first build; duplicate keys explicitly Unsupported (LC10).
41. InterpretPropertiesFileBrowser — complete: raw getter defaults, font precedence, mappings and pair-position filter keys.
42. InterpretPropertiesGrid — partial: safe published effects implemented; uninitialized scratch/resource and unsafe malformed cells Unsupported (LC10).
43. InterpretPropertiesFolders — complete: fonts, image-key effects/references, explicit unparsed-field zero choice, repeated second-pass elements.
44. InterpretPropertiesFlatRect — complete: all repeated type-6 styles ORed, unconditional style setter, edge colors.
45. InterpretPropertiesOutline — complete: nonzero-style setter, colors, 32-node cap, explicit unparsed-component zero choice.

The 21 vocabulary functions are table-driven core algorithms with 393 exact
ordered entries, 20 class and 20 interpreter map entries. Common has no map
entry; aliases add words without extra owner credit. Custom's shared bare-return
interpreter, AddRef/GetControl, area support and property readers are implemented
support, not additional named owner bodies. Broader 205-row/104642-byte
attribution is not implementation credit for this 45-entry scope.

## Core ownership and retained quirks

Core owns typed linear property scans, numeric/field parsing, type selection,
lookup order, defaults, maps, build ordering, two passes, resource/reference
decisions and interpreter payloads. Foreign provides individual factory/QI/ref,
field/raw-widget operations, fonts/colors/resources, notifier helper operations
and string allocation/appending. It never decides builder/interpreter outcomes.
Tests use a foreign recorder with literal event vectors and independent raw
getter/factory answers, not an outcome-producing host interpreter.

Successful SetParameters still returns false. Fallback attempts only the lowest
same-IID class, then Custom, while storing requested CLSID. The second build
forgets the old control without Release. Type effects precede common effects,
procedure handoff, Init, attach/front; type/common repeat in their second pass.
Pending parent/procedure lifetimes, equality no-ops, one-shot handoff,
owned-control detach/shutdown versus release-only, and zero-count destruction
remain literal.

LC4's 12 unknown interpreters are Btn, OptGrp, TextEdit, Spinner, LineInput,
ListBox, SC3WinGen, Combo, TextTicker, Text, Scrollbar and BMP. Failed QI is the
published no-effect path. Successful unknown QI is released then Unsupported.
Prior build effects are precisely old-control overwrite, factory/fallback,
requested-ID parameters, own=true, QI/Release. Common/procedure/Init/attach/front
and second pass have not run; pending parent and procedure remain retained.

LC5's manager-slot-3 parent is opaque. LC8's 19 unknown foreign writers remain
partial after successful writer QI; common output is never described as complete
native output. The fragment wrapper preserves true with a control in its valid
foreign-result domain. Resolution scans all 21 rows when either ID is missing.
LC9 retains UTF8/bounded numeric and indeterminate-field choices. LC10 retains
duplicate-map and Grid scratch/notifier limits. No question was closed, no new
question filed in round 2, and no shared question index was edited.

## Final validation (round 3/3)

This final round changed documentation only. The actual engine diff since
34dc4033 is exactly the two redundant-binding deletions committed in 628ec2a1;
production, literal fixture expectations, assertions and all 22 tests are
unchanged. Explicit UTF-8 comparison confirmed that identity. No additional
quick, baseline, focused Clippy check, suppression or tolerance change was used.

The one necessary corrected-source `qbuild all` ran on clean committed HEAD
`e948a309d0140c7b294894442fa4d411f5ff73fc`, tree `658d662f7653058422e9bf823a0c2e0ee1fd2be3`. Original qbuild exit was **0**. Before/after HEAD,
tree, clean status and all source/configuration hashes matched. The final report
commit changes only this lane file; it does not require another stable gate.
Full evidence: `build-runs/r3-gate-before.json`, `r3-gate-result.json`,
`r3-final-measurements.json` and `r3-qbuild-all-console.log`.

- Host Cargo: **5162 passed**, 0 failed/ignored/measured/filtered, 22 summaries.
- Existing owner fixtures: **22/22**, every original name matched, none ignored.
- Vocabulary: **393 exact ordered entries**, 21 headers, 20 class/interpreter maps.
- Clippy: completed under the unchanged workspace/all-targets `-D warnings` gate;
  no compiler or Clippy diagnostics. The path canonicalization warning remains.
- Native: the build completed, with final Ninja output **[172/172]**.
- CTest: **68 expected / 68 observed / 68 passed**, 0 failures; every generated
  registered name matched its result. No disabled tests, subdirectories or filter.
- CTest's x86 Cargo case independently recorded **5162/0**, 22 summaries and all
  **22 owner fixtures**; native LastTest.log preserves their complete output.
- Post-fix absolute tests.rs `qbuild fmt`: exit 0, files 1, diff_lines 1 (path
  warning only). Prior nine-owner-file fmt passed before the two deletions.
  Engine max_width=120 is unchanged; shared lib.rs was not mass-formatted.

### Saved final logs

| File in build-runs | Bytes | SHA256 |
|---|---:|---|
| qbuild-test.log | 312139 | `1c882ecd086abce005f20042567e5467bbcc08f1510872f6597cfea70675087c` |
| qbuild-clippy.log | 217 | `084cd228946765d4335e509b0fcd3ae451d42590758d643a67c13eb794da4abf` |
| fc4verify-configure.log | 564 | `d079052c50f6160eb82d7c4580306f4a28ec60c31e44c9c913d4111f4ce4ca34` |
| fc4verify-build.log | 17549 | `9d9cbf216a4806c47c1009d5cb8a3cccbff89d42f3189366abf8240273826e20` |
| fc4verify-ctest-list.log | 3027 | `c4afaa9fb5f1ee251baf0adb7060232f93091c2a34ee98b43d8b7e931d8d49a1` |
| fc4verify-ctest.log | 8690 | `dc06912f66f23f0b803a5b4b618855fd93c85645c2d1778ea2333f6b6a66f16b` |

### Configuration and artifacts

The existing engine configuration remains Debug CMake, Ninja, MSVC x86,
FC4_CHECKED=OFF, FC4_COMPILER_CACHE=ON and i686-pc-windows-msvc for native Cargo.
Static CRT flags and CMake's CARGO_INCREMENTAL=0 / dev/test debug=0 settings
remain unchanged. The observed CTest Cargo/compiler argv, cwd and output paths
point to this worktree's engine and engine/build-x86/cargo. CMake always invokes
the release CAPI Cargo target from engine, and links its matching x86 library.
`r3-final-correspondence.json` retains that running CTest attribution, including
service-owned compiler paths. Per-source and configuration hashes are in the
before/after snapshots; generated CMake/CTest/Ninja and LastTest hashes are in
the measurements. These are build/test artifacts, not native owner coverage.

| Artifact | Bytes | SHA256 |
|---|---:|---|
| engine/build-x86/FreeCity4.dll | 18082304 | `16039658c7779e677b7f4389457cbe68c0da42d90a1ac91ab302e08c75e32e18` |
| engine/build-x86/cargo/i686-pc-windows-msvc/release/fc4_capi.lib | 79014274 | `11e2a085f6d97c1c5b3a37635f90d9cd4242949dc3bc7c481fcc666d179e0b3c` |
| engine/build-x86/fc4_capi_smoke.exe | 11023360 | `7f5c3c5df37cc8e280515a334303870de876f4b3fbf39a72ea877e7d4bcdf624` |

### Preserved failures and operation lifetime

The original round-one/round-two checks remain historical evidence, not final
passes. Fresh round-three OS audit read all 26 candidate command lines; the old
validator PIDs were absent and no own checker/compiler/output work was observed.
Cwd reads for 15 other/service processes were denied; own observer cwd was read.
Old native/full exit and disappearance cause remain **UNKNOWN**, with no claim
of a provider-reset kill. Four overwrite-prone round-two logs were moved to
`r2-*` names and verified SHA256-identical: the 5162/0 Cargo result, the failed
redundant_locals Clippy log, 588-byte incomplete native log and configure log.
Their original hashes are in `r3-preflight-audit.json`. Round-one `r1-*`, mixed
NUL-containing quick evidence, prior failures and transport logs remain intact.

The new gate's runner first stopped before qbuild at a UTF-8/default-Windows
decoding identity assertion; helper decoding alone was corrected. That prelaunch
failure is preserved in `r3-runner-preflight-failure.txt`; only one actual new
qbuild all was launched. During native verification, the existing wrapper's
cache-stats child temporarily delayed CTest. Its PID/creation/cwd/argv and own
loopback connection were observed read-only. The first nine-minute result-file
wait timed out; that was not a qbuild failure. The same check advanced into
CTest at 00:38:13 UTC after a specific coordinator intervention: root terminated
ONLY the blocked read-only statistics client8672 (creation1792715430.3681138),
exit15 at01:38:13.238371 BST. The same validator5540 and daemon30848 remained
alive, and the same existing validation started CTest30684. This is an
interrupted/unavailable statistics request, not successful cache data. The
underlying delay/RPC cause remains unknown. Exact before/after evidence is
`coordinator-stats-client-intervention-legacy-r3.json` and the coordinator note
`coordinator-stats-client-intervention.md` in owned build-runs; root retains
`coordinator-legacy-r3-stats-client-interruption.json`.
`r3-cache-blocker.md`, both correspondence snapshots and the partial measurement
`r3-partial-measurements-003932.json` preserve the intermediate state separately
from the passing final result. No daemon/compiler/check was killed or restarted,
no duplicate gate was launched, and no source/environment/cache/provider/wrapper
policy was changed. The worker did not terminate a process or send a cache RPC
outside the authorized wrapper; the coordinator's exact client-only action is
separate, recorded above, and is not general worker permission or another
implementation round. This report-only correction reuses the completed stable
gate; it does not rerun or relabel the original checks.

## Questions, limits and next steps

No new question was filed or answered in this final round. LC4/LC5/LC8/LC9/LC10
remain in docs/specs/questions/legacy-control.md. The report remains **42 complete
safe-domain / 3 partial of the 45 named entries**, separately from the broader
205-row/104642-byte attribution. No whole-owner/native/layout/default or UI
equivalence credit follows from a passing build. Twelve unknown interpreters,
opaque parent identity, nineteen unknown writers and the malformed/duplicate/
scratch limits remain as described above, preserving reached effects.

No GUI/game/network/live-user-file run, C ABI/export/version change, native glue,
plugin/default/maps/coverage/region change or imported widget contract. Shared
lib.rs still only adds the final module declaration relative to base 82feac1f.
The coordinator may land this bounded portable implementation with its passing
source-corresponding gate. This exhausts original-family round 3/3: no fourth
round, new family or owner restart is requested.

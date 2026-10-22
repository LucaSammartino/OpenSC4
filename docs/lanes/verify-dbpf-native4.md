| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-dbpf-native4: native packed DBPF transport | `verify-dbpf-native4` / engine lane | in progress; baseline all started | Inspect public factory, segment and record contracts; implement independent comparison. |

## Handoff
- Goal: compare production Rust DBPF parse/index/get/decompress against the original public packed-file segment on immutable independent inputs.
- Worktree reserved, detached HEAD at base 2db25ea9497061e05dcf302de621469d729a668a; clean on entry.
- Baseline qbuild all started; inspect build-runs logs for result.
- Public SDK declares packed-file CLSID 0x086D4A89, IID 0x286D4A6A in cIGZDBSegmentPackedFile.h.
- Next: inspect typed SDK acquisition/persistence/record APIs and existing plugin seams; stop with precise refusal if public callable route unavailable.
- No game requests, questions, ABI changes, or serialized probe state.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-dbpf-native4-r1-manifest.json.

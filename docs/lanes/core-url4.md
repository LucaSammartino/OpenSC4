| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-url4 (pure-Rust cRZURL owner, web-network §2) | `core-url4` / (engine lane) | ready at `912c919` (core only, no C ABI/plugin/CMake change; gate: cargo 3765/0, clippy clean, ctest 63/63; no game run, game false; questions web-network Q4-Q6) | 1. answer Q4-Q6 and adjust `engine/core/src/rzurl.rs`; 2. a later lane may wire cRZURL into CAPI/plugin for a shadow |

## Report
- What: `engine/core/src/rzurl.rs` (+ `rzurl/tests.rs`, `pub mod rzurl;` at the end of lib.rs): `RzUrl` with the two
  constructors (default = guess mode, from_text = allowRelative), lazy getters (type, scheme, server, absolute path,
  IsURLRelative and the string overload), raw field accessors, ParseURL (scheme step 2.3, user-info, server/port,
  hierarchical/file/ftp/opaque/other paths 2.4), MakeURLStringFromComponents (2.6), ChangeURL (2.7) and the path
  normalisation (2.8, `normalize_path`). Byte strings, C-locale ASCII classes, no network.
- Tests: 9 tests, every 2.5 example, the 2.8 worked example via ChangeURL, quirks (https -> 2, `mailto:` -> type 0
  even as `mailto://`, user keeps `//`, `;` params running to `?` past a `#`, step-1 normalise skip), port edge cases.
- Literal readings marked in code: unchecked N/B parse results in ChangeURL, step-3 length test once, step-4 with no
  earlier `/` clears from start.
- Questions filed (docs/specs/questions/web-network.md): Q4 ChangeURL empty-path chain overwrite of N's query/fragment;
  Q5 port run >15 digits erase length and atol overflow; Q6 unrecognised `xyz://` scheme kept or cleared.
- Coverage: cRZURL (9498 B) implemented in core; not shadowed/verified (no plugin wiring by task).

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scrcmd8: the `nSCRes` command classes and helpers, closing sc-resources.md's parser, sc-resources-commands.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-mmutil8) | **ready** (docs only). Fifteen owners claimed: specified +5,613 bytes (+23 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The bodies sc-resources.md does not cite were read in the decompile with their string literals. Written on
2026-10-25.

### What changed

- docs/specs/sc-resources-commands.md (new): §1 each owner and the section that specifies it; §2 the parse-error
  constructors; §3 `define`, `enddef`, the macro's `AddCommand` and the macro-name map; §4 the block-command plumbing;
  §5 the file parser's input record and class id; §6 conformance tests.

### Coverage

- tools/coverage-map.tsv: rows for fourteen `nSCRes` owners before the `nSCRes*` catch-all and for
  `fn:GZCOM_nSCRes_cFileParserCLSID` before `fn:*`.
- Specified bytes 5,121,887 → 5,127,500 (+5,613, 91.8%, `cov_local.py` with owners-c1).

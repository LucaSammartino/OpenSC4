| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-render-properties-contract4: RP3–RP5 owner contracts | Codex / `re-render-properties-contract4` (RE; detached worktree) | ready; RP3–RP5 answered; RP6 filed; questions check and whitespace pass; cargo/clippy/ctest n/a (docs only); game n/a; ABI unchanged | 1. Implement lifecycle and first-NUL behavior in the core. 2. Retain the stated index/locale domain; resolve RP6 before extending text profiles. |

## Report

Published RP3–RP5 owner contracts in `docs/specs/render-properties.md`, without changing the
registry inventory. Early instruction-proof commit: `77ff12bc`; main answer/conformance
milestone: `31759e5a`. This final report commit adds the last literal citations and records
validation. The reserved worktree remains detached as required by the transport.

The lifecycle proof establishes P (complete/render-properties view), U=P+4, and the shared
32-bit count at P+8, initially zero. Both views accept only QueryInterface ID 1, return U,
and acquire one reference. Failure leaves the output cell/count unchanged. AddRef is a plain
increment; unsigned Release count above 1 decrements, while 1 or 0 invokes deletion. The U
destructor thunk restores P; flag bit 0 controls outer allocation freeing after table cleanup.
The actual two-view slot targets and all deciding instruction addresses are in §1.1.

The string proof establishes first-NUL byte queries, sign extension into CRT tolower, active
CRT locale/codepage dependency and bytewise exact comparison after folding. Names fold at
construction, queries at lookup; language-manager locale changes can affect the latter.
The C-locale ASCII profile covers all 77 registered names and abbreviations. Comparing an
entire Rust string through embedded NUL differs from native. No UTF-8 interpretation,
universal high-byte result or fixed Windows codepage was invented.

Index compatibility is explicitly 0≤i<count: Boolean 0–37, integer 0–23, float 0–14.
The native bodies use unchecked 32-bit address arithmetic. Invalid indices have no portable
native expected value, including when arithmetic happens to alias allocated memory. Core
None/false rejection is a safety extension. §7.1–7.2 supplies five owner scenarios, byte/NUL
queries for all three types, valid endpoint reads/names/writes, instance independence,
unclamped setters, and separately labeled core-only invalid-index checks. These are proposed
instruction-derived discriminators, not executed tests or native/game exactness evidence.

## Evidence and validation

- Read-only main-checkout export queries first, then bounded `asm --bytes` inspection and
  relevant constructor pseudocode; no REA call or REA fallback was needed.
- Vtable slot addresses cross-checked in exported `data-references.tsv`. Direct PE section
  reads confirmed the two setlocale argument literals (`C` and empty). Numeric float/range
  constants were checked against the instructions, including default bits `0x3F0CCCCD`.
- `python tools/questions.py index` and `python tools/questions.py check`: PASS.
- `questions.py find RP3/RP4/RP5`: archived answers; `find RP6`: open at the intended file.
  Generated index reports 3 open, 3 answered, next ID RP7.
- `git diff 512901f5108b5dde5810f85ebd5c8249b5fe0fbe HEAD --check` and final whitespace
  validation: PASS. Only this lane report, the spec, its question files and generated
  questions index changed.
- Cargo, clippy, ctest and game/shadow runs: n/a, docs-only. No runtime promotion or additional
  owner coverage credit is claimed. Engine, tools and ABI tables were neither read for
  implementation details nor edited. No C ABI exports changed; no ABI proposal TSV needed.
- Environment note: quiet Bash wrapper failed with E_ACCESSDENIED; bounded native PowerShell
  commands were used. Git emitted a packed-refs lock permission diagnostic but created both
  milestone commits successfully; HEAD/status were checked. No shared refs/config were edited.

## Questions and follow-up

RP3 and RP5 are DIFFERS, RP4 MATCHES, archived with original provenance and dated answer
markers in `docs/specs/questions/answered/render-properties.md`. RP1/RP2 are untouched.
New RP6 in `docs/specs/questions/render-properties.md` asks for shipped-language LC_CTYPE/
codepage profiles and controlled I/i/high-byte comparisons, including locale changes between
construction and lookup. This does not block implementing the stated C-locale ASCII core
domain; it blocks claiming complete native text behavior across locales.

Implementers can now add the specified lifecycle and first-NUL semantics and apply the new
discriminators. Keep the safe core invalid-index policy separate from native conformance.
The coordinator can review this docs-only result and publish its detached HEAD to the reserved
lane branch; no merge, push, self-restart or branch switch was performed here.

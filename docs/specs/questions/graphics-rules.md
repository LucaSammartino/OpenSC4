# graphics-rules: open questions
<!-- Questions about [graphics-rules.md](../graphics-rules.md). Append new entries at the END; when one is answered, move it to answered/graphics-rules.md. Rules: README.md. -->

No open questions.

<!-- from: core-graphics-rules4 (2026-10-22) -->
- **GR4 — §§3/4/12 shared parser contract:** Please publish the exact lexical and expression grammar required for complete-owner credit: whitespace/line endings, quoting/escapes, comment position, symbol expansion (whole token versus interpolation, recursion and unknown names), command/switch case and placement, arity/extra arguments, boolean/integer literals and overflow, real literals/operators/precedence/functions and exceptional conversions, block EOF/error behavior, and `sinclude` path/missing/cycle policy. What do the two file-registration arguments 1 and 0 mean for missing/read-failed files and parse ordering? Is any Lua/script command part of this owner? No public shared-parser grammar was found. [Implement only the explicitly documented literal command subset; reject unsupported expressions instead of inventing operators or delegating parsing/evaluation. No complete-owner credit pending this contract.]

<!-- from: core-graphics-rules4 (2026-10-22) -->
- **GR5 — §4.2 versus vectors V1 NaN contradiction:** §4.2 says NaN environment numbers pass all-mode numeric tests; V1 says a NaN "passes neither numeric condition in all mode". Which result is normative? [Follow §4.2: all-mode NaN passes, any-mode NaN supplies no match; mark the conflicting V1 edge explicitly.]

<!-- from: core-graphics-rules4 (2026-10-22) -->
- **GR6 — lifecycle/failure/logging boundaries:** Please specify behavior for Init without a properties service or failed information acquisition; fallback lookup failure; malformed/short driver descriptions and invalid fixed-offset hex IDs; dense group/option index `UINT32_MAX` (index+1 overflow/allocation failure); exact renderer-dump headings, preference label padding, line endings and nonfinite `%g` spelling; write/close failures after successful log open; and whether regather changes cardIdentity. [Require a supplied property service/provider; reject invalid IDs/index overflow recoverably; missing description fields empty; retain cardIdentity on gather, reset it on parse; successful open determines explicit-log success. Logging layout follows the literal published headings with LF and a 20-column label, without byte-exact credit.]

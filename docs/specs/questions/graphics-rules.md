# graphics-rules: open questions
<!-- Questions about [graphics-rules.md](../graphics-rules.md). Append new entries at the END; when one is answered, move it to answered/graphics-rules.md. Rules: README.md. -->

<!-- from: core-graphics-rules4 (2026-10-22) -->
- **GR4 — §§3/4/12 shared parser contract:** Please publish the exact lexical and expression grammar required for complete-owner credit: whitespace/line endings, quoting/escapes, comment position, symbol expansion (whole token versus interpolation, recursion and unknown names), command/switch case and placement, arity/extra arguments, boolean/integer literals and overflow, real literals/operators/precedence/functions and exceptional conversions, block EOF/error behavior, and `sinclude` path/missing/cycle policy. What do the two file-registration arguments 1 and 0 mean for missing/read-failed files and parse ordering? Is any Lua/script command part of this owner? No public shared-parser grammar was found. [Implement only the explicitly documented literal command subset; reject unsupported expressions instead of inventing operators or delegating parsing/evaluation. No complete-owner credit pending this contract.]

<!-- from: core-graphics-rules4 (2026-10-22) -->
- **GR6 — lifecycle/failure/logging boundaries:** Please specify behavior for Init without a properties service or failed information acquisition; fallback lookup failure; malformed/short driver descriptions and invalid fixed-offset hex IDs; dense group/option index `UINT32_MAX` (index+1 overflow/allocation failure); exact renderer-dump headings, preference label padding, line endings and nonfinite `%g` spelling; write/close failures after successful log open; and whether regather changes cardIdentity. [Require a supplied property service/provider; reject invalid IDs/index overflow recoverably; missing description fields empty; retain cardIdentity on gather, reset it on parse; successful open determines explicit-log success. Logging layout follows the literal published headings with LF and a 20-column label, without byte-exact credit.]

GR6 integration addendum: render-properties now correctly retains arbitrary foreign-CRT name bytes. What byte-exact dump/format behavior applies when names are non-UTF-8? The provisional unbound Rust logging adapter requires UTF-8 and rejects other profiles explicitly; it does not substitute, omit or stringify an Option. Native byte-name support remains missing and blocks owner credit/binding.

**Final-round evidence 2026-10-22 (re-graphics-rules-final4), partial; GR4/GR6 remain open:**
GR4 now has registration-bit/error-order semantics in §3, concrete native expression precedence,
signed versus unsigned conversion, symbol dispatch and eval boundaries in §4.5, and actual dependency
addresses in §12.1. Complete lexical tables, quoting/escapes/comment positioning, all command/switch
case/arity/placement, symbol boundary parenthesization, boolean aliases, full functions/exceptional
conversions, block EOF rules and sinclude missing/cycle policy remain unclosed. No general Lua parser
is justified by the examined eval/numeric bodies; a pseudocode Lua label is actually `_ftol2`.

GR6 now has regather/cardIdentity and description exhaustion/CR behavior (§2.2), permissive fixed hex
(§3), unsafe dense-index wrapping (§4.5), fallback lookup storage (§12.1), actual Options/Properties
headings, 21-byte preference prefix, CRLF/two-write behavior, ignored write/close results and arbitrary
non-UTF-8 `%s` bytes (§6.1). Provisional strict ID/index rejection and LF output differ from native.
Exact nonfinite `%g` output through the foreign CRT and exhaustive failure/allocator outcomes remain
unclosed; do not infer them from a host formatter or turn unsafe native paths into recoverable parity.
V11–V15 are independent source-derived discriminators, not implementation/test evidence. Parked after
the authorized final logical RE round; no complete-owner/default/shadow/replacement promotion.

<!-- from: core-config-rules5 (2026-10-23) -->
- **GR7 — §4.5 portable floating-point profile:** Which active x87 rounding-control and precision-control profiles must the portable shared numeric readers support? The published signed-property vectors specify round-to-nearest/even, but the text also makes the bit trick depend on active rounding control. Is nearest/even with binary64 expression accumulators the required supported profile, or must callers select other profiles? [Implement the explicitly described nearest/even profile, including the binary32 bias/bit trick; do not claim parity for other active control modes. The settled precedence, unsigned truncation and signed-property vectors are implemented; GR4 still bounds the rest of the grammar.]

<!-- from: core-config-rules5 (2026-10-23) -->
- **GR8 — §6.1 preference label width contradiction:** The listed `VariableSpeedAutomata` is 21 ASCII bytes, while §6.1 calls it 20 bytes and requires every label to occupy 20 bytes plus one space with `current:` starting at byte 21. Should this row truncate the label, omit its separating space, use a different literal spelling, or start `current:` at byte 22? [Follow the explicit 20-byte field and byte-21 offset literally, truncating this one label to `VariableSpeedAutomat`; byte-exact compatibility for this row stays open pending clarification. Other eleven labels are unaffected.]

<!-- from: replace-cfgrules30 (2026-10-26) -->
- **GR12 — §2.7 residual first-null-writer observation:** GR11's interface/order answer is implemented. Please obtain the RE-side matched replay and first-null-writer (or missing-publication) observation required by §2.7 before resolving s0 acceptance (c). No RE debugger/watchpoint tool is configured for this clean-room engine lane; no private storage address, extra restore, or writer cause is being inferred.
  - Original failing evidence remains source `afc66170e`, DLL SHA256 prefix `f2e48878`: `cfgrules30-r1-s0-grow-on2.log:9294` has `+0xE2C06` on 2026-01-27; `-on1.log` has the same fault on 2026-01-30, while `-off1.log:11409` has the distinct `+0x3529C0` terrain fault. Replay the corresponding source/DLL, save, script and graphics mode under both settings as §2.7 requests.
  - Corrected source `37219ef48`, DLL `7234fd3c`: opt-in ABI constructor/pre-city/Shutdown hooks record identity, thread and caller stack; independent Init/gather traces record driver identity, public view snapshots and acquired-reference cleanup. `cfgrules30-r2-s0-on1.log:363–419` has no public view throughout Init; its sole view is constructed at lines 8000–8005 and pre-city initialized at 8032–8037, after acquisition has completed. No view Shutdown occurs during gathering. These observations do not identify the earlier fault's first writer.
  - Current s0 on1/on2/off1 reach 2028-02-02/2028-02-01/2028-01-30; each then faults at `+0x1D0F94` during occupant type `096E763C` Shutdown (`-on1.log:17681`, `-on2.log:17756`, `-off1.log:17792`). This is the task's shared quit-time construction fault, not a reproduction or explanation of `+0xE2C06`. Current s1-on1 is clean to 2028-01-28; s1-off1/off2 both watchdog-stall at 2026-01-30, without foreign exceptions.
  - All named logs are under main `build-runs/game-results/`; all current runs have ref_warnings=0. Query properties 77/77, recommendations 12/12, shadow snapshots 13/13, cross-loads 2/2 and the house run through 2032 pass. Replacement credit remains held on the required residual observation; the passing repeats do not close it.

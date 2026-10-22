# Native lot configuration editor answered questions

**LE2 — Corrected Save As, identity-path success and new-name source**

**ANSWERED 2026-10-22 (re-loteditor-window4):** DIFFERS — Native identity-path
resolution returns the saved success byte, not the decompiler's unconditional
false. Full lowercased directory equality precedes provider stem/extension key
derivation; success installs the original path, derived key/ID and retained
references. Key lookup ignores the path-query return; type lookup uses final
group `0xA8FBD372`. New configurations are named `New Lot`. Save As sets window
flag `0x100`, tears down the browser before persistence, and explicitly transfers
the shallow-copied identity references, releasing old references on ordinary
success or selected references on restore. See
[§8.1, §8.2 and §8.4](../../lot-configuration-editor.md). No downstream file-format
or overwrite semantics inferred.

Original scope: complete real Save As `0x00485390`, browser flags/reference
cleanup, identity-path success `0x0047F200`, key/type helpers
`0x0047B1F0`/`0x0047BAD0`, directory matching/path-derived IDs and the new-name
source in `0x00483B20`.

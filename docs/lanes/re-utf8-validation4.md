| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-utf8-validation4: standalone UTF8IsStringValid | Codex, `re-utf8-validation4` (reserved detached worktree) | in progress | Qualify native boundary and publish byte-walk contract. |

## Handoff

- Goal: first dedicated standalone validator spec; no engine, native execution, ABI or converter work.
- Done: initial full normal-domain policy in `utf8-validation.md`, separate link in strings-language; pending first milestone commit.
- Next: independently derive literal ledger and whole-body branch coverage; static documentation checks.
- Evidence: export/body `0091016f..0091030e`, 416 bytes, next function `0091030f`, no calls, one direct caller `00910948`; two cdecl-compatible stack words, AL return.
- Rules: count-delimited; NUL ordinary; 1–6 byte forms; lower bounds C2, E0/A0, F0/90, F8/88, FC/84; accepts surrogates and above-Unicode forms.
- Identity: working-copy hash matches export `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`; file version 1.1.641.0.
- History: repository spec/lane search found no previous dedicated helper publication; re-strings4 leaves encoding helpers unstarted. Named re-string-storage-final4 report is absent in this snapshot; inherited task states it excluded encoding owner.
- Main: `C:/Users/tlori/FreeCity4`; manifest: `build-runs/run4-re-utf8-validation4-r1-manifest.json` there.
- Tool limitation: q.py Bash wrapper fails E_ACCESSDENIED; use bounded PowerShell commands. qgrep default output encoding failed on replacement characters; use Python UTF-8 mode.
- No native runs, fixtures, questions or ABI rows yet.

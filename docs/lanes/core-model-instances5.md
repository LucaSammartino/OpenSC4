| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-instances5: model instance owners | Codex, `core-model-instances5` | in progress | Implement spec owners and closure tests |

## Handoff
- Goal: pure Rust model_instances owners, complete spec coverage and owner closure README; no ABI or plugin changes.
- Done: 718a6a22c records initial audit. All spec sections audited against existing implementation. Added unbound animated construction, public BuildFrame with provider-side failure validation, sprite CreateModelInstance delegation/lifetime, and Owner closure README with all 20 vector mappings.
- Next: run final qbuild all on stable source, then explicit queued verify if needed; collect the same sessions to terminal results.
- Checks: focused cargo model_instances tests PASS 45/45; qbuild fmt PASS for the four edited Rust files. Initial q.py attempt failed before running cargo (Git Bash CreateFileMapping Win32 error 5); evidence build-runs/q/20261024-002952-10932.log. Direct PowerShell cargo succeeded.
- Limits: existing MI1/MI2 opaque meanings; MI11 x87 last bits in sections 2.1/3.1/4.1; MI12 vector-15 staging; MI13 section 6.1 wrapped interior-zero keys. No new questions; settled portable subset has no Unsupported stubs.
- Game requests: none (pure core module).
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-model-instances5-r1-manifest.json.

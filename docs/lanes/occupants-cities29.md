| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| occupants-cities29; kind engine; city shadow timing | Codex; occupants-cities29; detached HEAD (supervisor publishes branch) | WIP; ABI unchanged; game validation pending; no questions | 1. Build and validate nested removals and seaport Init inputs. 2. Verify existing PL24 fix across reported prop scenarios. 3. Broad sample, house check, all gate, enqueue. |

Resume evidence: initial HEAD 72bf35774; no prior lane file or implementation commit existed.
Reported baseline census: occmgr 54/147 scenarios inexact; ports 47/146; propmgr 6/150 (run7/census3/per_key.tsv).
Base already includes PL24 closed lot-prop bounds (23e3dbdc4); needs game verification against the older census DLL.
Working change: shadow single Remove now precedes the original's reentrant Shutdown; all return/message checks retained.
Working change: capture seaport Init CellHasWater inputs, replay for initial shadow map, log native water agreement and later input changes.
Next: qbuild quick; run failing representatives through authorized suite.py, at most three queued requests; establish evidence before claiming fixes.

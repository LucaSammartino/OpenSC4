tag: run-8
run: 8
date: 2026-10-12
lane: run
dll: unknown
pacing: unknown
verdict: corrupt
switches: all on, message-layout fix
result: corrupt: NT heap unlink AV right after load. Cause: `GetAllGroups` pushed a vector element holding a string by bitwise copy (double free). Fixed in 8d79c77 (merged).

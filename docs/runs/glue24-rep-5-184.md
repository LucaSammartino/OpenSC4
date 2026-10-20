tag: glue24-rep-5-184
run: 201
date: 2026-10-20
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: exited
switches: `enable = netmgr,netlot` (both replacements), turbo to 2032-01-01 on Konradshohe, queue request `glue24-rep-5`
result: **exited (code -1) without a verdict, 15 s wall, at the first `NetworkManager::Init` call.** The owner-block fix was incomplete: `MakeHost` took the owner but never wrote `owner->mgr`, so `Self(&owner_)` dereferenced null and `cFC4NetworkManager::City` (the inlined `city_` read at +0xC4) faulted on a null `this` (`ACCESS_VIOLATION reading 0x000000C4 at FreeCity4.dll+0x776AFA`). Fixed in the next commit (`MakeHost` assigns `owner->mgr = self`).
summary: rep-5 died at Init on a null manager (owner.mgr never assigned); fixed by e7a227b0

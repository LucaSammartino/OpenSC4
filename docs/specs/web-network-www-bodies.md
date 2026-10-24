# cGZWWWSystem owner body checklist

Native attribution snapshot: main `build-runs/run4-owners-current.tsv`, read 2026-10-22. Inventory was sorted by descending body bytes before body inspection; addresses/sizes were checked with the export. Rules are the snapshot attribution rules, not interface membership.

**Partial owner: no complete-owner or web-family credit.** C means a contract is written in web-network.md section 4; P means inspected/partly described but unfinished; O means open. C is documentation status, not tested/implemented/verified/replaced status. P/O bodies are explicitly excluded from completion claims.

| Status | Bodies | Bytes |
|---|---:|---:|
| C | 91 | 26757 |
| P | 1 | 64 |
| O | 56 | 7228 |
| Total | 148 | 34049 |

Final round 3/3 disposition: **PARK**, partial owner; no fourth round. Preserved bdd8e715's 71 bodies / 18705 B and added 20 bodies / 8052 B. Q7/Q9 remain partial for exactly the 57 P/O rows / 7292 B below; Q8 remains answered in §4.12. Q7 now covers remaining commands, initialization entry and scheduler controls/containers, not the newly proved login/listing/outer worker. Q9 covers remaining support/adapters and cookie constructor, not the newly proved HTTP constructor/completion/stop/drain boundary. The 148 / 34049 B inventory is unchanged. C includes explicit native unsafe-input limitations, not deterministic output for uninitialized bytes. 008f9bad has only capacity/default state inspected (§3.6); its whole constructor is not credited. Existing URL/cookie contracts are not duplicated. Public Release thunk 009cc22d, foreign HTTP worker/completion/stop/drain bodies and folded getters attributed to other owners are not added to this total.

| Address | Body bytes | Attribution | Contract status / section or gap | Export name |
|---|---:|---|---|---|
| 00812de0 | 3607 | reach | C: 4.13 / web-network-ftp.md | FUN_00812de0 |
| 00814560 | 1878 | reach | C: 4.13 / web-network-ftp.md | FUN_00814560 |
| 00813e90 | 1741 | reach | C: 4.13 / web-network-ftp.md | FUN_00813e90 |
| 00816ee0 | 1628 | reach | C: 4.13 / web-network-ftp-control.md §6 | FUN_00816ee0 |
| 008163d0 | 1466 | reach | C: 4.13 / web-network-ftp-control.md §3 | FUN_008163d0 |
| 00815d30 | 1245 | reach | C: 4.13 / web-network-ftp-control.md §1 | FUN_00815d30 |
| 008f7a50 | 1062 | reach | C: 4.12 | FUN_008f7a50 |
| 008f8ffe | 988 | reach | C: 4.4 | FUN_008f8ffe |
| 008f8c5a | 932 | reach | C: 4.7 | FUN_008f8c5a |
| 00812390 | 825 | reach | C: 4.13 / web-network-ftp-control.md §7 | FUN_00812390 |
| 00815a60 | 717 | reach | O: Q7 | FUN_00815a60 |
| 00814cc0 | 657 | reach | O: Q7 | FUN_00814cc0 |
| 00814f60 | 579 | reach | O: Q7 | FUN_00814f60 |
| 008153a0 | 579 | reach | O: Q7 | FUN_008153a0 |
| 008155f0 | 579 | reach | O: Q7 | FUN_008155f0 |
| 00815840 | 538 | reach | O: Q7 | FUN_00815840 |
| 008151b0 | 489 | reach | O: Q7 | FUN_008151b0 |
| 008f84ea | 483 | name-medium | C: 4.1–4.5, 4.9 | cGZWWWSystem::LoadFileDocument |
| 00818210 | 481 | reach | C: web-network-http-lifetime.md §1 | FUN_00818210 |
| 008f815f | 478 | reach | C: 4.8 | FUN_008f815f |
| 00816af0 | 451 | reach | C: 4.13 / web-network-ftp-control.md §4 | FUN_00816af0 |
| 00812ba0 | 437 | reach | C: 4.13 / web-network-ftp.md | FUN_00812ba0 |
| 00823000 | 432 | reach | C: 4.11 | FUN_00823000 |
| 008f833d | 429 | name-medium | C: 4.1–4.5, 4.9 | cGZWWWSystem::LoadDocument |
| 00816230 | 415 | reach | C: 4.13 / web-network-ftp-control.md §3.1 | FUN_00816230 |
| 00817600 | 399 | reach | O: Q7 | FUN_00817600 |
| 008126d0 | 360 | reach | C: 4.13 / web-network-ftp-control.md §2 | FUN_008126d0 |
| 0081cda0 | 319 | reach | C: 4.14 | FUN_0081cda0 |
| 00817bd0 | 300 | neighbour | C: 4.14 | FUN_00817bd0 |
| 008f9bed | 299 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::cGZWWWSystem |
| 00816db0 | 296 | reach | C: 4.13 / web-network-ftp-control.md §7 | FUN_00816db0 |
| 0081ca90 | 266 | reach | C: 4.14 | FUN_0081ca90 |
| 00816990 | 242 | reach | C: 4.13 / web-network-ftp.md | FUN_00816990 |
| 008f7966 | 234 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::SetCacheDirectory |
| 008f93da | 230 | reach | C: 4.8 | FUN_008f93da |
| 008f736e | 227 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::GetRAMDataPreferenceFromMIMEType |
| 00817af0 | 220 | neighbour | O: Q7 | FUN_00817af0 |
| 00811cd0 | 199 | reach | C: 4.13 / web-network-ftp.md | FUN_00811cd0 |
| 00816cc0 | 197 | reach | C: 4.13 / web-network-ftp-control.md §7 | FUN_00816cc0 |
| 0081cc10 | 191 | reach | C: 4.14 | FUN_0081cc10 |
| 00823310 | 190 | reach | C: 4.11 | FUN_00823310 |
| 008f7f04 | 182 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::CancelAllDocumentLoadRequests |
| 008120e0 | 174 | reach | C: 4.13 / web-network-ftp.md | FUN_008120e0 |
| 008f8992 | 160 | name-medium | C: 4.1–4.5, 4.9 | cGZWWWSystem::~cGZWWWSystem |
| 00813d70 | 154 | reach | O: Q7 | FUN_00813d70 |
| 00812b10 | 144 | reach | C: 4.13 / web-network-ftp.md | FUN_00812b10 |
| 008f6c47 | 143 | reach | C: 4.6 | FUN_008f6c47 |
| 008f7e76 | 142 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::CancelDocumentLoadRequest |
| 00811fc0 | 140 | reach | C: 4.13 / web-network-ftp.md | FUN_00811fc0 |
| 008f6bbc | 139 | reach | C: 4.6 | FUN_008f6bbc |
| 00811e70 | 138 | reach | C: 4.13 / web-network-ftp.md | FUN_00811e70 |
| 008129c0 | 135 | reach | O: Q7 | FUN_008129c0 |
| 008f70c3 | 135 | reach | C: 4.7 | FUN_008f70c3 |
| 00812300 | 132 | reach | O: Q7 | FUN_00812300 |
| 0081e640 | 128 | reach | O: Q9 | FUN_0081e640 |
| 0081da90 | 125 | reach | O: Q9 | FUN_0081da90 |
| 00817580 | 123 | reach | C: 4.13 / web-network-ftp-control.md §6.1 | FUN_00817580 |
| 008f6a9a | 123 | reach | C: 4.6 | FUN_008f6a9a |
| 00812d60 | 123 | reach | C: 4.13 / web-network-ftp.md | FUN_00812d60 |
| 00812190 | 122 | reach | C: 4.13 / web-network-ftp-control.md §6.1 | FUN_00812190 |
| 00813e10 | 120 | reach | O: Q7 | FUN_00813e10 |
| 00813cb0 | 119 | reach | C: 4.13 / web-network-ftp-control.md §6.1 | FUN_00813cb0 |
| 008f6b15 | 116 | name-high | C: 4.6 | cGZWWWSystem::SendJobUpdateNotification |
| 00817a70 | 116 | reach | O: Q7 | FUN_00817a70 |
| 00812210 | 114 | reach | O: Q7 | FUN_00812210 |
| 008f72fd | 113 | reach | O: Q9 | FUN_008f72fd |
| 00812290 | 110 | reach | C: 4.13 / web-network-ftp.md | FUN_00812290 |
| 008f80f2 | 109 | reach | O: Q9 | FUN_008f80f2 |
| 00812aa0 | 104 | reach | O: Q7 | FUN_00812aa0 |
| 00811da0 | 98 | reach | C: 4.13 / web-network-ftp.md | FUN_00811da0 |
| 00813c50 | 94 | reach | C: 4.9 | FUN_00813c50 |
| 00819650 | 94 | reach | C: 4.9 | FUN_00819650 |
| 00812840 | 94 | reach | C: 4.13 / web-network-ftp-control.md §3.1 | FUN_00812840 |
| 008f698b | 90 | name-high | C: 4.6 | cGZWWWSystem::DoMessage |
| 008f6e2b | 85 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::RemoveURLHandler |
| 00811e10 | 84 | reach | O: Q7 | FUN_00811e10 |
| 00811f30 | 84 | reach | C: 4.13 / web-network-ftp.md | FUN_00811f30 |
| 008f6d08 | 81 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::GetDocumentLoadStatus |
| 00817d00 | 80 | reach | C: 4.14 | FUN_00817d00 |
| 0081dd70 | 80 | reach | O: Q9 | FUN_0081dd70 |
| 0081d8d0 | 79 | reach | O: Q9 | FUN_0081d8d0 |
| 0081dc60 | 78 | reach | O: Q9 | FUN_0081dc60 |
| 00812a50 | 78 | reach | O: Q7 | FUN_00812a50 |
| 008f714a | 75 | reach | O: Q9 | FUN_008f714a |
| 008f687d | 73 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::QueryInterface |
| 008f72b5 | 72 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::LoadDocument |
| 00812970 | 71 | reach | C: 4.13 / web-network-ftp-control.md §3.1 | FUN_00812970 |
| 008f692a | 65 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::Shutdown |
| 008f9bad | 64 | reach | C: 3.6 | FUN_008f9bad |
| 008f708a | 57 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::AddURLHandler |
| 00813d30 | 53 | reach | C: 4.13 / web-network-ftp.md | FUN_00813d30 |
| 00818100 | 52 | reach | C: 4.14 | FUN_00818100 |
| 008f6cd6 | 50 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::RemoveAllURLHandlers |
| 008f747b | 49 | reach | O: Q9 | FUN_008f747b |
| 008120b0 | 46 | reach | O: Q7 | FUN_008120b0 |
| 00812050 | 44 | reach | O: Q7 | FUN_00812050 |
| 00816ac0 | 44 | reach | C: 4.13 / web-network-ftp-control.md §6.1 | FUN_00816ac0 |
| 008f68fe | 44 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::PostInit |
| 008f68c6 | 43 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::AddRef |
| 008f7451 | 42 | reach | C: 4.5 | FUN_008f7451 |
| 008196b0 | 42 | reach | C: 4.9 | FUN_008196b0 |
| 008f69e5 | 39 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::SetDefaultNotificationTarget |
| 00817a40 | 39 | reach | O: Q7 | FUN_00817a40 |
| 00812080 | 37 | reach | O: Q7 | FUN_00812080 |
| 008f86cd | 37 | reach | O: Q9 | FUN_008f86cd |
| 008f6a0c | 36 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::HTTPClientJobIDFromWWWSystemJobID |
| 00811f90 | 33 | reach | O: Q7 | FUN_00811f90 |
| 008f7fd1 | 33 | reach | O: Q9 | FUN_008f7fd1 |
| 00811f00 | 33 | reach | O: Q7 | FUN_00811f00 |
| 00817560 | 32 | reach | C: 4.13 / web-network-ftp-control.md §6.1 | FUN_00817560 |
| 008f6d59 | 31 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::GetJob |
| 0081ddc0 | 30 | reach | O: Q9 | FUN_0081ddc0 |
| 00817540 | 30 | reach | C: 4.13 / web-network-ftp-control.md §7 | FUN_00817540 |
| 00816210 | 30 | reach | C: 4.13 / web-network-ftp-control.md §7 | FUN_00816210 |
| 008f6a30 | 29 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::FTPClientJobIDFromWWWSystemJobID |
| 008f8c3e | 28 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::~cGZWWWSystem |
| 008f6d78 | 27 | reach | O: Q9 | FUN_008f6d78 |
| 008f8a7a | 26 | reach | O: Q9 | FUN_008f8a7a |
| 00816d90 | 23 | reach | C: 4.13 / web-network-ftp-control.md §7 | FUN_00816d90 |
| 008f7fba | 23 | reach | O: Q9 | FUN_008f7fba |
| 0081d7d0 | 22 | reach | O: Q9 | FUN_0081d7d0 |
| 00817790 | 22 | reach | C: 4.14 | FUN_00817790 |
| 00811c90 | 21 | reach | O: Q7 | FUN_00811c90 |
| 008f7195 | 20 | reach | O: Q9 | FUN_008f7195 |
| 0081e770 | 19 | reach | O: Q9 | FUN_0081e770 |
| 008f6a4d | 19 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::IsVisitedURL |
| 0081e740 | 18 | reach | O: Q9 | FUN_0081e740 |
| 008f68f1 | 13 | name-high | C: 4.1–4.5, 4.9 | cGZWWWSystem::Init |
| 008f6981 | 10 | name-low | C: 4.1–4.5, 4.9 | cGZWWWSystem::SetCacheUsage |
| 008f6973 | 10 | name-low | C: 4.1–4.5, 4.9 | cGZWWWSystem::SetCookieUsage |
| 0097b497 | 10 | name-low | C: 4.1–4.5, 4.9 | cGZWWWSystem::SetDefaultCustomData |
| 008f8a72 | 8 | vtable | O: Q9 | FUN_008f8a72 |
| 008f696b | 8 | name-low | C: 4.1–4.5, 4.9 | cGZWWWSystem::GetCacheDirectory |
| 009cd02a | 8 | vtable | O: Q9 | FUN_009cd02a |
| 008f8a6a | 8 | vtable | O: Q9 | FUN_008f8a6a |
| 008f8a42 | 8 | vtable | O: Q9 | FUN_008f8a42 |
| 008f8a3a | 8 | vtable | O: Q9 | FUN_008f8a3a |
| 008f8a32 | 8 | vtable | O: Q9 | FUN_008f8a32 |
| 008f8a4a | 8 | vtable | O: Q9 | FUN_008f8a4a |
| 008f8a5a | 8 | vtable | O: Q9 | FUN_008f8a5a |
| 008f8a52 | 8 | vtable | O: Q9 | FUN_008f8a52 |
| 008f8a62 | 8 | vtable | O: Q9 | FUN_008f8a62 |
| 00813bf7 | 6 | neighbour | C: 4.13 / web-network-ftp.md | Catch@00813bf7 |
| 0090ce99 | 5 | reach | O: Q9 | FUN_0090ce99 |
| 0081e760 | 5 | reach | O: Q9 | FUN_0081e760 |
| 009d41a5 | 4 | name-low | C: 4.1–4.5, 4.9 | cGZWWWSystem::GetCookieManager |
| 008f697d | 4 | name-low | C: 4.1–4.5, 4.9 | cGZWWWSystem::GetCacheUsage |
| 0097e89e | 4 | name-low | C: 4.1–4.5, 4.9 | cGZWWWSystem::GetCookieUsage |

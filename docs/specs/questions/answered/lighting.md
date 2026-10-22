# Answered lighting questions

<!-- from: core-lighting4 (2026-10-22) -->
- **LT6 — §2.2 light distance.** **ANSWERED 2026-10-22 (re-lighting-contracts4):** MATCHES — Euclidean RGB length, strictly above the binary32 threshold. Differences, squares, blue-plus-green-plus-red sum and square root stay in x87 precision until the threshold comparison. Equality and NaN fail the change test. See §2.2 and §11 vector 10; instructions `0x007D9E21–0x007D9E68`.

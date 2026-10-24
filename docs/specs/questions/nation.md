# nation: open questions
<!-- Questions about nation.md. Append new entries at the end. -->

NQ1–NQ3 are answered in [answered/nation.md](answered/nation.md).

<!-- from: core-nation10 round 2 (2026-10-24) -->
- **NQ4 - §§6.2/8, non-ASCII tutorial directory names:** which encoding and case map defines directory-name lowercasing for localized tutorial directories? The bytewise lowercase description does not specify mappings outside ASCII. [Preserve non-ASCII bytes and fold ASCII A–Z only; preference display-name comparison remains case sensitive as specified. The §6.2 registry default is compared verbatim with the folded catalog directory, and §8 directory lookup folds both names. ASCII conformance vectors are implemented; non-ASCII case equivalence remains an open contract.]

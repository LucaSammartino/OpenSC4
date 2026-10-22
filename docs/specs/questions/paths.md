# paths: open questions
<!-- Questions about [paths.md](../paths.md). Append new entries at the END; when one is answered, move it to answered/paths.md. Rules: README.md. -->

<!-- from: re-paths (2026-10-19) -->
- **PA1 - §4:** the four replacement function pointers at `0x00B6291C`, `0x00B62920`, `0x00B62924` and `0x00B62928`
  (find first, find next, create directory, remove directory): nothing in the executable writes them. Are they part
  of a framework hook a plugin can set (for example for wide-character paths), or dead code?

<!-- from: re-paths (2026-10-19) -->
- **PA2 - §3.5, §4:** which callers pass `includeTop` false to the recursive listing, and do mods rely on the plugin
  loader's directory order, which is the file system's find order (§4)?

- **PA3 - section 8 (PR integration review):** paths.md section 8 test 2 lists two outputs for the identical input `C:/`. Section 1.2 step 5 explains the doubled separator. Confirm which overload each vector exercises before implementation; do not guess.

<!-- from: core-paths-directories4 (2026-10-22) -->
- **PA4 - §§1–2, string boundaries:** Are all 255/520/521 limits byte counts in framework encoding, and are cRZPath setter equality checks case-sensitive, length-aware cRZString byte equality (including embedded NUL), while free C-string helpers stop at NUL? Which locale/encoding defines the `isalpha` used by IsFullPath/FixDriveString? [Portable core uses §1's explicit ASCII letters, byte limits and exact stored-byte equality; C-string helpers stop at NUL, unsplit setters retain supplied bytes. No Unicode folding or host locale substitution. Native encoding conversion remains unbound.]

<!-- from: core-paths-directories4 (2026-10-22) -->
- **PA5 - §3.1, cRZDirectoryEntry owner completeness:** The brief names cRZDirectoryEntry as an owner, but the spec publishes only its owner/name/kind fields, not its constructor, destructor, copy/assignment, getters, name/path composition, comparison or ownership operations. Please enumerate its actual methods and contracts, including whether the owner is borrowed and when entries/names become invalid on reread, sort, path change or directory destruction. [Core implements only the published list-owned state and returns a borrow tied to its directory; standalone entry methods and complete-owner credit are withheld rather than invented.]

<!-- from: core-paths-directories4 (2026-10-22) -->
- **PA6 - §1.6, SplitDirectory boundaries:** What happens for input capacity zero, exactly 254/255 bytes before an existing trailing separator, and a final aggregate prefix longer than 254 bytes whose individual components are short? Does the 255 rejection count a trailing separator, and is a drive root/leading separator returned as a separate part with spare capacity? [Literal core rejects capacity zero, counts component bytes before its optional trailing separator, applies the same bound to the final aggregate part, and preserves a root part; unresolved boundary choices are marked in code.]

<!-- from: core-paths-directories4 (2026-10-22) -->
- **PA7 - §4, failed FindNext:** Does “false at the end” describe the function result, or must the search object's cached directory flag/name be cleared on exhaustion? Is calling next again after failure supported? [Core returns false and retains the last match; closes the created handle exactly once when the search ends or is dropped. No native cached-state compatibility claim.]

<!-- from: core-paths-directories4 (2026-10-22) -->
- **PA8 - §3.2, AlphabetizeEntryList comparator:** Does the caller comparator receive entry objects or name strings, and is ordering of equal elements stable? [Portable list adapter accepts a caller byte-name comparator and preserves ties. Its signature is a Rust convenience, not a published native entry method or ABI claim.]

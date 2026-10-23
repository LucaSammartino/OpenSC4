# Independent expected ledger

Transcribed from canonical resource-catalog §7 before production implementation.
These are 24 published literal rows, not native executions or native goldens.
Test labels V01–V24 below identify rows; additional assertions are published-derived edges.

| ID | Published row | Independent expected result |
|---|---|---|
| V01 | Group filter and duplicate | Keep identities 1 and 3; two `0x0000002A` appends |
| V02 | Exact zero group | Keep group 0 only |
| V03 | Missing requested key | Select 0, preview instance 9 |
| V04 | Duplicate selection | Select 0 then 2; one preview instance 42 |
| V05 | Reorder boundary | Reverse primitive rows; select new first instance 3 |
| V06 | Invalid descriptor | FFFFFFFF retains rows, no clear or service |
| V07 | Service failure | Clear then enum request only |
| V08 | Empty/group-miss | Clear and order, no selection/preview |
| V09 | Null selected data | False, unchanged AAAA/BBBB/CCCC words, no refresh |
| V10 | Picker conversion | TGI 11223344/55667788/99AABBCC => GTI 55667788/11223344/99AABBCC |
| V11 | Picker missing selection | Output GTI zeros |
| V12 | Accept vs cancel | Output both; consumer accepts only modal 0 |
| V13 | QI lifetime | Catalog write then AddRef; picker AddRef then write |
| V14 | Descriptor wildcard vs row filter | (10,0) admits original 0,1 in order; row group stays literal |
| V15 | Group-only descriptor filter | (0,20) admits original 0,2 |
| V16 | Filename sanitation | A9-b c + 1234 => A__b_c_00001234 |
| V17 | Empty name | ABCDEF01 => abcdef01 |
| V18 | Initial high/signed normalization | 8000 => 1; FFFFFFFF => 7FFF |
| V19 | Search wrap to zero | Probe 7FFF occupied then 0 free => 0 |
| V20 | Decreasing search | Probe 1 occupied then 7FFF free => 7FFF |
| V21 | Text conversion boundary | Raw parser FFFFFFFF stored/written; suffix ffffffff; no QI Release |
| V22 | Spinner reentrancy | Recognized ID, byte 1 => true, no search/text/filename |
| V23 | Shutdown lifetime | Two aliased descriptor refs => two Releases, list once, preview clear only |
| V24 | Base creation failure | Creation false => Init true |

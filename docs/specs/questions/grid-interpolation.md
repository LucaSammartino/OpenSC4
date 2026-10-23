# Grid interpolation questions

- **GI1 — §2/§5, caller allocation and invalid foreign results:** what are the
  allocation/resize failure semantics while constructing the temporary grid,
  and which positive/equal city target dimensions are guaranteed by the foreign
  city interface? The two helper bodies allocate nothing. Instructions prove
  that the caller allocates first-count descriptors but sizes second-count
  elements; unequal counts do not establish safe rectangular storage. No native
  execution or allocator contract has been supplied by this lane.
- **GI2 — §3/§6, unchecked size boundary:** which counts beyond the sufficient
  1…32767 ordinary domain are reachable valid native grids? Source-count shifts
  and all coordinate arithmetic are 32-bit; no size guard is present. Do not
  infer arbitrary-size mathematical resampling or a graceful rejection result.
- **GI3 — §6, overlap and malformed descriptors:** is any caller permitted to
  alias source/destination storage or provide empty/ragged grids? The bodies
  contain no alias/shape checks or snapshot, read first elements before loop
  checks, and divide before zero-count admission. Recovery, portable fault
  behavior and a general overlap guarantee remain unspecified.

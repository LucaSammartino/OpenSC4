# renderer-gdrivers-director: open questions
<!-- Engine questions; append only. -->

<!-- from: core-gdrivers-director4 (2026-10-22) -->
- **GD1 - §5 registry growth:** what is the complete bucket-size table and the node-chain ordering during rehash? The spec gives 53,97,193,389 and ordinary head insertion but not redistribution order. [The bounded core supports the specified 193-bucket regime, preserving duplicates and collision order; growth beyond it reports an explicit specification-boundary fault, not a native Boolean failure. Question prefix GD is distinct from the paper-vector labels used in the parent spec.]

# framework-gz-services: open questions
<!-- Questions about [framework-gz-services.md](../framework-gz-services.md). Append new entries at the END; when one is answered, move it to answered/framework-gz-services.md. Rules: README.md. -->

No open GZ1–GZ7 questions; see [answered/framework-gz-services.md](answered/framework-gz-services.md).

<!-- from: core-gz-messages-complete4 (2026-10-22) -->
- **GZ8 - sections 3/4.7/4A cleanup visitation:** What is the observable queue-manager RemoveAll/destructor map visitation order, server registry type-list destruction order, and unlink-versus-Release order when apply-pending clears its records? Does server 1 destructor bulk-clear pending adds without dropping their holds, as its direct-apply raw clear does? These can call foreign destructors. [Portable policy: ascending queue/type IDs, registration order within a type; detach pending records before their generation-2 Releases; generation-1 bulk clear releases no raw pending holds. Whole-owner completion withheld for these unestablished traces.]

<!-- from: core-gz-messages-complete4 (2026-10-22) -->
- **GZ9 - sections 2/3/4.1 service/interface forwarding:** GZ1 establishes server-2 AddRef/getter/RemoveRef, but what are the base-service Release return/destruction rules (including count 0/1/wrap), all accepted base IIDs and QI reference effects, and queue slot-0 QI behavior? Are any owner-specific Init/Shutdown/base/deleting-destructor bodies beyond the documented server-2 tick attach/detach required? [Implement only the documented shared wrapping AddRef, count getter and nondeleting saturating RemoveRef. Own-IID predicates and platform/service forwarding remain host boundaries; no invented Release or base-QI behavior and no whole-owner credit.]

<!-- from: core-gz-messages-complete4 (2026-10-22) -->
- **GZ10 - sections 1/5.2 whole-owner body checklist:** Please publish the complete per-entry body inventory for the four owners (section 1 lists 30/17/38/81 functions, 166 total), identifying base-forwarding/folded aliases and behavior sections. Section 5.2 inventories the seven answered gaps, whereas queue/manager/server interface tables do not enumerate all owner-attributed bodies. [Checklist covers every published slot/body/helper and explicitly marks the uninventoried remainder unresolved; no inference that 14 vector matches establish all 166 bodies or complete-owner coverage.]

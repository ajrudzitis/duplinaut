# Metadata-only chain planning checkpoint

`SetDescriptor::from_manifest_name` revalidates completed full/incremental
manifest kind, absent volume ordinal and correct interval ordering even for
caller-constructed names. It rejects partial and PKCS7 artifacts. OpenPGP names
are usable metadata without a decryptability promise. Timestamp grammar/name
provenance, source identity, manifest contents and intended backup collection
remain caller obligations; this API validates interval structure only.

`ChainPlan` requires an explicitly selected full descriptor index and exact
target timestamp. It borrows the descriptor slice and exposes its original
descriptors in plan order; neither that slice nor plan indices can be mutated
through the plan. It does not choose the newest full set or round the target to
an available date. A target equal to the full anchor needs no incrementals.

Each hop considers incremental edges starting exactly at the current end and
ending at or before the requested target. Exactly one is required. No candidate
reports missing evidence; multiple candidates report ambiguity, including
duplicate objects for one interval and a competing branch that later dead-ends.
This is conservative local uniqueness, not a search silently choosing the only
eventually complete path. Edges ending after the target are irrelevant to that
plan. Strictly forward intervals prevent cycles without timestamp arithmetic.

Input record count, owned output metadata and candidate checks have independent
limits. Output capacity is charged before allocation, at most one slot per input
descriptor; source descriptor storage is borrowed. Every scanned descriptor
consumes candidate-check budget, bounding the otherwise quadratic search work.
Standard-library allocation/iteration remain trusted infrastructure. Native
tests cover four official plain producer versions, short/legacy profiles and
adversarial gaps, branches, descriptors and budgets. A fresh strict whole-module
Verus run passes 17 obligations. Exact deterministic success/error selection,
original descriptor/index/target provenance, and valid contiguous timestamp
progress refine an independent total model. Allocation failure is an explicit
infrastructure outcome after successful header validation. The unchanged
`sets()` iterator adapter is outside this proof. A separate strict check proves
the completed-name scalar predicate (one obligation); complete name parsing and
same-source complete-core acceptance remain pending.

The verified controller uses `try_reserve`, whose empty-vector growth in the
pinned Rust 1.98 implementation requests `max(input_count, 4)` slots. The existing
metadata charge covers those slots. Allocator private heap overhead remains
trusted; this change does not claim a global heap proof.

A successful metadata plan establishes no required volume existence, stored-byte
integrity, decryption/signature acceptance, complete multipart data, restored
snapshot state or safe filesystem output. Those checks remain necessary before
reporting restorability. Missing-volume errors belong to subsequent assembly;
`ChainError::Missing` here reports missing interval metadata only.

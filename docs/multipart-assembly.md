# Multipart assembly checkpoint

This software checkpoint implements the reviewed duplicity 3.0.7 multipart
profile. Native tests and primitive-adapter review are evidence; refinement
proofs remain pending. Historical multipart constants and native OpenPGP
assembly are not claimed.

`MultipartAssembler` borrows normalized manifest volume records. Its only data
input method accepts an original stored object, verifies every supported
manifest digest, decodes the classified envelope, and parses the entire tar
container before examining entries. There is no caller-provided integrity or
footer boolean. Outputs remain private and provisional until `finish` checks
that every expected volume was supplied and no multipart entry remains pending.
Any failed input poisons the assembler; it cannot subsequently report success.

The part number belongs to the logical entry, not the stored volume. Every
nonfinal part is exactly 65536 bytes. A shorter part, including zero bytes,
finishes the logical entry. Parts must begin at 1 and remain consecutive, with
one operation and path and no interleaved entry. Aggregate payload, metadata,
entry count and normalized volume count have independent configurable bounds.

The marker rules follow the specification and pinned producer implementation:
[TarBlockIter](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/diffdir.py)
records the next part number after emitting a full chunk and clears it after a
short final chunk;
[write_multivol.get_indicies](https://gitlab.com/duplicity/duplicity/-/blob/rel.3.0.7/duplicity/dup_main.py)
subtracts one when constructing the manifest. A boundary's marker therefore
equals that emitted part number only when it is still incomplete. A volume
starting with a short final continuing part has an absent StartingPath block,
even when that part is greater than 1. The first and last actual entry paths
must equal the corresponding nonempty-volume manifest paths.

Empty-volume fallback is explicitly supported rather than used as completion
proof: the producer reports a root StartingPath without a block and an
EndingPath equal to the previous emitted index/block, or root/no block if no
entry was emitted previously. Every such volume still requires stored digest
validation and complete container parsing. Empty volumes preserve pending
assembly state and must agree with the previous ending boundary. This shape is
source-derived and covered by adversarial synthetic tests; current recorded
multipart fixtures provide nonempty-volume evidence.

The full 3.0.7 fixture spans three stored gzip volumes and contains 40 full chunks
followed by an empty final chunk. The first incremental spans two volumes and
contains a multipart librsync delta. Integration tests use original objects and
recorded manifests; source-tree SHA256 oracles independently check reconstructed
snapshot bytes and delta application. No external program runs at read time.

Ordinary and multipart representations cannot mix for one path in a set;
repeated multipart starts are rejected. Repeated ordinary operations remain
ordered operations for the later snapshot-state policy, rather than becoming a
new normative assembly rejection rule.

The metadata budget explicitly charges component-vector capacity (at least
four outer slots, at most doubled component count), per-component byte-vector
capacity (eight-byte minimum plus doubled total path bytes), and separately
accounts for output/index/boundary path copies, entry slots and owned text.
Six temporary path buffers are checked before classification using name
separator counts as an upper bound. The pinned standard library
[BTree node layout](https://raw.githubusercontent.com/rust-lang/rust/1.98.0/library/alloc/src/collections/btree/node.rs)
uses eleven keys/values and twelve edges; two conservative internal-node charges
per distinct start leave insertion overlap allowance. Original manifest record
count bounds normalization allocations, including superseded restart records.
These are conservative Rust capacity budgets, excluding allocator bookkeeping;
allocator and standard-library allocation behavior remain trusted. Tar metadata
and decoded-object buffers have separate limits. Formal heap bounds are pending.

Queued restart compatibility now follows PARSE-MAN-009: retained raw repeated
records for the specific normalized volume permit either a sole logical entry
with root/no-block start, or an exact second-logical-entry starting boundary.
Only the first entry's lower-bound check is deferred; every later entry remains
inside the declared path interval, and every entry remains below the declared
end. Normal exact-first boundaries retain their existing container profile.
The queued exception requires a plain/gzip decoded stream ending exactly after
the last entry and padding; a footer-bearing queued exception returns an explicit
unsupported-boundary diagnostic. Ending boundaries, hashes, complete containers,
part ordering, poisoning and final completion remain mandatory. Metadata headers
do not count as logical entries. Raw occurrence counts have bounded precharged
storage, and first/second/last path scratch capacity is charged before cloning.

`Manifest::select_volumes` returns every normalized record whenever any raw
number repeats, including a repeated suffix later discarded by normalization.
Record-only `may_contain` describes a declared interval and cannot certify actual
decoded coverage. The normal-profile selection path uses those hints; repeated
sets cannot prune from raw hints without a future identity-bound content index.
Selection limits charge raw number storage and normalized/output capacities
before allocation. Sorting and allocation remain trusted standard-library work,
and selection/controller refinement remains pending.

The original official one-entry, two-entry and empty-tail restart fixtures pass
native assembly and independent source-tree content/type/mode/time oracles.
Their source-changing interruption recipe does not establish complete signature
inventory or a consistent concurrently changing source snapshot. Queued encrypted
and footer-bearing replay remain unsupported pending separate evidence. Native
acceptance is distinct from the pending multipart/restart controller proofs.

A normalized manifest without any volume records is incomplete; it cannot
produce a completed empty archive. A recorded, hash-validated empty tar volume
remains valid under the explicit empty-volume boundary rules.

The caller must select the intended backup chain and manifest provenance policy,
obtain the original stored object through `ObjectSource` with its identity and
change checks, and supply the envelope classified from that object's filename.
This API validates stored digests against the supplied manifest; it does not
independently authenticate the manifest, bind an object key or source identity,
or establish the selected chain's continuity. Those obligations remain with the
source/chain controller. SHA1/MD5 compatibility validation is not authenticity.

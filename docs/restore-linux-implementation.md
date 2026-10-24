# Native Linux restoration checkpoint

This software follows the independently adopted [Linux specification](restore-linux.md)
and RFC requirements RESTORE-001 through RESTORE-027. It consumes a borrowed,
immutable `RestorePlan`; it neither downloads objects nor launches subprocesses.
It forbids unsafe project code. Complete filesystem/controller formal verification
remains pending, distinct from existing parser/state/plan refinement evidence.

The public API is `restore(&RestorePlan, &Path, &[u8], &Cancellation, RestoreLimits)`.
The byte-valued destination is one new leaf under the caller-selected parent.
`RestoreOutcome` distinguishes `Published(Receipt)`, `Unpublished(Unpublished)` and
`PublicationIndeterminate(Indeterminate)`. Receipts retain parent/root device and
inode identities, destination/staging bytes, chosen profile and actual filesystem
magic. The parent pathname may initially resolve through caller-selected symlinks;
subsequent work stays anchored to its opened directory object. The threat model,
UID/filesystem policy and held guarantees are exactly those of the specification.

The owned parent must permit owner write/search, deny group/other write and have
an allowlisted filesystem type. Default profile selection uses same-parent private
staging when parent ordinary group/other bits are zero, otherwise a private 0700
wrapper. An explicitly requested private-parent profile fails on a searchable
parent. Wrapper publication requires root owner-write; private-parent publication
preserves read-only root modes. Neither strategy changes parent permissions.
Every backup lookup uses one original byte-valued component and cached directory
FDs. Symlink target bytes are never lookup paths. Access/default ACLs are removed
and checked absent before widening final ordinary modes; directory default ACLs
are removed before descendants exist. Metadata/readback finish bottom-up, root
last, before one NOREPLACE rename. Empty plans without root metadata publish
policy 0700 and mark their root mtime unspecified.

Creation journals distinguish Unattempted, PossiblyCreated and Created. A slot is
armed before a creation boundary; native success marks Created before any postcall
hook. Child late errors remain removable inside the exclusively owned namespace,
including entries whose FDs could not be acquired. Outer creation errors other
than definite EEXIST leave an uncertain candidate untouched. Any publication
error returns uncertainty without chmod/unlink/rollback of root, descendants or
wrapper. Successful publication wins racing cancellation; only an empty wrapper
can receive post-success removal, with faults recorded as success warnings.
Prepublication cleanup repairs cached directory modes parent-first, then unlinks
attempted entries child-first through original cached parents. It continues after
faults. Fixed-size reports count all failures, retain the first errno and flag
truncated per-failure detail. Crash/fatal-signal cleanup, secure erasure, source
ownership/ACL/xattr fidelity and crash durability are not claimed.

## Exact independent budgets

These are operational charge units, not a global heap or wall-time proof. Borrowed
snapshot content and paths, allocations elsewhere in the caller, standard allocator
overhead, syscall/kernel storage and trusted rustix/getrandom internals are excluded.
Parent-path C-string conversion is explicitly charged before allocation/open;
backup lookups use pre-owned CStrings and avoid joined-path conversion. Fixed stack
buffers include a 65536-byte write slice bound (borrowed payload) and 4096-byte link
readback buffer. No payload copy is allocated by the restorer.

Let E be plan-entry count, R be one when source-root metadata exists, H be 1 for
private-parent staging or 2 for wrapper staging, D be source-directory count plus
one for a root synthesized only for an empty plan, and F be one when any regular
entry exists. C = E - R + H is the maximum created-entry count.

| Limit | Default | Exact unit / charging policy |
| --- | ---: | --- |
| entries | 1000000 | C, including root and applicable wrapper |
| directory_fds | 100000 | D + H + F: cached source/root directories, parent/wrapper and at most one active regular FD; minimum parent slot checked before opening |
| metadata_bytes | 268435456 | sizeof Prepared + Receipt + CString + OwnedFd, all charged name bytes below, E * (sizeof Node + sizeof Option OwnedFd), and one extra Option OwnedFd root slot; capacities reserved before staging |
| name_bytes | 67108864 | parent byte-path length+1, destination length+1,128 staging-byte overlap allowance,5 inner-root bytes including NUL, every nonroot leaf length+1 and symlink target length+1 |
| payload_bytes | 8589934592 | checked sum of complete borrowed regular payload lengths |
| forward_work | 32000000 | one unit per initial parent/stat syscall, preflight entry/component/candidate comparison, plan-node construction, staging random-name attempt, entry/finalization visit, forward syscall boundary and write attempt; partial writes and Interrupted retries consume units |
| staging_attempts | 16 | unpredictable 128-bit candidate selections; EEXIST only permits another candidate |
| cleanup_work | 4000000 | pre-reserves 2*C syscall attempts before staging, sufficient for cached-directory mode repair and attempted-entry removal; reserved independently from forward work; successful wrapper removal consumes one reserved attempt |
| profile | automatic | stricter explicit PrivateParentSameParent or ProtectedParentWrapper available |

Full borrowed component paths are validated component-by-component and ancestor
closure supplies corresponding planned directory entries; name charging counts
the owned leaf strings once, never repeatedly materialized joined paths. Component
and candidate comparisons count one work unit each, not one CPU instruction per
byte. Ordinary OS blocking has no cancellation latency bound. Cancellation is
observed before forward actions/chunks and the final publication decision; cleanup
ignores cancellation to attempt removal. Source staging metadata and fixed reports
are prepared before creation, and receipt strings move without allocation at the
publication decision. Allocator exhaustion abort behavior/process termination
remains outside handled-failure recovery.

## Native evidence and pending proof work

Eleven fault/controller tests and two integration tests cover both profiles on the
observed ext-family filesystem magic 0xef53. Original official full plus two
incrementals publish all raw paths and match source-oracle modes, regular content
hashes and file/directory mtimes. Link mtimes match exact emitted snapshot metadata;
source-only symlink mtime changes excluded by the recorded official oracle remain
excluded. Tests include outward targets, restrictive directories/root modes,
empty trees, inherited named-user/default ACL clearing before final mode 0777,
negative/fractional/extreme times (exact success or prepublication fidelity failure),
existing destination types, concurrent destination creation, parent pathname
replacement, wrapper privacy, exact entry/FD/payload/cleanup/name/metadata budget
edges, short/Interrupted/zero writes, cancellation/rename race, cleanup diagnostics,
create-error-after-side-effect and rename-error-after-side-effect.

Fault injection is a private generic syscall boundary compiled with Native no-op
hooks for the public API. Test after-hooks simulate a backend returning a late
error after a native effect; they do not claim to reproduce each kernel error path.
Publication after-hook errors are part of the abstract injected syscall result,
not a production post-success callback. Native runtime has no exported hooks.
Pinned safe wrappers are rustix 1.1.5 fs/process/time and
[getrandom 0.3.4 fill](https://docs.rs/getrandom/0.3.4/getrandom/fn.fill.html).
The native tests do not establish behavior on untested Btrfs/XFS/tmpfs, hostile
same-UID/privileged writers or compromised kernels/filesystems. Required future
proofs include budget-before-action, exact parent mapping, journal coverage, syscall
refinement and publication/cancellation/uncertainty transitions. No complete
controller proof is claimed by implementation annotations or passing native tests.

Independent source review and independent execution accepted this native checkpoint. The named-user ACL inheritance test requires both the effective UID and UID 1001 to be mapped in the test process user namespace. A sandbox mapping only UID 1000 cannot install that ACL and fails test setup; the exact test passed outside that sandbox, without a skip. Complete filesystem-controller formal acceptance remains pending.

After adoption, the complete local `cargo xtask ci` gate passed at commit `fdce7c9`, source SHA-256 `834aeae159efd3ced91c980398c3ceb3e126aa334d8b01c11cf1ee3144f215f1`, including fresh strict core verification, generated-parser and Duvet gates, formatting, Clippy, and native tests. This accepts the existing tooling milestone and tested native checkpoint, not missing filesystem-controller contracts or untested filesystem families.

The timestamp validator was independently accepted at `6d064b9` (candidate
`cdb92257cb125aa83bb9e68758d9028adea24d27`, tree
`381e7381846e4dc5256737c8f63fdc08cfa1fc51`) with exact-source SHA-256
`156942ede31c5e1b914338cfaad24ffb918c190d3deab8427951bbf7a9e4c499`.
The full elevated `cargo xtask ci` gate passed, including 2,047 vstd, 3,183
Vest, and 1,038 core Verus obligations with zero errors, Duvet, Clippy, all
workspace tests and doctests. Independent review confirmed that the production
conversion uses the validator and that its tests and annotations match the
scoped claim. This does not prove filesystem timestamp effects or the overall
Linux restore controller.

The scoped regular-file write-offset transition was independently accepted at
`7d8b3e4` (tree `e125f42b7bb4795bcafdd3cc11899b77124a980f`) and passed the full
exact-source `cargo xtask ci` gate with SHA-256
`5cf9315643ba58bfc14a3c6acde83433acdb51ad04f57de2d0b0c650fdad3168`.
Verus reported 2,047 vstd, 3,183 Vest and 1,020 core obligations with zero errors;
Duvet matched 138 requirements and 519 references, and formatting, strict Clippy,
workspace tests and doctests passed. The verified transition accepts only positive
write counts bounded by the current slice end and returns the checked next offset;
syscall behavior, chunk selection, retry budgets and the broader filesystem
controller remain outside this proof.

The scalar forward-work decrement was independently accepted at `3ec75fd`
(tree `02b4e1a189a553df340c8fe7d8c4347308dc1f22`) and passed exact-source full
`cargo xtask ci` with SHA-256
`3739d0e5722074b3439eb1d0213bcffde7f938dcd3caafa16a4ec3370bb14d46`.
Verus reported 2,047 vstd, 3,183 Vest and 1,021 core obligations with zero errors;
Duvet matched 138 requirements and 522 references. Formatting, strict Clippy,
workspace tests and doctests passed. The helper proves one-unit decrement for
positive work and exhaustion at zero; complete call-site accounting, syscall
duration, cancellation races and cleanup remain outside its claim.

The creation-journal transition and possibly-created cleanup checkpoint were
independently accepted at `0e1b62f` (tree
`3f9f6cf2b877781245f22f5d698fbdf8754a5381`) and passed exact-source full
`cargo xtask ci` with SHA-256
`273ce0535c2427b152e1af95fdc3d416217d1113781df6d2a0663dcaf02a4e29`.
Verus reported 2,047 vstd, 3,183 Vest and 1,024 core obligations with zero
errors; Duvet, formatting, strict checks, all workspace tests and doctests passed.
The before-create-hook test exercises cleanup after the conservative journal
state is set but before the creation syscall. It does not prove cleanup syscall
safety or the complete controller.

The publication cleanup-authority decision was independently accepted at
`7debf1f` (tree `c57a2ffaa69760b3796eb3dce4df7919a4e3d64b`) and passed
exact-source full `cargo xtask ci` with SHA-256
`bf311f34179c7c864273d6787c9ca37db2280e715a7f63859e46dbdeabed9d65`.
Verus reported 2,047 vstd, 3,183 Vest and 1,024 core obligations with zero
errors; Duvet queries/trace, formatting, strict checks, tests and doctests passed.
The contract limits post-attempt cleanup authority to removing an empty wrapper
after reported success; it does not prove the rename's kernel effects or earlier
controller ordering.

The multidimensional restore preflight was independently accepted at `f70787c`
(tree `4e8465171f0610efa40db0c3e1c1e742c3090a00`) and passed exact-source full
`cargo xtask ci` with SHA-256
`bb8e47f376314601adbf411a60eda13db7d70ee12fa95d9e7c1977a319bcc41e`.
Verus reported 2,047 vstd, 3,183 Vest and 1,027 core obligations with zero
errors; Duvet, formatting, strict checks, workspace tests and doctests passed.
The proof establishes acceptance/rejection of aggregate entry, directory-FD,
cleanup-work, payload and staging-attempt charges. It does not establish that
the caller derives complete charges for every resource.

The cleanup-schedule reservation bound was independently accepted at `a8e2d94`
(tree `9e58ecbed67b948e08ab31101d8d0a9c387a8304`) and passed exact-source full
`cargo xtask ci` with SHA-256
`087b9c13b012ae61daecc97fb4faba660ce21c435dbc2a525d7e70ef86a9c4e2`.
Verus reported 2,047 vstd, 3,183 Vest and 1,031 core obligations with zero
errors; Duvet, strict checks and all workspace tests passed, including the
inherited-ACL integration test with the required host filesystem support. The
proof bounds cleanup-call allowance by the pre-reserved `2*C` budget; syscall
effects and the full cleanup controller remain outside its claim.

The paired name/metadata charge transition was independently accepted at
`e132e24` (tree `8b9e659c96732712fbcea19448474f800250f1fa`) and passed exact-source
full `cargo xtask ci` with SHA-256
`9a8b403be5d0e517ffb6e4464658cd30b97ef53e5b5a7270089a467f28a01ceb`.
Verus reported 2,047 vstd, 3,183 Vest and 1,033 core obligations with zero
errors; Duvet, formatting, strict checks and workspace tests passed. The proof
checks the NUL-terminated name charge and both counters atomically before the
corresponding `CString` allocations; it does not establish complete allocator
or resource accounting.

The streaming restore preflight-summary transition was independently accepted
at `602bd99` (tree `215572446a06a0aead8b6183c9997d6d41629e38`) and passed
exact-source full `cargo xtask ci` with SHA-256
`db9de6810e209ace4d417f20d6d7961562760c83225db9a138b719440457b12e`.
Verus reported 2,047 vstd, 3,183 Vest and 1,036 core obligations with zero
errors; Duvet, strict checks and all workspace tests passed. The transition
atomically updates paired name/metadata, payload, directory and regular-file
summary values from supplied entry facts. It does not prove the traversal's
coverage/order or the derivation of those facts from the underlying state.

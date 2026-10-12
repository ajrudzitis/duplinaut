# Native Linux restoration profile

This specification supplement was independently reviewed and adopted before filesystem implementation. Requirements use RFC 2119/8174 uppercase keywords. `RESTORE-001` through `RESTORE-006` remain the general contract; the following stable IDs narrow the initial executable Linux profile. Adoption and independent review do not establish implementation or proof completion.

## Reviewed boundary and source provenance

Primary documentation was consulted before the pinned wrapper source. `DOC-LINUX-RENAME` is [Linux rename(2)](https://man7.org/linux/man-pages/man2/rename.2.html): directory-relative rename, filesystem-dependent `RENAME_NOREPLACE`, write permission on a directory moved between parents, and the explicit NFS ambiguous-failure caveat. `DOC-LINUX-ACL` is [Linux acl(5)](https://man7.org/linux/man-pages/man5/acl.5.html): the group mode class corresponds to an extended ACL's mask, and default ACL inheritance is constrained initially by creation mode. `DOC-LINUX-TIME` is [Linux utimensat(2)](https://linuxman7.org/linux/man-pages/man2/utimensat.2.html): no-follow link timestamps, `UTIME_OMIT`, and filesystem rounding. `DOC-LINUX-LINK` is [Linux symlink(7)](https://man7.org/linux/man-pages/man7/symlink.7.html): ordinary Linux symlink mode is fixed at `0777`. `DOC-LINUX-FS` is [Linux statfs(2)](https://man7.org/linux/man-pages/man2/fstatfs.2.html). `DOC-LINUX-XATTR` is [Linux removexattr(2)](https://man7.org/linux/man-pages/man2/removexattr.2.html).

`DOC-RUSTIX` is the published [rustix 1.1.5 API](https://docs.rs/rustix/1.1.5/rustix/), including [openat](https://docs.rs/rustix/1.1.5/rustix/fs/fn.openat.html), [renameat_with](https://docs.rs/rustix/1.1.5/rustix/fs/fn.renameat_with.html), [unlinkat](https://docs.rs/rustix/1.1.5/rustix/fs/fn.unlinkat.html), [chmodat](https://docs.rs/rustix/1.1.5/rustix/fs/fn.chmodat.html), [fgetxattr](https://docs.rs/rustix/1.1.5/rustix/fs/fn.fgetxattr.html), and [fstatfs](https://docs.rs/rustix/1.1.5/rustix/fs/fn.fstatfs.html). Published crate SHA256: `891efababe418670775f199f0d233d84843c227a0949a883ce15b37c78d6629d`, independently recorded in the workspace lockfile. The pinned `src/fs/{at,fd,xattr}.rs` documentation/signatures were consulted where individual rendered pages failed to load; they expose safe `fchmod`, `futimens`, `utimensat`, `fremovexattr`, and `OwnedFd` APIs. Project code can forbid unsafe code while relying on reviewed upstream unsafe syscall implementations.



Public rename documentation does not settle late local-filesystem metadata-error effects. Official kernel source fallback is therefore required: `SRC-LINUX-EXT4` is [fs/ext4/namei.c](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/fs/ext4/namei.c) at Linux v6.18 commit `7d0a66e4bb9081d75c82ec4957c50034cb0ea449` (annotated tag object `f7b88edb52c8dd01b7e576390d658ae6eef0e134`), SHA256 `9f8d5fcbb7ca3105f514244ed0a90d8539b95a9afe3771cc2c55424e1fedd864`. The inspected rename path adds the new entry at line3929 and can fail later at3947..3949 or3965..3986 without a universal rollback. At2779..2791, `ext4_add_nondir` can insert/instantiate an entry while returning a later dirty-metadata error; regular creation2830 and symlink creation3428 use this helper. `SRC-LINUX-VFS` is [fs/namei.c](https://github.com/torvalds/linux/blob/7d0a66e4bb9081d75c82ec4957c50034cb0ea449/fs/namei.c), SHA256 `22a87d724eb43ece32f77c144fdee6e89180446a54f0c3bfef023fc8016a6d33`: at5216 an underlying rename error bypasses VFS dentry movement at5231. These paths show why neither an errno nor casual pathname stat checks prove unchanged disk/namespace state. They establish the need for conservative uncertainty handling, not that every failure creates an entry or that a native fault test has reproduced each kernel path.

The threat model includes malicious backup paths/content/link targets and other-UID processes able to search a public destination parent. It excludes hostile processes sharing the effective UID, privileged writers, hostile mount or security-policy changes, compromised kernels/filesystem implementations, and writers that receive directory capabilities from this process. Filesystem handles and process identity stay inside that trusted boundary: filesystem UID matches effective UID, and credentials do not change during restoration. A successful restore publishes into the directory object held by the parent FD; later changes to ancestors or the caller's textual parent pathname can change how that object is found. The contract does not promise that an externally mutable pathname will continue to resolve to it.

Two profiles share the contracts below. `PrivateParentSameParent` uses an anchored parent with no group/other ordinary permission bits and stages an exclusive restore-root leaf directly there. It renames that leaf within the same parent and supports read-only root modes. `ProtectedParentWrapper` permits a searchable parent with no group/other write bits and stages `anchored-parent / exclusive-wrapper(0700) / root / selected entries`; it moves `root` from the wrapper FD to the parent, requiring root owner-write permission. In that profile the wrapper remains private through final root metadata application. A single staging root under a searchable parent cannot provide that privacy when its final mode allows other users to enter or write it.

## Proposed atomic requirements

### RESTORE-007

The initial Linux backend MUST reject destination parents whose filesystem type is outside the reviewed local allowlist before creating staging.

Provenance: POLICY, DOC-LINUX-FS, DOC-LINUX-RENAME. The allowlist is `0xef53` (ext2/3/4 family), `0x9123683e` (Btrfs), `0x58465342` (XFS), and `0x01021994` (tmpfs), obtained through `fstatfs` on the anchored parent. A listed type is not a promise that every kernel/mount configuration supports the required operations or timestamp values. Unknown types, overlay, NFS, CIFS/SMB, FUSE and other remote/userspace filesystems are held. Even an allowed local filesystem can mutate an entry and then return a late error; the allowlist never establishes error-means-no-change (SRC-LINUX-EXT4/VFS). No permission-check-and-ordinary-rename fallback is permitted if `NOREPLACE` is unavailable. Native tests name the actual filesystem exercised, without claiming coverage of other families.

### RESTORE-008

The backend MUST reject a destination parent unless its opened directory is owned by the effective UID, has owner write/search permissions, and has neither group-class nor other-class write permission.

Provenance: POLICY, DOC-LINUX-ACL. Check the opened parent with `fstat`: directory type; `st_uid == geteuid()`; `(st_mode & 0300) == 0300`; `(st_mode & 0022) == 0`. An extended POSIX ACL's effective named-user/group write grants are bounded by its mask, represented by the group mode bits. Sticky bits do not substitute for these checks. Parent write/search denial discovered by a syscall is an error, not grounds for changing parent permissions. The parent pathname may contain user-selected symlinks during initial opening; all subsequent work stays relative to the fixed FD. The excluded same-UID/privileged writers can invalidate this policy and are not silently covered by an inode check followed by rename.

### RESTORE-009

The backend MUST keep every provisional restored entry behind its selected profile's private directory boundary until publication or cleanup.

Provenance: POLICY. Use bounded collision retries for an unpredictable staging leaf; never adopt an existing leaf. In `ProtectedParentWrapper`, an exclusive wrapper holds the newly created inner root, and creation mode, ACL removal and explicit mode initialization establish an effective-user-owned wrapper with ordinary mode `0700` before descendant creation, verified through its opened FD. Other UIDs may list a searchable parent's wrapper name but cannot traverse it. Never publish by moving that wrapper or expanding its permissions. In `PrivateParentSameParent`, the opened parent itself denies other UIDs traversal; its permissions are not changed, and the provisional root may safely acquire its exact final mode before same-parent publication.

### RESTORE-010

All operations on backup entries MUST use their original byte-valued single path components relative to retained directory FDs, without reopening a directory by a joined path or following a restored symlink.

Provenance: POLICY, DOC-RUSTIX, PARSE-PATH. Retain `OwnedFd` handles for the parent, wrapper when applicable, root, and every created directory through publication or cleanup; directory opens use `DIRECTORY | NOFOLLOW | CLOEXEC`. A lookup's leaf is exactly one nonempty component, excluding NUL, slash, `.` and `..`. No `AT_FDCWD`, backup-controlled absolute syscall path, display-decoded path, or `/proc/self/fd` path workaround is used. Initially created directories may need a protected, single-component `chmodat(..., 0700, empty flags)` before their readable FD can be opened when umask removed owner permissions; exclusivity and the trusted parent/private boundary establish that this leaf is the directory just created. The operation is not a general no-follow chmod primitive.

### RESTORE-011

The backend MUST validate the complete restore plan's destination names, supported metadata representations, and initial-profile restrictions before creating staging.

Provenance: POLICY. Validate the destination leaf as one component distinct from the generated staging name; source-root metadata, if present, belongs to a directory. Regular entries have their complete immutable content. Symlink targets are nonempty bytes without NUL and are never interpreted as lookup paths; absolute, dangling and `..` targets may be preserved. Initial targets are limited to 4095 bytes and filesystem name bounds still apply. Reject any selected symlink whose ordinary mode differs from `0777`. Exact mtime seconds must fit the pinned backend's `Timespec` representation and nanoseconds must be below `1_000_000_000`. For `ProtectedParentWrapper` the effective restore-root ordinary mode must include owner write (`0200`) for movement between wrapper and parent; unsupported read-only root modes fail before staging, including an empty tree with supplied read-only root metadata. `PrivateParentSameParent` preserves read-only root modes because the containing parent does not change. The wrapper limitation is explicit, not permission normalization. Descendant directory modes may be `0000` or otherwise restrictive because cached FDs survive finalization.

### RESTORE-012

The backend MUST enforce independent checked budgets for created entries, retained directory FDs, owned metadata, path/name/target bytes, total regular payload bytes, forward work, staging-name attempts, and cleanup work before the associated allocation or syscall.

Provenance: POLICY, PARSE-GEN-005. Checked preflight sums include applicable wrapper and root overhead, journal/cache capacities, C-string terminators, and regular payload lengths. The FD budget accounts for the parent, applicable wrapper, root, all created directory handles and the active regular-file handle; existing descriptors elsewhere in the process can still cause a reported `EMFILE`. Retained descriptors cannot be replaced with insecure path reopening to satisfy a limit. Per-syscall partial-write/retry work is charged; exhausted forward work must not disable already reserved cleanup work. Ordinary kernel blocking and allocator/kernel internal storage are not claimed to be bounded by these budgets. The published interface documents exact defaults, charge units, and excluded borrowed snapshot storage before implementation acceptance.

### RESTORE-013

The backend MUST pre-reserve a journal state for every attempted creation and preserve possibly-created as well as definitely-created entries through handled failure cleanup.

Provenance: POLICY, SRC-LINUX-EXT4. Prepare the journal slot, parent-FD identity, original component, type, namespace-ownership state and capacity before the creation syscall. Arm `Attempted/PossiblyCreated` before that syscall; success records definite creation immediately without intervening allocation, callback, polling or other syscall. An error does not mean no entry was created. Inside an already exclusively owned private namespace, a unique planned leaf may be removed without following symlinks even if creation reported error; do not require an acquired child FD or a success-only journal bit. Failed first outer staging-leaf creation is handled by RESTORE-027 instead, because exclusive ownership was not established. Process termination can interrupt journal transitions and is outside the handled-cleanup guarantee.

### RESTORE-014

Regular files MUST be created exclusively with `CREATE | EXCL | NOFOLLOW | CLOEXEC | WRONLY`, initial creation mode `0600`, and written with the exact immutable content in chunks no larger than 65536 bytes.

Provenance: POLICY, DOC-RUSTIX. Handle short writes by continuing at the exact unwritten offset; zero writes with remaining data are errors. Interruption retries observe cancellation and consume bounded work. Never append, truncate an existing file, reuse a preexisting leaf, or create a backup-controlled hard link. A creation umask may remove owner permissions; the returned file FD remains usable for writing and final mode operations. After content is complete, remove inherited access ACL, then apply ordinary final mode and exact mtime through the FD. No backup ownership or special mode bits are applied.

### RESTORE-015

Symlinks MUST preserve the exact supported target bytes and receive their exact mtime through a no-follow operation relative to their retained parent FD.

Provenance: POLICY, DOC-LINUX-LINK, DOC-LINUX-TIME. Create with `symlinkat`; set time with `utimensat(..., SYMLINK_NOFOLLOW)` and verify type, fixed ordinary mode `0777`, target bytes and time without dereferencing. No ownership restoration or symlink chmod is attempted. A link target outside the destination is preserved as link data; the restorer never follows it. Future consumer traversal is outside this creation-time containment contract.

### RESTORE-016

The backend MUST remove inherited POSIX access ACLs from created regular files/directories and inherited default ACLs from created directories before publishing the tree.

Provenance: POLICY, DOC-LINUX-ACL, DOC-LINUX-XATTR. Remove `system.posix_acl_access` and, for directories, `system.posix_acl_default` using anchored FDs. Remove each directory's default ACL before creating its children, including the wrapper when present. Verify attribute absence before final mode application and publication; an inherited named grant can otherwise regain effective access when final chmod expands the ACL mask. `ENODATA` from removal means absent; `EOPNOTSUPP` is accepted only under the reviewed local filesystem contract that POSIX ACLs are unavailable, with the corresponding absence/capability check. Other errors fail before publication. No source ACLs or unrelated xattrs are restored; security labels inherited under trusted system policy are outside backup metadata fidelity.

### RESTORE-017

Before publication the backend MUST verify each created entry's supported type, ordinary permissions and exact mtime, and each regular file's complete size, against the restore plan.

Provenance: POLICY, DOC-LINUX-TIME. Final ordinary permissions are `source_mode & 0777`, without ownership, setuid, setgid or sticky restoration. Read back regular files/directories using `fstat` while their FDs remain available, and links with `statat(..., SYMLINK_NOFOLLOW)` relative to the cached parent; target read-back is separately bounded. Filesystem rounding, clamping, failed chmod or mismatched size is a prepublication fidelity error, not degraded success. Use `UTIME_OMIT` for atime; source atime/ctime, birth time, allocation layout, hard-link identity, ACLs/xattrs and ownership are not promised. A successful write syscall is not a claim of persistence after power loss.

### RESTORE-018

Directory final modes and mtimes MUST be applied through retained FDs after descendant creation, in reverse ancestor order, with restore-root metadata last.

Provenance: POLICY. Child creation changes directory mtime. Finalization after creation preserves even non-searchable descendant directory modes without reopening them. When present, the wrapper remains `0700` and is never assigned source-root metadata; a private anchored parent is likewise never assigned source-root metadata. Finishing metadata and read-back before rename ensures publication cannot expose an unfinished tree. Moving the root updates the containing staging/published parent directory metadata and status metadata; it does not justify subsequent modification of restored mtime or ordinary mode.

### RESTORE-019

When a genuinely empty complete plan has no source-root metadata, the backend MUST publish an empty root with explicit policy mode `0700` without reporting an invented source-root mtime.

Provenance: POLICY, reviewed RestorePlan. Its actual creation-time mtime is an unspecified result, explicitly identified in the receipt; no original timestamp is claimed. A nonempty plan requiring missing source ancestor/root metadata is invalid rather than a reason to synthesize metadata.

### RESTORE-020

The backend MUST make at most one publication attempt using its selected profile's directory-relative `NOREPLACE` rename, and only after all content, metadata and validation steps succeed.

Provenance: POLICY, DOC-RUSTIX, DOC-LINUX-RENAME. For `ProtectedParentWrapper` the call is `renameat_with(wrapper_fd, root_leaf, parent_fd, destination_leaf, NOREPLACE)`; for `PrivateParentSameParent` it is `renameat_with(parent_fd, staging_root_leaf, parent_fd, destination_leaf, NOREPLACE)`. This rename is the publication linearization point. An existing file, directory, dangling symlink or any other destination entry is a conflict; never replace it. Unsupported rename flags fail, without fallback. After success, perform no fallible metadata operation or rollback on the published tree. Any publication syscall error enters the indeterminate state in RESTORE-026; even the reviewed local filesystem boundary does not establish that the destination was untouched or that the root remains exclusively in staging. Parent-FD anchoring, restrictive parent policy and the excluded writers are essential; an inode-check/rename pair alone is not a race-proof guarantee.

### RESTORE-021

Cancellation MUST be observed before each entry, each bounded write attempt and the final publication attempt, with successful publication taking precedence over a racing cancellation request.

Provenance: POLICY. Cancellation observed before the final publication decision triggers staging cleanup and an unpublished result. Once rename is attempted, an error yields publication uncertainty rather than an unpublished cancellation result. A request arriving after the final cancellation observation can race with rename; successful rename yields a published receipt, even if cancellation is now set. No postpublication cancellation result may claim the destination remained unpublished. This is observation-based cooperative cancellation, not a guarantee that the syscall cannot start after another thread sets a flag or that kernel I/O is interruptible within a fixed time.

### RESTORE-022

On a handled failure or observed cancellation before any publication attempt, cleanup MUST attempt removal of journaled definitely-created or possibly-created entries inside exclusively owned staging using retained parent FDs and original component bytes, without following symlinks.

Provenance: POLICY, DOC-RUSTIX. First restore `0700` to cached created directories in parent-before-child order; an open parent FD does not bypass its search permission during relative unlink. Then remove entries in child-before-parent order using `unlinkat` with type-appropriate `REMOVEDIR`, finally removing root and any wrapper. A definitely or possibly created directory inside the owned private namespace whose FD could not be acquired remains journaled under its protected parent and may be removed as an empty directory there. RESTORE-027 restricts failed first outer-leaf cleanup. No such modifying cleanup runs after an indeterminate publication attempt. Continue independent cleanup attempts after an error; bounded diagnostics retain the original fault and all counted cleanup failures, indicating any diagnostic truncation. Never traverse `remove_dir_all`, enumerate unrelated parent entries, or remove the requested destination. Cleanup is an attempt, not secure erasure; crashes and fatal signals can leave artifacts.

### RESTORE-023

The result MUST distinguish unpublished failure/cancellation, published success, and publication-indeterminate outcomes, retaining post-success wrapper-removal faults as cleanup warnings on published success.

Provenance: POLICY. An unpublished error includes its initiating cause, cleanup outcome and any first-stage candidate uncertainty. A publication-indeterminate result identifies its attempted publication and syscall error with the pre-recorded parent/root identities and staging/destination names; it does not report a complete restore or an untouched/absent destination. Published success is recorded immediately when rename succeeds and cannot be converted to failure by a later allocation, cancellation, callback or wrapper cleanup fault; reserve the receipt's required storage beforehand. When a wrapper exists, cleanup after success touches only that now-empty wrapper, never the published root. The same-parent profile has no remaining staging entry to remove after publication. A successful publication provides completed-tree visibility and no replacement; a failed attempt can have uncertain effects. Neither outcome promises crash durability, secure deletion, protection after consumers access the published tree, or indefinite identity of the caller's textual parent path.

### RESTORE-024

Restoration assurance evidence MUST separate pure plan/controller guarantees from trusted syscall, rustix, kernel, filesystem and allocator assumptions.

Provenance: User assurance decision, ASSURE-PROOF/TRACE. Duvet implementation/test annotations attach to these stable headings after adoption; annotation presence does not establish passing evidence. Project restoration code forbids unsafe code and preserves the project's no-admission rules for any proved controller. Runtime integration and failure/race tests are required for the filesystem boundary; no existing parser/plan proof establishes syscall containment or atomic publication. Future proof goals include creation/journal correspondence, component/parent-FD mapping, charge-before-action, cleanup coverage, exact-write offsets, and publication/cancellation/indeterminate state transitions and no modifying cleanup after attempted-publication error. No filesystem/controller proof is claimed complete in this proposal.

### RESTORE-025

The backend MUST select and validate its publication profile before creating staging, without changing destination-parent permissions to qualify for either profile.

Provenance: POLICY, DOC-LINUX-ACL, DOC-LINUX-RENAME. Prefer `PrivateParentSameParent` when the common parent checks in `RESTORE-008` hold and `(parent_mode & 0077) == 0`; POSIX ACL group-mask absence also excludes effective named-user/group traversal. Otherwise use `ProtectedParentWrapper` only if its common parent checks and root-owner-write restriction hold. A caller may request a stricter explicit profile and receive a diagnostic if unsupported. The parent security permissions and trusted process/mount boundary remain valid throughout the operation; no claim covers hostile same-UID/privileged permission changes. Profile names appear in the successful receipt and native evidence so a public-parent wrapper result is not mistaken for tested read-only-root support.

### RESTORE-026

If a publication rename is attempted and returns an error, the backend MUST report publication uncertainty and refrain from modifying cleanup of the restore root, its descendants or its staging wrapper.

Provenance: POLICY, SRC-LINUX-EXT4/VFS. Initial policy treats every publication errno conservatively, including errors which a future reviewed recovery contract may classify more narrowly. Do not chmod, unlink, rename again, rollback, or remove staged names that might alias published inodes; retained FDs are not permission to modify a potentially published tree. Close/release FDs normally. Preserve reserved diagnostics, candidate names and recorded parent/root identities for future recovery. Read-only observations may be reported but cannot silently promote uncertainty to published success or authorize deletion: VFS cache and underlying namespace updates can disagree after a late error. This rule does not exclude metadata/I/O faults to preserve a convenient no-change assumption. Recovery and independently proved outcome resolution are a separate held profile.

### RESTORE-027

If creation of the first outer staging leaf returns an error other than a definite existing-entry collision, the backend MUST report its candidate as uncertain and leave that candidate untouched because exclusive ownership was not established.

Provenance: POLICY, SRC-LINUX-EXT4. Do not adopt, chmod, open for modification, remove, or retry/reuse an uncertain candidate on arbitrary creation errors. A definite `EEXIST` collision may be skipped with bounded selection of a different leaf, always leaving the existing entry untouched. Success, followed by private-boundary validation, establishes the owned namespace needed for descendant creation and attempted-entry cleanup under RESTORE-013/022. An outer creation error occurs before any backup plaintext descendant is written, but can leave a metadata artifact; neither a late errno nor an inode observation alone proves it is safe to delete a potentially preexisting entry. Caller-selected unrelated entries, including the requested destination, remain untouched.

## Required correction to the existing general cancellation requirement

Replace only the normative paragraph of `RESTORE-006` with:

> Cancellation observed before a publication attempt or a handled failure before that attempt MUST leave the restored tree unpublished at the requested destination and attempt removal of temporary plaintext from exclusively owned staging.

Its existing cleanup/crash provenance stays; add that any publication syscall error yields an indeterminate result without modifying cleanup (`RESTORE-026`), successful publication wins a racing cancellation request, and post-success wrapper cleanup warnings do not make the result an unpublished failure (`RESTORE-021`/`RESTORE-023`). First outer staging-leaf uncertainty is separately diagnosed (`RESTORE-027`). This makes the general requirement agree with an explicit achievable linearization contract.

## Acceptance evidence before calling the backend supported

| Evidence | Required observation |
| --- | --- |
| Official corpus restoration | Selected/full fixture contents, raw byte paths, file/directory/link types, ordinary modes and exact available mtimes agree with the independent source oracle and official restore. Cover empty files, raw UTF-8/non-UTF-8 names, spaces/quotes, incremental deletions/recreations/type changes and multipart content. |
| Entry safety | Outward/dangling/absolute links preserved without touching sentinels; restrictive directory modes still finalize and clean up; selected unsupported types/modes and invalid destination leaves fail before staging. |
| Publication | Every existing destination type survives byte-for-byte; concurrent destination creation prevents publication; metadata/root read-back succeeds before success; wrapper remains private with backed-up root mode `0777`; private-parent staging preserves root modes `0000`/`0555`/`0777` before same-parent publication. |
| Parent policy | Owned0755 accepted where operations permitted; writable group/other parents, including sticky/shared directories and effective named-ACL write grants, rejected; explicit anchored-parent behavior remains after ancestor pathname changes. |
| Metadata | Inherited access/default ACL removal including final widened modes; extreme/negative/fractional supported timestamps read back exactly or fail before publication; unsupported symlink modes fail in preflight; root owner-nonwrite modes succeed only in the private-parent same-parent profile and fail before staging in the searchable-parent wrapper profile. |
| Resource/fault/cancellation | Exact budget edges; real or injected creation including error-after-entry-creation, short-write/zero-write, chmod/time/read-back, publication including error-after-namespace-change, and cleanup faults; cancellation in entries/chunks and the rename race; attempted creation journaled before its syscall; no restore publication before an attempted rename; failed first staging-leaf creation preserves the uncertain candidate; publication error preserves potentially published data and returns uncertainty; published success retains wrapper-cleanup warnings. |
| Backend record | Identify kernel, actual filesystem type, rustix pin and operation capabilities used by each native run; no claim that one tmpfs/ext-family run tested Btrfs/XFS. |

## Held follow-up profiles

Root ordinary modes without owner write under searchable parents remain held; the reviewed private-parent same-parent strategy supports them only with its stricter parent policy. Adding write permission permanently or finalizing root mode after publication is not accepted. Overlay, distributed/userspace filesystems and hostile same-UID/privileged mutation need separate contracts. Crash durability/recovery and durable directory/file sync protocol, source ownership, ACL/xattr restoration, hard links and special types remain independent follow-up work. These holds do not weaken the initial profile's diagnostics or assurance claims.

## Native reference probe status

An isolated Python syscall probe at effective UID 1000 on the current `/tmp` filesystem moved a directory from a `0700` wrapper into its parent. Modes `0000`, `0555` and `0400` failed with `EACCES`; modes `0200`, `0700` and `0777` succeeded, preserving exact mode. A separate same-parent probe under parent mode `0700` succeeded for all six modes, including `0000`/`0555`, preserving each exactly. This confirms the documented cross-parent root write requirement and the private-parent alternative for the observed environment. The probe uses ordinary rename solely to test that permission rule; it is not evidence for production `NOREPLACE`, cancellation, ACL removal or complete filesystem restoration. See the [reproduction script](evidence/restore-linux/probe_linux_rename.py) and [JSON receipt](evidence/restore-linux/probe-linux-rename.receipt.json) for actual identities and outcomes.

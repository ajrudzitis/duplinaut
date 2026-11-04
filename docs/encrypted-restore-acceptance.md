# Official encrypted snapshot restoration acceptance plan

Status: proposed; this is specification only and makes no implementation or
backlog status change.

## Goal and scope

Exercise the public native flow from an official encrypted catalog through a
completed immutable snapshot, restore selection, and Linux filesystem
publication. This acceptance depends on the hosted gate for the
[encrypted-reader official oracle](encrypted-reader-oracle-acceptance.md)
passing first, and on the existing native Linux restore acceptance remaining
green.

Keep the case bounded to the official duplicity 3.0.7 **signed** profile at
all three recorded dates. One profile and a reusable three-date loop cover the
full-set snapshot and both incremental transitions without multiplying crypto
profiles. The existing Linux restore suite already covers the plain profile's
date matrix.

This plan traces to [PARSE-ENV-024](parsing-requirements.md#parse-env-024),
[PARSE-STATE-001](parsing-requirements.md#parse-state-001),
[RESTORE-001](parsing-requirements.md#restore-001),
[RESTORE-004](parsing-requirements.md#restore-004), and
[RESTORE-017](parsing-requirements.md#restore-017). These links provide
traceability and do not claim complete controller proofs.

## Acceptance requirements

1. Use the checked-in original 3.0.7 `signed/` profile through
   `LocalSource` and `Catalog::read_snapshot_with_crypto`. Select the full
   anchor and read each recorded target date: `1700000000`, `1700086400`, and
   `1700172800`. Supply the protected recipient secret export and fixture
   passphrase, signer certificate, and exact permitted primary fingerprint
   with `require_signature: true`; `message_passwords` MUST be empty for this
   public-key encrypted profile. Keep the read policy unchanged for each
   complete-chain read.
2. Before constructing a restore plan for each returned snapshot, require the
   evidence policy to report configured authorization and required signatures.
   Require `permitted_primary_fingerprints` to equal exactly the singleton
   list containing the expected full primary fingerprint.
   Require the evidence record count and exact object-key set to match the
   selected chain: two keys for the full date, four after the first
   incremental, and six after the second incremental. The key set MUST contain
   the required manifest and data volume for each included set, with no missing
   or additional records. Exact expected LocalSource keys are:

   | Target date | Exact evidence keys |
   | --- | --- |
   | `1700000000` | `duplicity-full.20231114T221320Z.manifest.gpg`; `duplicity-full.20231114T221320Z.vol1.difftar.gpg` |
   | `1700086400` | The two full keys above, plus `duplicity-inc.20231114T221320Z.to.20231115T221320Z.manifest.gpg`; `duplicity-inc.20231114T221320Z.to.20231115T221320Z.vol1.difftar.gpg` |
   | `1700172800` | The four prior keys, plus `duplicity-inc.20231115T221320Z.to.20231116T221320Z.manifest.gpg`; `duplicity-inc.20231115T221320Z.to.20231116T221320Z.vol1.difftar.gpg` |

   Only after these cardinality/key-set assertions,
   require every record to have kind `AuthenticationKind::OpenPgpAuthorized`
   and an authorized signer whose full primary fingerprint equals the
   permitted fingerprint. The filesystem backend receives the completed plan;
   do not claim that it independently validates reader authentication
   evidence.
3. Build the full `RestorePlan` from each snapshot with the existing default
   selection limits and call the native Linux `restore` API with a distinct
   new destination leaf beneath a test-owned `0700` parent on a filesystem
   allowed by the reviewed Linux profile. With default restore limits, require
   the `PrivateParentSameParent` profile and `RestoreOutcome::Published`; each
   destination MUST be absent before the call and present afterward. Do not
   expose decrypted intermediate bytes through an alternate restore path.
4. Walk each published destination using raw Unix path components. Inspect
   each entry with `symlink_metadata`, and recurse only into entries reported
   as directories; never follow a symlink while walking. Compare the complete
   tree with the corresponding `expected.json` snapshot: exact component
   sequences, supported entry types, ordinary mode bits, regular-file size
   and SHA-256, directory/file modification timestamps at supported exact
   precision, symlink target bytes, and no extra or missing entries. Hash
   restored file bytes and compare size/hash with the oracle; do not compare
   or print plaintext bytes. Use the recorded `symlink_mtime_ns` exclusion:
   do not compare symlink mtime to the producer oracle. Do not claim fidelity
   for fields absent from the oracle.
5. Treat `Receipt.destination` as the requested destination leaf, not a full
   path. Require it to equal the exact byte-valued leaf passed to `restore`.
   Require the receipt's parent device/inode identity to match the test-owned
   parent, its profile to equal `PrivateParentSameParent`, and its filesystem
   value to match the test-owned target filesystem and be in the reviewed
   allowlist. Test diagnostics MUST NOT contain secret exports, passphrases,
   decrypted content, or raw source/syscall error details.
6. If the backend returns `PublicationIndeterminate`, the test's temporary
   parent guard MUST preserve the parent and any possible published root and
   report their path; it MUST NOT recursively delete an uncertain destination.
   The acceptance's expected successful path remains `Published`.
7. Add Duvet test annotations for the linked existing requirements. The
   annotation/test description MUST state that this one signed profile across
   three dates exercises encrypted-to-filesystem composition; it does not
   cover other crypto profiles or formal controller refinement.
8. After all three restores succeed, enumerate the test-owned parent and
   require it to contain exactly the three destination leaves requested by
   the test. No staging, wrapper, or other entry may remain.

## Verification and evidence

- First require the hosted encrypted-reader oracle gate and existing native
  Linux restore tests to pass.
- On Linux, run the focused encrypted restore test for all three signed-profile
  dates and the complete
  `cargo test -p duplicity-core` suite. The focused test MUST verify that its
  destination parent is on a filesystem accepted by the existing test profile;
  it MUST fail explicitly rather than silently skip if that condition is not
  met.
- Run the targeted Duvet implementation/test queries for the linked
  requirements, `cargo xtask trace-update`, and `cargo xtask trace`. Run
  formatting and strict core Clippy checks, followed by required assurance CI
  (`cargo xtask ci`). Record hosted Linux CI evidence before marking this
  acceptance complete.
- Independent review MUST confirm caller credentials and signer policy are
  correct, the exact evidence count/key set is asserted before checking that
  each record is authorized, the normal
  reader and `RestorePlan` APIs are used, publication is no-replace through the
  reviewed backend, the filesystem walk does not follow links, the oracle
  checks exact fields, the indeterminate-result harness preserves uncertainty,
  and no source fixture is modified.
- The test establishes one native encrypted-reader-to-filesystem integration
  result. It does not prove OpenPGP primitives, reader or filesystem
  controller semantics, or behavior on untested filesystems.

## Exclusions

This acceptance does not cover live S3, other encrypted profiles, unsigned
encrypted data volumes, symlink mtime comparison against the producer oracle,
uid/gid, ownership, ACL/xattr restoration, or any other metadata absent from
`expected.json`. It makes no formal proof claim for cryptographic primitives,
the reader controller, or the filesystem publication controller.

## Checklist

- [ ] Encrypted-reader official oracle hosted gate passes.
- [ ] Existing native Linux restore suite passes on the supported filesystem.
- [ ] Implement the signed-profile three-date integration case and Duvet links.
- [ ] Focused/full Linux tests, Duvet trace, formatting, strict Clippy, and
      local assurance CI pass.
- [ ] Independent review and hosted Linux assurance pass.

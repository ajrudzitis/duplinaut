# PARSE-ENV-008 AES-128 symmetric payload acceptance

Status: implementation independently reviewed and accepted. Full core/workspace
tests, targeted Duvet queries/full trace, formatting, strict Clippy, and full
local `cargo xtask ci` pass at exact-source receipt SHA-256
`314798423dee202e877f291ce39b954858e9528b5d35edce19107b8f16567ea4`
(Verus 2,047/3,183/1,371, zero errors). The native-attendance gate is enabled.
Hosted Linux/Pages checks remain pending.

## Goal and scope

Verify public-reader compatibility with one official Duplicity 3.0.7 backup
whose symmetric OpenPGP payload uses AES-128 (OpenPGP symmetric algorithm ID
7). The currently checked-in official encrypted corpus uses AES-256 payloads;
the adapter accepts AES-128/192/256 by policy, but policy acceptance alone is
not producer interoperability evidence. This case adds that evidence for the
AES-128 symmetric profile under
[PARSE-ENV-008](parsing-requirements.md#parse-env-008).

The acceptance is limited to one Full manifest, one required data volume, and
one selected snapshot date. It exercises the public
`Catalog::read_snapshot_with_crypto` path with a caller-supplied message
password. It does not expand compatibility claims to AES-192, public-key
wrappers, or the full OpenPGP capability matrix.

## Fixture and source requirements

1. The fixture MUST be generated offline by official Duplicity 3.0.7 with the
   pinned GnuPG version from a deterministic source tree. The Duplicity/GnuPG
   producer configuration MUST request AES-128 for symmetric payload
   encryption. Provenance MUST record exact tool versions, normalized
   producer argv, the cipher-selection option, public test password source,
   source recipe, output object names, sizes, and SHA-256 hashes.
2. The fixture MUST contain a completed Full manifest and exactly one
   required Full volume. Both MUST be symmetric OpenPGP objects using exactly
   one SKESKv4 packet and zero PKESK packets, followed by authenticated
   SEIPDv1. Independent fixture inspection MUST establish those exact wrapper
   counts and the payload cipher for each object as AES-128, algorithm ID 7.
   Record the exact GnuPG status evidence (expected `DECRYPTION_INFO` cipher
   field `7`) and preserve the inspection command/output in provenance.
   Runtime tests MUST NOT call GnuPG or infer the payload cipher from producer
   options alone. Hybrid SKESK+PKESK objects are outside this case.
3. Provenance MUST pin the public-reader source identity policy: fixed
   version ID `symmetric-aes128-fixture-v1` per stored object, ETag equal to
   lowercase hexadecimal SHA-256 of that exact object's stored bytes, and
   listed size equal to the byte length. The test MUST assert exact catalog
   keys, parsed kind/interval/ordinal/envelope, size, version ID, and
   ETag-to-SHA correspondence. The source MUST reject any open request whose
   identity differs from its listed object.
4. Provenance MUST include an independent source-content oracle for the
   selected snapshot: raw component paths and order, entry types, modes,
   file/directory timestamps, regular-file sizes and SHA-256 content hashes,
   and symlink targets where present. Any official-producer comparison
   exclusion MUST be recorded and honored; no unrecorded field may be skipped.
5. The test MUST load checked-in artifacts, use only public test credentials,
   and perform no live S3 access, network key lookup, or runtime crypto-tool
   invocation.

## Normative acceptance requirements

### 1. Metadata-only discovery and exact source read set

`Catalog::discover` MUST identify the completed Full manifest and required
FullVolume 1 without opening object bodies. The test MUST verify that discovery
leaves the source-open log empty. The successful snapshot read MUST open
exactly the Full manifest followed by Full volume 1, each once, in that order.

### 2. AES-128 reader success

Using a `CryptoReadContext` whose only authentication credential is the
fixture's symmetric message password, `Catalog::read_snapshot_with_crypto`
MUST return the selected Full snapshot. Secret-key exports, signer exports,
permitted signers, and `require_signature` MUST be absent/empty/false. The
context MUST remain unchanged across the manifest and volume reads.

The snapshot MUST match the independent source oracle for every selected
entry: raw component sequence, entry type, `mode() & 0o777`, exact mtime
seconds and nanoseconds, regular-file length and content SHA-256, and symlink
target bytes. `SnapshotState::entries()` MUST enumerate entries in the same
raw component order as the oracle, as required by
[PARSE-STATE-003](parsing-requirements.md#parse-state-003).

The successful result MUST contain exactly two authentication records, one
for the manifest and one for Full volume 1. Both MUST be `OpenPgpIntegrity`
with no authorized signer. No key exports, password bytes, or plaintext may
appear in evidence, diagnostics, or test output.

## Requirement and assurance mapping

The test MUST link to PARSE-ENV-008 for the AES-128 payload capability,
PARSE-ENV-009 for caller-supplied symmetric passphrase use, PARSE-ENV-021 for
credential non-disclosure, and PARSE-ENV-023 for the integrity-only evidence
classification. It MUST link to PARSE-STATE-003 only for the explicitly
asserted snapshot-entry order. It MUST NOT claim AES-192 compatibility or
general crypto correctness.

Register the test under the existing OpenPGP reader edge-test ledger. Add a
narrow native-attendance assurance milestone such as
`openpgp-aes128-symmetric-compatibility` with this exact test and the public
reader composition implementation. The milestone MUST set
`proof_required = false` and state that it enforces native Duvet/test
attendance only, without Verus proof of cryptographic primitives or whole
reader control flow. Enable the gate only after independent implementation
review accepts the fixture and test mapping.

## Explicit exclusions

This acceptance does not test AES-192, AES-256 interoperability beyond the
existing official corpus, PKESK/RSA/cv25519 wrappers, signed objects, multiple
volumes, incremental chains, certificate validity, side-channel properties,
cryptographic primitive correctness, live S3, filesystem publication, or
complete reader/controller refinement. OpenPGP packet and crypto dependencies
remain trusted upstream boundaries.

## Verification and acceptance checklist

- [x] Independent specification review approves scope and the AES-128 fixture
      recipe.
- [x] Producer provenance pins versions, commands, exact object hashes and
      identities; independent GnuPG inspection confirms exactly one SKESKv4
      and zero PKESK packets, SEIPDv1/MDC, and AES-128 algorithm ID 7 for both
      objects.
- [x] The public-reader test verifies the empty/nonempty credential profile,
      metadata-only discovery, exact ordered two-key open log, complete
      snapshot oracle, and exactly two integrity-only evidence records.
- [x] Duvet links cover PARSE-ENV-008, PARSE-ENV-009, PARSE-ENV-021,
      PARSE-ENV-023, and the scoped PARSE-STATE-003 order assertion; targeted
      queries and full trace pass.
- [x] The focused test is registered in the existing reader edge-test ledger
      and the narrow native-only `proof_required = false` gate, enabled after
      independent implementation review.
- [x] The focused test, formatting, strict Clippy, targeted Duvet queries,
      full trace, full core tests, and full `cargo xtask ci` pass with the
      exact-source receipt recorded above.
- [ ] Hosted Linux assurance and Duvet Pages pass.

Acceptance establishes only the single AES-128 symmetric Full-volume profile
described here. It does not close the complete AES or OpenPGP compatibility
matrix.

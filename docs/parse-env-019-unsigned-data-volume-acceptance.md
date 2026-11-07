# PARSE-ENV-019 unsigned encrypted data-volume acceptance

Status: implementation, fixture provenance, focused local checks, and
independent implementation review pass. Full core tests and full local
`cargo xtask ci` pass on `aarch64-macos` (Verus 2,047/3,183/1,371, zero
errors). Hosted verification remains pending.

## Goal and scope

Verify that one unchanged chain-wide signer policy rejects an integrity-
protected OpenPGP data volume that has no authorized document signature, after
an authorized signed manifest has been read successfully. The existing
PARSE-ENV-019 mixed-chain tests stop at the first unsigned incremental
manifest; this case moves the failure boundary to the required data volume.

This is a bounded native public-reader acceptance for one Full set with one
required data volume. It does not require live S3 access and does not expand
the reader's Verus controller claims.

## Fixture and source requirements

### 1. Provenance-controlled pair

The test MUST use a checked-in, offline fixture package containing two
one-volume read sources with the same logical manifest interval, logical
archive content, signing certificate, and permitted primary fingerprint. The
signed manifest and positive OpenPGP volume MUST use the same test recipient
and signer. The unsigned symmetric volume has no recipient; the immutable
reader context MUST additionally carry its message password, which is used
only for that negative volume. The test signer, protected secret export, and
recipient credentials MUST come from the checked-in test fixtures:

- **Authorized positive control:** a Duplicity 3.0.7 producer-generated signed
  manifest and signed OpenPGP data volume.
- **Unsigned-volume case:** a validly signed OpenPGP manifest whose sole
  volume record contains the exact stored-object digest of an
  integrity-protected, symmetric-encrypted OpenPGP volume with no document
  signature. The volume's logical content and required parsed identity MUST
  match the positive control.

If the unsigned-volume case is assembled from producer output, the fixture
provenance MUST describe each transformation and record the final key, byte
length, and SHA-256 for every stored object. Any changed manifest MUST be
properly re-encrypted and signed with the checked-in test signer; editing a
signed manifest while retaining its old signature is forbidden. Record the
exact generator/reference commands and test-only credentials source. Tests
MUST load the checked-in objects and MUST NOT invoke duplicity, GnuPG, or
another crypto executable at runtime.

The fixture provenance MUST record the SHA-256 of each decrypted logical
volume tar payload and show that the positive and negative payload digests
match. The test MUST also read both sources once with signer authorization
unconfigured and compare each resulting snapshot with the same exact
one-volume source oracle. These integrity-only reads establish that the
negative volume is valid and reconstructs the intended logical data; the
subsequent positive and negative signer-policy reads MUST use the same
unchanged strict `CryptoReadContext`.

The mixed signed-manifest/unsigned-volume pair is an adversarial conformance
fixture; it MUST NOT be described as a combination emitted by one ordinary
Duplicity invocation unless provenance establishes that fact. The positive
control MUST remain a genuine official Duplicity signed output.

### 2. Exact read inputs

Each source MUST contain only the Full manifest and its one required volume,
with no duplicate keys. Names MUST parse to the same completed Full manifest
and FullVolume interval/ordinal. The negative volume MUST have a matching
stored digest in the authenticated manifest, valid source identity and exact
size, and a valid SEIPDv1 MDC. This ensures the intended outcome is the
missing signer authorization, not an earlier digest, identity, truncation,
credential, or framing failure.

Use one unchanged strict `CryptoReadContext` for the authorized positive and
unsigned-volume negative controls: signer verification enabled, the exact
fixture primary fingerprint permitted, the protected secret export for its
signed public-key objects, and the symmetric message password for the
negative volume. The negative symmetric volume does not use or require a
recipient key. The permitted-signer policy, credential slices, crypto limits,
and evidence limits MUST not change between the strict reads. The separate
integrity-only equivalence reads MUST use the same key/password material with
signer authorization disabled and MUST be identified as fixture controls,
not signer-policy acceptance cases.

## Normative requirements

### 1. Authorized positive control

With the positive-control source, `Catalog::read_snapshot_with_crypto` MUST
return the expected one-volume snapshot. The authentication evidence MUST
contain exactly two records, one for the manifest and one for the data
volume. Both records MUST be `OpenPgpAuthorized`, and each MUST report the
configured permitted primary fingerprint. The ordered open log MUST contain
exactly the manifest followed by volume 1.

### 2. Unsigned data-volume rejection

With the unsigned-volume source and the same context, discovery MUST remain
metadata-only. Reading the same Full anchor and target MUST return exactly
`ReaderError::Assembly(AssemblyError::Crypto(CryptoError::Unsigned))` after
the authorized manifest and unsigned required volume have been opened. The
ordered open log MUST contain exactly those two keys, in manifest-then-volume
order. The call MUST return no snapshot and no authentication-evidence
receipt.

The unsigned result MUST be reached only after source identity, exact stored
size/EOF, the manifest-declared stored digest, SEIPDv1 integrity, and
OpenPGP packet/literal-body framing succeed. `CryptoError::Unsigned` is
reported before Duplicity tar or multipart framing validation; tar/container
validation is explicitly outside this acceptance. A failure at any earlier
OpenPGP/source stage does not satisfy this acceptance.

## Requirement and assurance mapping

The new reader test MUST link to
[PARSE-ENV-019](parsing-requirements.md#parse-env-019) only. That requirement
mandates authorization for every required manifest and data volume under one
unchanged policy. Do not add claims for arbitrary envelope mixtures,
signature-archive behavior, unrelated artifacts, or current signer validity.

Register the exact public reader test under the existing
`openpgp-reader-edge-tests` assurance milestone. This is native integration
evidence; no Verus proof of OpenPGP primitives, signature validation,
manifest-to-volume composition, or whole-reader control flow is claimed. The
existing `openpgp-reader-edge-tests` milestone remains a scoped test ledger,
not proof completion of reader orchestration.

The test MUST also be registered in a separate narrow gated assurance
milestone named `openpgp-unsigned-volume-signer-policy` (or an independently
reviewed equivalent) with `PARSE-ENV-019`, this exact test, and
`crates/core/src/reader.rs::read_snapshot_inner` as its implementation. This
milestone MUST set `proof_required = false` and MUST state that it enforces
native Duvet/test attendance only; it makes no Verus claim about cryptographic
primitives or whole-reader control flow. Keep the test's existing registration
under `openpgp-reader-edge-tests` as well. After independent implementation
review accepts the exact test and mapping, enable the narrow milestone gate so
xtask enforces this Duvet/test coverage.

## Exclusions

This acceptance does not prove cryptographic primitives or side-channel
properties, GnuPG interoperability generally, arbitrary mixed-envelope
chains, unsigned incremental manifests, plain/gzip downgrade behavior,
unused signature archives, RSA or other algorithm profiles, certificate
expiry/revocation policy, cancellation timing, resource accounting,
filesystem publication, live S3 behavior, or complete reader refinement.
Fixture provenance MUST not imply this mixed combination is a standard
Duplicity-produced profile.

## Acceptance evidence

- [x] Independent review accepts the fixture's signed-manifest/unsigned-volume
      provenance, and confirms the negative fixture's stored digest and MDC
      are valid before the intended signer failure.
- [x] Fixture provenance records equal decrypted logical-volume tar SHA-256
      values; integrity-only public-reader reads reconstruct both exact
      expected one-volume snapshots before the strict signer-policy tests.
- [x] The authorized positive control succeeds with exact two-record,
      per-record signer evidence and exact two-key open log.
- [x] Under the identical policy/context, the unsigned volume returns exact
      `ReaderError::Assembly(AssemblyError::Crypto(CryptoError::Unsigned))`,
      with exact two-key open log, no snapshot, and no returned evidence.
- [x] The focused test links only to PARSE-ENV-019 and is registered under
      `openpgp-reader-edge-tests`.
- [x] A gated narrow `PARSE-ENV-019` assurance entry also names
      `read_snapshot_inner` and this exact test, sets `proof_required = false`,
      and is enabled after independent implementation review.
- [x] Targeted Duvet PARSE-ENV-019 implementation/test queries, trace update,
      and full trace pass.
- [x] Focused reader test, formatting, and strict Clippy pass.
- [x] Full core tests and full `cargo xtask ci` pass. The isolated receipt
      records source SHA-256
      `0d556df683d74b312de34983ad479773181084fc530e2ac7ef57e755f8ab7cd7`.
- [x] Independent implementation review accepts the source ordering, same
      policy inputs, typed outcome, and scope limits.
- [ ] Hosted Linux assurance and Duvet Pages pass.

## Local implementation evidence

- Full `nix-shell tools/shell.nix --run 'source tools/env.sh && cargo xtask ci'` passes with fresh isolated Verus (2,047/3,183/1,371, zero errors), full workspace tests, Duvet trace/gate queries, and gated test replay. Receipt: `target/assurance/ci.json` (source SHA-256 above; `aarch64-macos`).

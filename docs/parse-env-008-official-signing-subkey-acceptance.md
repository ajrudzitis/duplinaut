# PARSE-ENV-008 official signing-subkey compatibility acceptance

Status: proposed; pending independent specification review. No fixture, test,
assurance-gate, or implementation work is included in this specification yet.

## Goal and scope

Establish successful public-reader verification of an official Duplicity
3.0.7 signed Full manifest and its one required data volume when both
OpenPGP document signatures are made by a bound Ed25519 signing subkey. The
checked-in official signed fixture uses its primary key for document
signatures; existing test-generated back-signature cases do not establish
official-producer compatibility for a signing subkey. This closes that
specific gap in the initial supported profile under
[PARSE-ENV-008](parsing-requirements.md#parse-env-008).

The acceptance covers one Full snapshot date and the two required encrypted
objects. It checks that authorization remains by the caller's complete
primary fingerprint while evidence identifies the actual signing-subkey
component. It does not prove OpenPGP primitives or the full reader controller.

## Fixture and source requirements

1. The fixture MUST be generated offline using official Duplicity 3.0.7 and
   GnuPG. Its v4 certificate MUST contain a primary certificate key, a
   signing-capable Ed25519 subkey, and the encryption-capable recipient
   component needed for the selected signed public-key profile. The key
   material and passwords MUST be public test credentials with no dependency
   on a developer keyring. Provenance MUST record tool versions, exact
   normalized producer argv, key-generation recipe, fixture object names,
   byte lengths, SHA-256 values, and target date.
2. Provenance MUST establish that the v4 signing subkey is bound to the
   exported primary certificate, has signing capability, and has a valid
   embedded subkey-to-primary back signature. It MUST independently inspect
   the actual manifest and volume document signatures and resolve each to
   that signing-subkey fingerprint, not merely infer subkey use from a
   one-pass signature hint or from key preference. The exported certificate
   MUST contain the binding needed by the reader. The fixture must not be
   described as official producer output unless the recorded commands and
   hashes establish that provenance.
3. The fixture MUST represent one completed Full set with one required data
   volume. The test source MAY expose any producer-generated signature
   archive as an unused catalog object, but it MUST assert that the only
   opened keys are the Full manifest followed by Full volume 1. No signature
   archive is used as authorization evidence.
4. Provenance MUST contain an independent source-content oracle for the
   selected date, sufficient to compare raw path components, entry type,
   modes, file/directory timestamps, file sizes and SHA-256 content digests,
   and symlink targets where present. Any field excluded from comparison MUST
   be explicitly justified by recorded official-producer behavior.
5. Tests MUST load checked-in artifacts and MUST NOT invoke Duplicity, GnuPG,
   or another crypto executable at runtime. Live S3 access is not required.

## Normative acceptance requirements

### 1. Public-reader reconstruction and policy

The test MUST call `Catalog::read_snapshot_with_crypto` using one unchanged
context containing the checked-in protected secret export and its test
passphrase, the exported primary certificate, the exact permitted primary
fingerprint, and `require_signature = true`. Message-password inputs MUST be
empty. The permitted list MUST contain the primary fingerprint, not the
signing-subkey fingerprint.

The reader MUST return the selected Full snapshot matching the independent
source-content oracle. Discovery MUST be metadata-only. The ordered open log
MUST contain exactly the Full manifest and Full volume 1, each once, in that
order.

### 2. Per-object signer evidence

The successful result MUST contain exactly two authentication records: one
for the Full manifest and one for Full volume 1. Both MUST have kind
`OpenPgpAuthorized`; each authorized signer MUST report the exact permitted
primary fingerprint and the exact signing-subkey component fingerprint
independently recorded in provenance. The test MUST assert that the component
fingerprint differs from the primary fingerprint. It MUST NOT treat a key ID,
issuer hint, one-pass packet, or the subkey fingerprint alone as authorization.

Each evidence record MUST refer to its matching original source key and
identity, and MUST have the correct Full set role and ordinal. No secret
export or passphrase bytes may appear in the snapshot, evidence, diagnostics,
or test output.

## Requirement and assurance mapping

The focused test MUST link to PARSE-ENV-008 for supported Ed25519 document
signatures and signing-key binding, PARSE-ENV-012 for authorization by the
complete primary fingerprint, PARSE-ENV-019 for applying the configured
signer policy to every required object, PARSE-ENV-021 for credential
non-disclosure, and PARSE-ENV-023 for distinguishing authorized evidence and
retaining the actual signer component fingerprint. The test MUST NOT claim
that these five scoped links prove cryptographic primitives, certificate
validity policy, or all signing-subkey variants.

Register the test in the existing OpenPGP reader edge-test ledger. To make
Duvet attendance enforceable without implying formal crypto proof, add a
narrow native milestone such as `openpgp-signing-subkey-compatibility`, with
this exact test and the public reader composition implementation. The
milestone MUST set `proof_required = false` and explain that it enforces only
native Duvet/test attendance. Enable the gate only after independent
implementation review accepts the fixture provenance, test, and requirement
mapping. No Verus claim is made for OpenPGP cryptography or reader control flow.

## Explicit exclusions

This acceptance does not establish RSA document-signature compatibility,
other signing algorithms or hash combinations, multiple signing subkeys,
multiple signers, key rollover, expiry/revocation handling, Web-of-Trust
policy, timing/side-channel resistance, cryptographic primitive correctness,
other Duplicity versions, incremental signed chains, live S3 behavior,
filesystem publication, or complete reader/controller refinement.

## Verification and acceptance checklist

- [ ] Independent specification review accepts the fixture-generation and
      exact component-attribution criteria.
- [ ] Checked-in producer provenance pins tool versions, exact command,
      fixture object hashes, date, primary/subkey fingerprints, subkey
      binding/back-signature facts, and actual signature component for both
      required objects.
- [ ] The public-reader test matches the independent snapshot oracle and
      exact two-key open sequence using a nonempty primary allowlist and no
      message passwords.
- [ ] Exactly two authorized evidence records preserve the expected primary
      and signing-subkey fingerprints for their respective source objects.
- [ ] Duvet links cover PARSE-ENV-008, PARSE-ENV-012, PARSE-ENV-019,
      PARSE-ENV-021, and PARSE-ENV-023; targeted queries and full trace pass.
- [ ] The exact test is in the existing reader edge-test ledger and the
      narrow `proof_required = false` gate; independent implementation review
      approves enabling that gate.
- [ ] Focused and full core tests, formatting, strict Clippy, and full
      `cargo xtask ci` pass with a recorded exact-source receipt.
- [ ] Hosted Linux assurance and Duvet Pages pass.

Acceptance applies only to this one official Ed25519 signing-subkey Full
snapshot profile. It does not close the broader signing-subkey or crypto
compatibility matrix.

# Encrypted manifest signer rejection before volume access

Status: specification and implementation independently reviewed and accepted.
The focused/full core tests, targeted Duvet queries, full trace, formatting,
strict Clippy, and full local `cargo xtask ci` pass at exact-source receipt
SHA-256 `bd449ed17c1dc507297cfb29c194d6b478ee643efabb0d7cde7f5e819653d1b8`
(Verus 2,047/3,183/1,371, zero errors). The narrow native-attendance gate is
enabled; hosted Linux/Pages checks remain pending.

## Goal and scope

Add an observable public-reader failure boundary for
[PARSE-ENV-018](parsing-requirements.md#parse-env-018): a validly signed
official encrypted Full manifest must be rejected when the caller's signer
allowlist excludes its signer, before the reader opens any volume named by that
manifest. The existing disallowed-signer regression checks the typed error but
does not assert the source-open sequence.

This is a fixture-specific reader ordering test. It does not alter the signer
policy or establish OpenPGP primitive correctness, complete reader control
flow, or general manifest-parser ordering for every failure mode.

## Fixture and harness requirements

1. The test MUST use the checked-in signed Duplicity 3.0.7 Full manifest and
   its corresponding Full volume 1, with the existing public test recipient
   secret, password, signer certificate, and independent first-date source
   oracle. It MUST NOT call Duplicity, GnuPG, or another crypto executable at
   test runtime.
2. A test-local source MUST list the same immutable object identities for both
   reads and record each successful `open` call's exact key in order. Discovery
   MUST leave that log empty. The source MUST reject an `open` request whose
   stored metadata differs from its listed object.
3. The positive and negative reads MUST use the same catalog/source, recipient
   secret export and passphrase, signer certificate, crypto limits, and
   evidence limits. They MUST differ only in `permitted_signers`: the positive
   uses the fixture's exact primary fingerprint; the negative uses the
   existing 20-byte suffix-collision candidate, whose last eight bytes match
   the fixture fingerprint while preceding bytes differ.

## Normative acceptance requirements

### 1. Authorized positive control

With the exact permitted primary fingerprint, the public reader MUST return
the official Full snapshot matching the independent oracle. The ordered open
log MUST be exactly:

1. `backup/duplicity-full.20231114T221320Z.manifest.gpg`
2. `backup/duplicity-full.20231114T221320Z.vol1.difftar.gpg`

The returned authentication evidence MUST contain exactly two records, for
that manifest and volume, and both records MUST report `OpenPgpAuthorized`
with the fixture's complete 20-byte primary fingerprint. The test MUST clear
the open log after capturing and checking this positive control.

### 2. Disallowed manifest signer fails before volume access

With only the permitted fingerprint changed to the suffix-collision
candidate, the same public-reader operation MUST return exactly
`ReaderError::Crypto(OpenPgpError::SignerNotAllowed)`. The ordered open log
MUST contain exactly one key:

1. `backup/duplicity-full.20231114T221320Z.manifest.gpg`

In particular, the Full volume MUST NOT be opened. The call MUST return no
snapshot or evidence receipt. The supplied password and key bytes MUST NOT
appear in the error's `Debug` or `Display` text or test logs.

This one-object open prefix is the observable oracle for the selected
PARSE-ENV-018 boundary: configured signer rejection of the encrypted manifest
occurs before a volume record can cause a data-volume read. It does not prove
that no manifest bytes were decoded internally before signature authorization;
the crypto adapter's own ordering contract is maintained separately.

## Requirement and assurance mapping

The new public-reader test MUST link to PARSE-ENV-018 for rejecting a manifest
before its volume records are used, PARSE-ENV-012 for full-fingerprint signer
authorization, PARSE-ENV-014 for fail-closed behavior without publication,
and PARSE-ENV-021 for credential non-disclosure. The added acceptance
evidence is specifically the exact source-open prefix and typed failure; it
does not replace or broaden the existing PARSE-ENV-012 fingerprint-content
acceptance.

The implementation SHOULD register the test under the existing
`openpgp-reader-edge-tests` ledger. A narrow native-attendance assurance gate
may be added if required to enforce the new Duvet trace. Such a gate MUST set
`proof_required = false` and explicitly make no Verus claim about signature
cryptography, manifest parsing, or the complete reader controller. Enable any
new gate only after independent implementation review accepts its exact test
and mapping.

## Exclusions

This case does not cover malformed or unsupported encrypted manifests,
signature-archive semantics, incremental chains, unauthorized data-volume
signatures, unsigned fallback, every crypto failure boundary, live S3,
filesystem restoration, cancellation timing, resource limits, cryptographic
primitive correctness, or reader-controller formal refinement.

## Acceptance evidence

- [x] Independent specification review accepts the exact fixture, credentials,
      and open-sequence oracle.
- [x] The focused positive control checks exact snapshot/oracle fields,
      two authorized evidence records, and the exact two-key open sequence.
- [x] The negative path changes only the allowlist, returns the exact typed
      signer error, and records only the signed Full-manifest key in its open
      log; no snapshot or evidence receipt is returned.
- [x] The test is Duvet-linked to PARSE-ENV-018, PARSE-ENV-012, PARSE-ENV-014,
      and PARSE-ENV-021, and its source/test mapping is accepted in the
      assurance ledger without a crypto or controller proof claim.
- [x] Focused and full core tests, targeted Duvet queries/full trace,
      formatting, strict Clippy, and full `cargo xtask ci` pass.
- [x] Independent implementation review accepts the positive control,
      unchanged credentials, exact opened-key boundary, and typed failure.
- [ ] Hosted Linux assurance and Duvet Pages pass.

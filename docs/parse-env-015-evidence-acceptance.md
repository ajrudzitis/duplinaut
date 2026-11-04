# PARSE-ENV-015 trusted-boundary evidence acceptance plan

Status: implementation, independent review, and full core verification pass;
hosted assurance CI remains pending.

This is a bounded traceability slice for
[PARSE-ENV-015](parsing-requirements.md#parse-env-015). The module trust-boundary
documentation and positive/negative original-fixture integrations now carry
scoped Duvet links. No AWS access or new parser behavior is required.

## Acceptance requirements

1. An implementation annotation MUST link the native OpenPGP adapter's
   documented trusted boundary to PARSE-ENV-015. The accompanying
   [OpenPGP profile](openpgp-profile.md) and
   [implementation record](openpgp-implementation.md) MUST document the pinned
   dependency identity and its limitations for reviewers.
2. At least one positive integration test MUST link to PARSE-ENV-015 and
   exercise an original official encrypted chain through native decode and
   reader reconstruction with independent source-content expectations.
3. At least one negative integration test MUST link to PARSE-ENV-015 and use
   original encrypted producer data with a controlled integrity or credential
   failure. It MUST require an error and no completed snapshot for that
   failed read.
4. The linked evidence MUST identify pinned `pgp` version and archive/source
   identity, make clear that packet parsing and cryptographic primitives are
   trusted upstream code, and state that no Verus proof of those primitives is
   claimed. Credentials and decrypted payloads MUST remain out of diagnostics.

## Verification and evidence

- The linked positive and negative integration tests MUST pass using checked-in
  fixture data, without external crypto commands, network access, or live AWS
  credentials.
- Duvet's implementation and test checks for PARSE-ENV-015 MUST pass. The
  implementation annotation, tests, and profile links establish provenance
  and evidence traceability; they do not prove cryptographic correctness.
- Full core native tests, Duvet trace, and required assurance CI MUST pass
  before this evidence slice is recorded complete. Independent review MUST
  verify that the documentation matches the dependency pinned in the lockfile
  and that stated limitations are not broadened by the tests.

## Current evidence

- Positive: `original_encrypted_chains_reconstruct_each_official_date_with_bounded_evidence`
  runs original symmetric, public-key, and signed chains through the native
  reader and compares all three dates per profile against independent source
  content expectations.
- Negative: `public_crypto_reader_rejects_wrong_symmetric_credentials_without_snapshot_or_secret_disclosure`
  supplies a controlled wrong passphrase for original ciphertext, requires a
  typed crypto error (and therefore no completed snapshot result), and checks
  that the passphrase is absent from debug and display diagnostics.
- The implementation record identifies locked `pgp` 0.21.0, the published
  archive SHA-256 and source commit, trusted upstream operations, the RSA
  timing/Marvin caveat, and the absence of Verus crypto proofs. It keeps
  reader composition source-reviewed and encrypted filesystem restoration
  outside this claim.
- Focused positive and negative tests, `cargo test -p duplicity-core`, the
  targeted PARSE-ENV-015 Duvet query, `cargo xtask trace`, formatting checks,
  and strict core Clippy pass. Clippy emits existing warnings in vendored
  `vest_lib`, while `duplicity-core` passes `-D warnings`. Independent review
  confirms the lockfile/archive identity and scoped trust limits; hosted
  assurance CI remains pending.

## Non-goals

This slice does not add algorithms, keys, packet variants, or compatibility
claims; test every supported crypto failure mode; prove the trusted upstream
cryptographic implementation; complete the reader controller refinement; or
require live S3 access.

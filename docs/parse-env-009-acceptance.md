# PARSE-ENV-009 caller-credential acceptance plan

Status: proposed before adding Duvet implementation and test links.

This is a local traceability and regression-test slice for
[PARSE-ENV-009](parsing-requirements.md#parse-env-009). Its bounded scope is
the caller-provided credential path: symmetric message passphrases and
exported supported secret keys. The current section-scoped Duvet query reports
no implementation annotation; the core README already records successful
official symmetric and public-key encrypted-chain tests.

## Acceptance requirements

1. An implementation annotation MUST link the decode path that consumes
   caller-supplied message passphrases and exported secret-key bytes to
   PARSE-ENV-009. Source review MUST confirm that the adapter uses those
   borrowed inputs directly and does not invoke an external key agent or
   crypto application.
2. A positive symmetric test MUST use the official encrypted 3.0.7 chain and
   the fixture's caller-supplied message passphrase. It MUST reconstruct the
   expected snapshots and authenticate their contents against the independent
   fixture oracle.
3. A positive public-key test MUST use the official encrypted 3.0.7 chain,
   the caller-supplied exported secret-key bytes and their fixture password.
   It MUST reconstruct the expected snapshots and authenticate their contents
   against the same independent fixture oracle.
4. The positive tests MUST be Duvet-linked to PARSE-ENV-009 and run locally
   without network access, live AWS credentials, a keyring, a password prompt,
   or fixture modification. Diagnostics MUST NOT expose passphrases, secret
   key bytes, or decrypted payloads.

## Verification and evidence

- The focused official symmetric and public-key reader tests MUST pass and
  match the recorded independent snapshot/source-content oracle.
- Duvet's implementation and test checks for PARSE-ENV-009 MUST pass. The
  implementation annotation and test links establish traceability, not
  cryptographic primitive correctness.
- Source review MUST verify that credential use is limited to supplied
  message passwords and exported key material. The crypto adapter MUST NOT
  spawn GnuPG, an agent, or another external helper.
- Full core native tests, Duvet trace, and required assurance CI MUST pass
  before this slice is recorded complete. No Verus proof is claimed for the
  pinned OpenPGP library or its cryptographic primitives.

## Non-goals

This slice does not add key formats, algorithms, S2K or secret-material
protection profiles, import keys into a user's keyring, alter credential
storage policy, or prove confidentiality or side-channel resistance. It does
not expand signer authorization or OpenPGP reader-controller proofs, and it
does not require live S3 access.

# PARSE-ENV-012 primary-fingerprint acceptance plan

Status: implementation and full core verification complete; independent review
and hosted assurance CI remain pending.

This is a bounded local acceptance slice for
[PARSE-ENV-012](parsing-requirements.md#parse-env-012): exact matching of the
caller-authorized full primary-certificate fingerprint. The implementation
and existing official signed-chain positive and disallowed-signer test paths
now carry scoped Duvet links. No AWS access is needed.

## Acceptance requirements

1. An implementation annotation MUST link the authorization decision to
   PARSE-ENV-012 at the code that derives the primary certificate fingerprint
   and compares it with the caller's permitted fingerprints. The annotation
   and review MUST show that a short key ID or packet issuer hint is not an
   authorization input.
2. A positive test MUST read the official signed 3.0.7 fixture with the
   fixture's correct credentials and the exact 20-byte permitted primary
   fingerprint. It MUST require successful snapshot reconstruction and
   authentication evidence whose authorized signer reports that same complete
   primary fingerprint. The test MUST be Duvet-linked to PARSE-ENV-012.
3. A negative test MUST use the same valid fixture signature and credentials,
   but supply a different 20-byte permitted fingerprint that shares the
   signer's final eight fingerprint bytes and differs in its preceding bytes.
   This is a suffix/key-ID collision candidate, not the authorized primary
   fingerprint. The reader MUST reject it as `SignerNotAllowed` and return no
   snapshot. The test MUST be Duvet-linked to PARSE-ENV-012.
4. Both cases MUST use checked-in, publicly labeled test fixtures and run
   without network access, secret prompts, or fixture modification. They MUST
   not emit credential bytes or decrypted payloads in diagnostics.

## Verification and evidence

- The focused positive and negative signed-reader tests MUST pass, preserving
  the exact full-fingerprint success and suffix-collision rejection results.
- Duvet's implementation and test checks for PARSE-ENV-012 MUST pass. Duvet
  links provide requirement traceability; they do not prove the OpenPGP
  library's fingerprint or signature primitives.
- Full core native tests, Duvet trace, and required assurance CI MUST pass
  before this slice is recorded complete. Independent source review MUST
  confirm that authorization compares the complete caller-supplied primary
  fingerprint and does not authorize via key ID or issuer hints.
- No Verus proof is claimed for upstream certificate parsing, fingerprint
  calculation, or signature verification.

## Current evidence

- The official signed-chain test requires the exact 20-byte permitted
  fingerprint, successful snapshot reconstruction, and matching primary
  fingerprint in authorization evidence.
- The disallowed-signer test uses a different 20-byte value with the same
  final eight bytes as the fixture signer and requires `SignerNotAllowed`.
  It returns no snapshot on this error path.
- Focused positive and negative tests pass. `cargo test -p duplicity-core`
  passes all unit, integration, and doc tests. The PARSE-ENV-012 Duvet query,
  `cargo xtask trace`, formatting checks, and strict core Clippy pass. Clippy
  reports existing warnings in vendored `vest_lib`; the core crate passes
  `-D warnings`. Hosted assurance CI remains pending. Duvet traceability is
  not proof of the upstream fingerprint or signature primitives.

## Non-goals

This slice does not change signer policy or supported certificate versions,
test signing-subkey binding and embedded back-signature variants, establish
current certificate validity or Web-of-Trust status under
[PARSE-ENV-013](parsing-requirements.md#parse-env-013), add OpenPGP algorithms,
or claim complete encrypted-chain controller refinement. It does not require
live S3 credentials.

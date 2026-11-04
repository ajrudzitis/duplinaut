# PARSE-ENV-015 trusted-boundary evidence acceptance plan

Status: proposed before adding Duvet implementation and test links.

This is a bounded traceability slice for
[PARSE-ENV-015](parsing-requirements.md#parse-env-015). A section-scoped Duvet
query currently reports no implementation annotation. The repository already
has a pinned OpenPGP profile, an implementation/trust-boundary record, and
positive and negative tests using original local producer fixtures. No AWS
access or new parser behavior is required.

## Acceptance requirements

1. An implementation annotation MUST link the native OpenPGP adapter's
   documented trusted boundary to PARSE-ENV-015 and point reviewers to the
   pinned dependency identity and limitations in the
   [OpenPGP profile](openpgp-profile.md) and
   [implementation record](openpgp-implementation.md).
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

## Non-goals

This slice does not add algorithms, keys, packet variants, or compatibility
claims; test every supported crypto failure mode; prove the trusted upstream
cryptographic implementation; complete the reader controller refinement; or
require live S3 access.

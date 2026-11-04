# PARSE-ENV-013 offline signer-scope acceptance plan

Status: implementation, independent review, and full core verification pass;
hosted assurance CI remains pending.

This is a local traceability and regression-evidence slice for
[PARSE-ENV-013](parsing-requirements.md#parse-env-013). The scoped Duvet query
now traces the policy-summary construction and the official signed-chain
evidence test. The existing local fixture path supplies positive evidence
without AWS.

## Acceptance requirements

1. An implementation annotation MUST link the adapter's offline
   fingerprint-authorization boundary to PARSE-ENV-013. It MUST identify that
   authorization uses caller-supplied primary fingerprints and does not
   implement current certificate-validity, online revocation, user-identity,
   trust-database, or Web-of-Trust decisions.
2. A Duvet-linked integration test MUST read the official signed 3.0.7 chain
   with its supplied fixture credentials and permitted primary fingerprint,
   then require successful snapshot reconstruction against the independent
   source oracle.
3. The same test MUST inspect `AuthenticationPolicySummary` and certificate
   evidence: authorization is recorded as configured, the exact permitted
   primary fingerprint is retained, and available creation, expiry, and
   revocation observations are preserved. Evidence MUST NOT label the
   certificate currently valid, revoked under project policy, or Web-of-Trust
   trusted. Absence of such a project decision is explicit scope, not a
   favorable validity result.
4. The implementation and test annotations MUST quote and link PARSE-ENV-013.
   The fixture remains checked in and publicly labeled; the test MUST require
   no network, live AWS credentials, online key lookup, or fixture changes.

## Verification and evidence

- The focused signed-chain reader test MUST pass for each official backup date
  exercised by the existing fixture test and continue to match the independent
  source-content oracle.
- Duvet's implementation and test checks for PARSE-ENV-013 MUST pass. The
  trace records the stated scope but does not prove OpenPGP certificate or
  signature primitives.
- Full core native tests, Duvet trace, and required assurance CI MUST pass
  before the slice is recorded complete. Source review MUST confirm that
  expiry and revocation fields are observations and are not promoted into an
  unconfigured current-validity policy.
- No Verus proof is claimed for upstream certificate parsing, fingerprint
  calculation, or signature verification.

## Current evidence

- The official signed-chain test checks the configured policy summary, exact
  full permitted/authorized primary fingerprint, and certificate creation,
  expiry, and revocation observations. It makes no current-validity or
  Web-of-Trust claim.
- The scoped implementation link is on `policy_summary`, which records
  configured offline authorization inputs and policy bounds. Expired/revoked
  certificate fixtures and time-based acceptance remain open as stated in the
  non-goals.
- The focused signed-chain test, `cargo test -p duplicity-core`, targeted
  PARSE-ENV-013 Duvet query, `cargo xtask trace`, formatting checks, and strict
  core Clippy pass. Clippy emits existing warnings in vendored `vest_lib`,
  while `duplicity-core` passes `-D warnings`. Independent review passes;
  hosted assurance CI remains pending.

## Non-goals

This slice does not construct an expired/revoked certificate fixture or test
time-based acceptance decisions; that follow-up evidence remains open and the
normative policy is unchanged. It adds no certificate-validity or
Web-of-Trust policy, performs no online key retrieval, expands no signer/key
profiles, and does not complete the reader controller proof. It does not
require live S3 access.

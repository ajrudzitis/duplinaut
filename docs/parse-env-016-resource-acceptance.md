# PARSE-ENV-016 resource-admission acceptance plan

Status: implementation and test links are in place. Focused tests, the full
core suite, targeted Duvet query, full Duvet trace, formatting, and strict
Clippy pass locally on macOS. Independent review and hosted assurance remain
pending; keep the backlog item open until those gates pass.

This is a local OpenPGP resource-preflight acceptance slice for
[PARSE-ENV-016](parsing-requirements.md#parse-env-016). Native code and local
fixtures exercise the configured admission boundaries. All tests use reduced
local bounds and cheap admission inputs; no live AWS credentials are needed.

## Acceptance requirements

1. Implementation annotations MUST link the adapter's configured key,
   session-key packet, signature, nesting, and S2K-work admission paths to
   PARSE-ENV-016. The linked code MUST show that each limit is checked before
   its associated parsing, allocation, key-unlock, derivation, or recursive
   work.
2. Tests MUST demonstrate each configured limit at its boundary and one unit
   beyond it: aggregate exported-key bytes, exported certificate and subkey
   counts, permitted-signer fingerprint count, session-key packet and
   signature counts, wrapping/embedded depth, S2K work, and total attempted
   derivations. Tests MUST set small test-local limits
   appropriate to their tiny fixtures, rather than allocate objects near the
   production defaults (including the 16 MiB key-byte cap). Rejection MUST
   return the corresponding `CryptoError::Limit` and MUST NOT produce a
   decoded object.
3. At least one precedence test MUST provide malformed or truncated packet
   contents behind an exhausted session-packet/signature admission and require
   the configured `Limit` result before body parsing. The depth test MUST
   use a reduced local maximum, accept that exact nesting depth, and reject the
   next level. S2K admission tests MUST separately exercise a SKESK wrapper
   and a protected secret-key export. Each MUST use a discrete valid encoded
   S2K count, accept its exact decoded work bound, and reject a bound one unit
   below it before derivation. Use cheap generated fixtures and admission-only
   inputs so the tests never run an expensive high-work S2K just to test the
   boundary. A separate derivation-edge test MUST use a generated SKESK with a
   cheap valid S2K configuration and prove that a zero derivation budget
   rejects before invoking the password KDF.
4. Duvet test annotations MUST link the boundary and precedence cases to
   PARSE-ENV-016. Verify the shared derivation counter independently in both
   credential paths: a message-passphrase/SKESK attempt and a secret-key/PKESK
   candidate must each charge the same `Work.derivations` budget and reject at
   its configured edge. The plan does not require a mixed SKESK-plus-PKESK
   envelope to prove cross-path aggregation. Use local generated packet bytes
   and checked-in, publicly labeled test-key exports; do not invoke external
   crypto tools or alter producer fixtures.

## Verification and evidence

- Focused adapter tests MUST assert exact `CryptoError::Limit` variants at
  each bound and show that over-limit input cannot return plaintext. Boundary
  fixtures MUST use reduced local byte/count/depth limits, including a
  separately reduced permitted-signer count, and valid discrete encoded S2K
  counts for both SKESK wrappers and protected secret-key exports. Use a cheap
  generated SKESK for derivation rejection; avoid large allocations and
  high-work KDF execution.
- Tests MUST establish the derivation counter's exact edge and its use in both
  the SKESK message-password path and PKESK secret-key-candidate path. Source
  review MUST confirm those paths charge the same counter; separate-path tests
  do not claim to exercise a combined envelope.
- Duvet's implementation and test checks for PARSE-ENV-016 MUST pass. The
  links establish traceability, not correctness of upstream cryptographic
  primitives.
- Full core native tests, Duvet trace, and required assurance CI MUST pass
  before this slice is recorded complete. Independent review MUST verify that
  checks precede the work they bound and that per-item limits do not bypass
  aggregate accounting.
- No Verus proof is claimed for the native OpenPGP controller or upstream key,
  packet, or S2K implementations.

## Current local evidence

- `cargo test -p duplicity-core --lib resource_acceptance_tests`: 4 passed.
- `cargo test -p duplicity-core`: passed, including official encrypted-chain
  source-oracle coverage.
- `cargo xtask trace-update`, `cargo xtask trace`, and the targeted
  `duvet query --check implementation,test --section
  docs/parsing-requirements.md#parse-env-016`: passed.
- `cargo fmt --all -- --check`,
  `cargo clippy -p duplicity-core --all-targets -- -D warnings`, and
  `git diff --check`: passed.
- The generated fixtures exercise only admission behavior with small S2K
  counts; this does not establish limits on total process memory or verify
  cryptographic primitives.

## Non-goals

This slice does not broaden the OpenPGP key, algorithm, or packet profile;
change configured default limits; prove cryptographic primitives or global
peak-memory use; complete nested-message/controller refinement; or test live
S3. Stored-object, authenticated-plaintext, decoded-output, and compression
bounds remain governed by their separate requirements.

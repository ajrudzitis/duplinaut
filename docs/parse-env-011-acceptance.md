# PARSE-ENV-011 compressed-stream acceptance plan

Status: proposed before implementation and Duvet annotation changes.

This is a bounded core-reader test milestone for
[PARSE-ENV-011](parsing-requirements.md#parse-env-011), with negative results
also supporting [PARSE-ENV-014](parsing-requirements.md#parse-env-014). The
section-scoped Duvet query currently reports no implementation annotation for
PARSE-ENV-011. This plan targets exact completion of supported compression
streams and rejection of trailing data; it does not claim acceptance of every
OpenPGP message, signer, or nested-layer profile covered by the broader
requirement.

## Acceptance requirements

1. Implementation annotations MUST connect the `content` and compression
   parser path to PARSE-ENV-011. Tests MUST link their exact normative text to
   PARSE-ENV-011 and error assertions to PARSE-ENV-014.
2. Deterministic packet fixtures MUST demonstrate successful exact-end
   decoding for uncompressed content and each supported compression mode:
   ZIP/raw DEFLATE, ZLIB, and BZip2. Each positive case MUST return the exact
   literal bytes and consume the complete framed input.
3. For each compressed mode, a valid stream followed by one extra byte MUST be
   rejected as malformed. A literal-data packet followed by an unexpected
   packet MUST also be rejected. For each compressed mode, truncating the
   otherwise valid stream MUST be rejected as malformed. No failing case may
   return a partial `Content` value.
4. BZip2 decoder errors caused by malformed or truncated compressed data MUST
   be normalized to `CryptoError::Malformed`, rather than leaking the generic
   reader's `AuthenticationFailed` mapping. An expansion-limit rejection MUST
   remain `CryptoError::Limit(Limit::Expanded)`. This mapping change MUST be
   local to BZip2 decompression; outer SEIPDv1 authentication errors retain
   their existing classification.
5. The test helpers MUST generate the packet/compression bytes locally and
   deterministically. They MUST NOT require network access, live AWS
   credentials, subprocess invocation of OpenPGP/compression utilities, or
   modification of checked-in producer fixtures.

## Verification and evidence

- Focused OpenPGP content-parser tests MUST pass, with exact expected bytes
  and error variants for positive, trailing-byte, and truncated-stream cases
  in each compressed mode. BZip2 malformed/truncated cases MUST assert
  `CryptoError::Malformed`, and its expanded-output boundary MUST assert the
  unchanged `CryptoError::Limit(Limit::Expanded)` result.
- Duvet's implementation and test checks for PARSE-ENV-011 and test coverage
  for PARSE-ENV-014 MUST pass. Duvet links provide traceability; they do not
  prove every profile variation in the full normative requirement.
- Full core native tests and the required assurance CI MUST pass before the
  README marks this slice complete. No Verus proof is claimed for upstream
  OpenPGP, DEFLATE, ZLIB, or BZip2 implementations; code review must verify
  that adapter control flow rejects unconsumed bytes and returns no partial
  plaintext.
- The PARSE-ENV-010 official-MDC positive and negative controls remain the
  separate evidence for SEIPDv1 authentication. This plan does not substitute
  compression completion for successful outer-message authentication.

## Non-goals

This slice does not broaden supported OpenPGP algorithms or key profiles,
change recursion/resource bounds, prove authenticated-message cryptography,
complete nested compression/signature controller refinement, validate
signer authorization, or establish complete-reader snapshot publication. The
BZip2 error normalization MUST NOT alter SEIPDv1 authentication classification,
configured expansion-limit errors, or cancellation/failure behavior. It does
not cover live S3.

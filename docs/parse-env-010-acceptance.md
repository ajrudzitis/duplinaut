# PARSE-ENV-010 test acceptance plan

Status: accepted. The focused boundary, adapter, and official MDC-mutation tests
pass; independent review, Duvet traceability, and hosted Linux assurance are
complete.

This plan closes the test-coverage gap for
[PARSE-ENV-010](parsing-requirements.md#parse-env-010). It does not change the
normative requirement or claim formal verification of cryptographic
primitives.

Acceptance evidence:

- `read_ciphertext` accepts the exact stored-byte bound and returns
  `Limit::Stored` one byte over after consuming exactly one probe byte.
- `decode` returns `Limit::Stored` for malformed input above the configured
  bound, before packet parsing.
- The correct fixture credential accepts the unmodified official encrypted
  manifest and returns its snapshot and authentication evidence. After changing
  only its final MDC byte and updating fixture identity metadata, the same
  reader rejects it with `AuthenticationFailed` and returns no snapshot or
  evidence.
- Focused Rust tests, independent review, the PARSE-ENV-010 Duvet query, and
  `cargo xtask trace` pass. No Verus claim covers cryptographic primitives.
- Full core tests and hosted Linux assurance pass in
  [run 41799721617](https://github.com/ajrudzitis/duplinaut/actions/runs/41799721617).
  That full assurance run is recorded by the subsequent ENV-011 acceptance and
  includes these ENV-010 tests from its ancestor source commit.

## Acceptance requirements

1. A boundary test MUST show that `read_ciphertext` accepts exactly
   `stored_bytes` and rejects one byte over that limit as `Limit::Stored`.
   Use a counting reader and require that overflow detection consumes at most
   one byte past the configured bound and never returns an over-limit buffer.
   This test MUST be Duvet-linked to PARSE-ENV-010.
2. An adapter test MUST show that `decode` rejects an input slice larger than
   `stored_bytes` as `Limit::Stored` before packet parsing or cryptographic
   work. Supply malformed packet bytes above the configured bound and require
   `Limit::Stored`, rather than a packet-parse error. This test MUST also be
   Duvet-linked to PARSE-ENV-010.
3. An integrity test MUST use an official encrypted manifest and the fixture's
   correct credentials. Before mutation, it MUST accept the unmodified
   official manifest with those same credentials as a positive control. It
   then MUST mutate only the SEIPDv1 MDC byte while preserving stored length
   and packet framing, and require authentication failure. The reader MUST
   return no manifest, snapshot, or authentication evidence for the altered
   object. The test MUST be Duvet-linked to PARSE-ENV-010; it may add that link
   to the existing authenticated-manifest mutation test if its assertions meet
   this plan.

## Verification and evidence

- The focused Rust tests MUST pass, including exact-boundary and one-byte
  overflow cases and the official MDC-mutation case.
- Duvet's implementation and test checks for `PARSE-ENV-010` MUST pass with no
  missing test coverage.
- Review MUST confirm that the over-limit path does not construct a returned
  ciphertext buffer and that failed authentication does not yield plaintext
  to manifest or tar parsing.
- Native tests and source review establish the adapter contract only. They do
  not prove the correctness or side-channel resistance of the pinned OpenPGP
  library. No Verus claim is made for cryptographic implementation details.

## Non-goals

This slice does not add OpenPGP algorithms, key types, packet profiles, or
resource limits; validate data-volume digest ordering under PARSE-ENV-018;
prove complete snapshot publication under PARSE-ENV-024; or establish
cryptographic primitive correctness. Broader crypto compatibility and reader
controller proof work remain separate.

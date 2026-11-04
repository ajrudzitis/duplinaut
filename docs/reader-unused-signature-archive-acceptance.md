# Unused OpenPGP signature archive acceptance plan

Status: proposed; no implementation or backlog status changes are included.

This local reader-integration test checks one bounded catalog behavior: a
malformed OpenPGP signature archive that is not required to reconstruct the
selected snapshot is not opened as part of a signer-authorized read. It
complements the [OpenPGP reader integration profile](openpgp-reader-integration.md)
and its chain-wide policy contract in
[PARSE-ENV-019](parsing-requirements.md#parse-env-019). PARSE-ENV-019 applies
authorization to required manifests and data volumes and explicitly leaves
unused catalog artifacts outside its scope. This case MUST NOT be presented as
coverage of every irrelevant artifact type or as an expansion of that
requirement.

## Acceptance requirements

1. Load the checked-in official duplicity 3.0.7 `signed/` fixture profile
   into the in-memory test source. Replace the bytes of only
   `duplicity-full-signatures.20231114T221320Z.sigtar.gpg` with same-length
   malformed bytes before catalog discovery. Preserve its key and size, and
   recompute its synthetic `ObjectIdentity` from the replacement bytes so the
   source's identity checks remain valid. Do not modify checked-in producer
   fixtures.
2. Read the official signed chain through its final snapshot date using the
   unchanged valid recipient secret export, fixture passphrase, signer
   certificate, exact permitted primary fingerprint, and
   `require_signature: true`. The test MUST receive a successful snapshot and
   exactly six authentication records, one for each required manifest and data
   volume. Every record MUST report authorized-signer evidence for the exact
   permitted primary fingerprint. This is the positive control that the
   malformed auxiliary archive does not affect a valid required chain.
3. Require exactly six source opens: the three required manifests and their
   three required data volumes. The malformed full-signatures archive MUST
   not be opened. A successful read with malformed archive bytes and this
   exact count demonstrates only that this unused archive is not consumed by
   this reconstruction path.
4. Add Duvet test annotations for the bounded integration context, linking to
   PARSE-ENV-019 and PARSE-ENV-024. The annotations and test description MUST
   state that the archive is unused and that this test does not establish
   handling for all catalog artifacts, signature archives, or chain-policy
   combinations.

## Verification and evidence

- Run the focused test and `cargo test -p duplicity-core`; both MUST pass using
  checked-in fixture data only.
- Run the targeted Duvet implementation/test query for the linked
  requirements, `cargo xtask trace-update`, and `cargo xtask trace`. Duvet
  provides traceability only; the bounded test does not prove complete reader
  controller behavior.
- Run formatting and strict core Clippy checks, then required assurance CI
  (`cargo xtask ci`). Record hosted Linux CI evidence before marking this
  acceptance complete.
- Independent review MUST confirm that only the synthetic archive bytes
  change, the catalog object name/size remain stable, the valid credentials and
  signer fingerprint match the official signed-chain positive control, and
  the open count includes all required objects but excludes the auxiliary
  archive.

## Scope limits

This case covers only the unused official full-signatures archive in the
selected 3.0.7 signed-chain read. It does not cover unsigned encrypted data
volumes, arbitrary unused manifests or volumes, other signature archives,
irrelevant object-name classes, other producer versions, or broad mixed-envelope
selection. Complete reader-controller refinement remains open.

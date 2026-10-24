# Native OpenPGP reader composition

Adopted integration profile, independently reviewed before pipeline implementation. The concrete reader composition is now implemented in a scoped native checkpoint, pending independent review. It composes
[PARSE-ENV-002..016](parsing-requirements.md#parse-env-002), the independently
specified [OpenPGP recovery adapter](openpgp-profile.md), and the existing
[reader](reader-orchestration.md)/[multipart](multipart-assembly.md) contracts.
PARSE-ENV-017..024 make composition boundaries explicit. Native adapter acceptance,
reader integration tests and complete controller refinement remain separate.
See the acceptance checklist below for the evidence still required before this profile is complete.

## Reader API and policy scope

Keep the existing no-credential `Catalog::read_snapshot` plain/gzip path. Add an
explicit read API taking a borrowed crypto context, for example
`read_snapshot_with_crypto(anchor, target, cancel, &context)`. The context contains
borrowed `Credentials`, explicit `CryptoLimits` and receipt-retention limits. A
single private controller can serve both APIs without duplicating reconstruction.
An OpenPGP-required object reached without the crypto context returns an explicit
unsupported/credentials-required error; metadata discovery never attempts decrypt.
Names, catalog rows and source object identities do not contain credentials or
promise decryptability. PKCS7 and out-of-profile OpenPGP remain unsupported.

The context stays immutable for one complete read. Construct per-object adapter
`Credentials` values by copying references/flags only: message password slice,
borrowed `SecretExport` slice, signer exports and permitted fingerprints. Do not
clone `Password`, exported secret bytes or credentials into catalog/snapshot/
receipt state, and do not derive Debug for a secret-bearing context. Caller owns
credential acquisition, lifetime and worker setup. The adapter still temporarily
parses/unlocks keys and uses trusted crypto buffers; this integration promises no
zero-internal-copy or secure-erasure property.

Signer authorization is enabled by `require_signature` **or** any permitted
primary fingerprint. Once enabled it applies to **all required manifests and data
volumes** from full anchor through selected incremental target. An unsigned
OpenPGP object or a required plain/gzip object fails rather than downgrading. An
empty allowed list with required verification also fails. Different required
objects may use different permitted primary signers; this is an allowlist policy,
not an implicit same-signer constraint. Extra unused objects/signature archives
in the catalog are not decoded or required to be signed by this reconstruction.
In integrity-only mode plain/gzip and OpenPGP can compose, with the authentication
status of each required object made explicit. No credential or policy is inferred
from suffixes, short issuer IDs, keys discovered online or backup dates.

## Object processing order

| Required object | Processing order before accepting its content |
| --- | --- |
| Plain/gzip manifest | Reject if global signer policy requires authorization; otherwise original source identity and exact size/EOF; native envelope bounds; manifest parse |
| OpenPGP manifest | Ciphertext read bounds, original identity and exact size/EOF; complete native adapter MDC/framing/compression/configured-signature checks; final literal manifest bound; manifest parse |
| Plain/gzip data volume | Reject if global signer policy requires authorization; otherwise original identity and exact size/EOF; every supported manifest stored-byte digest; native envelope/container/boundary/part checks |
| OpenPGP data volume | Ciphertext read bounds, original identity and exact size/EOF; **every supported manifest digest over ciphertext before decryption**; complete adapter checks; encrypted tar footer/top-off; strict boundaries and multipart completion |

Manifests have no enclosing duplicity volume digest. Their source identity/MDC
checks cannot be replaced by an invented plaintext hash. SHA1/MD5 are compatibility
checks only: unsigned manifests and their volume digests give no signer identity.
MDC protects an encrypted message; it neither proves a signing certificate nor
prevents replacement by another well-formed message encrypted to known recipients.
Configured signatures verify the exact binary literal body. They do not attest
catalog key spelling, filename-derived backup dates, intended collection, present
certificate validity, freshness or Web-of-Trust identity.

Before opening/materializing a selected encrypted object, validate declared size
and platform conversion against the minimum of source/reader stored limits, the
role's envelope stored limit and `CryptoLimits.stored_bytes`. Source CheckedReader
still requires real EOF, exact size, conditional identity and cancellation.
Do not first allocate an 8 GiB general envelope buffer and only later discover
that the crypto bound is 256 MiB. Volume hash failure must not invoke the decoder.
Source-read and crypto errors remain distinct redacted result variants; a wrong
symmetric password can still be indistinguishable from an MDC failure.

For encrypted manifests use the minimum of crypto decoded limit, reader metadata
envelope decoded limit and `decoded_manifest_bytes` (default 32 MiB) for the final
literal. Keep authenticated packet bytes, cumulative compression expansion,
packet-body/depth/count, key and S2K-work bounds independent. Do not shrink their
meaning to the final manifest size or raise them implicitly after a rejection.
For volumes also enforce the configured assembly envelope/tar bounds on final
literal/container interpretation. Default CheckFirst/stored crypto limits remain
256 MiB and final literal/expansion default 8 GiB. These bounds describe different
coexisting buffers and work; their minimums do not form a peak-heap/latency proof.

## Private assembly boundary

Retain public `push_stored_volume` for raw/plain/gzip callers. It still computes
its own required stored digests and never treats decrypted bytes as an OpenPGP
object. Add a crate-private encrypted stored-volume entry taking original bytes,
borrowed crypto policy and limits. That entry performs the stored digest check
itself, invokes the concrete adapter itself, and then passes its owned literal
through a private container/assembly routine. No public `push_decoded`, generic
untrusted decoder trait, `verified: bool` or externally constructed DecodedObject/
receipt can furnish successful assembly authentication.

The private `OpenPgpAdmission::verify_stored_digest` native wrapper is the only
Rust-visible producer of `DigestPermit`. Its body invokes
`verify_stored_digests` and only then calls the Verus cursor transition. This is
a source-reviewed native boundary: Verus proves the cursor transition contract,
not the wrapper's factual call order, the SHA1/MD5 implementations or their
comparison with manifest declarations. An `external_body` bridge was rejected
because the repository's no-cheating audit intentionally prohibits project
algorithm external bodies. SHA1 and MD5 remain format-compatibility checks, not
authenticity claims.

`openpgp::DecodedObject` and its fields are crate-private. Its construction alone
is still not authority at the pipeline boundary: the private assembler receives it
only directly from the checked adapter call, never from caller input. Public API
exposes borrowed policy inputs and completed observations, not a plaintext-plus-
verified flag, adapter result, or capability to mark arbitrary plaintext checked.

Parse encrypted literals with
`Termination::Encrypted { stored_ciphertext: original_stored, nonfinal }`.
`nonfinal` comes from the current ordinal's position in the complete normalized
manifest volume list, not raw appended-record length, an untrusted parameter or
tar EOF. Enforce the exact reviewed 10240-byte record zero footer and any allowed
nonfinal top-off equal to the original ciphertext prefix. Final volumes forbid
top-off. Retain original ciphertext until these comparisons complete. Never
reuse `Termination::Data` or compare top-off with decrypted literal bytes.

Normal StartingPath/EndingPath, part sequence/65536-byte final-short rules, empty
volume markers and complete normalized set scanning stay unchanged. Encrypted
queued-replay StartingPath exceptions remain held even when raw duplicate-volume
records are present; they do not inherit raw/gzip no-footer compatibility. Return
an explicit unsupported boundary profile. Do not omit an encrypted volume using
raw range hints or treat successful MDC/footer alone as complete logical content.

## Bounded immutable evidence

Return authentication evidence attached to the successful immutable snapshot;
do not return a successful partial-chain authentication receipt when a later set
fails. Each required manifest/volume has a record tied to its original source
identity, set/role/ordinal and envelope. Use owned bounded key/identity fields or
an immutable retained owner; raw catalog indices alone cannot name objects after
their catalog is dropped/replaced. No password/key/session/plaintext is retained.

Per-object distinctions are plain/gzip compatibility checks, complete OpenPGP
MDC with signer policy unconfigured, and complete OpenPGP MDC plus configured
fingerprint authorization. `authorized_signer = None` means authorization was not
configured; it does **not** prove the object was unsigned. Configured success means
every presented document signature passed under a supplied permitted certificate.
The private adapter currently returns the first authorized full primary/component
fingerprint as a representative, not an exhaustive signer set. Keep that scope
explicit rather than inventing complete per-signature receipts.

The adapter returns public signer-export certificate metadata: primary fingerprint,
creation time and ordered signature observations with creation, optional key/
signature expiry durations and revocation flags. Intern each exact metadata version
once per read and reference the pool from object receipts. Equality includes all
fields, not only fingerprint. Preserve distinct versions or reject their bound;
never merge/drop a revocation/expiry observation. Receipt observations are not a
current/revocation/WOT policy result, and missing metadata is not evidence of no
expiry/revocation. Recipient secret-key certificate validity is not established by
these signer-export observations. The caller still has its exported certificates
for any richer independent assessment.

Initial configurable per-read evidence defaults are 1,000,000 object records,
32 MiB retained metadata, 64 distinct certificate versions and 16 million equality/
lookup comparisons. Precharge exact/upper-bounded Vec capacities, source key/token
bytes, per-object pool-index storage and certificate signature vectors before
retaining them; charge comparison work before lookups. Move unique adapter metadata
rather than deep-cloning it per object, and drop duplicate transient records after
comparison. Per-object adapter temporary metadata is already independently bounded;
it does not count as a claim of globally bounded total heap alongside snapshot,
network and plaintext buffers. Certificate parsing/unlocking may repeat per object
under the adapter's bounded work; caching it requires a separately reviewed secret
lifetime/trust contract and is not required for this slice.

Evidence getters include a redacted policy summary with the configured fingerprint
allowlist and crypto/evidence bounds; they expose no credentials. The summary makes
offline-policy scope explicit and does not report current certificate validity.
Do not format Password, raw exports, upstream secret-error strings or decrypted
content into source errors, TUI status, restore receipts, logs or test failure
messages. Frontend credential collection is a separate specification/implementation
step; this pipeline need not provide a password widget or OS keyring integration.

## Completion, cancellation and acceptance

Keep all decoded objects, assembled entries, evolving states and evidence provisional
through the selected full/incremental sequence. Scan every normalized required
volume and verify original identities/hashes even for restart-rewritten manifests.
Only complete chain/container/multipart/state success plus final cancellation check
returns a ValidatedSnapshot with evidence. On a late failure drop every provisional
result; do not hand a partial state/evidence stream to the restoration API.
Cancellation is observed around source reads and crypto calls and between objects/
sets. Derivation, hashing, parsing and decompression can remain bounded synchronous
work without internal interruption or a fixed cancellation-latency guarantee.
The reader does not publish filesystem plaintext; later backend/TUI contracts still
control explicit confirmation, staging, cancellation and uncertain publication.

- [x] Independently accept this composition profile.
- [x] Independently accept the prerequisite native adapter checkpoint.
- [x] Implement concrete borrowed crypto read context and private stored-volume path.
- [x] Read all three dates of each original symmetric/public-key/signed corpus through
      ObjectSource and compare file contents against independent source SHA256 oracles.
      Path/type/mode/mtime comparisons remain incomplete in this reader integration test.
- [x] Test wrong symmetric credentials and a same-size authenticated-manifest
      ciphertext mutation through public `Catalog::read_snapshot_with_crypto`
      calls; both return typed native crypto errors and no snapshot. The mutated
      manifest is altered before discovery and keeps its name/size, so no volume
      digest or malformed listing can mask native MDC rejection. Credentials are
      borrowed and the wrong test secret is absent from the formatted error.
      Independent review and the exact-source full assurance gate passed.
- [ ] Test wrong signer authorization, unsupported algorithms, and additional
      reader-boundary crypto cases, yielding no completed snapshot.
- [x] Exercise source identity change, truncation and excess through the public
      `Catalog::read_snapshot_with_crypto`/`ObjectSource` path; no snapshot is returned.
- [x] Prove digest-first behavior by a negative object that would otherwise decrypt;
- [x] Require signer policy for a plain required object, rejecting downgrade.
- [x] Test integrity-only versus authorized evidence semantics, evidence object cap,
      metadata-byte cap, certificate-version cap and comparison-work cap.
- [x] Reject a plain incremental manifest after a signed encrypted full set under the
      same chain-wide signer policy. An unsigned OpenPGP incremental and irrelevant
      catalog artifacts remain pending.
- [x] Test encrypted record footer and normalized final/nonfinal status with original
      full/incremental official encrypted volumes; directly reject encrypted queued
      restart StartingPath compatibility.
- [x] At the private multipart boundary, start from an official encrypted full-volume
      tar stream, append a deliberately incorrect nonfinal top-off, encrypt it as a
      fresh SEIPDv1 object, verify native decryption/MDC and decoded-byte equality,
      regenerate the in-memory manifest record's supported stored-ciphertext
      digests, and require `Termination` rejection without admission advancement.
      This exercises authenticated encrypted termination and top-off comparison;
      it does not exercise re-encrypting the manifest or the public catalog/source
      path. Independent review accepted the test and the exact-source full
      assurance gate passed.
- [x] Exercise cancellation after stored-digest verification and after successful
      native OpenPGP decoding through the actual private multipart wrapper. A
      test-only, thread-local checkpoint requests cancellation and is absent from
      non-test builds. Wrong credentials at the first checkpoint establish that
      decryption was not reached; official valid ciphertext at the second establishes
      that cancellation prevents tar validation and admission commit.
- [x] Exercise those same two internal checkpoints through public
      `Catalog::read_snapshot_with_crypto` calls using official symmetric fixture
      ciphertext. Both return the public `Source(Cancelled)` outcome and no snapshot.
- [x] Independently review the scoped lifecycle/evidence charging/dedup behavior; focused and complete gates pass on the adopted source.
- [ ] Retain controller refinement, additional cipher/recipient/signing-subkey cases,
      native encrypted multipart producer coverage and broader historical encryption
      as explicit pending assurance work; no count/annotation confers completion.

## Primary format and reference evidence

The duplicity manual and RFC/rPGP primary references already recorded in
[the capability profile](openpgp-profile.md) govern encryption and authorization.
Where documentation does not define stored-hash ordering, immutable SRC-MAIN
`write_multivol` writes through GPGWriteFile before `vi.set_hash(SHA1, get_hash(tdp))`;
SRC-HASH opens that stored temp path in binary mode. Their pinned identities are
in [the requirements sources](parsing-requirements.md#sources-and-compatibility-profile).

A separate official GnuPG 2.4.9 reference-only check decrypted nine original encrypted manifests and nine corresponding
volumes from symmetric, public-key and signed chains. Every declared
SHA1 equals its original stored ciphertext SHA1 and differs from its decoded
literal SHA1. The isolated [reference receipt](evidence/openpgp-reader/official-ciphertext-digest-check.json) records
object SHA256s and those equalities; [reference script](evidence/openpgp-reader/check_ciphertext_digests.py) repeats the check
with intentionally public fixture credentials and a private disposable GnuPG home.
Sandbox agent-start restrictions required running this reference outside the
sandbox; it modifies no user keyring and is never reader runtime. These checks
establish official hash semantics, not pipeline or crypto formal acceptance.

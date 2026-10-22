# Native OpenPGP implementation progress

The core contains crate-private `openpgp::read_ciphertext` and `openpgp::decode` for the reviewed [SEIPDv1 recovery profile](openpgp-profile.md). The module, decoder and decoded result are unavailable to external callers: they cannot construct a plaintext/authentication assertion to bypass stored-source or digest validation. Only the future private composed reader path may connect this adapter to production object handling. The adapter uses native Rust libraries and invokes neither GnuPG nor duplicity. Pipeline and frontend credential integration remain separate pending work.

| Task | Implementation | Native evidence | Independent review / proof |
| --- | --- | --- | --- |
| Bounded binary ciphertext and armored/binary key exports | Implemented | Bounds and original-key tests | Review pending; controller proof pending |
| SKESKv4 symmetric / PKESKv3 recipient recovery, AES payloads | Implemented | All 27 original official crypto objects | Original AES256/cv25519 cases tested; additional RSA/AES variants pending |
| Complete SEIPDv1 MDC before plaintext parsing/release | Implemented | Mutation/truncation and authenticated-malformed-payload tests | Crypto primitives trusted; controller proof pending |
| Strict framing, single literal body, bounded compression | Implemented | Raw/ZIP/ZLIB/BZIP2, trailing/truncated streams, partial lengths, depth/work limits | Native controller proof pending |
| Primary certificate, subkey and signing back-bindings | Implemented | Original exports, corrupted/absent binding and protected-secret negatives | Native rPGP binding verification trusted; signing-subkey fixture coverage pending |
| Required full-fingerprint document signature authorization | Implemented | Original signed objects, unsigned/wrong fingerprint/changed signature negatives | Offline policy only; review/proof pending |
| Exact outer/embedded signature MPI tails | Implemented | RSA/legacy-Ed framing boundaries and authenticated valid-back-signature plus appended-byte negatives | Independent controller check; native crypto trust remains |
| Private decoder/result capability boundary | Implemented | Existing behavior tests moved into crate test modules; external-access compile-fail doctest | Composed source/digest/crypto pipeline review remains pending |
| Encrypted source-content restoration oracle | Implemented in tests | Independent test tar reader and source-recipe SHA256 maps for all nine original crypto snapshots | Production pipeline integration pending |

This table reports native implementation evidence, not formal acceptance. The new controller is ordinary Rust and has no Verus refinement proof yet. Existing parser proofs do not establish its behavior or crypto correctness. The complete reader milestone remains open.

## API and policy

The private `decode` takes the original stored binary OpenPGP bytes, borrowed `Credentials`, and explicit `CryptoLimits`. It is not a public reader operation and does not independently establish original-source identity or manifest digest checks. Its `DecodedObject` and fields are crate-private; there is no public decoded-object constructor or raw-decoder callback. `SecretExport` accepts optional passwords for protected secret-key exports; `message_passwords` supplies symmetric passwords. The `Password` type comes from pinned rPGP. The adapter neither discovers credentials nor logs passwords, keys, decrypted bodies or upstream error strings. A localized dead-code allowance records that this private native adapter awaits composed reader integration; it does not suppress any correctness or proof check.

Setting `require_signature = true` requires cryptographic authorization even if `permitted_signers` is empty. Supplying a nonempty permitted list also enables authorization. Each entry is an entire 20-byte v4 primary fingerprint. Key-ID and issuer hints do not authorize a signer. Every presented document signature must verify under a permitted supplied certificate; a signing subkey additionally requires a verified binding, authenticated signing key flag and verified primary-key back-signature. The receipt identifies the first authorized primary and actual component fingerprints. An empty permitted list with required verification rejects a signed object; an unsigned object yields `Unsigned`.

Certificate self/binding verification runs before private-key or document-signature use. Composition must preserve every preflighted certificate packet: unsigned components or unexpected signatures silently dropped by rPGP composition are rejected. Certificate creation, signature creation/expiry, key expiry and revocation metadata are exposed for a caller's separate validity policy. No current-time, revocation-acceptance, certification identity or Web-of-Trust decision is made.

The whole object is authenticated in `Seipdv1ReadMode::CheckFirst`, with its explicit maximum, before the controller parses any decrypted packet. It then requires exactly one literal body, consumes the complete enclosing packet sequence and each compressed stream, enforces all limits, and completes configured signature checks before returning plaintext. There is no streaming fallback. Literal filenames are ignored. Multiple successful session unwraps must agree. Missing passwords/keys, failed unwraps, MDC failures, unsupported profiles, malformed data, unauthorized/invalid signatures and resource limits produce distinct redacted errors. A wrong symmetric password can also manifest as an MDC failure; those causes cannot always be distinguished.

Signature preflight recursively consumes exactly one MPI for RSA or two for legacy Ed25519 after the hashed/unhashed fields and two hash-prefix bytes. Extra bytes or truncated material are malformed, including inside embedded signatures. A deterministic native test creates a real legacy-Ed signing subkey and verifies its back-signature, then embeds it in the unhashed area of an authenticated document signature: the valid baseline succeeds, and an appended byte fails. rPGP itself also rejects that concrete appended-byte case through its unconditional generic subpacket reserialization-length guard. Its inner embedded-signature parser lacks an exposed EOF position, but the test does **not** demonstrate an accepted trailing-byte vulnerability. The explicit project check establishes exact supported framing independently. This test-generated key is not independent official signing-subkey compatibility evidence; that milestone remains pending.

RSA modulus policy also checks that the highest set bit of the actual encoded modulus agrees with its declared bit length before applying the 2048..8192 bound. The primitive MPI parser strips leading zero bytes, so trusting only the declaration would not establish the actual key-size policy. Wire-policy tests reject both a 2047-bit value labeled 2048 and heavily padded weak material; they do not substitute for independent RSA encryption/signature compatibility fixtures.

## Bounds

| Limit | Default | Scope |
| --- | --- | --- |
| `stored_bytes` | 256 MiB | Whole binary ciphertext, before framing/key work; `read_ciphertext` probes actual EOF/one-byte overflow |
| `authenticated_bytes` | 256 MiB | Encrypted SEIPDv1 body passed to CheckFirst and authenticated decoded packet bytes |
| `decoded_bytes` | 8 GiB | Final literal body |
| `expanded_bytes` | 8 GiB | Cumulative output of every compression layer, including raw compression and intermediate layers |
| `packet_body_bytes` | 8 GiB | Each reassembled packet body, checked before growth |
| `key_bytes` | 16 MiB | Aggregate supplied armored/binary exports before key parsing; each decoded export also bounded |
| `certificates` / `subkeys_per_certificate` | 64 / 64 | Supplied certificates and each certificate's retained components |
| `session_packets` / `signatures` | 64 / 64 | Session wrappers; total certificate/document/embedded signatures |
| `packets` | 4096 | Aggregate outer/content/key packets and signature subpackets |
| `depth` | 8 | Combined compression/signature wrappers; embedded signature recursion also bounded |
| `s2k_bytes` / `derivations` | 65,011,712 / 64 | Per iterated-S2K expansion and total attempted password/private-key unwraps, including unprotected RSA/ECDH attempts |

The workspace's supported 64-bit targets represent these defaults. Bounds are logical byte/work limits, not a claim that peak resident memory equals one listed limit: ciphertext, authenticated bytes, native crypto buffers and reassembled/intermediate packet copies can coexist. Pinned library allocations, codec buffers, allocator behavior and cryptographic execution remain trusted. Only project buffer growth uses fallible reservations. Credentials and source I/O/cancellation are supplied by callers; this adapter does not promise that a blocking cryptographic derivation is interruptible.

## Dependencies and armor finding

Production additions are `pgp = "=0.21.0"` with default features disabled and `bzip2` enabled, plus `bzip2 = "=0.6.1"`. Existing flate2 supplies ZIP/ZLIB primitives. `rand = "=0.8.6"` is test-only for deterministic authenticated negative fixtures. The published pgp archive checksum, source identity and unresolved upstream RSA timing concern are recorded in the [reviewed profile](openpgp-profile.md). The entire resolved primitive dependency graph remains a trusted boundary, governed by Cargo.lock and independent review. No extra algorithms reachable inside rPGP expand the adapter's explicit wire policy.

Armor CRC24 checking remains disabled, matching [RFC9580 section 6.1](https://www.rfc-editor.org/rfc/rfc9580.html#section-6.1) and rPGP's default. CRC is not authentication. Independent test code uses manual base64 and the standard CRC24 initial value/polynomial, checks the `123456789` vector, matches both original key footers, and confirms original export SHA256 identities against `provenance.json`. For these valid inputs, rPGP 0.21.0's optional checker incorrectly reports the untouched initial CRC `0xb704ce`: [pinned `armor/reader.rs::read_body`](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/src/armor/reader.rs#L546) updates a copied local hasher and does not store it back in `self.crc`. The adapter retains key binding verification, protected-secret integrity, complete MDC and required document signatures. No production library patch or checksum-based authentication claim is introduced.

## Repeatable native checks

Run with the workspace's pinned Rust toolchain:

```text
cargo test -p duplicity-core --lib openpgp::
cargo test -p duplicity-core --doc
cargo clippy -p duplicity-core --all-targets --no-deps -- -D warnings
```

The 14 native tests live under `src/openpgp/` (plus local controller tests), preserving official-producer integration coverage without publishing a decoder solely for external test access. The original-fixture test decrypts 27 untouched files across symmetric/public-key/signed full-plus-two-incremental chains. All nine source-content maps are compared to independently recorded source-tree SHA256 expectations using a separate test-only tar decoder. A compile-fail doctest separately rejects external access to the private decoder/result module. This evidence does not yet establish full production restoration, metadata fidelity, every permitted key/algorithm variant, signer-subkey compatibility or formal controller correctness. Those tasks require further independent fixtures, review, pipeline integration and proofs.

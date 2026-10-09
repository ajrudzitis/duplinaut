# Native OpenPGP adapter profile

This is an implementation note for [PARSE-ENV-002..016](parsing-requirements.md#parse-env-002), not independent proof or native compatibility evidence. The initial profile accepts authenticated SEIPDv1 archives. SEIPDv2/v6, proprietary GnuPG AEAD, unauthenticated SED, additional algorithms, and general certificate-validity policy remain follow-up work.

## Pinned upstream reference

Use published `pgp = "=0.21.0"` (rPGP), released 2026-10-03, with BZip2 support. The crate declares Rust 1.88 minimum; the workspace toolchain is Rust 1.98. Crate SHA-256 is `ae70f4d9325a391db30d115d6191a7bb67cec856aa0bba131290f0c4e09a532a`; its `.cargo_vcs_info.json` identifies upstream commit `c2b192e52b2e7f99777196539ea79b37f1f9ff30`.

Documentation was read before source: [versioned API](https://docs.rs/pgp/0.21.0/pgp/), [implementation status](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/docs/IMPL_STATUS.md), and [security status](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/docs/SECURITY_STATUS.md). Source fallback established exact buffering, full-read, and binding behavior. RFC references are [9580](https://www.rfc-editor.org/rfc/rfc9580) sections 5.7/5.9/5.13/10.3, [4880](https://www.rfc-editor.org/rfc/rfc4880), and [6637](https://www.rfc-editor.org/rfc/rfc6637). Current `latest` URLs can resolve to different cached releases; the version and published archive identity govern this profile.

The upstream dependency graph, OpenPGP packet handling, crypto primitives, secret-key unlocking, and signer verification are an explicitly reviewed trusted adapter boundary. rPGP implements mechanisms rather than a complete certificate-validity policy and documents an unresolved RSA timing/Marvin concern. The project must not describe those mechanisms as formally verified.

## Reproducible API findings

These names and behavior were inspected in the published 0.21.0 archive. They are a starting point for the adapter, not a claim that the calls alone establish the contract.

| Operation | Public API and contract |
| --- | --- |
| Parse bounded input | `composed::Message::from_bytes` takes a `BufRead + Debug + Send` reader; enforce the stored-object input limit independently |
| Parse key export | `composed::Deserializable` supplies binary/armor constructors for `SignedSecretKey` and `SignedPublicKey`; bound export bytes before parsing |
| Authenticate before parsing plaintext | `DecryptionOptions::new().set_seipdv1_read_mode(types::Seipdv1ReadMode::CheckFirst { max_message_size: limit })` checks complete SEIPDv1 MDC before releasing plaintext; the library's default is 1 GiB, while project default is 256 MiB |
| Try supported credentials | `TheRing { secret_keys, key_passwords, message_password, session_keys, decrypt_options }`, then `message.decrypt_the_ring(ring, false)`; false checks consistency among discovered session keys |
| Secret-key passphrase | `types::Password` supplies the key/message passwords; encrypted exported keys are unlocked during session-key decryption |
| Inspect encrypted variant | `Message::Encrypted { esk, edata, .. }`; `Edata` distinguishes SED, protected data, and proprietary AEAD; inspect protected-data version before decrypting |
| Decompress | `message.decompress()` unwraps compressed messages, including compressed content under a signature; retain the signed reader for verification |
| Consume content | `std::io::Read` to real EOF with bounded output; a truncated `take(limit)` stream is not evidence of complete EOF |
| Verify certificate | `SignedPublicKey::verify_bindings()` and corresponding secret-key methods verify self/binding signatures; signing subkeys also require embedded back signatures |
| Verify message | After complete reading, `verify_nested_explicit(index, key)` verifies a particular signature; inspect its binary type/hash/algorithm and match the authorized complete primary fingerprint |
| Identify key | `types::KeyDetails::fingerprint()`; a component's own fingerprint is distinct from its authorizing primary certificate fingerprint |

Relevant immutable sources: [message/types.rs](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/src/composed/message/types.rs), [message/parser.rs](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/src/composed/message/parser.rs), [types.rs](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/src/types.rs), [signed_key/public.rs](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/src/composed/signed_key/public.rs), and [encrypted_secret.rs](https://github.com/rpgp/rpgp/blob/c2b192e52b2e7f99777196539ea79b37f1f9ff30/src/types/params/encrypted_secret.rs).

## Adapter checks beyond convenience APIs

`decrypt_the_ring` hides the selected session key. For PKESK, its public-key algorithm does not identify the payload cipher. Strict AES policy requires inspecting the successfully unwrapped session key through public packet/key APIs before passing approved session keys to decryption, or an explicitly reviewed upstream adapter extension. Do not infer AES from the recipient algorithm or certificate preferences.

`MessageReader::check_trailing_data` consumes some padding/marker/noncritical/experimental packets and drains excess compressed-body data. Therefore library EOF alone does not establish the project's strict trailing-data rule. Packet-shape and complete framing checks must enforce the project's narrower profile, including counts, unknown packets, nested encrypted containers, and compression exhaustion. The library's recursive signature helper has its own depth limit (1024), which does not establish the project's depth-8 limit. No success milestone can rely on those helpers without demonstrating the additional checks.

CheckFirst bounds an encrypted body, not every outer wrapper or key input. The adapter must bound the complete stored object before buffering it, bound packet counts and key inputs before their associated work, and bound decompression output independently. It must never replace CheckFirst with Streaming after a resource-limit failure.

Fingerprint authorization is an offline recovery policy. It verifies data and bindings under explicitly supplied primary certificates; it does not assert current wall-clock validity, user-ID identity, revocation acceptance, or Web-of-Trust certification. Certificate metadata is exposed so callers can apply a separate validity policy.

## Existing official evidence and pending acceptance

The checked-in official duplicity 3.0.7 / GnuPG 2.4.9 corpus includes symmetric, public-key, and signed full-plus-two-incremental chains, encrypted secret-key exports, and known public test credentials. Its v4 signing primary is legacy Ed25519; its bound recipient subkey is legacy cv25519 ECDH. Inspection of the exported key confirms AES-128 usage-254 secret protection, SHA1 iterated/salted S2K with count 58720256, and SHA512 certification/binding signatures. Inspection of the signed full volume confirms PKESKv3 ECDH, tag18 SEIPDv1, ZLIB compression, and a version-3 one-pass binary Ed25519/SHA512 signature envelope. A one-pass packet is a hint, not signature-verification evidence.

Native acceptance still requires decrypting every official crypto object, comparing decoded objects/source-tree restoration, and negative tests for wrong credentials/signers, mutations, truncations, unsupported variants, packet boundaries/counts/depth, and all input/output limits. RSA and other permitted algorithm variants require targeted independently produced positive/negative evidence before their compatibility milestones are complete. Tests and trusted-contract review do not constitute a Verus proof of cryptography.

An additional isolated GnuPG 2.4.9 inspection of `signed/duplicity-full.20231114T221320Z.vol1.difftar.gpg` returned `DECRYPTION_INFO 2 9 0` (MDC plus AES-256), `GOODMDC`, `DECRYPTION_OKAY`, and `VALIDSIG D06847BAC3B117C46BB68BDA6F23173BAD8A7656 ... 4 0 22 10 00 ...` (v4 binary Ed25519/SHA512 signature). Its recipient subkey fingerprint is `33D7BB233183C2F1310DED2E5B28A0C84002F3F7`. GnuPG reported undefined certification trust; that does not invalidate cryptographic fingerprint-based verification. Signature/literal packet timestamps reflect actual producer wall time, separately from the deterministic duplicity backup dates.

To repeat this reference inspection, use a new private temporary GnuPG home, import only `test-keys/secret.asc`, and run official GnuPG with `--batch --pinentry-mode loopback --passphrase duplicity-reader-public-test-fixture --status-fd 2 --output /path/to/temporary/plaintext --decrypt /path/to/signed/duplicity-full.20231114T221320Z.vol1.difftar.gpg`. The passphrase and key are deliberately public test credentials. This reference-only invocation is never part of native reader runtime.

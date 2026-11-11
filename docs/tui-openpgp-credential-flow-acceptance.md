# TUI OpenPGP credential-flow acceptance

Status: accepted specification; implementation and acceptance test are
pending. No credential UI, frontend implementation, or acceptance test is
claimed by this document.

## Goal and bounded profile

Connect the existing public reader API, `Catalog::read_snapshot_with_crypto`
and borrowed `CryptoReadContext`, to the terminal browser and Linux restore
flow. A user must be able to enter the credential needed to read a supported
encrypted backup without putting passphrases or exported key bytes in command
arguments, environment variables, terminal output, application history, or
automatic diagnostics. The frontend must not construct a successful snapshot,
preview, or restore request before the core reader has completed its configured
integrity and signer checks.

Initial compatibility scope is limited to the three official Duplicity 3.0.7
full-plus-two-incremental profiles already accepted by the core reader:

1. `symmetric`: message passphrase; no exported key or signer policy.
2. `public-key`: the fixture's protected recipient secret-key export and its
   passphrase; no message passphrase or signer policy.
3. `signed`: the same recipient-key inputs plus the fixture signer public-key
   export, its permitted primary fingerprint, and `require_signature = true`.

These rows are the existing three-profile reader evidence, not a complete
OpenPGP or credential-provider matrix. The AES-128/AES-192 one-volume probes,
RSA-2048 recipient probe, signing-subkey probe, expired-key probe, unsigned
negative cases, other producers, and other key/cipher/signature combinations
are outside this acceptance.

`RESTORE-UI-015` defines an explicit archive-unlock interaction before
snapshot construction, separate from destination selection, restore preview,
and confirmation. It permits only no-echo passphrases and explicitly selected
key-export files under its non-disclosure rules. SDK/storage credentials
remain governed by RESTORE-UI-013 and the configured provider.

## Traceability

The planned test is scoped to:

- [PARSE-ENV-009](parsing-requirements.md#parse-env-009): caller-provided
  message passphrase and supported exported secret-key profile;
- [PARSE-ENV-014](parsing-requirements.md#parse-env-014): authentication
  failures do not produce a successful restore;
- [PARSE-ENV-021](parsing-requirements.md#parse-env-021): caller-owned secrets
  remain borrowed and absent from reader metadata and diagnostics;
- [PARSE-ENV-023](parsing-requirements.md#parse-env-023): successful evidence
  distinguishes integrity-only from configured signer authorization;
- [PARSE-ENV-024](parsing-requirements.md#parse-env-024): only complete,
  validated chains produce snapshots and evidence;
- [RESTORE-UI-003](parsing-requirements.md#restore-ui-003),
  [RESTORE-UI-004](parsing-requirements.md#restore-ui-004), and
  [RESTORE-UI-005](parsing-requirements.md#restore-ui-005): preview,
  acknowledgment, and backend execution are available only after a successful
  snapshot;
- [RESTORE-UI-013](parsing-requirements.md#restore-ui-013) and
  [RESTORE-UI-014](parsing-requirements.md#restore-ui-014): source remains
  read-only, secrets are not disclosed, and evidence separately records the
  real-terminal frontend path.
- [RESTORE-UI-015](parsing-requirements.md#restore-ui-015): archive-unlock
  credential acquisition and non-disclosure are separate from restore
  confirmation.

The test MUST use Duvet links only for the exercised clauses. It MUST NOT claim
that this flow formally verifies the reader, cryptography, the credential
provider, or the restore controller.

## Normative acceptance requirements

1. The TUI MUST acquire a message or key passphrase through an explicit
   no-echo terminal prompt. The passphrase MUST NOT be accepted as an argv or
   environment value. Typed characters MUST NOT appear literally or as a
   length-revealing mask in the terminal screen, scrollback transcript, normal
   logs, worker replies, catalog, snapshot, authentication evidence, or restore
   receipt. The prompt MUST NOT display a character mask, length, or other
   input-dependent indication. It MUST bound passphrase input to 4096 UTF-8
   bytes and compute each candidate character's `len_utf8` before appending;
   reject a multibyte character that would cross the limit without growing the
   input. Cancellation at the prompt MUST cancel the read and leave no
   snapshot or destination.
2. Public-key and signed profile key exports MUST be loaded from explicitly
   selected files; export bytes MUST NOT be accepted in argv/environment or
   rendered. The visible key-file path MUST be byte-escaped for the terminal,
   limited to 4096 input bytes, and rendered within a fixed bounded field.
   Before allocation/read, the selected export MUST be verified as a regular
   file with metadata length at most 16 MiB; the read MUST remain bounded to
   that limit and reject if bytes read exceed it, including growth beyond the
   cap during reading. File contents and passphrases
   MUST stay outside browser history, status labels, and diagnostic
   formatting. This acceptance makes no secure erasure guarantee for process
   memory or trusted crypto-library temporaries.
3. For each profile above, the production worker MUST build one immutable
   borrowed `CryptoReadContext` for the read and call
   `Catalog::read_snapshot_with_crypto`. It MUST preserve the exact profile
   policy for the whole chain. The successful snapshot MUST match the official
   3.0.7 `expected.json` final date (`1700172800`) and retain the corresponding
   core authentication classification: symmetric/public-key are integrity
   only; signed is authorized under the configured primary fingerprint.
4. The worker MUST preserve typed reader failure categories across the worker
   boundary. The UI may render a bounded redacted category, but MUST NOT claim
   that a cryptographic failure reliably means “wrong password” when the core
   error is authentication-ambiguous. For a wrong symmetric passphrase, the
   test MUST require the exact public reader error observed from the core
   fixture test (one of the exact typed results
   `ReaderError::Crypto(WrongCredentials)` or
   `ReaderError::Crypto(AuthenticationFailed)`) and MUST observe no
   `Reply::Snapshot`, preview, or Execute request. The test MUST NOT assume
   which of these authentication-ambiguous variants the core returns.
5. Until a successful snapshot reply exists, the frontend MUST NOT offer an
   executable restore preview or enqueue Execute. The signed-profile positive
   path MUST proceed through the ordinary preview and exact `restore` plus
   Enter acknowledgment before execution. A wrong-credential path MUST leave
   the requested destination absent and produce no publication/staging
   artifact. The successful signed path MUST report its typed backend outcome;
   this acceptance does not repeat the filesystem metadata oracle already
   covered by restore acceptance.
6. Credential and cancellation changes MUST invalidate any in-flight read
   token, retire its operation ID, and cancel the old token before accepting a
   new credential set. A stale reply MUST NOT install a snapshot under a
   different credential/policy set or arm restore confirmation. After the
   credential-submit event, prompt input MUST be quarantined/drained before
   returning to normal browser input; queued characters, especially `restore`,
   Enter, or `r`, MUST NOT become acknowledgment or command input. A fresh
   successful snapshot may arm a restore preview only after the user issues a
   separate browser command and the ordinary preview/acknowledgment sequence.
7. No OpenPGP credential, exported key bytes, message/key passphrase, or
   plaintext payload bytes may be written to the backup source or S3. The
   credential prompt MUST restore terminal mode on success, error, and
   cancellation.

## Acceptance sequence and harness

Use an integration harness that starts the actual
`CARGO_BIN_EXE_duplicity-tui` executable in a Linux PTY against each official
fixture directory (`symmetric/`, `public-key/`, and `signed/`). Use a new
empty working directory and destination parent per case, bounded screen and
stderr capture, fixed PTY dimensions, and deadlines for every prompt/state.
The exported test keys and passphrase are the public fixture credentials in
`tests/fixtures/duplicity-3.0.7/test-keys/`; no external GPG or AWS credentials
are used.

For each profile, synchronize input on observed UI state, enter the required
passphrase only after the hidden prompt appears, and require the final snapshot
to match `expected.json` for that mode/date. Prompt tests MUST fill each bound
to its limit and verify that one extra UTF-8 byte/character is rejected before
allocation, including a multibyte character that would cross the boundary.
The path-field validator test MUST accept the 4096-byte input boundary and
reject a 4097-byte candidate before append without opening that synthetic
path; separate file-load tests MUST use shorter valid paths and reject a
selected directory or other non-regular file. For secret input, the PTY test
MUST queue a restore acknowledgment, Enter, and `r` immediately behind the
submit keystroke, then prove that no preview or Execute is created until fresh
post-snapshot user input. Path tests MUST cover an escaped/bounded display and
export files at the size limit and one byte over it. Inspect the child command
line and
environment on Linux to verify they contain neither passphrase nor key bytes;
assert captured terminal output, screen model, stderr, history/status, and
worker diagnostics do not contain the passphrase or key-export contents.

For the signed positive case, continue to a production restore preview, type
the exact acknowledgment, and require the ordinary Published result. For the
negative case, rerun the symmetric fixture with an intentionally wrong
passphrase; require the typed core error, no snapshot/preview/Execute, absent
destination and no `.duplicity-*` staging entry. In every case verify the
terminal returns to its initial mode after exit. Preserve temporary artifacts
on timeout or uncertain publication; successful runs may clean them after all
assertions pass.

## Verification and limits

- Duvet: targeted queries and full trace for the requirement links above;
  coverage must identify the actual production prompt, worker bridge and
  executable PTY integration test.
- Tests: focused credential-prompt/key-input tests, full core and TUI suites,
  plus the Linux-gated production-binary PTY cases described above.
- Tooling: formatting, strict Clippy, `git diff --check`, local
  `cargo xtask ci`, independent implementation/security review, then hosted
  Linux assurance (`cargo xtask ci`) and Duvet Pages publication.
- Evidence must record the exact fixture profiles, no-echo results, argv/env
  inspection, typed negative result, terminal restoration, and which positive
  path reached backend execution.

This acceptance does not add new OpenPGP algorithms, support external agents or
keyrings, prove secure memory erasure, authenticate arbitrary profiles, verify
all three dates through the TUI, prove no transient filesystem writes, or prove
the cryptographic adapter, full reader/controller, S3 provider, or Linux kernel.
S3 SDK credentials remain governed by the existing external SDK credential
provider and are not part of this OpenPGP prompt.

## Checklist

- [x] Clarify RESTORE-UI-013 for read-only restore/source handling and add RESTORE-UI-015 for the not-yet-implemented archive-unlock flow.
- [x] Independently review and adopt this specification.
- [ ] Implement the no-echo prompt and typed worker/context bridge.
- [ ] Bound input/file reads and quarantine queued prompt keystrokes before
      normal browser input resumes; prove stale credential replies are retired.
- [ ] Add real-executable Linux PTY tests for all three named profiles and the
      wrong-passphrase/no-publication case.
- [ ] Complete targeted Duvet queries/full trace, full tests, formatting,
      strict Clippy and local assurance CI.
- [ ] Pass independent implementation/security review.
- [ ] Pass hosted Linux assurance and Duvet Pages publication.

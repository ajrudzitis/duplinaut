# Supported capabilities and assurance boundaries

This page summarizes what Duplinaut can currently read and restore, what its
evidence establishes, and which components remain trusted. It is a snapshot of
the project state, not a promise of complete duplicity compatibility. Detailed
normative rules and per-checkpoint evidence are in the [requirements](parsing-requirements.md),
[assurance ledger](assurance.toml), and linked implementation records.

## Available capabilities

| Area | Current capability | Limits and status |
| --- | --- | --- |
| Sources | Read-only local archives and read-only S3 objects; source identity and conditional-read checks are supported | Live S3 service validation and complete-reader consistency testing remain open; no bucket writes |
| History | Discover catalog metadata, plan full-plus-incremental chains, browse snapshots, select byte-valued paths, and build restore plans | Controller/resource proofs remain incomplete; unsupported entries can be shown where metadata permits but cannot be restored |
| Producer evidence | The checked-in producer/profile rows and their evidence levels are listed below; full-chain snapshot claims are limited to the named 3.0.7 profiles | Fixture presence or producer-side restore is not by itself evidence that this reader can reconstruct that profile |
| Containers and deltas | Bounded plain/gzip envelopes, manifests, tar records, signatures, librsync signatures/deltas, and ordered multipart assembly have implementations and official-fixture tests | Full parser/controller proofs, some historical forms, footer-bearing replay, and additional multipart cases remain open; check the requirement ledger for each format |
| OpenPGP | Native reader composition for the three recorded duplicity 3.0.7 full-chain profiles below, plus the specifically listed one-object probes | The probes do not establish equivalent full-chain compatibility. Uses the pinned rPGP adapter; SEIPDv2/v6, GnuPG AEAD, unauthenticated SED, CMS/GPGSM `.p7m`, unlisted algorithms, and complete-reader proof are not supported claims. See [profile and boundary](openpgp-profile.md) |
| Restoration | Linux native restoration of selected regular files, directories, and symlinks, with supported ordinary permissions and modification times; publication uses private staging and no-replace rename | Other operating systems, ownership, special files, ACL/xattr fidelity, hard links, crash recovery/durability, and hostile same-UID or privileged writers are outside the contract. The full filesystem controller is not formally proved. See [Linux profile](restore-linux.md) |
| Terminal UI | Browse local/S3 history, inspect snapshots, select paths, prepare a frozen preview, and explicitly confirm a restore to a caller-selected local destination | Credential entry, comprehensive resize/event-loop fault coverage, live S3 testing, and full controller proofs remain open. There is no bucket mutation or automatic restore on startup |

## Compatibility evidence by producer and profile

This is a finite inventory of checked-in evidence, not a release-wide
compatibility promise or a producer-version cross product. “Local” means the
named tests and local assurance receipt recorded in the linked project records
passed. “Hosted” names a run that exercised the relevant acceptance; it does
not extend the tested profile.

| Producer/profile | Evidence level | Local and hosted status | Boundary |
| --- | --- | --- | --- |
| Duplicity 0.8.23: standard, short names, legacy names/timestamps | Official full-plus-two-incremental plain fixtures and source-tree oracles; native tests plan all three dates from manifest names, and separately apply the official delta payloads | Checked-in native tests; no separate hosted compatibility acceptance is recorded here | Not a full native Catalog snapshot reconstruction claim for these profiles; no crypto profile tested |
| Duplicity 2.1.3 standard; 3.2.1 standard | Official full-plus-two-incremental plain fixtures and source-tree oracles; native tests plan all three dates and separately apply official delta payloads, including 3.2.1 zero-footer observations | Checked-in native tests; no separate hosted compatibility acceptance is recorded here | No full native Catalog snapshot reconstruction or crypto-profile claim for these rows |
| Duplicity 3.0.7 plain | Full, first incremental, and final incremental; native reader reconstructs all three dates against `expected.json`, including raw paths and supported metadata | Local tests are part of workspace assurance; the hosted acceptance run [41835819167](https://github.com/ajrudzitis/duplinaut/actions/runs/41835819167) passed Linux assurance and Duvet Pages | Symlink mtime is excluded by the producer oracle; this does not establish every 3.0.7 archive shape |
| Duplicity 3.0.7 symmetric OpenPGP | Full-plus-two-incremental chain; native reader reconstructs all three dates against the source oracle | Local tests and hosted Linux assurance/Duvet Pages passed in [run 41835819167](https://github.com/ajrudzitis/duplinaut/actions/runs/41835819167) | This recorded fixture's cipher/key profile only; no general symmetric-algorithm matrix |
| Duplicity 3.0.7 public-key OpenPGP | Full-plus-two-incremental chain; native reader reconstructs all three dates against the source oracle | Local tests and hosted Linux assurance/Duvet Pages passed in [run 41835819167](https://github.com/ajrudzitis/duplinaut/actions/runs/41835819167) | This recorded cv25519 recipient/key profile only; not RSA full-chain coverage |
| Duplicity 3.0.7 signed OpenPGP | Full-plus-two-incremental chain; native reader reconstructs all three dates with configured signer authorization and bounded evidence | Local tests and hosted Linux assurance/Duvet Pages passed in [run 41835819167](https://github.com/ajrudzitis/duplinaut/actions/runs/41835819167) | This recorded Ed25519-primary/cv25519-recipient chain only; it does not establish other signer or recipient profiles |
| Duplicity 3.0.7 supplemental AES-128 and AES-192 symmetric profiles | Each is a separate one-Full-manifest/one-volume oracle test; the fixture has exactly one SKESK and no PKESK | Independent review and full local CI receipts are recorded for [AES-128](parse-env-008-aes128-acceptance.md) and [AES-192](parse-env-008-aes192-acceptance.md); hosted checks remain pending | One object per cipher profile, not a multi-volume chain; other cipher modes and combinations remain untested |
| Duplicity 3.0.7 supplemental RSA-2048 PKESK profile | One Full-manifest/one-volume oracle test with recorded recipient component and key-protection provenance | Full local CI and independent review are recorded in the [RSA PKESK acceptance](parse-env-008-rsa-pkesk-acceptance.md); hosted checks remain pending | One RSA-2048 recipient profile; no RSA signature or RSA full-chain compatibility claim |
| Duplicity 3.0.7 supplemental Ed25519 signing-subkey and expired-primary probes | Separate one-Full snapshot tests observe a bound Ed25519 signing subkey and a historical RSA-2048 primary expiry, respectively | Full local CI and independent review are recorded in the [signing-subkey](parse-env-008-official-signing-subkey-acceptance.md) and [expired-signer](parse-env-013-expired-signer-acceptance.md) records; hosted checks remain pending | These are distinct one-object observations, not a general signing-subkey or certificate-validity policy |
| Duplicity 3.0.7 signed-full/public-key-incremental negative case | Six-object composition retains the signed Full and later incremental chain while the first incremental manifest is public-key encrypted and unsigned; signer-required reading rejects at the exact expected boundary | Local checks and hosted Linux assurance/Duvet Pages passed in [run 41818457786](https://github.com/ajrudzitis/duplinaut/actions/runs/41818457786) | Negative signer-policy regression only; it does not establish a supported unsigned-incremental profile |
| Duplicity 3.0.7 signed-manifest/unsigned-symmetric-Full-volume negative case | One signed manifest plus an unsigned symmetric-encrypted Full data volume, with matching logical payload evidence; signer-required reading rejects the volume | Full local CI and independent review are recorded in the [unsigned data-volume acceptance](parse-env-019-unsigned-data-volume-acceptance.md); hosted checks remain pending | This is a one-volume negative policy regression, not an unsigned incremental or supported unsigned profile |

The 3.0.7 crypto full-chain row and the supplemental probes intentionally remain
separate: passing a one-object probe does not fill the corresponding full-chain
cell. Historical producer fixtures likewise establish only their recorded
plain naming, chain-planning, and delta-payload observations. Historical
symmetric/public-key/signed chains, RSA signatures, unlisted cipher or public-key
algorithms, SEIPDv2/v6, AEAD, SED, and CMS/GPGSM `.p7m` are unsupported or
untested by this evidence. Unsupported-envelope diagnostics are not format
support.

The precise configurable resource defaults and format requirements are specified
in [parsing-requirements.md](parsing-requirements.md). Exceeding a configured
limit is an error; the defaults are not a guarantee that every historical
repository fits them.

## What the evidence means

Duplinaut keeps four kinds of evidence distinct:

1. **Official-producer integration tests** compare native reads/restores with
   original duplicity-produced objects and independent source-tree expectations.
   They establish behavior for the recorded fixtures and exercised profiles.
2. **Verus and Vest verification** establish the contracts of listed bounded
   kernels and Vest parser definitions. A scoped proof does not automatically
   establish its caller, adapter, whole parser, reader controller, or operating
   system interaction.
3. **Independent review** checks the exact candidate source and its verification
   receipt before selected checkpoints are adopted. The [assurance ledger](assurance.toml)
   records accepted scope and open obligations.
4. **Native tests and fault injection** exercise integration and error paths on
   the tested toolchain and host. They do not prove all kernels, filesystems,
   network services, or scheduling behaviors.

Passing a workspace build or tests alone is not a formal verification claim.
The complete core and reader-controller assurance gates remain in progress;
the Linux restoration milestone is explicitly unproved. The pinned Duvet
snapshot and coverage queries track requirement-to-code/test links and prevent
accepted-evidence regressions; traceability is not itself proof of correctness.

## Trusted boundaries

Formal claims cover the particular deterministic algorithms named in the
assurance ledger. These surrounding components are reviewed or tested, but are
not Verus-proved as implementations:

- Rust compiler, standard library, allocator, and generated-code/toolchain
  behavior;
- Vest and Verus frontends, SMT solver, build scripts, and pinned dependencies;
- hash, compression, OpenPGP, and cryptographic primitives, including the
  upstream rPGP packet/decryption/signature implementation;
- AWS SDK, TLS/network stack, remote S3 service, and local source filesystem;
- rustix wrappers, Linux kernel, and filesystem implementation;
- terminal emulator, operating-system event delivery, and process environment.

The project uses explicit limits, typed errors, independent tests, pinned
versions, and source review around these boundaries. None of those measures
turns a trusted dependency or external service into a formally verified
component. See [OpenPGP implementation notes](openpgp-implementation.md),
[Linux implementation notes](restore-linux-implementation.md), and
[GitHub assurance workflow](github-actions.md) for component-level detail.

## Not a backup writer

Duplinaut does not create backups, upload or modify bucket objects, repair
incomplete chains, or claim a general-purpose duplicity replacement. Its goal
is read-only history browsing and carefully bounded restoration of supported
content from available archives.

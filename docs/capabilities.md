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
| Producer evidence | Original fixtures from duplicity 0.8.23, 2.1.3, 3.0.7, and 3.2.1; official restore oracles exist for the recorded fixture sets | Fixture compatibility is not a claim that every release feature, naming profile, or archive can be read natively |
| Containers and deltas | Bounded plain/gzip envelopes, manifests, tar records, signatures, librsync signatures/deltas, and ordered multipart assembly have implementations and official-fixture tests | Full parser/controller proofs, some historical forms, footer-bearing replay, and additional multipart cases remain open; check the requirement ledger for each format |
| OpenPGP | Native reader composition for recorded duplicity 3.0.7 symmetric, public-key, and signed full-plus-incremental profiles; ciphertext digests, authenticated decryption, configured signer policy, and encrypted tar completion are checked | Uses the pinned rPGP adapter. SEIPDv2/v6, GnuPG AEAD, unauthenticated SED, CMS/GPGSM `.p7m`, broader historical algorithms, and complete-reader proof are not supported claims. See [profile and boundary](openpgp-profile.md) |
| Restoration | Linux native restoration of selected regular files, directories, and symlinks, with supported ordinary permissions and modification times; publication uses private staging and no-replace rename | Other operating systems, ownership, special files, ACL/xattr fidelity, hard links, crash recovery/durability, and hostile same-UID or privileged writers are outside the contract. The full filesystem controller is not formally proved. See [Linux profile](restore-linux.md) |
| Terminal UI | Browse local/S3 history, inspect snapshots, select paths, prepare a frozen preview, and explicitly confirm a restore to a caller-selected local destination | Credential entry, comprehensive resize/event-loop fault coverage, live S3 testing, and full controller proofs remain open. There is no bucket mutation or automatic restore on startup |

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

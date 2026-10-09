# Development tools

Pinned tool identities and native archive checksums live in [toolchain.toml](toolchain.toml). The initial bootstrap supports Linux x86_64.

```sh
nix-shell tools/shell.nix --run 'bash tools/bootstrap.sh'
source tools/env.sh
cargo xtask doctor
```

On systems providing Rustup, Python 3.11+, and native libraries, run `bash tools/bootstrap.sh` directly. Tools install in ignored `.tools`; `DUPLICITY_TOOLS_DIR` selects another directory. `VERUS`, `CARGO_VERUS`, `VEST`, and `DUVET` select executables. Source `tools/env.sh` using Bash or Zsh. Duvet has no version command; the doctor checks its Cargo installation receipt and query capabilities.

| Command | Behavior |
| --- | --- |
| `cargo xtask doctor` | Check pinned tools and solver; missing or mismatched tools fail |
| `cargo xtask generate` | Regenerate declared Vest outputs, including reviewed deterministic trailers |
| `cargo xtask build [--release]` | Check ledger, algorithm audit, generated identity and Duvet; run a verified native build |
| `cargo xtask verify` | Run the same prerequisites and full verification without native product artifacts |
| `cargo xtask trace` | Check the Duvet snapshot and links for enabled milestones; alias `traceability` |
| `cargo xtask trace-update` | Explicitly replace the snapshot for review and commit |
| `cargo xtask test` | Run workspace tests, including checked-in official-produced fixtures |
| `cargo xtask ci` | Verified build, format/lint checks, workspace tests and execution of declared milestone tests; alias `check` |
| `cargo xtask producer` | Run official-fixture regeneration when established; currently fails explicitly while pending |

The [assurance ledger](../docs/assurance.toml) separates specified, implemented, proved, tested, and reviewed status. Enable a milestone only after independent acceptance; pending components confer no completion claims. Inapplicable proofs require a reviewed rationale. An enabled baseline with no active scope fails. Missing canonical milestones, empty requirements, missing algorithm sources, absent proof declarations, project proof shortcuts, and unreviewed trust fail acceptance.

Declarations and Duvet links provide structural traceability. Proof references must be reachable through the verified core's unconditional module/include tree; conditional proof files and custom module-path arrangements require explicit future support. The doctor requires core verification metadata and exact resolved upstream versions, sources and independently pinned archive identities. Independent semantic review must establish that contracts express their requirements, referenced proofs apply to production code, and tests provide meaningful compatibility evidence. Counts establish none of those properties. Project algorithms are audited and verified with `--no-cheating`; pinned upstream Verus/Vest trusted primitives remain explicit boundaries.

Successful artifacts in `target/assurance/<command>.json` record repository source SHA-256, pinned tools and executable hashes, target/configuration, declared obligations and executed phases. Sources changing during a run prevent evidence. Builds record tests as not run; CI records successful tests only after executing their exact declared names and rejecting ignored/filtered tests. Failed commands do not create new successful evidence.

Bootstrap and tooling passed twelve failure-mode tests and independent coordinator review. The tooling milestone and its development trust boundary are enabled in the ledger. Parser proofs and complete restoration remain separate milestones.

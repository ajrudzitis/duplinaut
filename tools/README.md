# Development tools

Pinned tool identities and native archive checksums live in [toolchain.toml](toolchain.toml). The bootstrap supports Linux x86_64 and Apple Silicon macOS using platform-specific archives at identical Verus, Z3, Vest, and Duvet versions.

```sh
nix-shell tools/shell.nix
source tools/env.sh
bash tools/bootstrap.sh
cargo xtask doctor
```

On systems providing Rustup, Python 3.11+, and native libraries, run `bash tools/bootstrap.sh` directly. Tools install in ignored `.tools`; `DUPLICITY_TOOLS_DIR` selects another directory. `VERUS`, `CARGO_VERUS`, `VEST`, and `DUVET` select executables. Source `tools/env.sh` using Bash or Zsh. Duvet has no version command; the doctor checks its Cargo installation receipt and query capabilities.

| Command | Behavior |
| --- | --- |
| `cargo xtask doctor` | Check pinned tools and solver; missing or mismatched tools fail |
| `cargo xtask generate` | Regenerate declared Vest outputs, including reviewed deterministic proof additions and trailers |
| `cargo xtask build [--release]` | Check ledger, algorithm audit, generated identity and Duvet; run a verified native build |
| `cargo xtask verify` | Run the same prerequisites and full verification without native product artifacts |
| `cargo xtask trace` | Check the Duvet snapshot and links for enabled milestones; alias `traceability` |
| `cargo xtask trace-update` | Explicitly replace the snapshot for review and commit |
| `cargo xtask test` | Run workspace tests, including checked-in official-produced fixtures |
| `cargo xtask ci` | Verified build, format/lint checks, workspace tests and execution of declared milestone tests; alias `check` |
| `cargo xtask producer` | Run official-fixture regeneration when established; currently fails explicitly while pending |

The [assurance ledger](../docs/assurance.toml) separates specified, implemented, proved, tested, and reviewed status. Enable a milestone only after independent acceptance; pending components confer no completion claims. Inapplicable proofs require a reviewed rationale. An enabled baseline with no active scope fails. Missing canonical milestones, empty requirements, missing algorithm sources, absent proof declarations, project proof shortcuts, and unreviewed trust fail acceptance.

Declarations and Duvet links provide structural traceability. Proof references must be reachable through the verified core's unconditional module/include tree; conditional proof files and custom module-path arrangements require explicit future support. The doctor requires core verification metadata and exact resolved upstream versions, sources and independently pinned archive identities. The sole local dependency exception is the [reviewed Vest repair](../vendor/README.md), checked against its independently pinned complete source-tree digest; other path/git substitutions fail. Independent semantic review must establish that contracts express their requirements, referenced proofs apply to production code, and tests provide meaningful compatibility evidence. Counts establish none of those properties. Project algorithms are audited and verified with `--no-cheating`; pinned upstream Verus/Vest trusted primitives remain explicit boundaries.

Proof references default to `kind = "lemma"`, requiring a parsed proof declaration inside `verus!`. `kind = "contract"` requires a verified executable declaration with an explicit postcondition beyond literal `true`. Parsed items exclude examples in comments, unexpanded macros, function bodies, and conditional declarations. This structural check does not establish the meaning or strength of a contract; the complete verifier and independent review remain mandatory.

The declared `manifest-token-facts-v1` generation profile activates only `crate::verified::token_refinement::group_token_facts` in the unique unconditional inline `derived_proofs` and `exec_impls` modules inside the generated `verus!` block. It adds the fact to the existing braced `derived_proofs` broadcast list and adds one standalone broadcast directive in `exec_impls`; unexpected existing broadcast structure fails. It preserves every original Vest output byte and fails if module structure changes. It activates separately checked proof facts without replacing executable parser code, specifications, or proof obligations. The ledger declares this addition, and generation identity checks reproduce it before verification.

Successful artifacts in `target/assurance/<command>.json` record repository source SHA-256, pinned tools and executable hashes, target/configuration, declared obligations and executed phases. Sources changing during a run prevent evidence. Builds record tests as not run; CI records successful tests only after executing their exact declared names and rejecting ignored/filtered tests. Failed commands do not create new successful evidence.

Each acceptance verification allocates an empty unique `target/assurance/verification-*` directory and passes it explicitly to cargo-verus. Shared Cargo artifacts from developer `focus` commands cannot supply acceptance results. Inherited compiler flags, wrappers and internal Verus driver arguments are removed. Successful exit alone is insufficient: the run must emit a nonempty full verification summary with zero errors; partial verification summaries fail. Standard output and error logs remain in that directory. This rebuilds dependencies and uses additional time and disk space; cached dependency reuse requires a separately reviewed strategy. The freshness guard has a subprocess regression test and received independent coordinator review.

Bootstrap and tooling have executable failure-mode tests. The original tooling boundary received independent coordinator review; subsequent dependency-identity, vendor, contract-evidence, generation-profile and freshness checks also received independent coordinator review. The tooling milestone and its development trust boundary are enabled in the ledger. Parser proofs and complete restoration remain separate milestones.

Proof reachability follows unconditional module and literal top-level include declarations through the Verus-aware syntax tree. Conditional imports or unrelated conditional modules do not hide unconditional siblings. Inner file conditions, conditional module/include edges, custom module paths, symlink source paths or ancestors, unknown expansion macros and function-local includes and module edges inside included files remain outside the accepted reachability profile; regression tests prevent credit from those paths. The separate evidence audit still requires the exact function to have an unconditional verified declaration with a substantive contract.

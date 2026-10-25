# Codex handoff

Use this note if continuing Duplinaut from a new computer or a fresh Codex chat.
It summarizes durable project decisions and the current checkpoint; the README,
requirements, assurance ledger, and implementation documents remain authoritative.

## Start on another computer

The repository is published at `git@github.com:ajrudzitis/duplinaut.git`. Clone it
into the normal project directory, then start Codex from that directory:

```sh
mkdir -p "$HOME/code"
git clone git@github.com:ajrudzitis/duplinaut.git "$HOME/code/duplicity-tui-rs"
cd "$HOME/code/duplicity-tui-rs"
codex
```

If the directory already exists, use `git pull --ff-only` instead of cloning.
Follow [the pinned tool setup](tools/README.md) before running project checks.
Do not copy the entire `~/.codex` directory as a session-transfer shortcut; it
may contain machine-specific state and authentication data.

If the original conversation appears in Codex Recent chats after signing in on
the new computer, it can be reopened there. Otherwise, start a new local Codex
chat in the cloned directory and ask it to read this file, `README.md`,
`docs/parsing-requirements.md`, and `docs/assurance.toml` before continuing.
Conversation history and local project files are separate: syncing a chat does
not make the original computer's working tree available on another computer.

## Project decisions

- Duplinaut is a read-only duplicity backup browser and restore tool, not a
  backup writer. The workspace has `duplicity-core`, `duplicity-s3`,
  `duplicity-tui`, and `xtask` crates.
- `docs/parsing-requirements.md` is the RFC 2119/8174 requirements source.
  Duvet annotations, snapshots, and coverage gates in `cargo xtask trace`
  enforce requirement links and prevent regressions.
- Vest DSL is preferred for parsers. Documented direct-combinator fallbacks
  must stay explicit. Verus proofs state exact contracts; proof counts do not
  imply whole-component correctness.
- Official duplicity-produced fixtures and independent producer/restoration
  oracles are required alongside formal methods.
- Make a Git commit for each completed step and push useful checkpoints.
  For scoped proof work, use a frozen candidate, exact-source CI receipt,
  independent review, adoption, and a committed review record.

## Current checkpoint

The latest accepted work adds a Verus ACL absence evidence state and gates
`clear_acl` success on access-ACL absence for files and directories, plus
default-ACL absence for directories. This gate runs before permission widening.
The focused state proof and exact-source CI passed; independent review accepted
the implementation. Its scope excludes syscall truth, ACL name/loop mapping,
filesystem ACL semantics, races, chmod effects, and the full restore controller.
Linux restoration remains explicitly unproved in `docs/assurance.toml`.

The hosted repository is on `main`; see the README backlog for the current next
items. Major open work includes repository-controller and complete resource
accounting proofs, remaining parser/controller refinement, ordered snapshot
map and filesystem-controller refinement, live S3 validation, TUI completion,
and the supported official-producer compatibility matrix.

## Project commands

```sh
cargo xtask doctor
cargo xtask build
cargo xtask test
cargo xtask trace
cargo xtask ci
```

The full assurance gate is still in progress. Keep component acceptance
separate from project-wide completion claims.

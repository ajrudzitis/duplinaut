# Reviewed Vest primitive repair

`vest_lib` is copied from the published 0.2.1 crate (SHA256 `f3f4d4698b6a56f5b23ae330d294a8d77fbb728fbf6d74946a18e0e46ba6e7a9`). The upstream MIT license is retained. This is verification infrastructure, not another product crate.

The sole change to specification semantics replaces `array_from_seq(i)` with fixed array literals in the six `u16`, `u32`, and `u64` little/big-endian specification constructors. Each constructor follows `Fixed<N>`, which supplies exactly N bytes before the mapper runs. Native parser code is unchanged. The explicit arrays expose constructive byte semantics to independent refinement proofs without invoking the upstream uninterpreted sequence-to-array primitive under `--no-cheating`.

Six checked constructive mapper lemmas and eighteen calls at existing soundness/nonmalleability proof sites establish the same library contracts under the explicit arrays. These add no assumptions and do not weaken preconditions or conclusions. Independent review checked the semantic and proof-only changes. Full verification of the patched upstream source passes: 3183 obligations, zero errors. Upstream trusted primitives remain the enumerated dependency boundary; the core still uses `--no-cheating`.

The project keeps the remaining upstream trusted infrastructure explicit. This repair neither authorizes new assumptions in core algorithms nor substitutes upstream generic round-trip lemmas for duplicity semantic proofs. Source identities are recorded in `vest_lib/PROVENANCE.json`; changes require a fresh review and updated pin.

# duplicity-tui

Planned terminal backup browser and restore interface. The executable currently prints its scaffold status and exits unsuccessfully.

| Component | Specification review | Implementation | Proof | Integration tests | Independent acceptance |
| --- | --- | --- | --- | --- | --- |
| Browser and restore flow | Requirements written; interaction contracts pending | Pending | Calls verified core; display/I/O trusted | Pending | Pending |

- [x] Create the executable crate with no unsupported feature claims.
- [x] Escape terminal controls, non-ASCII bytes and literal escapes while retaining original path identity; native tests pass.
- [ ] Implement snapshot and byte-preserving path navigation.
- [ ] Implement restore selection, private credential entry and cancellation.
- [ ] Test display escaping, errors and interaction flows.

See [requirements](../../docs/parsing-requirements.md) and the [machine ledger](../../docs/assurance.toml).

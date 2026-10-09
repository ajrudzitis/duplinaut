# duplicity-s3

Planned read-only S3 source adapter. No network implementation or AWS dependency is present yet.

| Component | Specification review | Implementation | Proof | Integration tests | Independent acceptance |
| --- | --- | --- | --- | --- | --- |
| Listing, versioned reads, retries and cancellation | Requirements written; adapter contracts pending | Pending | Trusted integration boundary; contracts pending | Pending | Pending |

- [x] Create the adapter crate without coupling AWS dependencies to the core.
- [ ] Implement the reviewed object-source interface and read-only AWS adapter.
- [ ] Test pagination, changed objects, permission errors and cancellation.

See [requirements](../../docs/parsing-requirements.md) and the [machine ledger](../../docs/assurance.toml).


# duplicity-s3

Read-only AWS SDK adapter for the core `ObjectSource` interface. The reviewed contract covers bounded pagination, version/ETag conditions, exact-size streams and cancellation. Two unit tests and eleven local HTTP integration tests pass. Independent source review is complete; live-service validation remains pending. The new complete-reader tests are awaiting independent test review.

`S3Source::connect(S3Config, Cancellation)` uses the SDK's normal credential and region providers. Configuration supports profile, region, endpoint, bucket prefix, path-style addressing, request timeout and stream idle timeout. `head` discovers version IDs when available; listing returns ETags. Every GET requires an ETag and uses `If-Match`, plus a known version ID. Returned identity and size must match before any bytes reach the caller.

Synchronous methods run SDK futures on an owned runtime thread, including when called from another Tokio runtime. Each stream has its own stop token; dropping it does not cancel unrelated operations. Cancellation is polled at 20 ms intervals during requests, blocking receives and queue backpressure. Listing accounts for key/identity strings, token history and conservative container overhead before retaining entries. Streaming holds at most two queued 64 KiB buffers, one producer buffer and one reader buffer. The SDK's internal buffers, credentials, HTTP implementation, TLS and scheduling remain explicit trusted integration boundaries.

[SDK timeout documentation](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/timeouts.html) distinguishes request timeouts from streaming consumption. This adapter separately times each body read. [S3 GetObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html) specifies version IDs and conditional reads; ETags are identity tokens rather than manifest digest replacements.

| Component | Specification review | Implementation | Proof | Integration tests | Independent acceptance |
| --- | --- | --- | --- | --- | --- |
| Listing, versioned reads and cancellation | Contract reviewed | Implemented | Trusted I/O boundary | Two unit and eleven actual SDK HTTP tests pass | Source review complete; live-service integration pending |

- [x] Keep AWS dependencies out of the core.
- [x] Implement the reviewed read-only object-source contract.
- [x] Pass pagination, identity, permission, truncation, cancellation and backpressure tests.
- [x] Complete independent adapter review, including persistent terminal stream failures.

Tests inject public dummy credentials and contact only a loopback HTTP fixture. They exercise actual SDK request/response handling without writing to a bucket. The actual SDK now transfers original official objects through catalog discovery, chain reconstruction and native Linux publication. All three dates match raw paths, content hashes, types, modes, file/directory mtimes and symlink targets against the source/official-restore oracle; link mtimes match actually archived metadata. Late conditional-read failures and cancellation return no snapshot after earlier sets have succeeded. Tests enforce GET-only requests and original If-Match tokens. This loopback fixture does not validate AWS authentication, live S3 or MinIO service behavior; those checks remain pending. See [requirements](../../docs/parsing-requirements.md) and the [machine ledger](../../docs/assurance.toml).

Terminal stream failures remain errors on subsequent nonempty reads. Tests cover a missing EOF marker and an actual SDK checksum failure reported after exactly the expected body bytes; retries cannot convert either failure into successful EOF.

- [x] Exercise the complete reader and Linux restorer through actual SDK HTTP transfers against original official three-date backups.
- [ ] Independently review the new full-reader test fixture and oracle assertions.

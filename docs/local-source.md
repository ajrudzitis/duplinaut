# Read-only local source checkpoint

`LocalSource` provides the Unix native `ObjectSource` contract for one flat
backup directory. Construction retains an anchored directory FD. The caller
chooses the root and its ancestors; its final component cannot be a symlink.
Trailing slashes are rejected (except `/`) because Linux can otherwise follow
the final directory symlink despite NOFOLLOW. Explicit `/.` makes the preceding
name an ancestor selected by the caller.
After construction all entry operations use that FD, never a reconstructed
absolute path. Flat UTF-8 keys exclude slash, backslash, NUL and dot components.
Non-UTF-8 keys fail explicitly. Symlinks and nonregular entries are omitted from
listing, and opening uses NOFOLLOW and NONBLOCK followed by regular-file checks.

Public [rustix openat documentation](https://docs.rs/rustix/1.1.5/rustix/fs/fn.openat.html),
[Dir documentation](https://docs.rs/rustix/1.1.5/rustix/fs/struct.Dir.html) and
[AtFlags documentation](https://docs.rs/rustix/1.1.5/rustix/fs/struct.AtFlags.html)
provide the directory-relative and no-follow primitives. The implementation uses
safe Rust; rustix, standard allocation and the operating system remain trusted.

Discovery stores an opaque version token comprising device, inode, size, mtime
and ctime, including nanoseconds. Open validates the original size and token.
Every read validates both the opened FD and the current anchored pathname before
and after reading, including EOF. Exact-size CheckedReader rejects short/excess
bodies and cancellation. Changes, replacement, deletion, errors and cancellation
remain terminal for a stream, except retryable interruption.
Reads are positional, with a logical offset committed only after the second
identity check succeeds. An interrupted check can retry the same bytes without
skipping data. Concurrent changes may expose provisional bytes; complete reader
validation must reach successful EOF before returning a snapshot.
Metadata tokens detect observable changes but are neither cryptographic digests
nor immutable snapshots. Filesystem timestamp granularity, inode reuse, privileged
metadata manipulation and kernel behavior remain trust constraints.

Listing precharges each retained key, a conservative 256-byte version-token
allocation allowance and one exact-capacity vector slot before retaining them.
Count, metadata and stored-size bounds fail explicitly. Open also precharges
retained key/token/stream metadata. Rustix directory buffers, transient stat/token
checks and allocator overhead are outside this retained-metadata budget.
Retained diagnostics for terminal I/O errors are also outside that budget;
successful stream charging includes the complete CheckedReader allocation.
Scanning filtered/nonregular directory entries has no separate work-count bound; cancellation
is checked per entry. Filesystem calls can block and carry no cancellation latency
guarantee. The adapter performs no runtime writes or subprocess calls.

Four integration tests and one injected-interruption test cover changes, replacements,
rename-before-EOF, symlink rejection, unsafe keys, cancellation, untrusted identities
and limits. The fourth reconstructs all three original official backup dates
through this adapter. Whole-backend formal verification is pending.

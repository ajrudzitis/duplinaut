# Linux restoration specification evidence

The full RFC requirements were independently accepted before filesystem
implementation. Accepted full-spec SHA256:
`b22b31f619d35caa06c7663ad1d5fb133e8cd594ecbfc8bb506926237b582374`.
The adopted [requirements](../../parsing-requirements.md) retain those exact bytes.

The accompanying [profile](../../restore-linux.md) has editorial adoption wording
and repository-relative probe links; its normative requirements are unchanged
from reviewed SHA256
`795390fe34450bd5092d962b7e5b89d7c0519baecf01e121a2c5f5d41c0bc6a2`.

[Provenance](provenance.json) preserves the author's frozen artifact/source hashes
and status at preparation. Final independent review resolved creation-journal
uncertainty and late rename errors before adoption. The [probe](probe_linux_rename.py)
and [receipt](probe-linux-rename.receipt.json) establish only the observed ordinary
rename permission rule; they do not establish native restoration or NOREPLACE.

Kernel references in the profile link immutable upstream source commits. Their
hashes describe the reviewed source, which can be retrieved independently; kernel
sources are not vendored or treated as project proofs. Native fault tests remain
required for attempted creation followed by error, publication side effect
followed by error, uncertain first staging creation and restrictive cleanup order.

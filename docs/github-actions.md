# GitHub Actions assurance

[The workflow](../.github/workflows/assurance.yml) runs the repository's complete
`cargo xtask ci` acceptance command on a fresh GitHub-hosted Linux x86_64
`ubuntu-24.04` runner for pushes, pull requests, and manual dispatches. On pull
requests, checkout uses GitHub's event reference, normally the proposed merge
commit. There are no path filters that could silently omit a required check.

The job first installs native prerequisites, then runs `tools/bootstrap.sh` and
sources `tools/env.sh`. Bootstrap uses the versions and native archive checksums
in [toolchain.toml](../tools/toolchain.toml). The acceptance command checks tool
identity and the assurance ledger, reproduces Vest generation, checks the Duvet
snapshot, performs fresh full verification, checks formatting and Clippy, runs
workspace tests, and executes the exact tests declared for enabled milestones.
The existing acceptance command rejects partial verification summaries and source
changes during a run. It creates a success receipt only after those phases pass;
pending milestones remain pending. See [the tooling contract](../tools/README.md).

Failures in prerequisite installation, bootstrap, proofs, generation, traceability,
compilation, linting, or tests fail the job. Bash `pipefail` preserves failing
command statuses when output is copied into logs. No gate has `continue-on-error`,
no unverified build fallback is used, and proof resource exhaustion is a failure.
A workflow definition or an uploaded log establishes no proof or reader completion.

## Runner and dependency boundary

Ubuntu 24.04 supplies glibc 2.39, compatible with the checksum-pinned Z3 archive;
the workflow checks Linux/x86_64, glibc >=2.39, and Python >=3.11 before bootstrap.
Python's standard-library `tomllib` parses the tool pins. GitHub's runner image
provides Rustup; bootstrap installs the pinned Rust toolchain with rustfmt and
Clippy. `build-essential`, `pkg-config`, `libssl-dev`, `ca-certificates`, and
`python3` are installed explicitly. The pinned Duvet source/lockfile includes
native TLS and OpenSSL dependencies, so the OpenSSL headers and system linker
are required even though the reader's network dependency uses Rustls.

GitHub's runner image, kernel, apt package versions, network availability, Rustup
service, and action runtime remain development trust boundaries. The runner image
and apt package versions are maintained by their upstreams rather than frozen by
this workflow. Native Verus/Z3 downloads are checked against committed SHA-256
pins; Cargo tool installation uses exact versions with `--locked`. Bootstrap and
`xtask doctor` remain authoritative for identity checks.

The workflow allows 180 minutes and limits parallel Cargo build jobs to two.
Neither setting weakens proof obligations. Fresh verification rebuilds dependencies
in a new target directory and can require substantial memory and disk space.
Hosted-runner resource sufficiency has not been demonstrated until an actual run
passes. No existing proof or build output is restored from a cache. Introducing a
cache requires a separate review; successful acceptance proofs must remain fresh.

## Credentials, actions, and artifacts

The assurance job grants only `contents: read`, does not consume repository secrets,
and sets checkout's `persist-credentials: false`. It uses ordinary
`pull_request`, avoiding the privileged `pull_request_target` execution context.
It does not write repository content, comment on PRs, publish a package, or
connect to an S3 bucket. Fixtures use public test credentials already declared
in their provenance. Uploading diagnostic artifacts uses GitHub's job-scoped
artifact service, not a repository write token. A separate Pages job publishes
only the generated Duvet report after successful pushes to `main`; it has the
`pages: write` and `id-token: write` permissions required for OIDC deployment.
Pull requests do not publish a Pages site. Configure the repository's Pages
deployment source as **GitHub Actions** once before the first hosted deployment.

Both actions use complete immutable commit identities, resolved against their
official release tags on 2026-10-11:

| Action | Reviewed release | Commit |
| --- | --- | --- |
| [actions/checkout](https://github.com/actions/checkout/releases/tag/v7.0.1) | v7.0.1 | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| [actions/upload-artifact](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) | v7.0.2 | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` |
| [actions/upload-pages-artifact](https://github.com/actions/upload-pages-artifact/releases/tag/v5.0.0) | v5.0.0 | `fc324d3547104276b827a68afc52ff2a11cc49c9` |
| [actions/deploy-pages](https://github.com/actions/deploy-pages/releases/tag/v4.0.5) | v4.0.5 | `d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e` |

Both pinned action definitions use Node24, which GitHub-hosted runners support.
Updating an action pin requires checking the release, immutable action definition,
inputs, runner compatibility, and workflow validation again.

The final step uses `always()` to attempt uploading available prerequisite,
bootstrap, and CI logs, fresh verifier stdout/stderr, assurance JSON receipts,
and Duvet HTML/JSON reports. It enumerates those paths rather than uploading the
workspace, tool directory, fixture credentials, or compiled targets. Hidden files
are enabled only because the enumerated Duvet reports live below `.duvet`.
Artifacts expire after 14 days and have a run-ID/attempt-specific name.

On successful `main` pushes, CI also packages the generated HTML as `index.html`
and publishes it with GitHub Pages. The deployment environment and run summary
show the live URL. The Pages artifact contains only the Duvet report; detailed
logs and JSON receipts remain in the expiring workflow artifact.

An artifact may contain successful prerequisite receipts and failing verification
logs without a successful `ci.json`. Read the job conclusion and the individual
receipt's command/source/tool identities; artifact presence alone is not acceptance.
Upload failure fails the job. If checkout fails and no diagnostic files exist,
`if-no-files-found: warn` retains the original failure instead of fabricating
output. Runner loss, cancellation, job timeout, or artifact-service failure can
prevent upload despite `always()`; GitHub's step logs then remain the available
record. No concurrency rule cancels an older run automatically.

## Validation and progress

- [x] Confirm current official release tags and exact commit pins.
- [x] Inspect immutable action definitions and runner/prerequisite documentation.
- [x] Validate workflow YAML and GitHub Actions expressions with actionlint 1.7.12.
- [x] Validate each embedded Bash script with `bash -n`.
- [x] Independent workflow and artifact-scope review.
- [x] Publish the Duvet HTML report to GitHub Pages after successful `main` CI runs and show the site URL in the Actions summary.
- [ ] First GitHub-hosted bootstrap and complete assurance run.

Local actionlint validation used the official Linux amd64 archive with SHA-256
`8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`, matching
the upstream checksum manifest. ShellCheck was unavailable for that validation;
`bash -n` checks shell syntax, not shell behavior. The local workflow validation
is not a remote Actions run or a successful full formal build. No workflow was
pushed or dispatched during preparation. Remote execution remains pending until
the workflow is adopted and GitHub runs it; no authentication was requested just
to create or review these local files. Any formal-gate failure, including proof resource exhaustion, deliberately remains
a failed workflow rather than a reason to bypass verification.

To reproduce workflow linting with an available actionlint installation:

```sh
actionlint -shellcheck= .github/workflows/assurance.yml
```

Primary references: [workflow syntax and permissions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax),
[status functions including always](https://docs.github.com/en/actions/reference/workflows-and-actions/expressions#always),
[hosted runner platforms and limits](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
[Ubuntu 24.04 runner contents](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md),
[checkout's pinned action definition](https://github.com/actions/checkout/blob/3d3c42e5aac5ba805825da76410c181273ba90b1/action.yml),
and [upload-artifact's pinned action definition](https://github.com/actions/upload-artifact/blob/cf430e030ddbb5b0abf93d22962f4752f3646cd9/action.yml),
[GitHub Pages custom workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows),
[upload-pages-artifact's pinned action definition](https://github.com/actions/upload-pages-artifact/blob/fc324d3547104276b827a68afc52ff2a11cc49c9/action.yml),
and [deploy-pages' pinned action definition](https://github.com/actions/deploy-pages/blob/d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e/action.yml).
